//! Shell integration: default editor association and start-with-Windows.
//!
//! Windows 10+ protects the *default* file association (`UserChoice`) with a
//! per-user hash that only Explorer may write, so the app registers a ProgID and
//! adds itself to the "Open with" list; the user confirms the default app once
//! in Windows Settings. This is the only supported way and is what the README
//! documents.

use std::path::Path;

use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_NONE};
use winreg::{RegKey, RegValue};

const PROG_ID: &str = "NoteRust.txt";
const APP_NAME: &str = "NoteRust";
const APP_EXE: &str = "NoteRust.exe";
const EXTENSIONS: [&str; 4] = ["txt", "log", "md", "csv"];
/// Set when the user switches the shell integration off, so the next start does
/// not register the app again behind their back.
const OPT_OUT_KEY: &str = "Software\\NoteRust";
const OPT_OUT_VALUE: &str = "OpenWithDisabled";
/// Written by the NSIS installer after a fresh install so the app can ask once
/// whether to open Settings → Default apps for NoteRust.
const INSTALL_PROMPT_VALUE: &str = "ShowDefaultAppsPrompt";

/// Tells Explorer that file associations changed, so the "Open with" list and
/// the Default apps page pick up the new handler without a sign-out.
fn notify_shell() {
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHChangeNotify(
            event: i32,
            flags: u32,
            item1: *const std::ffi::c_void,
            item2: *const std::ffi::c_void,
        );
    }
    // SHCNE_ASSOCCHANGED | SHCNF_FLUSH
    unsafe { SHChangeNotify(0x0800_0000, 0x1000, std::ptr::null(), std::ptr::null()) }
}

fn register_open_with_progid(key: &RegKey, prog_id: &str) {
    // Windows stores OpenWithProgids entries as REG_NONE values.
    let _ = key.set_raw_value(
        prog_id,
        &RegValue {
            bytes: Vec::new(),
            vtype: REG_NONE,
        },
    );
}

fn hkcu() -> std::io::Result<RegKey> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software", KEY_READ | KEY_WRITE)
}

fn open_with_opted_out() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(OPT_OUT_KEY, KEY_READ)
        .and_then(|key| key.get_value::<String, _>(OPT_OUT_VALUE))
        .is_ok()
}

