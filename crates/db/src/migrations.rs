//! Schema migrations.
//!
//! FTS5 indexes `notes.search_title` / `notes.search_body`, which are filled by
//! Rust with Arabic-normalised text. Triggers keep the index in sync, so a
//! plain SQL `UPDATE` can never desynchronise search.

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::Result;

/// Full initial schema.
const SCHEMA_V1: &str = r#"
CREATE TABLE folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id   INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    icon        TEXT    NOT NULL DEFAULT '📁',
    color       TEXT    NOT NULL DEFAULT '#e8b830',
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);
CREATE INDEX idx_folders_parent ON folders(parent_id, sort_order);

CREATE TABLE notes (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id      INTEGER REFERENCES folders(id) ON DELETE SET NULL,
    title          TEXT    NOT NULL DEFAULT '',
    content        TEXT    NOT NULL DEFAULT '',
    content_format TEXT    NOT NULL DEFAULT 'markdown',
    is_pinned      INTEGER NOT NULL DEFAULT 0,
    is_archived    INTEGER NOT NULL DEFAULT 0,
    is_locked      INTEGER NOT NULL DEFAULT 0,
    color          TEXT,
    direction      TEXT    NOT NULL DEFAULT 'auto',
    source_path    TEXT,
    encoding       TEXT,
    line_ending    TEXT,
    lock_salt      TEXT,
    search_title   TEXT    NOT NULL DEFAULT '',
    search_body    TEXT    NOT NULL DEFAULT '',
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    deleted_at     INTEGER
);
CREATE INDEX idx_notes_folder  ON notes(folder_id, deleted_at);
CREATE INDEX idx_notes_updated ON notes(updated_at DESC);
CREATE INDEX idx_notes_source  ON notes(source_path);

CREATE VIRTUAL TABLE notes_fts USING fts5(
    search_title,
    search_body,
    content = 'notes',
    content_rowid = 'id',
    tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, search_title, search_body)
    VALUES (new.id, new.search_title, new.search_body);
END;

CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, search_title, search_body)
    VALUES ('delete', old.id, old.search_title, old.search_body);
END;

CREATE TRIGGER notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, search_title, search_body)
    VALUES ('delete', old.id, old.search_title, old.search_body);
    INSERT INTO notes_fts(rowid, search_title, search_body)
    VALUES (new.id, new.search_title, new.search_body);
END;

CREATE TABLE note_versions (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id  INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    content  TEXT    NOT NULL,
    title    TEXT    NOT NULL DEFAULT '',
    saved_at INTEGER NOT NULL
);
CREATE INDEX idx_versions_note ON note_versions(note_id, saved_at DESC);

CREATE TABLE tags (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    name  TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#8ab4f8'
);

CREATE TABLE note_tags (
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
    PRIMARY KEY (note_id, tag_id)
);
CREATE INDEX idx_note_tags_tag ON note_tags(tag_id);

CREATE TABLE tasks (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id      INTEGER REFERENCES notes(id) ON DELETE CASCADE,
    title        TEXT    NOT NULL,
    description  TEXT    NOT NULL DEFAULT '',
    due_at       INTEGER,
    priority     INTEGER NOT NULL DEFAULT 2,
    is_done      INTEGER NOT NULL DEFAULT 0,
    repeat_rule  TEXT,
    created_at   INTEGER NOT NULL,
    completed_at INTEGER
);
CREATE INDEX idx_tasks_done ON tasks(is_done, due_at);
CREATE INDEX idx_tasks_note ON tasks(note_id);

CREATE TABLE categories (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    name  TEXT    NOT NULL,
    kind  TEXT    NOT NULL,
    color TEXT    NOT NULL DEFAULT '#8ab4f8',
    UNIQUE(name, kind)
);

