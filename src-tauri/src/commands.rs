//! Tauri commands: the only bridge between the webview and the Rust layers.
//!
//! Every database call is forwarded to the database thread; file and dialog
//! work runs on the blocking pool. No command ever blocks the UI thread.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use noterust_core::{
    Category, MonthlySummary, Note, NoteSummary, NoteVersion, SearchHit, Snippet, Tag, TaskItem,
    TransactionItem,
};
use noterust_db::export::ImportStats;
use noterust_db::finance::TransactionDraft;
use noterust_db::snippets::SnippetDraft;
use noterust_db::tasks::TaskDraft;
use noterust_db::{Db, DbError, ListQuery, SortBy};
use noterust_fileio::{watch_path, TextDocument};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::paths::Paths;
use crate::watcher::{stop_watching as stop_file_watching, watch_note};

/// Saved window geometry before the small "sticky note" mode:
/// x, y, inner width, inner height and whether it was maximized.
pub type CompactGeometry = (i32, i32, u32, u32, bool);

/// Shared application state.
pub struct AppState {
    pub db: crate::db_worker::DbHandle,
    pub paths: Paths,
    pub app: AppHandle,
    /// Notes opened directly from disk that are being watched.
    pub watchers: Arc<Mutex<HashMap<i64, Arc<AtomicBool>>>>,
    /// Files passed on the command line / to a second instance.
    pub pending_open: Arc<Mutex<Vec<String>>>,
    /// Window geometry saved before entering the small "sticky note" mode.
    pub compact_restore: Arc<Mutex<Option<CompactGeometry>>>,
    /// Latest HTML playground document served to the preview frame.
    pub html_preview: Arc<Mutex<String>>,
}

impl AppState {
    pub fn stop_all_watchers(&self) {
        if let Ok(mut map) = self.watchers.lock() {
            map.clear();
        }
    }
}

/// Runs a database closure on the worker thread.
pub(crate) async fn db_call<T, F>(db: crate::db_worker::DbHandle, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut Db) -> Result<T, DbError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || db.call(f))
        .await
        .map_err(|err| err.to_string())?
}

async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|err| err.to_string())?
}

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListQueryDto {
    pub folder_id: Option<i64>,
    pub trash: bool,
    pub include_archived: bool,
    pub sort: String,
    pub limit: i64,
}

impl Default for ListQueryDto {
    fn default() -> Self {
        Self {
            folder_id: None,
            trash: false,
            include_archived: false,
            sort: "updated".to_string(),
            limit: 1000,
        }
    }
}

#[tauri::command]
pub async fn list_notes(
    state: State<'_, AppState>,
    query: ListQueryDto,
) -> Result<Vec<NoteSummary>, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::list(
            db.conn(),
            &ListQuery {
                folder_id: query.folder_id,
                trash: query.trash,
                include_archived: query.include_archived,
                sort: SortBy::from_name(&query.sort),
                limit: query.limit.clamp(1, 5000),
            },
        )
    })
    .await
}

#[tauri::command]
pub async fn get_note(state: State<'_, AppState>, id: i64) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::notes::get(db.conn(), id)).await
}

#[tauri::command]
pub async fn create_note(
    state: State<'_, AppState>,
    folder_id: Option<i64>,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::create(db.conn(), folder_id)
    })
    .await
}

#[tauri::command]
pub async fn update_note_content(
    state: State<'_, AppState>,
    id: i64,
    content: String,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::update_content(db.conn(), id, &content)
    })
    .await
}

#[tauri::command]
pub async fn update_note_title(
    state: State<'_, AppState>,
    id: i64,
    title: String,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::update_title(db.conn(), id, &title)
    })
    .await
}

#[tauri::command]
pub async fn move_note(
    state: State<'_, AppState>,
    id: i64,
    folder_id: Option<i64>,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::set_folder(db.conn(), id, folder_id)
    })
    .await
}

#[tauri::command]
pub async fn pin_note(state: State<'_, AppState>, id: i64, pinned: bool) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::set_pinned(db.conn(), id, pinned)
    })
    .await
}

#[tauri::command]
pub async fn archive_note(
    state: State<'_, AppState>,
    id: i64,
    archived: bool,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::set_archived(db.conn(), id, archived)
    })
    .await
}

#[tauri::command]
pub async fn set_note_color(
    state: State<'_, AppState>,
    id: i64,
    color: Option<String>,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::set_color(db.conn(), id, color.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn set_note_direction(
    state: State<'_, AppState>,
    id: i64,
    direction: String,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::set_direction(db.conn(), id, &direction)
    })
    .await
}

#[tauri::command]
pub async fn delete_note(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::notes::soft_delete(db.conn(), id)).await
}

#[tauri::command]
pub async fn restore_note(state: State<'_, AppState>, id: i64) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::notes::restore(db.conn(), id)).await
}

#[tauri::command]
pub async fn purge_note(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::notes::purge(db.conn(), id)).await
}