fn set_opt_out(disabled: bool) {
    let Ok((key, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(OPT_OUT_KEY) else {
        return;
    };
    if disabled {
        let _ = key.set_value(OPT_OUT_VALUE, &"1");
    } else {
        let _ = key.delete_value(OPT_OUT_VALUE);
    }
}

/// Adds NoteRust to Explorer's "Open with" list and to Settings → Default apps
/// for the supported extensions, without touching any existing default.
fn register_open_with(exe: &Path) -> Result<(), String> {
    let exe = exe
        .to_str()
        .ok_or_else(|| "مسار التطبيق غير صالح".to_string())?;
    let command = format!("\"{exe}\" \"%1\"");

    let classes = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey("Software\\Classes")
        .map_err(|err| err.to_string())?
        .0;

    let (prog, _) = classes
        .create_subkey(PROG_ID)
        .map_err(|err| err.to_string())?;
    prog.set_value("", &"ملف نصي (NoteRust)")
        .map_err(|err| err.to_string())?;
    let (icon, _) = prog
        .create_subkey("DefaultIcon")
        .map_err(|err| err.to_string())?;
    icon.set_value("", &format!("\"{exe}\",0"))
        .map_err(|err| err.to_string())?;
    let (shell, _) = prog
        .create_subkey("shell\\open\\command")
        .map_err(|err| err.to_string())?;
    shell
        .set_value("", &command)
        .map_err(|err| err.to_string())?;

    for ext in EXTENSIONS {
        let (key, _) = classes
            .create_subkey(format!(".{ext}\\OpenWithProgids"))
            .map_err(|err| err.to_string())?;
        register_open_with_progid(&key, PROG_ID);
        // The classic "Open with" list reads this key.
        if let Ok((list, _)) = classes.create_subkey(format!(".{ext}\\OpenWithList")) {
            let _ = list.set_value(APP_EXE, &"");
        }
    }

    // Entry shown by the Windows 11 app picker.
    if let Ok((app_key, _)) = classes.create_subkey(format!("Applications\\{APP_EXE}")) {
        let _ = app_key.set_value("FriendlyAppName", &APP_NAME);
        if let Ok((app_icon, _)) = app_key.create_subkey("DefaultIcon") {
            let _ = app_icon.set_value("", &format!("\"{exe}\",0"));
        }
        if let Ok((types, _)) = app_key.create_subkey("SupportedTypes") {
            for ext in EXTENSIONS {
                let _ = types.set_value(format!(".{ext}"), &"");
            }
        }
        if let Ok((app_cmd, _)) = app_key.create_subkey("shell\\open\\command") {
            let _ = app_cmd.set_value("", &command);
        }
    }

    // "Set defaults for applications" entry (Windows 10/11 Settings).
    if let Ok((caps, _)) =
        RegKey::predef(HKEY_CURRENT_USER).create_subkey("Software\\NoteRust\\Capabilities")
    {
        let _ = caps.set_value("ApplicationName", &APP_NAME);
        let _ = caps.set_value("ApplicationDescription", &"تطبيق ملاحظات عربي سريع وخفيف");
        let _ = caps.set_value("ApplicationIcon", &format!("\"{exe}\",0"));
        if let Ok((assoc, _)) = caps.create_subkey("FileAssociations") {
            for ext in EXTENSIONS {
                let _ = assoc.set_value(format!(".{ext}"), &PROG_ID);
            }
        }
    }
    if let Ok(registered) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Software\\RegisteredApplications", KEY_WRITE | KEY_READ)
        .or_else(|_| {
            RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey("Software\\RegisteredApplications")
                .map(|(key, _)| key)
        })
    {
        let _ = registered.set_value(APP_NAME, &"Software\\NoteRust\\Capabilities");
    }

    // Right-click verb in Explorer for every file type.
    if let Ok((verb, _)) = classes.create_subkey("*\\shell\\NoteRust") {
        let _ = verb.set_value("", &"فتح باستخدام NoteRust");
        let _ = verb.set_value("Icon", &format!("\"{exe}\",0"));
        if let Ok((verb_cmd, _)) = verb.create_subkey("command") {
            let _ = verb_cmd.set_value("", &command);
        }
    }

    notify_shell();
    Ok(())
}

/// Called on every start so an installed copy appears in Explorer's "Open with"
/// list without the user having to open Settings first.
pub fn ensure_open_with(exe: &Path) -> Result<(), String> {
    if open_with_opted_out() {
        return Ok(());
    }
    register_open_with(exe)
}

/// Registers (or removes) the app as an available editor for text files.
pub fn set_default_editor(exe: &Path, enabled: bool) -> Result<String, String> {
    let exe = exe
        .to_str()
        .ok_or_else(|| "مسار التطبيق غير صالح".to_string())?;

    if enabled {
        set_opt_out(false);
        register_open_with(Path::new(exe))?;

        let classes = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
            .map_err(|err| err.to_string())?;

        // Best effort: when Windows never stored a user choice, point the extension
        // itself at our ProgID so a double click opens NoteRust directly.
        for ext in EXTENSIONS {
            let user_choice = RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey_with_flags(
                    format!(
                        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\.{ext}\\UserChoice"
                    ),
                    KEY_READ,
                )
                .is_ok();
            if !user_choice {
                if let Ok((key, _)) = classes.create_subkey(format!(".{ext}")) {
                    let _ = key.set_value("", &PROG_ID);
                }
            }
        }

        Ok(
            "تم تسجيل NoteRust لملفات txt / log / md / csv، وأُضيف إلى «فتح باستخدام» و«التطبيقات الافتراضية». \n\
             ليصبح الافتراضي اضغط زر «جعله التطبيق الافتراضي…» ثم اختر .txt ← NoteRust ← تعيين كافتراضي."
                .to_string(),
        )
    } else {
        let classes = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
            .map_err(|err| err.to_string())?;
        let _ = classes.delete_subkey_all(PROG_ID);
        let _ = classes.delete_subkey_all(format!("Applications\\{APP_EXE}"));
        let _ = classes.delete_subkey_all("*\\shell\\NoteRust");
        if let Ok(registered) = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\RegisteredApplications", KEY_WRITE)
        {
            let _ = registered.delete_value(APP_NAME);
        }
        if let Ok(software) =
            RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software", KEY_WRITE)
        {
            let _ = software.delete_subkey_all("NoteRust\\Capabilities");
        }
        for ext in EXTENSIONS {
            if let Ok(key) =
                classes.open_subkey_with_flags(format!(".{ext}\\OpenWithProgids"), KEY_WRITE)
            {
                let _ = key.delete_value(PROG_ID);
            }
            if let Ok(key) =
                classes.open_subkey_with_flags(format!(".{ext}\\OpenWithList"), KEY_WRITE)
            {
                let _ = key.delete_value(APP_EXE);
            }
        }
        // Remember the choice so the next launch does not register it again.
        set_opt_out(true);
        notify_shell();
        Ok("تمت إزالة تسجيل المحرر".to_string())
    }
}

/// Adds or removes a `Run` entry so the app starts with Windows.
pub fn set_autostart(exe: &Path, enabled: bool) -> Result<(), String> {
    let key = hkcu()
        .and_then(|software| {
            software
                .open_subkey_with_flags("Microsoft\\Windows\\CurrentVersion\\Run", KEY_WRITE)
                .or_else(|_| {
                    software
                        .create_subkey("Microsoft\\Windows\\CurrentVersion\\Run")
                        .map(|(key, _)| key)
                })
        })
        .map_err(|err| err.to_string())?;

    if enabled {
        let value = format!("\"{}\"", exe.display());
        key.set_value(APP_NAME, &value)
            .map_err(|err| err.to_string())
    } else {
        match key.delete_value(APP_NAME) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

/// Opens Settings directly on the NoteRust page (Windows 11 21H2+); older
/// builds fall back to the generic Default apps page.
pub fn open_default_apps_page() -> Result<(), String> {
    std::process::Command::new("cmd")
        .args([
            "/C",
            "start",
            "",
            "ms-settings:defaultapps?registeredAppUser=NoteRust",
        ])
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// Reads and clears the "show the Default apps prompt" marker written by the
/// installer after a fresh install.
pub fn take_install_prompt() -> bool {
    let Ok(key) =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(OPT_OUT_KEY, KEY_READ | KEY_WRITE)
    else {
        return false;
    };
    let present = key.get_value::<u32, _>(INSTALL_PROMPT_VALUE).is_ok();
    if present {
        let _ = key.delete_value(INSTALL_PROMPT_VALUE);
    }
    present
}

/// Whether the app is currently registered in the "Open with" list for text files.
pub fn default_editor_registered() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            format!("Software\\Classes\\{PROG_ID}\\shell\\open\\command"),
            KEY_READ,
        )
        .and_then(|key| key.get_value::<String, _>(""))
        .is_ok()
}

/// Whether the start-with-Windows entry exists.
pub fn autostart_enabled() -> bool {
    hkcu()
        .and_then(|software| {
            software.open_subkey_with_flags("Microsoft\\Windows\\CurrentVersion\\Run", KEY_READ)
        })
        .and_then(|key| key.get_value::<String, _>(APP_NAME))
        .is_ok()
}

/// True when the WebView2 runtime is installed (Windows 10/11 usually have it).
pub fn webview2_installed() -> bool {
    const GUIDE: &str = "Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    let machine = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(format!("SOFTWARE\\WOW6432Node\\{GUIDE}"), KEY_READ)
        .or_else(|_| {
            RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(format!("SOFTWARE\\{GUIDE}"), KEY_READ)
        })
        .and_then(|key| key.get_value::<String, _>("pv"));
    if machine.is_ok() {
        return true;
    }
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(format!("Software\\{GUIDE}"), KEY_READ)
        .and_then(|key| key.get_value::<String, _>("pv"))
        .is_ok()
}
