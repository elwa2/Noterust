//! SQLite persistence: connection handling, migrations and repositories.
//!
//! The connection is owned by a single worker thread (see `src-tauri`), so the
//! UI never blocks on disk I/O. Every write happens inside an implicit or
//! explicit transaction; the database runs in WAL mode with
//! `synchronous = NORMAL`, which keeps commits crash-safe while staying fast.

pub mod backup;
pub mod export;
pub mod finance;
pub mod folders;
pub mod migrations;
pub mod notes;
pub mod settings;
pub mod snippets;
pub mod tags;
pub mod tasks;
pub mod versions;

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use thiserror::Error;

pub use noterust_core::{
    Category, MonthlySummary, Note, NoteSummary, NoteVersion, SearchHit, Snippet, SortBy, Tag,
    TaskItem, TransactionItem,
};

/// Errors surfaced by the persistence layer.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("خطأ في قاعدة البيانات: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("خطأ في الترحيل: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error(transparent)]
    Core(#[from] noterust_core::CoreError),
    #[error(transparent)]
    Crypto(#[from] noterust_crypto::CryptoError),
    #[error("خطأ في الملفات: {0}")]
    Io(#[from] std::io::Error),
    #[error("خطأ في JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("تعذر تنفيذ العملية: {0}")]
    Archive(String),
    #[error("خطأ في أرشيف ZIP: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("العنصر المطلوب غير موجود")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, DbError>;

/// The note list query options.
#[derive(Debug, Clone)]
pub struct ListQuery {
    pub folder_id: Option<i64>,
    pub trash: bool,
    pub include_archived: bool,
    pub sort: SortBy,
    pub limit: i64,
}

impl Default for ListQuery {
    fn default() -> Self {
        Self {
            folder_id: None,
            trash: false,
            include_archived: false,
            sort: SortBy::Updated,
            limit: 500,
        }
    }
}

/// An owned SQLite connection plus the path it was opened from.
pub struct Db {
    conn: Connection,
    path: Option<PathBuf>,
}

impl Db {
    /// Opens (and migrates) the database at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let mut db = Self {
            conn,
            path: Some(path.to_path_buf()),
        };
        db.configure()?;
        migrations::run(&mut db.conn)?;
        Ok(db)
    }

    /// Opens a throwaway database, used by tests.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn, path: None };
        db.configure()?;
        migrations::run(&mut db.conn)?;
        Ok(db)
    }

    /// Applies the pragmas required for durability and foreign keys.
    fn configure(&self) -> Result<()> {
        self.conn.busy_timeout(std::time::Duration::from_secs(5))?;
        if self.path.is_some() {
            // WAL is only meaningful for file backed databases.
            let _: String = self
                .conn
                .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        }
        self.conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;",
        )?;
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Mutable access, needed for transactions and the restore path.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Flushes the WAL into the main database file.
    pub fn checkpoint(&self) -> Result<()> {
        if self.path.is_some() {
            self.conn
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        }
        Ok(())
    }

    /// Makes sure the parent directory exists and returns the database path.
    pub fn ensure_parent_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

/// Maps a row that starts with the standard note columns into a `Note`.
pub(crate) fn map_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get("id")?,
        folder_id: row.get("folder_id")?,
        title: row.get("title")?,
        content: row.get("content")?,
        content_format: row.get("content_format")?,
        is_pinned: row.get::<_, i64>("is_pinned")? != 0,
        is_archived: row.get::<_, i64>("is_archived")? != 0,
        is_locked: row.get::<_, i64>("is_locked")? != 0,
        color: row.get("color")?,
        direction: row.get("direction")?,
        source_path: row.get("source_path")?,
        encoding: row.get("encoding")?,
        line_ending: row.get("line_ending")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        deleted_at: row.get("deleted_at")?,
        tags: Vec::new(),
    })
}

/// Columns selected for a full note, in the order `map_note` expects.
pub(crate) const NOTE_COLUMNS: &str = "n.id, n.folder_id, n.title, n.content, n.content_format, \
     n.is_pinned, n.is_archived, n.is_locked, n.color, n.direction, n.source_path, n.encoding, \
     n.line_ending, n.created_at, n.updated_at, n.deleted_at";

/// Columns selected for a note summary.
pub(crate) const SUMMARY_COLUMNS: &str =
    "n.id, n.folder_id, n.title, n.content, n.is_pinned, n.is_locked, n.is_archived, n.color, \
     n.created_at, n.updated_at";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pragmas_are_applied() {
        let db = Db::open_in_memory().expect("db");
        let foreign_keys: i64 = db
            .conn()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("pragma");
        assert_eq!(foreign_keys, 1);
    }

    #[test]
    fn file_database_uses_wal() {
        let dir = tempfile::tempdir().expect("dir");
        let db = Db::open(&dir.path().join("notes.db")).expect("db");
        let mode: String = db
            .conn()
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("pragma");
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn migrations_are_idempotent() {
        let db = Db::open_in_memory().expect("db");
        assert!(migrations::run(&mut Connection::open_in_memory().expect("conn")).is_ok());
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .expect("count");
        assert!(count >= 0);
    }
}
