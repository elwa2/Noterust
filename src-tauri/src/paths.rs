//! Data locations: `AppData` by default, next to the exe in portable mode.

use std::path::{Path, PathBuf};

use tracing::warn;

/// Where the app keeps its database, backups and logs.
#[derive(Debug, Clone)]
pub struct Paths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub log_dir: PathBuf,
    /// True when a `portable.flag` file sits next to the executable.
    pub portable: bool,
}

impl Paths {
    /// Resolves the data directory. A `portable.flag` file next to the exe
    /// switches storage to `<exe dir>/NoteRust-Data`, which makes the whole app
    /// self-contained on a USB stick.
    pub fn resolve() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        let portable = exe_dir
            .as_ref()
            .map(|dir| dir.join("portable.flag").is_file())
            .unwrap_or(false);

        let data_dir = match (&exe_dir, portable) {
            (Some(dir), true) => dir.join("NoteRust-Data"),
            _ => default_app_dir(),
        };

        Self::from_data_dir(data_dir, portable)
    }

    fn from_data_dir(data_dir: PathBuf, portable: bool) -> Self {
        Self {
            db_path: data_dir.join("noterust.db"),
            backup_dir: data_dir.join("backups"),
            log_dir: data_dir.join("logs"),
            data_dir,
            portable,
        }
    }

    /// Creates the directories the app writes to.
    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.backup_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }
}

fn default_app_dir() -> PathBuf {
    match directories::ProjectDirs::from("io", "noterust", "NoteRust") {
        Some(dirs) => dirs.data_dir().to_path_buf(),
        None => {
            warn!("تعذر تحديد مجلد AppData، سيتم استخدام مجلد المستخدم");
            std::env::temp_dir().join("NoteRust")
        }
    }
}
