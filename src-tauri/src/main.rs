#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! NoteRust — Arabic desktop notes app.
//!
//! Startup order matters: the WebView2 runtime is verified first (with a clear
//! message instead of an obscure panic), then the data directories and the
//! database are prepared, and only then is the window created.

mod ai;
mod base64;
mod commands;
mod db_worker;
mod paths;
#[cfg(windows)]
mod registry;
mod watcher;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager};
use tracing_subscriber::EnvFilter;

use commands::AppState;
use db_worker::DbHandle;
use paths::Paths;

/// Files passed on the command line (e.g. "Open with"), ignoring flags.
fn file_args() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .filter(|arg| std::path::Path::new(arg).is_file())
        .collect()
}

fn init_tracing(paths: &Paths) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file = std::fs::File::create(paths.log_dir.join("noterust.log")).ok();
    match file {
        Some(file) => {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(Mutex::new(file))
                .with_ansi(false)
                .try_init();
        }
        None => {
            let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
        }
    }
}

/// Shows a native error box (used when the app cannot start at all).
fn fatal(title: &str, message: &str) {
    tracing::error!("{title}: {message}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn main() {
    let paths = Paths::resolve();
    if let Err(err) = paths.ensure() {
        fatal(
            "NoteRust",
            &format!(
                "تعذر إنشاء مجلد البيانات:\n{}\n\n{err}",
                paths.data_dir.display()
            ),
        );
        return;
    }
    init_tracing(&paths);
    tracing::info!(dir = %paths.data_dir.display(), portable = paths.portable, "starting");

    #[cfg(windows)]
    if !registry::webview2_installed() {
        fatal(
            "مطلوب WebView2",
            "تطبيق NoteRust يحتاج مكوّن Microsoft Edge WebView2 Runtime.\n\n\
             حمّله من: https://developer.microsoft.com/microsoft-edge/webview2/\n\
             (وهو مثبّت افتراضياً في ويندوز 10 و11).",
        );
        return;
    }

    let db = match noterust_db::Db::open(&paths.db_path) {
        Ok(db) => db,
        Err(err) => {
            fatal("NoteRust", &format!("تعذر فتح قاعدة البيانات:\n{err}"));
            return;
        }
    };

    // Housekeeping that must never block startup for long.
    let retention =
        noterust_db::settings::get_i64(db.conn(), "trash_retention_days", 30).unwrap_or(30);
    if let Ok(removed) = noterust_db::notes::purge_expired(db.conn(), retention) {
        if removed > 0 {
            tracing::info!("purged {removed} expired notes from the trash");
        }
    }
    let backup_retention =
        noterust_db::settings::get_i64(db.conn(), "backup_retention", 14).unwrap_or(14);
    match noterust_db::backup::auto_daily(db.conn(), &paths.backup_dir, backup_retention) {
        Ok(Some(path)) => tracing::info!(path = %path.display(), "daily backup created"),
        Ok(None) => {}
        Err(err) => tracing::warn!("daily backup failed: {err}"),
    }

    let handle = DbHandle::spawn(db);
    let pending: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(file_args()));
    let pending_for_plugin = pending.clone();
    let paths_for_state = paths.clone();

    let app = tauri::Builder::default()
        // Preview frame origin: the app's CSP would otherwise be inherited by
        // `srcdoc` and block the page's own scripts, fonts and images.
        .register_uri_scheme_protocol("htmlpreview", |ctx, _request| {
            let html = ctx
                .app_handle()
                .try_state::<AppState>()
                .and_then(|state| state.html_preview.lock().map(|code| code.clone()).ok())
                .unwrap_or_default();
            match tauri::http::Response::builder()
                .header(
                    tauri::http::header::CONTENT_TYPE,
                    "text/html; charset=utf-8",
                )
                .header(tauri::http::header::CACHE_CONTROL, "no-store")
                .body(html.into_bytes())
            {
                Ok(response) => response,
                Err(_) => tauri::http::Response::new(Vec::new()),
            }
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // Signed updates from the GitHub releases feed (see tauri.conf.json).
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(
            move |app, argv, _cwd| {
                // The forwarded command line starts with the executable itself;
                // treating it as a document would try to open the .exe as text.
                let exe = std::env::current_exe().ok();
                let files: Vec<String> = argv
                    .into_iter()
                    .filter(|arg| !arg.starts_with('-'))
                    .filter(|arg| {
                        let path = std::path::Path::new(arg);
                        path.is_file() && exe.as_deref() != Some(path)
                    })
                    .collect();
                if !files.is_empty() {
                    if let Ok(mut guard) = pending_for_plugin.lock() {
                        guard.extend(files);
                    }
                    let _ = app.emit("open-files", ());
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            },
        ))
        .setup(move |app| {
            let state = AppState {
                db: handle.clone(),
                paths: paths_for_state.clone(),
                app: app.handle().clone(),
                watchers: Arc::new(Mutex::new(HashMap::new())),
                pending_open: pending.clone(),
                compact_restore: Arc::new(Mutex::new(None)),
                html_preview: Arc::new(Mutex::new(String::new())),
            };
            app.manage(state);

            // Make the app visible in Explorer's "Open with" picker and in
            // Settings → Default apps. This also runs for the portable build,
            // otherwise a portable copy can never be chosen as the .txt editor.
            #[cfg(windows)]
            match std::env::current_exe() {
                Ok(exe) => {
                    if let Err(err) = crate::registry::ensure_open_with(&exe) {
                        tracing::error!("shell registration failed: {err}");
                    }
                }
                Err(err) => tracing::error!("current_exe failed: {err}"),
            }

            // Frameless window: the UI draws its own title bar.
            match app.get_webview_window("main") {
                Some(window) => match window.set_decorations(false) {
                    Ok(()) => tracing::info!("main window set undecorated"),
                    Err(err) => tracing::error!("set_decorations failed: {err}"),
                },
                None => tracing::error!("main window not available in setup"),
            }
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                if let Err(err) = window.set_shadow(true) {
                    tracing::error!("set_shadow failed: {err}");
                }
            }

            #[cfg(windows)]
            maybe_prompt_for_default_apps();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_notes,
            commands::get_note,
            commands::create_note,
            commands::update_note_content,
            commands::update_note_title,
            commands::move_note,
            commands::pin_note,
            commands::archive_note,
            commands::set_note_color,
            commands::set_note_direction,
            commands::delete_note,
            commands::restore_note,
            commands::purge_note,
            commands::empty_trash,
            commands::duplicate_note,
            commands::search_notes,
            commands::counts,
            commands::set_note_tags,
            commands::list_tags,
            commands::delete_tag,
            commands::note_versions,
            commands::restore_version,
            commands::lock_note,
            commands::unlock_note,
            commands::save_locked_note,
            commands::remove_lock,
            commands::list_folders,
            commands::create_folder,
            commands::update_folder,
            commands::delete_folder,
            commands::reorder_folder,
            commands::list_tasks,
            commands::create_task,
            commands::update_task,
            commands::complete_task,
            commands::delete_task,
            commands::list_snippets,
            commands::create_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            commands::snippet_count,
            commands::list_categories,
            commands::create_category,
            commands::add_transaction,
            commands::list_transactions,
            commands::delete_transaction,
            commands::monthly_summary,
            commands::export_transactions_csv,
            commands::get_settings,
            commands::set_setting,
            commands::data_info,
            commands::open_data_folder,
            commands::set_default_editor,
            commands::set_autostart,
            commands::shell_state,
            commands::open_default_apps,
            commands::backup_now,
            commands::list_backups,
            commands::restore_backup,
            commands::export_json,
            commands::export_zip,
            commands::import_json_file,
            commands::open_path,
            commands::open_file_dialog,
            commands::open_startup_files,
            commands::save_external_file,
            commands::save_file_as,
            commands::export_note_file,
            commands::import_dialog,
            commands::import_folder_dialog,
            commands::stop_watching,
            commands::reload_external_file,
            commands::discard_external_link,
            commands::set_window_title,
            commands::set_always_on_top,
            commands::set_compact_mode,
            commands::save_board,
            commands::load_board,
            commands::save_html_lab,
            commands::load_html_lab,
            commands::set_html_preview,
            commands::save_attachment,
            commands::read_attachment,
            commands::delete_attachment,
            commands::restore_attachment,
            commands::refresh_daily_backup,
            ai::ai_connections,
            ai::ai_save_connection,
            ai::ai_delete_connection,
            ai::ai_test_connection,
            ai::ai_chat,
        ])
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|_app, _event| {}),
        Err(err) => fatal("NoteRust", &format!("تعذر تشغيل التطبيق:\n{err}")),
    }
}

/// After a fresh install the NSIS installer leaves a marker behind. Ask once
/// whether the user wants to open the per-app Default Apps page; Windows does
/// not allow an app to change the default handler silently.
#[cfg(windows)]
fn maybe_prompt_for_default_apps() {
    if !crate::registry::take_install_prompt() {
        return;
    }
    std::thread::spawn(|| {
        // Let the main window paint before the modal box appears.
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let choice = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("NoteRust")
            .set_description(
                "هل تريد تعيين NoteRust كتطبيق افتراضي لملفات النصوص (.txt) بدل Notepad؟\n\n\
                 سيفتح ويندوز صفحة NoteRust في «التطبيقات الافتراضية». اختر .txt ثم NoteRust \
                 واضغط «تعيين كافتراضي».\n\n\
                 ويندوز يمنع أي تطبيق من تغيير هذا الاختيار بصمت، لذا هذه الخطوة مرة واحدة فقط.",
            )
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        if choice == rfd::MessageDialogResult::Yes {
            let _ = crate::registry::open_default_apps_page();
        }
    });
}
