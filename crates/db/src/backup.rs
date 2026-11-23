//! Backups: automatic daily snapshots, manual copies and restore.
//!
//! Backups use `VACUUM INTO`, which produces a consistent copy of a live WAL
//! database without blocking readers. Restore replays a backup file into the
//! open connection with the SQLite online backup API, so the app keeps running
//! and the restored data is visible immediately.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::Result;

/// A backup file discovered on disk.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_at: i64,
}

const PREFIX: &str = "noterust-";

fn timestamp_name() -> String {
    format!("{PREFIX}{}.db", Local::now().format("%Y%m%d-%H%M%S"))
}

/// Writes a consistent copy of the database into `dir`.
pub fn backup_now(conn: &Connection, dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let target = dir.join(timestamp_name());
    let temp = dir.join(format!(
        ".{}.tmp",
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("backup")
    ));
    let _ = fs::remove_file(&temp);

    conn.execute("VACUUM INTO ?1", params![temp.to_string_lossy()])?;
    if target.exists() {
        let _ = fs::remove_file(&target);
    }
    fs::rename(&temp, &target)?;
    tracing::info!(path = %target.display(), "backup written");
    Ok(target)
}

/// Lists existing backups, newest first.
pub fn list_backups(dir: &Path) -> Result<Vec<BackupInfo>> {
    let mut backups = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(backups);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with(PREFIX) || !name.ends_with(".db") {
            continue;
        }
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        let created_at = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        backups.push(BackupInfo {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            size,
            created_at,
        });
    }
    backups.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(backups)
}

/// Keeps only the newest `keep` backups.
pub fn prune(dir: &Path, keep: i64) -> Result<i64> {
    let backups = list_backups(dir)?;
    let keep = keep.max(1) as usize;
    let mut removed = 0i64;
    for backup in backups.iter().skip(keep) {
        if fs::remove_file(&backup.path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Creates at most one automatic backup per day, then prunes old ones.
pub fn auto_daily(conn: &Connection, dir: &Path, keep: i64) -> Result<Option<PathBuf>> {
    let today = Local::now().format("%Y%m%d").to_string();
    let already = list_backups(dir)?
        .first()
        .is_some_and(|backup| backup.name.contains(&today));
    if already {
        return Ok(None);
    }
    let path = backup_now(conn, dir)?;
    prune(dir, keep)?;
    Ok(Some(path))
}

/// Replaces the live database contents with `backup_path`.
pub fn restore_into(conn: &mut Connection, backup_path: &Path) -> Result<()> {
    if !backup_path.is_file() {
        return Err(crate::DbError::NotFound);
    }
    let source = Connection::open(backup_path)?;
    {
        let backup = rusqlite::backup::Backup::new(&source, conn)?;
        backup.run_to_completion(64, std::time::Duration::from_millis(20), None)?;
    }
    Ok(())
}

impl crate::Db {
    /// Restores from a backup file into the running database.
    pub fn restore_from_file(&mut self, backup_path: &Path) -> Result<()> {
        restore_into(&mut self.conn, backup_path)
    }

    /// Convenience wrapper used on startup.
    pub fn daily_backup(&self, dir: &Path, keep: i64) -> Result<Option<PathBuf>> {
        auto_daily(&self.conn, dir, keep)
    }
}

/// Deletes every backup file (used by "delete all backups").
pub fn delete_all(dir: &Path) -> Result<i64> {
    let mut removed = 0;
    for backup in list_backups(dir)? {
        if fs::remove_file(&backup.path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    #[test]
    fn backup_and_restore_roundtrip() {
        let dir = tempfile::tempdir().expect("dir");
        let mut db = Db::open(&dir.path().join("notes.db")).expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        crate::notes::update_content(db.conn(), note.id, "محتوى محفوظ").expect("update");

        let backup = backup_now(db.conn(), &dir.path().join("backups")).expect("backup");
        assert!(backup.is_file());

        crate::notes::update_content(db.conn(), note.id, "تغيير لاحق").expect("update");
        db.restore_from_file(&backup).expect("restore");
        let restored = crate::notes::get(db.conn(), note.id).expect("get");
        assert_eq!(restored.content, "محتوى محفوظ");
    }

    #[test]
    fn prune_keeps_newest() {
        let dir = tempfile::tempdir().expect("dir");
        let backups = dir.path().join("backups");
        fs::create_dir_all(&backups).expect("dir");
        for name in [
            "noterust-20260101-000000.db",
            "noterust-20260102-000000.db",
            "noterust-20260103-000000.db",
        ] {
            fs::write(backups.join(name), b"x").expect("write");
        }
        let removed = prune(&backups, 2).expect("prune");
        assert_eq!(removed, 1);
        assert_eq!(list_backups(&backups).expect("list").len(), 2);
    }

    #[test]
    fn daily_backup_runs_only_once() {
        let dir = tempfile::tempdir().expect("dir");
        let db = Db::open(&dir.path().join("notes.db")).expect("db");
        let backups = dir.path().join("backups");
        assert!(auto_daily(db.conn(), &backups, 14)
            .expect("first")
            .is_some());
        assert!(auto_daily(db.conn(), &backups, 14)
            .expect("second")
            .is_none());
    }

    #[test]
    fn list_ignores_foreign_files() {
        let dir = tempfile::tempdir().expect("dir");
        fs::write(dir.path().join("random.txt"), b"x").expect("write");
        assert!(list_backups(dir.path()).expect("list").is_empty());
    }
}