CREATE TABLE transactions (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    kind         TEXT    NOT NULL CHECK (kind IN ('income', 'expense')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor >= 0),
    currency     TEXT    NOT NULL DEFAULT 'EGP',
    category_id  INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    note         TEXT,
    occurred_at  INTEGER NOT NULL,
    created_at   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_transactions_date ON transactions(occurred_at DESC);
CREATE INDEX idx_transactions_kind ON transactions(kind, occurred_at DESC);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO settings(key, value) VALUES
    ('theme', 'dark'),
    ('font_size', '16'),
    ('font_family', 'Cairo'),
    ('trash_retention_days', '30'),
    ('backup_retention', '14'),
    ('sort_by', 'updated'),
    ('word_wrap', 'true'),
    ('last_folder', ''),
    ('default_currency', 'EGP');

INSERT INTO categories(name, kind, color) VALUES
    ('راتب', 'income', '#57c785'),
    ('أعمال حرة', 'income', '#4fa8e0'),
    ('مصروف يومي', 'expense', '#e8b830'),
    ('طعام', 'expense', '#e07a5f'),
    ('مواصلات', 'expense', '#8ab4f8'),
    ('فواتير', 'expense', '#b48ce8');
"#;

/// Optional attachments table, added in a later migration to keep V1 lean.
const SCHEMA_V2: &str = r#"
CREATE TABLE attachments (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id  INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    filename TEXT    NOT NULL,
    mime     TEXT,
    path     TEXT,
    data     BLOB
);
CREATE INDEX idx_attachments_note ON attachments(note_id);
"#;

/// Saved code snippets and prompts.
const SCHEMA_V3: &str = r#"
CREATE TABLE snippets (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT    NOT NULL DEFAULT '',
    kind       TEXT    NOT NULL DEFAULT 'code' CHECK (kind IN ('code', 'prompt')),
    language   TEXT    NOT NULL DEFAULT '',
    content    TEXT    NOT NULL DEFAULT '',
    tags       TEXT    NOT NULL DEFAULT '',
    is_pinned  INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_snippets_kind ON snippets(kind, updated_at DESC);
"#;

/// AI provider endpoints added by the user (local servers and cloud APIs).
const SCHEMA_V4: &str = r#"
CREATE TABLE ai_connections (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL DEFAULT '',
    kind       TEXT    NOT NULL DEFAULT 'openai'
               CHECK (kind IN ('openai', 'gemini', 'anthropic')),
    base_url   TEXT    NOT NULL DEFAULT '',
    api_key    TEXT    NOT NULL DEFAULT '',
    model      TEXT    NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_ai_connections_created ON ai_connections(created_at);
"#;

/// Rebuilds the search index from `notes`.
///
/// If the FTS5 document list ever drifts away from the `notes` table, SQLite
/// reports it as `database disk image is malformed` and every later delete of
/// the affected note fails. Re-indexing once on upgrade heals such databases.
const REPAIR_SEARCH_INDEX: &str = r#"
INSERT INTO notes_fts(notes_fts) VALUES('rebuild');
"#;

/// All migrations, oldest first.
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(SCHEMA_V1),
        M::up(SCHEMA_V2),
        M::up(SCHEMA_V3),
        M::up(SCHEMA_V4),
        M::up(REPAIR_SEARCH_INDEX),
    ])
}

/// Brings `conn` up to the latest schema version.
pub fn run(conn: &mut Connection) -> Result<()> {
    migrations().to_latest(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_contains_all_tables() {
        let mut conn = Connection::open_in_memory().expect("conn");
        run(&mut conn).expect("migrate");
        for table in [
            "folders",
            "notes",
            "notes_fts",
            "note_versions",
            "tags",
            "note_tags",
            "tasks",
            "transactions",
            "categories",
            "settings",
            "attachments",
            "snippets",
        ] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("query");
            assert_eq!(found, 1, "missing table {table}");
        }
    }

    #[test]
    fn fts_stays_in_sync_through_triggers() {
        let mut conn = Connection::open_in_memory().expect("conn");
        run(&mut conn).expect("migrate");
        conn.execute(
            "INSERT INTO notes(title, content, search_title, search_body, created_at, updated_at)
             VALUES ('عنوان', 'جسم', 'عنوان', 'جسم', 0, 0)",
            [],
        )
        .expect("insert");
        let hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notes_fts WHERE notes_fts MATCH '\"جسم\"'",
                [],
                |row| row.get(0),
            )
            .expect("search");
        assert_eq!(hits, 1);
    }

    #[test]
    fn deleting_note_removes_fts_entry() {
        let mut conn = Connection::open_in_memory().expect("conn");
        run(&mut conn).expect("migrate");
        conn.execute(
            "INSERT INTO notes(title, content, search_title, search_body, created_at, updated_at)
             VALUES ('a', 'b', 'a', 'b', 0, 0)",
            [],
        )
        .expect("insert");
        conn.execute("DELETE FROM notes", []).expect("delete");
        let hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notes_fts WHERE notes_fts MATCH '\"b\"'",
                [],
                |row| row.get(0),
            )
            .expect("search");
        assert_eq!(hits, 0);
    }

    #[test]
    fn defaults_are_seeded() {
        let mut conn = Connection::open_in_memory().expect("conn");
        run(&mut conn).expect("migrate");
        let theme: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .expect("theme");
        assert_eq!(theme, "dark");
        let categories: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .expect("categories");
        assert!(categories >= 6);
    }
}