#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::notes::empty_trash(db.conn())).await
}

#[tauri::command]
pub async fn duplicate_note(state: State<'_, AppState>, id: i64) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::notes::duplicate(db.conn(), id)).await
}

#[tauri::command]
pub async fn search_notes(
    state: State<'_, AppState>,
    query: String,
    folder_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<SearchHit>, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::search(db.conn(), &query, folder_id, limit.unwrap_or(200))
    })
    .await
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub all: i64,
    pub trash: i64,
    pub archived: i64,
    pub tasks_open: i64,
}

#[tauri::command]
pub async fn counts(state: State<'_, AppState>) -> Result<Counts, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        Ok(Counts {
            all: noterust_db::notes::count(db.conn(), None)?,
            trash: noterust_db::notes::trash_count(db.conn())?,
            archived: noterust_db::notes::archived_count(db.conn())?,
            tasks_open: noterust_db::tasks::open_count(db.conn())?,
        })
    })
    .await
}

#[tauri::command]
pub async fn set_note_tags(
    state: State<'_, AppState>,
    id: i64,
    tags: Vec<String>,
) -> Result<Vec<Tag>, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::tags::set_for_note(db.conn(), id, &tags)
    })
    .await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    #[serde(flatten)]
    pub tag: Tag,
    pub count: i64,
}

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> Result<Vec<TagCount>, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        let rows = noterust_db::tags::all(db.conn())?;
        Ok(rows
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn delete_tag(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::tags::delete(db.conn(), id)).await
}

// ---------------------------------------------------------------------------
// Versions and locking
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn note_versions(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Vec<NoteVersion>, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::versions::list(db.conn(), id)).await
}

#[tauri::command]
pub async fn restore_version(state: State<'_, AppState>, version_id: i64) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        let note_id = noterust_db::versions::restore(db.conn(), version_id)?;
        noterust_db::notes::get(db.conn(), note_id)
    })
    .await
}

#[tauri::command]
pub async fn lock_note(
    state: State<'_, AppState>,
    id: i64,
    password: String,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::lock(db.conn(), id, &password)
    })
    .await
}

#[tauri::command]
pub async fn unlock_note(
    state: State<'_, AppState>,
    id: i64,
    password: String,
) -> Result<String, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::unlock(db.conn(), id, &password)
    })
    .await
}

#[tauri::command]
pub async fn save_locked_note(
    state: State<'_, AppState>,
    id: i64,
    password: String,
    content: String,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::save_locked(db.conn(), id, &password, &content)
    })
    .await
}

#[tauri::command]
pub async fn remove_lock(
    state: State<'_, AppState>,
    id: i64,
    password: String,
) -> Result<Note, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::unlock_permanently(db.conn(), id, &password)
    })
    .await
}

// ---------------------------------------------------------------------------
// Folders
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_folders(
    state: State<'_, AppState>,
) -> Result<Vec<noterust_core::FolderSummary>, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::folders::tree(db.conn())).await
}

#[tauri::command]
pub async fn create_folder(
    state: State<'_, AppState>,
    parent_id: Option<i64>,
    name: String,
    icon: Option<String>,
    color: Option<String>,
) -> Result<noterust_core::Folder, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::folders::create(
            db.conn(),
            parent_id,
            &name,
            icon.as_deref().unwrap_or("📁"),
            color.as_deref().unwrap_or("#e8b830"),
        )
    })
    .await
}

#[tauri::command]
pub async fn update_folder(
    state: State<'_, AppState>,
    id: i64,
    name: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    parent_id: Option<Option<i64>>,
) -> Result<noterust_core::Folder, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::folders::update(
            db.conn(),
            id,
            name.as_deref(),
            icon.as_deref(),
            color.as_deref(),
            parent_id,
        )
    })
    .await
}

#[tauri::command]
pub async fn delete_folder(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::folders::delete(db.conn(), id)).await
}

#[tauri::command]
pub async fn reorder_folder(state: State<'_, AppState>, id: i64, index: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::folders::reorder(db.conn(), id, index)
    })
    .await
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_tasks(
    state: State<'_, AppState>,
    include_done: bool,
) -> Result<Vec<TaskItem>, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::tasks::list(db.conn(), include_done)
    })
    .await
}

#[tauri::command]
pub async fn create_task(state: State<'_, AppState>, draft: TaskDraft) -> Result<TaskItem, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::tasks::create(db.conn(), &draft)).await
}

#[tauri::command]
pub async fn update_task(
    state: State<'_, AppState>,
    id: i64,
    title: Option<String>,
    description: Option<String>,
    due_at: Option<Option<i64>>,
    priority: Option<i64>,
    repeat_rule: Option<Option<String>>,
) -> Result<TaskItem, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::tasks::update(
            db.conn(),
            id,
            title.as_deref(),
            description.as_deref(),
            due_at,
            priority,
            repeat_rule,
        )
    })
    .await
}

