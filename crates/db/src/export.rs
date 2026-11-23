//! JSON / Markdown-ZIP export and import.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use noterust_core::{now_millis, Category, Folder, Note, TaskItem, TransactionItem};
use noterust_search::normalize_arabic;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::Result;

/// Current export format version.
pub const EXPORT_VERSION: u32 = 1;

/// A note inside an export bundle (tags are stored by name).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedNote {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub content: String,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub is_locked: bool,
    pub color: Option<String>,
    pub direction: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Everything the app stores, in one portable document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub app: String,
    pub version: u32,
    pub exported_at: i64,
    pub folders: Vec<Folder>,
    pub notes: Vec<ExportedNote>,
    pub tasks: Vec<TaskItem>,
    pub transactions: Vec<TransactionItem>,
    pub categories: Vec<Category>,
}

/// Counts returned by an import.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStats {
    pub folders: i64,
    pub notes: i64,
    pub tasks: i64,
    pub transactions: i64,
}

/// Builds the export bundle from the database.
pub fn collect(conn: &Connection) -> Result<ExportBundle> {
    let folders = {
        let mut statement = conn.prepare(
            "SELECT id, parent_id, name, icon, color, sort_order, created_at FROM folders
             ORDER BY sort_order",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(Folder {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    name: row.get(2)?,
                    icon: row.get(3)?,
                    color: row.get(4)?,
                    sort_order: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let notes = {
        let mut statement = conn.prepare(
            "SELECT n.id, n.folder_id, n.title, n.content, n.is_pinned, n.is_archived, n.is_locked,
                    n.color, n.direction, n.created_at, n.updated_at
             FROM notes n WHERE n.deleted_at IS NULL ORDER BY n.id",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(ExportedNote {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    is_pinned: row.get::<_, i64>(4)? != 0,
                    is_archived: row.get::<_, i64>(5)? != 0,
                    is_locked: row.get::<_, i64>(6)? != 0,
                    color: row.get(7)?,
                    direction: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    tags: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let mut notes = notes;
    for note in &mut notes {
        note.tags = crate::tags::for_note(conn, note.id)?
            .into_iter()
            .map(|tag| tag.name)
            .collect();
    }

    let tasks = crate::tasks::list(conn, true)?;
    let categories = crate::finance::categories(conn)?;
    let transactions = crate::finance::list_transactions(conn, 0, i64::MAX, None)?;

    Ok(ExportBundle {
        app: "NoteRust".to_string(),
        version: EXPORT_VERSION,
        exported_at: now_millis(),
        folders,
        notes,
        tasks,
        transactions,
        categories,
    })
}

/// Serialises everything to JSON.
pub fn to_json(conn: &Connection) -> Result<String> {
    let bundle = collect(conn)?;
    Ok(serde_json::to_string_pretty(&bundle)?)
}

/// Writes the JSON export to `path`.
pub fn write_json(conn: &Connection, path: &Path) -> Result<()> {
    let json = to_json(conn)?;
    crate::Db::ensure_parent_dir(path)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn sanitise(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t' => '-',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "بدون-عنوان".to_string()
    } else {
        noterust_core::truncate_chars(trimmed, 80)
    }
}

/// Writes every note as a Markdown file inside a ZIP archive.
pub fn write_zip(conn: &Connection, path: &Path) -> Result<usize> {
    crate::Db::ensure_parent_dir(path)?;
    let bundle = collect(conn)?;
    let folder_names: std::collections::HashMap<i64, String> = bundle
        .folders
        .iter()
        .map(|folder| (folder.id, sanitise(&folder.name)))
        .collect();

    let file = File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut used: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut written = 0usize;

    for note in &bundle.notes {
        if note.is_locked {
            continue;
        }
        let folder = note
            .folder_id
            .and_then(|id| folder_names.get(&id).cloned())
            .unwrap_or_else(|| "بدون-مجلد".to_string());
        let mut stem = format!("ملاحظات/{folder}/{}", sanitise(&note.title));
        let counter = used.entry(stem.clone()).or_insert(0);
        if *counter > 0 {
            stem = format!("{stem} ({counter})");
        }
        *counter += 1;

        let mut body = String::new();
        if !note.tags.is_empty() {
            body.push_str(&format!("---\ntags: {}\n---\n\n", note.tags.join(", ")));
        }
        body.push_str(&note.content);
        body.push('\n');

        zip.start_file(format!("{stem}.md"), options)?;
        zip.write_all(body.as_bytes())?;
        written += 1;
    }

    zip.start_file("بيانات.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&bundle)?.as_bytes())?;
    zip.finish()?;
    Ok(written)
}

/// Imports a bundle, creating new rows (existing notes are never overwritten).
pub fn import_json(conn: &mut Connection, json: &str) -> Result<ImportStats> {
    let bundle: ExportBundle = serde_json::from_str(json)?;
    let transaction = conn.transaction()?;
    let mut stats = ImportStats::default();
    let now = now_millis();

    let mut folder_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    // Insert parents before children (the export is sorted by sort order, which
    // keeps parents first in practice); repeat passes to be safe.
    let mut pending: Vec<&Folder> = bundle.folders.iter().collect();
    for _ in 0..3 {
        let mut remaining = Vec::new();
        for folder in pending {
            let parent_ready = folder
                .parent_id
                .map(|parent| folder_map.contains_key(&parent))
                .unwrap_or(true);
            if !parent_ready {
                remaining.push(folder);
                continue;
            }
            let new_parent = folder
                .parent_id
                .and_then(|parent| folder_map.get(&parent).copied());
            transaction.execute(
                "INSERT INTO folders(parent_id, name, icon, color, sort_order, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    new_parent,
                    folder.name,
                    folder.icon,
                    folder.color,
                    folder.sort_order,
                    folder.created_at
                ],
            )?;
            folder_map.insert(folder.id, transaction.last_insert_rowid());
            stats.folders += 1;
        }
        if remaining.is_empty() {
            break;
        }
        pending = remaining;
    }

    for note in &bundle.notes {
        let folder_id = note.folder_id.and_then(|id| folder_map.get(&id).copied());
        transaction.execute(
            "INSERT INTO notes(folder_id, title, content, is_pinned, is_archived, is_locked, color,
                               direction, search_title, search_body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                folder_id,
                note.title,
                note.content,
                note.is_pinned as i64,
                note.is_archived as i64,
                note.is_locked as i64,
                note.color,
                note.direction,
                normalize_arabic(&note.title),
                if note.is_locked {
                    String::new()
                } else {
                    normalize_arabic(&note.content)
                },
                note.created_at,
                note.updated_at
            ],
        )?;
        let new_id = transaction.last_insert_rowid();
        for tag in &note.tags {
            let tag = crate::tags::ensure(&transaction, tag, "#8ab4f8")?;
            transaction.execute(
                "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES (?1, ?2)",
                params![new_id, tag.id],
            )?;
        }
        stats.notes += 1;
    }

    for task in &bundle.tasks {
        if task.from_checklist {
            continue; // rebuilt from note content instead
        }
        transaction.execute(
            "INSERT INTO tasks(note_id, title, description, due_at, priority, is_done, repeat_rule,
                               created_at, completed_at)
             VALUES (NULL, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                task.title,
                task.description,
                task.due_at,
                task.priority,
                task.is_done as i64,
                task.repeat_rule,
                task.created_at,
                task.completed_at
            ],
        )?;
        stats.tasks += 1;
    }

    for category in &bundle.categories {
        transaction.execute(
            "INSERT OR IGNORE INTO categories(name, kind, color) VALUES (?1, ?2, ?3)",
            params![category.name, category.kind, category.color],
        )?;
    }

    for tx in &bundle.transactions {
        let category_id: Option<i64> = tx.category_name.as_ref().and_then(|name| {
            transaction
                .query_row(
                    "SELECT id FROM categories WHERE name = ?1 AND kind = ?2",
                    params![name, tx.kind],
                    |row| row.get(0),
                )
                .ok()
        });
        transaction.execute(
            "INSERT INTO transactions(kind, amount_minor, currency, category_id, note, occurred_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                tx.kind,
                tx.amount_minor,
                tx.currency,
                category_id,
                tx.note,
                tx.occurred_at,
                now
            ],
        )?;
        stats.transactions += 1;
    }

    transaction.commit()?;
    Ok(stats)
}

/// Imports a single note from plain text (used by "import files").
pub fn import_text_note(
    conn: &Connection,
    folder_id: Option<i64>,
    title: &str,
    content: &str,
    source_path: Option<&str>,
) -> Result<Note> {
    let now = now_millis();
    conn.execute(
        "INSERT INTO notes(folder_id, title, content, source_path, encoding, line_ending,
                           search_title, search_body, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'UTF-8', 'LF', ?5, ?6, ?7, ?7)",
        params![
            folder_id,
            title,
            content,
            source_path,
            normalize_arabic(title),
            normalize_arabic(content),
            now
        ],
    )?;
    crate::notes::get(conn, conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    #[test]
    fn json_roundtrip_restores_notes_and_tags() {
        let dir = tempfile::tempdir().expect("dir");
        let db = Db::open(&dir.path().join("a.db")).expect("db");
        let folder = crate::folders::create(db.conn(), None, "عمل", "📁", "#fff").expect("folder");
        let note = crate::notes::create(db.conn(), Some(folder.id)).expect("note");
        crate::notes::update_content(db.conn(), note.id, "محتوى التصدير").expect("update");
        crate::tags::set_for_note(db.conn(), note.id, &["مهم".into()]).expect("tags");

        let json = to_json(db.conn()).expect("json");
        assert!(json.contains("محتوى التصدير"));

        let mut target = Db::open(&dir.path().join("b.db")).expect("db2");
        let stats = import_json(target.conn_mut(), &json).expect("import");
        assert_eq!(stats.notes, 1);
        assert_eq!(stats.folders, 1);
        let notes = crate::notes::list(target.conn(), &crate::ListQuery::default()).expect("list");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "محتوى التصدير");
        assert_eq!(notes[0].tags.len(), 1);
    }

    #[test]
    fn zip_export_writes_markdown() {
        let dir = tempfile::tempdir().expect("dir");
        let db = Db::open(&dir.path().join("a.db")).expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        crate::notes::update_content(db.conn(), note.id, "# عنوان\nنص").expect("update");
        let path = dir.path().join("export.zip");
        let written = write_zip(db.conn(), &path).expect("zip");
        assert_eq!(written, 1);
        assert!(path.is_file());
        assert!(std::fs::metadata(&path).expect("meta").len() > 0);
    }

    #[test]
    fn text_import_creates_note() {
        let db = Db::open_in_memory().expect("db");
        let note = import_text_note(db.conn(), None, "ملف مستورد", "نص", Some("C:/a.txt"))
            .expect("import");
        assert_eq!(note.title, "ملف مستورد");
        assert_eq!(note.source_path.as_deref(), Some("C:/a.txt"));
    }

    #[test]
    fn invalid_json_is_rejected() {
        let mut db = Db::open_in_memory().expect("db");
        assert!(import_json(db.conn_mut(), "{ not json").is_err());
    }
}