#[tauri::command]
pub async fn complete_task(
    state: State<'_, AppState>,
    id: i64,
    done: bool,
) -> Result<TaskItem, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::tasks::set_done(db.conn(), id, done)
    })
    .await
}

#[tauri::command]
pub async fn delete_task(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::tasks::delete(db.conn(), id)).await
}

// ---------------------------------------------------------------------------
// Snippets (code + prompts)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_snippets(
    state: State<'_, AppState>,
    query: Option<String>,
    kind: Option<String>,
) -> Result<Vec<Snippet>, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::snippets::list(db.conn(), query.as_deref(), kind.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn create_snippet(
    state: State<'_, AppState>,
    draft: SnippetDraft,
) -> Result<Snippet, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::snippets::create(db.conn(), &draft)
    })
    .await
}

/// Fields the UI may change on an existing snippet.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SnippetPatch {
    pub title: Option<String>,
    pub kind: Option<String>,
    pub language: Option<String>,
    pub content: Option<String>,
    pub tags: Option<String>,
    pub pinned: Option<bool>,
}

#[tauri::command]
pub async fn update_snippet(
    state: State<'_, AppState>,
    id: i64,
    patch: SnippetPatch,
) -> Result<Snippet, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        let updated = noterust_db::snippets::update(
            db.conn(),
            id,
            patch.title.as_deref(),
            patch.kind.as_deref(),
            patch.language.as_deref(),
            patch.content.as_deref(),
            patch.tags.as_deref(),
        )?;
        match patch.pinned {
            Some(value) => noterust_db::snippets::set_pinned(db.conn(), id, value),
            None => Ok(updated),
        }
    })
    .await
}

#[tauri::command]
pub async fn delete_snippet(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::snippets::delete(db.conn(), id)).await
}

#[tauri::command]
pub async fn snippet_count(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::snippets::count(db.conn())).await
}

// ---------------------------------------------------------------------------
// Finance
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::finance::categories(db.conn())).await
}

#[tauri::command]
pub async fn create_category(
    state: State<'_, AppState>,
    name: String,
    kind: String,
    color: Option<String>,
) -> Result<Category, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::finance::create_category(
            db.conn(),
            &name,
            &kind,
            color.as_deref().unwrap_or("#8ab4f8"),
        )
    })
    .await
}

#[tauri::command]
pub async fn add_transaction(
    state: State<'_, AppState>,
    draft: TransactionDraft,
) -> Result<TransactionItem, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::finance::add_transaction(db.conn(), &draft)
    })
    .await
}

#[tauri::command]
pub async fn list_transactions(
    state: State<'_, AppState>,
    from: i64,
    to: i64,
    kind: Option<String>,
) -> Result<Vec<TransactionItem>, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::finance::list_transactions(db.conn(), from, to, kind.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn delete_transaction(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::finance::delete_transaction(db.conn(), id)
    })
    .await
}

#[tauri::command]
pub async fn monthly_summary(
    state: State<'_, AppState>,
    year: i32,
    month: u32,
) -> Result<MonthlySummary, String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::finance::monthly_summary(db.conn(), year, month)
    })
    .await
}

#[tauri::command]
pub async fn export_transactions_csv(
    state: State<'_, AppState>,
    year: i32,
    month: u32,
) -> Result<Option<String>, String> {
    let db = state.db.clone();
    let (from, to) = noterust_db::finance::month_range(year, month);
    let csv = db_call(db, move |db| {
        noterust_db::finance::export_csv(db.conn(), from, to)
    })
    .await?;

    blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title("حفظ ملف CSV")
            .set_file_name(format!("معاملات-{year}-{month:02}.csv"))
            .add_filter("CSV", &["csv"])
            .save_file()
        else {
            return Ok(None);
        };
        std::fs::write(&path, csv.as_bytes()).map_err(|err| err.to_string())?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataInfo {
    pub data_dir: String,
    pub db_path: String,
    pub backup_dir: String,
    pub portable: bool,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<BTreeMap<String, String>, String> {
    let db = state.db.clone();
    db_call(db, move |db| noterust_db::settings::all(db.conn())).await
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::settings::set(db.conn(), &key, &value)?;
        // Shortening the retention window should take effect immediately
        // instead of waiting for the next launch.
        if key == "trash_retention_days" {
            if let Ok(days) = value.trim().parse::<i64>() {
                let _ = noterust_db::notes::purge_expired(db.conn(), days);
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub fn data_info(state: State<'_, AppState>) -> DataInfo {
    DataInfo {
        data_dir: state.paths.data_dir.to_string_lossy().to_string(),
        db_path: state.paths.db_path.to_string_lossy().to_string(),
        backup_dir: state.paths.backup_dir.to_string_lossy().to_string(),
        portable: state.paths.portable,
    }
}

#[tauri::command]
pub async fn open_data_folder(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.paths.data_dir.clone();
    blocking(move || open_in_explorer(&dir)).await
}

fn open_in_explorer(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|err| err.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("غير مدعوم على هذا النظام".to_string())
    }
}

#[tauri::command]
pub async fn set_default_editor(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let _ = &state;
    blocking(move || crate::registry::set_default_editor(&exe, enabled)).await
}

/// Current shell-integration state, so the settings page reflects reality.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellState {
    pub default_editor: bool,
    pub autostart: bool,
}

#[tauri::command]
pub async fn open_default_apps() -> Result<(), String> {
    blocking(crate::registry::open_default_apps_page).await
}

#[tauri::command]
pub async fn shell_state() -> Result<ShellState, String> {
    blocking(|| {
        Ok(ShellState {
            default_editor: crate::registry::default_editor_registered(),
            autostart: crate::registry::autostart_enabled(),
        })
    })
    .await
}

#[tauri::command]
pub async fn set_autostart(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let _ = &state;
    blocking(move || crate::registry::set_autostart(&exe, enabled)).await
}

// ---------------------------------------------------------------------------
// Backups and import/export
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_at: i64,
}

#[tauri::command]
pub async fn backup_now(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.clone();
    let dir = state.paths.backup_dir.clone();
    let keep = db_call(db.clone(), |db| {
        noterust_db::settings::get_i64(db.conn(), "backup_retention", 14)
    })
    .await?;

    let path = db_call(db, move |db| {
        let path = noterust_db::backup::backup_now(db.conn(), &dir)?;
        noterust_db::backup::prune(&dir, keep)?;
        Ok(path)
    })
    .await?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupEntry>, String> {
    let dir = state.paths.backup_dir.clone();
    blocking(move || {
        let backups = noterust_db::backup::list_backups(&dir).map_err(|err| err.to_string())?;
        Ok(backups
            .into_iter()
            .map(|backup| BackupEntry {
                name: backup.name,
                path: backup.path,
                size: backup.size,
                created_at: backup.created_at,
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let db = state.db.clone();
    state.stop_all_watchers();
    db_call(db, move |db| {
        db.restore_from_file(Path::new(&path))?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn export_json(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let db = state.db.clone();
    let json = db_call(db, move |db| noterust_db::export::to_json(db.conn())).await?;
    blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title("تصدير كل البيانات إلى JSON")
            .set_file_name("noterust-export.json")
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return Ok(None);
        };
        std::fs::write(&path, json.as_bytes()).map_err(|err| err.to_string())?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn export_zip(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let db = state.db.clone();
    blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title("تصدير الملاحظات كملفات Markdown")
            .set_file_name("noterust-notes.zip")
            .add_filter("ZIP", &["zip"])
            .save_file()
        else {
            return Ok(None);
        };
        let target = path.clone();
        let written = db
            .call(move |db| noterust_db::export::write_zip(db.conn(), &target))
            .map_err(|err| err.to_string())?;
        Ok(Some(format!(
            "{} ملاحظة → {}",
            written,
            path.to_string_lossy()
        )))
    })
    .await
}

#[tauri::command]
pub async fn import_json_file(state: State<'_, AppState>) -> Result<Option<ImportStats>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("استيراد نسخة JSON")
        .add_filter("JSON", &["json"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let db = state.db.clone();
    let stats = db_call(db, move |db| {
        noterust_db::export::import_json(db.conn_mut(), &json)
    })
    .await?;
    Ok(Some(stats))
}

// ---------------------------------------------------------------------------
// External files
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedFile {
    pub note_id: i64,
    pub path: String,
    pub text: String,
    pub encoding: String,
    pub line_ending: String,
    pub bytes: u64,
    pub truncated: bool,
}

/// Maximum characters loaded into the editor; larger files are truncated and
/// flagged so the UI can refuse to save over them.
const MAX_EDITABLE_CHARS: usize = 4_000_000;

fn load_file(path: &Path) -> Result<OpenedFilePayload, String> {
    let document =
        TextDocument::open_capped(path, MAX_EDITABLE_CHARS).map_err(|err| err.to_string())?;
    Ok(OpenedFilePayload {
        text: document.text(),
        encoding: document.encoding.clone(),
        line_ending: document.line_ending.clone(),
        bytes: document.bytes,
        truncated: document.truncated,
    })
}

struct OpenedFilePayload {
    text: String,
    encoding: String,
    line_ending: String,
    bytes: u64,
    truncated: bool,
}

#[tauri::command]
pub async fn open_path(state: State<'_, AppState>, path: String) -> Result<OpenedFile, String> {
    let file_path = PathBuf::from(&path);
    let payload = blocking({
        let file_path = file_path.clone();
        move || load_file(&file_path)
    })
    .await?;

    let db = state.db.clone();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ملف")
        .to_string();
    let normalized_path = file_path.to_string_lossy().to_string();
    let encoding = payload.encoding.clone();
    let line_ending = payload.line_ending.clone();
    let text = payload.text.clone();
    let source_path = normalized_path.clone();

    let note = db_call(db, move |db| {
        match noterust_db::notes::by_source_path(db.conn(), &source_path)? {
            Some(existing) => {
                noterust_db::notes::set_source(
                    db.conn(),
                    existing.id,
                    Some(&source_path),
                    Some(&encoding),
                    Some(&line_ending),
                )?;
                if !existing.is_locked {
                    noterust_db::notes::update_content(db.conn(), existing.id, &text)
                } else {
                    Ok(existing)
                }
            }
            None => {
                let note = noterust_db::export::import_text_note(
                    db.conn(),
                    None,
                    &title,
                    &text,
                    Some(&source_path),
                )?;
                noterust_db::notes::set_source(
                    db.conn(),
                    note.id,
                    Some(&source_path),
                    Some(&encoding),
                    Some(&line_ending),
                )
            }
        }
    })
    .await?;

    // Watch the file so external edits can be reported to the user.
    watch_note(&state, note.id, &file_path);

    Ok(OpenedFile {
        note_id: note.id,
        path: normalized_path,
        text: payload.text,
        encoding: payload.encoding,
        line_ending: payload.line_ending,
        bytes: payload.bytes,
        truncated: payload.truncated,
    })
}

#[tauri::command]
pub async fn open_file_dialog(state: State<'_, AppState>) -> Result<Option<OpenedFile>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("فتح ملف")
        .add_filter(
            "كل الملفات النصية",
            &[
                "txt", "log", "md", "json", "xml", "csv", "ini", "cfg", "bat", "ps1", "sh", "rs",
                "html", "css", "js", "ts", "sql",
            ],
        )
        .add_filter("كل الملفات", &["*"])
        .pick_file()
    else {
        return Ok(None);
    };
    open_path(state, path.to_string_lossy().to_string())
        .await
        .map(Some)
}

#[tauri::command]
pub async fn open_startup_files(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut guard = state
        .pending_open
        .lock()
        .map_err(|_| "تعذر قراءة الملفات المعلقة".to_string())?;
    let files = std::mem::take(&mut *guard);
    Ok(files)
}

#[tauri::command]
pub async fn save_external_file(
    state: State<'_, AppState>,
    id: i64,
    text: String,
) -> Result<Note, String> {
    let db = state.db.clone();
    let note = db_call(db.clone(), move |db| noterust_db::notes::get(db.conn(), id)).await?;
    let Some(source) = note.source_path.clone() else {
        return Err("الملاحظة غير مرتبطة بملف على القرص".to_string());
    };
    let encoding = note.encoding.clone().unwrap_or_else(|| "UTF-8".to_string());
    let line_ending = note.line_ending.clone().unwrap_or_else(|| "LF".to_string());

    let (text_for_disk, encoding_clone, ending_clone) =
        (text.clone(), encoding.clone(), line_ending.clone());
    blocking(move || {
        noterust_fileio::write_atomic(
            Path::new(&source),
            &text_for_disk,
            &encoding_clone,
            &ending_clone,
        )
        .map_err(|err| err.to_string())
    })
    .await?;

    db_call(db, move |db| {
        noterust_db::notes::update_content(db.conn(), id, &text)
    })
    .await
}

#[tauri::command]
pub async fn save_file_as(
    text: String,
    suggested_name: Option<String>,
    encoding: Option<String>,
    line_ending: Option<String>,
) -> Result<Option<String>, String> {
    blocking(move || {
        let dialog = rfd::FileDialog::new().set_title("حفظ باسم");
        let dialog = match suggested_name {
            Some(name) => dialog.set_file_name(name),
            None => dialog.set_file_name("ملاحظة.txt"),
        };
        let Some(path) = dialog
            .add_filter("نص", &["txt"])
            .add_filter("Markdown", &["md"])
            .add_filter("كل الملفات", &["*"])
            .save_file()
        else {
            return Ok(None);
        };
        noterust_fileio::write_atomic(
            &path,
            &text,
            encoding.as_deref().unwrap_or("UTF-8"),
            line_ending.as_deref().unwrap_or("CRLF"),
        )
        .map_err(|err| err.to_string())?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn export_note_file(
    state: State<'_, AppState>,
    id: i64,
    format: String,
) -> Result<Option<String>, String> {
    let db = state.db.clone();
    let note = db_call(db, move |db| noterust_db::notes::get(db.conn(), id)).await?;
    let markdown = format.eq_ignore_ascii_case("md");
    let content = if markdown {
        note.content.clone()
    } else {
        noterust_core::strip_markdown(&note.content)
    };
    let suggested = format!(
        "{}.{}",
        noterust_core::truncate_chars(
            if note.title.trim().is_empty() {
                "ملاحظة"
            } else {
                note.title.trim()
            },
            60
        ),
        if markdown { "md" } else { "txt" }
    );
    save_file_as(
        content,
        Some(suggested),
        Some("UTF-8".into()),
        Some("CRLF".into()),
    )
    .await
}

#[tauri::command]
pub async fn import_dialog(
    state: State<'_, AppState>,
    folder_id: Option<i64>,
) -> Result<Option<ImportStats>, String> {
    let Some(paths) = rfd::FileDialog::new()
        .set_title("استيراد ملفات نصية")
        .add_filter(
            "نص",
            &[
                "txt", "log", "md", "json", "xml", "csv", "ini", "cfg", "bat", "ps1",
            ],
        )
        .add_filter("كل الملفات", &["*"])
        .pick_files()
    else {
        return Ok(None);
    };
    import_paths(
        state,
        folder_id,
        paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
    )
    .await
    .map(Some)
}

#[tauri::command]
pub async fn import_folder_dialog(
    state: State<'_, AppState>,
    folder_id: Option<i64>,
) -> Result<Option<ImportStats>, String> {
    let Some(dir) = rfd::FileDialog::new()
        .set_title("استيراد مجلد كامل")
        .pick_folder()
    else {
        return Ok(None);
    };
    let files = noterust_fileio::collect_text_files(&dir)
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    noterust_fileio::TEXT_EXTENSIONS
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(ext))
                })
                .unwrap_or(false)
        })
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    import_paths(state, folder_id, files).await.map(Some)
}

async fn import_paths(
    state: State<'_, AppState>,
    folder_id: Option<i64>,
    paths: Vec<String>,
) -> Result<ImportStats, String> {
    let db = state.db.clone();
    let payload: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    blocking(move || {
        let mut stats = ImportStats::default();
        for path in payload {
            let Ok(document) = TextDocument::open_capped(&path, MAX_EDITABLE_CHARS) else {
                continue;
            };
            let text = document.text();
            let title = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("ملف")
                .to_string();
            let source = path.to_string_lossy().to_string();
            db.call(move |db| {
                noterust_db::export::import_text_note(
                    db.conn(),
                    folder_id,
                    &title,
                    &text,
                    Some(&source),
                )
            })?;
            stats.notes += 1;
        }
        Ok(stats)
    })
    .await
}

#[tauri::command]
pub async fn stop_watching(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    stop_file_watching(&state, id);
    Ok(())
}

/// Reloads a file that changed on disk and returns its new contents.
#[tauri::command]
pub async fn reload_external_file(state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let db = state.db.clone();
    let note = db_call(db.clone(), move |db| noterust_db::notes::get(db.conn(), id)).await?;
    let Some(source) = note.source_path else {
        return Err("الملاحظة غير مرتبطة بملف".to_string());
    };
    let payload = blocking(move || load_file(Path::new(&source))).await?;
    let text = payload.text.clone();
    db_call(db, move |db| {
        noterust_db::notes::update_content(db.conn(), id, &text)
    })
    .await?;
    Ok(payload.text)
}

#[tauri::command]
pub async fn discard_external_link(state: State<'_, AppState>, id: i64) -> Result<Note, String> {
    stop_file_watching(&state, id);
    let db = state.db.clone();
    db_call(db, move |db| {
        noterust_db::notes::set_source(db.conn(), id, None, None, None)
    })
    .await
}

/// Emits the current window title, used for the taskbar label.
#[tauri::command]
pub fn set_window_title(window: tauri::Window, title: String) -> Result<(), String> {
    window.set_title(&title).map_err(|err| err.to_string())
}

/// Keeps the window above every other program on the desktop.
#[tauri::command]
pub fn set_always_on_top(window: tauri::Window, value: bool) -> Result<(), String> {
    window
        .set_always_on_top(value)
        .map_err(|err| err.to_string())
}

/// Turns the window into a small, light "sticky note" panel: only the open
/// note, always on top, parked at the top-right corner of the current monitor.
#[tauri::command]
pub fn set_compact_mode(
    state: State<'_, AppState>,
    window: tauri::Window,
    value: bool,
) -> Result<(), String> {
    if value {
        // Size and position are saved as inner geometry because `set_size`
        // applies the client area, which restores the box exactly.
        let size = window.inner_size().map_err(|err| err.to_string())?;
        let position = window.outer_position().map_err(|err| err.to_string())?;
        let maximized = window.is_maximized().unwrap_or(false);
        if let Ok(mut restore) = state.compact_restore.lock() {
            *restore = Some((position.x, position.y, size.width, size.height, maximized));
        }
        let _ = window.unmaximize();
        window
            .set_min_size(Some(tauri::LogicalSize::new(320.0, 260.0)))
            .map_err(|err| err.to_string())?;
        window
            .set_size(tauri::LogicalSize::new(430.0, 620.0))
            .map_err(|err| err.to_string())?;
        window
            .set_always_on_top(true)
            .map_err(|err| err.to_string())?;

        let monitor = match window.current_monitor() {
            Ok(Some(monitor)) => Some(monitor),
            _ => window.primary_monitor().ok().flatten(),
        };
        if let Some(monitor) = monitor {
            let width = window.outer_size().map(|size| size.width).unwrap_or(430) as i32;
            let x = monitor.position().x + monitor.size().width as i32 - width - 36;
            let y = monitor.position().y + 48;
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        }
        let _ = window.set_focus();
        return Ok(());
    }

    window.set_always_on_top(false).ok();
    window
        .set_min_size(Some(tauri::LogicalSize::new(900.0, 560.0)))
        .map_err(|err| err.to_string())?;
    let restore = state
        .compact_restore
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some((x, y, width, height, maximized)) = restore {
        window
            .set_size(tauri::PhysicalSize::new(width, height))
            .map_err(|err| err.to_string())?;
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|err| err.to_string())?;
        if maximized {
            let _ = window.maximize();
        }
    }
    Ok(())
}

/// Extensions accepted for pasted images, normalized (`jpeg` becomes `jpg`).
fn image_extension(raw: &str) -> Result<&'static str, String> {
    match raw.trim().trim_start_matches('.').to_ascii_lowercase().as_str() {
        "png" => Ok("png"),
        "jpg" | "jpeg" => Ok("jpg"),
        "gif" => Ok("gif"),
        "webp" => Ok("webp"),
        "bmp" => Ok("bmp"),
        "svg" => Ok("svg"),
        _ => Err("صيغة الصورة غير مدعومة".to_string()),
    }
}

fn image_mime(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    }
}

/// Only plain file names inside the assets folder are addressable.
fn attachment_name(name: &str) -> Result<&str, String> {
    let valid = !name.is_empty()
        && !name.contains(['/', '\\', ':'])
        && name != ".."
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'));
    if valid {
        Ok(name)
    } else {
        Err("اسم الصورة غير صالح".to_string())
    }
}

const MAX_ATTACHMENT_BYTES: usize = 30 * 1024 * 1024;

/// Stores a pasted image (Base64) in the assets folder and returns its name,
/// which is what the note text references as `noteimg://<name>`.
fn write_attachment(paths: &Paths, data: &str, ext: &str) -> Result<String, String> {
    let ext = image_extension(ext)?;
    let bytes = crate::base64::decode(data)?;
    if bytes.is_empty() {
        return Err("الصورة فارغة".to_string());
    }
    if bytes.len() > MAX_ATTACHMENT_BYTES {
        return Err("الصورة أكبر من ٣٠ ميجابايت".to_string());
    }
    let dir = &paths.assets_dir;
    std::fs::create_dir_all(dir).map_err(|err| format!("تعذر إنشاء مجلد الصور: {err}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let mut name = format!("img-{stamp}.{ext}");
    let mut attempt = 1;
    while dir.join(&name).exists() {
        attempt += 1;
        name = format!("img-{stamp}-{attempt}.{ext}");
    }
    std::fs::write(dir.join(&name), &bytes).map_err(|err| format!("تعذر حفظ الصورة: {err}"))?;
    Ok(name)
}

/// Reads a stored image back as a `data:` URL for the Markdown preview.
fn load_attachment(paths: &Paths, name: &str) -> Result<String, String> {
    let name = attachment_name(name)?;
    let path = paths.assets_dir.join(name);
    let bytes = std::fs::read(&path).map_err(|err| format!("تعذر قراءة الصورة: {err}"))?;
    Ok(format!(
        "data:{};base64,{}",
        image_mime(name),
        crate::base64::encode(&bytes)
    ))
}

/// Writes a small state file (whiteboard, HTML lab) atomically: a temporary
/// file is written first, so a crash mid-write never truncates the original.
fn write_data_file(paths: &Paths, name: &str, data: &str, label: &str) -> Result<(), String> {
    std::fs::create_dir_all(&paths.data_dir)
        .map_err(|err| format!("تعذر إنشاء مجلد البيانات: {err}"))?;
    let target = paths.data_dir.join(name);
    let temp = paths.data_dir.join(format!("{name}.tmp"));
    std::fs::write(&temp, data).map_err(|err| format!("تعذر حفظ {label}: {err}"))?;
    std::fs::rename(&temp, &target).map_err(|err| format!("تعذر حفظ {label}: {err}"))?;
    Ok(())
}

fn read_data_file(
    paths: &Paths,
    name: &str,
    label: &str,
) -> Result<Option<String>, String> {
    match std::fs::read_to_string(paths.data_dir.join(name)) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("تعذر قراءة {label}: {err}")),
    }
}

fn write_board(paths: &Paths, data: &str) -> Result<(), String> {
    write_data_file(paths, "board.json", data, "السبورة")
}

fn read_board(paths: &Paths) -> Result<Option<String>, String> {
    read_data_file(paths, "board.json", "السبورة")
}

/// Auto-saved whiteboard state (same JSON the board keeps in localStorage).
#[tauri::command]
pub fn save_board(state: State<'_, AppState>, data: String) -> Result<(), String> {
    write_board(&state.paths, &data)
}

#[tauri::command]
pub fn load_board(state: State<'_, AppState>) -> Result<Option<String>, String> {
    read_board(&state.paths)
}

fn write_html_lab(paths: &Paths, data: &str) -> Result<(), String> {
    write_data_file(paths, "html-lab.html", data, "معمل HTML")
}

fn read_html_lab(paths: &Paths) -> Result<Option<String>, String> {
    read_data_file(paths, "html-lab.html", "معمل HTML")
}

/// Auto-saved HTML playground document.
#[tauri::command]
pub fn save_html_lab(state: State<'_, AppState>, data: String) -> Result<(), String> {
    write_html_lab(&state.paths, &data)
}

#[tauri::command]
pub fn load_html_lab(state: State<'_, AppState>) -> Result<Option<String>, String> {
    read_html_lab(&state.paths)
}

/// Stores the document the `htmlpreview://` protocol serves to the preview frame.
/// It has to live outside the main document's CSP, which is why the frame loads
/// it from a separate origin instead of using `srcdoc`.
#[tauri::command]
pub fn set_html_preview(state: State<'_, AppState>, html: String) -> Result<(), String> {
    let mut guard = state
        .html_preview
        .lock()
        .map_err(|_| "تعذر تحديث المعاينة".to_string())?;
    *guard = html;
    Ok(())
}

#[tauri::command]
pub fn save_attachment(
    state: State<'_, AppState>,
    data: String,
    ext: String,
) -> Result<String, String> {
    write_attachment(&state.paths, &data, &ext)
}

#[tauri::command]
pub fn read_attachment(state: State<'_, AppState>, name: String) -> Result<String, String> {
    load_attachment(&state.paths, &name)
}

#[tauri::command]
pub async fn refresh_daily_backup(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.clone();
    let dir = state.paths.backup_dir.clone();
    let keep = db_call(db.clone(), |db| {
        noterust_db::settings::get_i64(db.conn(), "backup_retention", 14)
    })
    .await?;
    let _ = db_call(db, move |db| {
        noterust_db::backup::auto_daily(db.conn(), &dir, keep)?;
        Ok(())
    })
    .await;
    Ok(())
}

/// Polls a file watcher on a background thread, one per external note.
pub fn spawn_watch_thread(state: &AppState, note_id: i64, path: PathBuf, stop: Arc<AtomicBool>) {
    let app = state.app.clone();
    std::thread::Builder::new()
        .name(format!("watch-{note_id}"))
        .spawn(move || {
            let Ok(watcher) = watch_path(&path) else {
                return;
            };
            while !stop.load(Ordering::Relaxed) {
                if watcher.next_change(Duration::from_millis(400)).is_some() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Err(err) = app.emit("file-changed", note_id) {
                        tracing::warn!("failed to emit file-changed: {err}");
                    }
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod attachment_tests {
    use super::{
        load_attachment, read_board, read_html_lab, write_attachment, write_board, write_html_lab,
    };
    use crate::paths::Paths;

    fn temp_paths(tag: &str) -> Paths {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!("noterust-{tag}-{stamp}"));
        Paths {
            db_path: dir.join("noterust.db"),
            backup_dir: dir.join("backups"),
            log_dir: dir.join("logs"),
            assets_dir: dir.join("assets"),
            data_dir: dir,
            portable: false,
        }
    }

    #[test]
    fn round_trip_attachment() {
        let paths = temp_paths("attach");
        let data = crate::base64::encode(b"png-bytes");
        let name = write_attachment(&paths, &data, "png").expect("write");
        assert!(name.ends_with(".png"), "{name}");
        let url = load_attachment(&paths, &name).expect("read");
        assert_eq!(url, format!("data:image/png;base64,{data}"));
        let _ = std::fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn board_round_trip_leaves_no_temp_file() {
        let paths = temp_paths("board");
        assert!(read_board(&paths).expect("read empty").is_none());
        write_board(&paths, "{\"shapes\":[]}").expect("write");
        assert_eq!(
            read_board(&paths).expect("read").as_deref(),
            Some("{\"shapes\":[]}")
        );
        assert!(!paths.data_dir.join("board.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn html_lab_round_trip() {
        let paths = temp_paths("html-lab");
        assert!(read_html_lab(&paths).expect("read empty").is_none());
        write_html_lab(&paths, "<html>تجربة</html>").expect("write");
        assert_eq!(
            read_html_lab(&paths).expect("read").as_deref(),
            Some("<html>تجربة</html>")
        );
        assert!(paths.data_dir.join("html-lab.html").exists());
        assert!(!paths.data_dir.join("html-lab.html.tmp").exists());
        let _ = std::fs::remove_dir_all(&paths.data_dir);
    }

    #[test]
    fn rejects_bad_names_and_formats() {
        let paths = temp_paths("attach-bad");
        assert!(write_attachment(&paths, "AAAA", "exe").is_err());
        assert!(write_attachment(&paths, "!!!!", "png").is_err());
        assert!(load_attachment(&paths, "../noterust.db").is_err());
        assert!(load_attachment(&paths, "missing.png").is_err());
        let _ = std::fs::remove_dir_all(&paths.data_dir);
    }
}
