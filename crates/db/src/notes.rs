//! Note repository: CRUD, search, trash and encryption.

use std::collections::HashMap;

use noterust_core::{
    counts, derive_title, now_millis, preview, Note, NoteSummary, SearchHit, SortBy, Tag,
};
use noterust_crypto as crypto;
use noterust_search::{build_match_query, clean_snippet, normalize_arabic};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};

use crate::{tags, versions, DbError, ListQuery, Result, NOTE_COLUMNS, SUMMARY_COLUMNS};

/// Creates an empty note inside `folder_id`.
pub fn create(conn: &Connection, folder_id: Option<i64>) -> Result<Note> {
    if let Some(folder) = folder_id {
        super::folders::get(conn, folder)?;
    }
    let now = now_millis();
    conn.execute(
        "INSERT INTO notes(folder_id, title, content, created_at, updated_at)
         VALUES (?1, '', '', ?2, ?2)",
        params![folder_id, now],
    )?;
    get(conn, conn.last_insert_rowid())
}

/// Loads a note with its tags.
pub fn get(conn: &Connection, id: i64) -> Result<Note> {
    let mut note = conn
        .query_row(
            &format!("SELECT {NOTE_COLUMNS} FROM notes n WHERE n.id = ?1"),
            params![id],
            crate::map_note,
        )
        .optional()?
        .ok_or(DbError::NotFound)?;
    note.tags = tags::for_note(conn, id)?;
    Ok(note)
}

/// Loads a note by the external file path it is attached to.
pub fn by_source_path(conn: &Connection, path: &str) -> Result<Option<Note>> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM notes WHERE source_path = ?1 AND deleted_at IS NULL LIMIT 1",
            params![path],
            |row| row.get(0),
        )
        .optional()?;
    match id {
        Some(id) => Ok(Some(get(conn, id)?)),
        None => Ok(None),
    }
}

/// Keeps the normalised FTS columns in sync with the real title/body.
fn refresh_search(conn: &Connection, id: i64, title: &str, content: &str) -> Result<()> {
    conn.execute(
        "UPDATE notes SET search_title = ?1, search_body = ?2 WHERE id = ?3",
        params![normalize_arabic(title), normalize_arabic(content), id],
    )?;
    Ok(())
}

/// Replaces the body of a note and refreshes derived data.
pub fn update_content(conn: &Connection, id: i64, content: &str) -> Result<Note> {
    let existing = get(conn, id)?;
    if existing.is_locked {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "الملاحظة مقفلة، استخدم حفظ الملاحظات المقفلة".into(),
        )));
    }
    if existing.content == content {
        return Ok(existing);
    }
    // No point storing an empty first revision.
    if !existing.content.is_empty() {
        versions::snapshot(conn, &existing)?;
    }

    let title = derive_title(content);
    conn.execute(
        "UPDATE notes SET content = ?1, title = ?2, updated_at = ?3 WHERE id = ?4",
        params![content, title, now_millis(), id],
    )?;
    refresh_search(conn, id, &title, content)?;
    super::tasks::sync_from_checklist(conn, id, content)?;
    get(conn, id)
}

/// Renames a note (the body is untouched).
pub fn update_title(conn: &Connection, id: i64, title: &str) -> Result<Note> {
    let trimmed = title.trim();
    let note = get(conn, id)?;
    conn.execute(
        "UPDATE notes SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![trimmed, now_millis(), id],
    )?;
    if !note.is_locked {
        refresh_search(conn, id, trimmed, &note.content)?;
    }
    get(conn, id)
}

/// Moves a note to another folder (`None` = the "all notes" bucket).
pub fn set_folder(conn: &Connection, id: i64, folder_id: Option<i64>) -> Result<Note> {
    if let Some(folder) = folder_id {
        super::folders::get(conn, folder)?;
    }
    conn.execute(
        "UPDATE notes SET folder_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![folder_id, now_millis(), id],
    )?;
    get(conn, id)
}

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) -> Result<Note> {
    conn.execute(
        "UPDATE notes SET is_pinned = ?1 WHERE id = ?2",
        params![pinned as i64, id],
    )?;
    get(conn, id)
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> Result<Note> {
    conn.execute(
        "UPDATE notes SET is_archived = ?1, updated_at = ?2 WHERE id = ?3",
        params![archived as i64, now_millis(), id],
    )?;
    get(conn, id)
}

pub fn set_color(conn: &Connection, id: i64, color: Option<&str>) -> Result<Note> {
    conn.execute(
        "UPDATE notes SET color = ?1 WHERE id = ?2",
        params![color, id],
    )?;
    get(conn, id)
}

pub fn set_direction(conn: &Connection, id: i64, direction: &str) -> Result<Note> {
    if !matches!(direction, "auto" | "rtl" | "ltr") {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "اتجاه غير معروف".into(),
        )));
    }
    conn.execute(
        "UPDATE notes SET direction = ?1 WHERE id = ?2",
        params![direction, id],
    )?;
    get(conn, id)
}

/// Attaches a note to an external file (or detaches it with `None`).
pub fn set_source(
    conn: &Connection,
    id: i64,
    source_path: Option<&str>,
    encoding: Option<&str>,
    line_ending: Option<&str>,
) -> Result<Note> {
    conn.execute(
        "UPDATE notes SET source_path = ?1, encoding = ?2, line_ending = ?3 WHERE id = ?4",
        params![source_path, encoding, line_ending, id],
    )?;
    get(conn, id)
}

// ---------------------------------------------------------------------------
// Trash
// ---------------------------------------------------------------------------

pub fn soft_delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE notes SET deleted_at = ?1, is_pinned = 0 WHERE id = ?2",
        params![now_millis(), id],
    )?;
    Ok(())
}

pub fn restore(conn: &Connection, id: i64) -> Result<Note> {
    conn.execute(
        "UPDATE notes SET deleted_at = NULL WHERE id = ?1",
        params![id],
    )?;
    get(conn, id)
}

/// True when SQLite refuses the write because the FTS5 index no longer matches
/// the `notes` table (`database disk image is malformed`).
fn is_stale_search_index(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == rusqlite::ErrorCode::DatabaseCorrupt
    )
}

/// Runs a delete, repairing a desynchronised search index once and retrying.
fn delete_with_index_repair(
    conn: &Connection,
    operation: impl Fn(&Connection) -> rusqlite::Result<usize>,
) -> Result<usize> {
    match operation(conn) {
        Ok(deleted) => Ok(deleted),
        Err(err) if is_stale_search_index(&err) => {
            conn.execute("INSERT INTO notes_fts(notes_fts) VALUES('rebuild')", [])?;
            Ok(operation(conn)?)
        }
        Err(err) => Err(err.into()),
    }
}

pub fn purge(conn: &Connection, id: i64) -> Result<()> {
    delete_with_index_repair(conn, |c| {
        c.execute("DELETE FROM notes WHERE id = ?1", params![id])
    })?;
    Ok(())
}

pub fn empty_trash(conn: &Connection) -> Result<i64> {
    let deleted = delete_with_index_repair(conn, |c| {
        c.execute("DELETE FROM notes WHERE deleted_at IS NOT NULL", [])
    })?;
    Ok(deleted as i64)
}

/// Permanently deletes notes that have been in the trash for `days` days.
pub fn purge_expired(conn: &Connection, days: i64) -> Result<i64> {
    let cutoff = now_millis() - days.max(1) * 86_400_000;
    let deleted = delete_with_index_repair(conn, |c| {
        c.execute(
            "DELETE FROM notes WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
            params![cutoff],
        )
    })?;
    Ok(deleted as i64)
}

// ---------------------------------------------------------------------------
// Listing and search
// ---------------------------------------------------------------------------

fn map_summary(row: &Row<'_>) -> rusqlite::Result<NoteSummary> {
    let content: String = row.get("content")?;
    Ok(NoteSummary {
        id: row.get("id")?,
        folder_id: row.get("folder_id")?,
        title: row.get("title")?,
        preview: preview(&content),
        is_pinned: row.get::<_, i64>("is_pinned")? != 0,
        is_locked: row.get::<_, i64>("is_locked")? != 0,
        is_archived: row.get::<_, i64>("is_archived")? != 0,
        color: row.get("color")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        word_count: counts(&content).words,
        tags: Vec::new(),
    })
}

/// Loads the tags of many notes in a single query.
fn attach_tags(conn: &Connection, mut notes: Vec<NoteSummary>) -> Result<Vec<NoteSummary>> {
    if notes.is_empty() {
        return Ok(notes);
    }
    let placeholders = std::iter::repeat_n("?", notes.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT nt.note_id, t.id, t.name, t.color
         FROM note_tags nt JOIN tags t ON t.id = nt.tag_id
         WHERE nt.note_id IN ({placeholders})
         ORDER BY t.name COLLATE NOCASE"
    );
    let ids = notes.iter().map(|note| note.id);
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(ids), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            Tag {
                id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
            },
        ))
    })?;

    let mut map: HashMap<i64, Vec<Tag>> = HashMap::new();
    for row in rows {
        let (note_id, tag) = row?;
        map.entry(note_id).or_default().push(tag);
    }
    for note in &mut notes {
        if let Some(tags) = map.remove(&note.id) {
            note.tags = tags;
        }
    }
    Ok(notes)
}

/// Lists note summaries for the middle column.
pub fn list(conn: &Connection, query: &ListQuery) -> Result<Vec<NoteSummary>> {
    let sort = match query.sort {
        SortBy::Updated => "n.updated_at DESC",
        SortBy::Created => "n.created_at DESC",
        SortBy::Title => "n.title COLLATE NOCASE ASC",
    };
    let sql = format!(
        "SELECT {SUMMARY_COLUMNS} FROM notes n
         WHERE (CASE WHEN ?1 = 1 THEN n.deleted_at IS NOT NULL ELSE n.deleted_at IS NULL END)
           AND (?2 = 1 OR n.is_archived = 0)
           AND (?3 IS NULL OR n.folder_id = ?3)
         ORDER BY n.is_pinned DESC, {sort}
         LIMIT ?4"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement
        .query_map(
            params![
                query.trash as i64,
                query.include_archived as i64,
                query.folder_id,
                query.limit
            ],
            map_summary,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    attach_tags(conn, rows)
}

/// Full-text search with Arabic normalisation and FTS5 snippets.
pub fn search(
    conn: &Connection,
    text: &str,
    folder_id: Option<i64>,
    limit: i64,
) -> Result<Vec<SearchHit>> {
    let Some(match_query) = build_match_query(text) else {
        return Ok(Vec::new());
    };
    let sql = format!(
        "SELECT {SUMMARY_COLUMNS}, snippet(notes_fts, 1, '<b>', '</b>', '…', 12) AS snippet
         FROM notes_fts
         JOIN notes n ON n.id = notes_fts.rowid
         WHERE notes_fts MATCH ?1
           AND n.deleted_at IS NULL
           AND (?2 IS NULL OR n.folder_id = ?2)
         ORDER BY bm25(notes_fts), n.is_pinned DESC
         LIMIT ?3"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement
        .query_map(params![match_query, folder_id, limit], |row| {
            let snippet: String = row.get("snippet")?;
            Ok(SearchHit {
                note: map_summary(row)?,
                snippet: clean_snippet(&snippet),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let summaries: Vec<NoteSummary> = rows.iter().map(|hit| hit.note.clone()).collect();
    let with_tags = attach_tags(conn, summaries)?;
    let tags_by_id: HashMap<i64, Vec<Tag>> = with_tags
        .into_iter()
        .map(|note| (note.id, note.tags))
        .collect();

    Ok(rows
        .into_iter()
        .map(|mut hit| {
            if let Some(tags) = tags_by_id.get(&hit.note.id) {
                hit.note.tags = tags.clone();
            }
            hit
        })
        .collect())
}

/// Number of notes in a folder (or overall) excluding the trash.
pub fn count(conn: &Connection, folder_id: Option<i64>) -> Result<i64> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM notes
         WHERE deleted_at IS NULL AND is_archived = 0
           AND (?1 IS NULL OR folder_id = ?1)",
        params![folder_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn trash_count(conn: &Connection) -> Result<i64> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM notes WHERE deleted_at IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn archived_count(conn: &Connection) -> Result<i64> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM notes WHERE is_archived = 1 AND deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

// ---------------------------------------------------------------------------
// Encryption
// ---------------------------------------------------------------------------

/// Encrypts a note's body and marks it as locked.
pub fn lock(conn: &Connection, id: i64, password: &str) -> Result<Note> {
    let note = get(conn, id)?;
    if note.is_locked {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "الملاحظة مقفلة بالفعل".into(),
        )));
    }
    let blob = crypto::encrypt(&note.content, password)?;
    conn.execute(
        "UPDATE notes SET content = ?1, is_locked = 1, updated_at = ?2 WHERE id = ?3",
        params![blob, now_millis(), id],
    )?;
    refresh_search(conn, id, &note.title, "")?;
    get(conn, id)
}

/// Verifies `password` and returns the decrypted body.
pub fn unlock(conn: &Connection, id: i64, password: &str) -> Result<String> {
    let note = get(conn, id)?;
    if !note.is_locked {
        return Ok(note.content);
    }
    crypto::decrypt(&note.content, password).map_err(DbError::Crypto)
}

/// Re-encrypts a locked note after editing.
pub fn save_locked(conn: &Connection, id: i64, password: &str, plaintext: &str) -> Result<Note> {
    let note = get(conn, id)?;
    if !note.is_locked {
        return update_content(conn, id, plaintext);
    }
    // Verify the password against the existing ciphertext first.
    crypto::decrypt(&note.content, password)?;
    versions::snapshot(conn, &note)?;
    let blob = crypto::encrypt(plaintext, password)?;
    let title = derive_title(plaintext);
    conn.execute(
        "UPDATE notes SET content = ?1, title = ?2, updated_at = ?3 WHERE id = ?4",
        params![blob, title, now_millis(), id],
    )?;
    get(conn, id)
}

/// Removes the lock, keeping the current body (requires the password).
pub fn unlock_permanently(conn: &Connection, id: i64, password: &str) -> Result<Note> {
    let plaintext = unlock(conn, id, password)?;
    let title = derive_title(&plaintext);
    conn.execute(
        "UPDATE notes SET content = ?1, title = ?2, is_locked = 0, updated_at = ?3 WHERE id = ?4",
        params![plaintext, title, now_millis(), id],
    )?;
    refresh_search(conn, id, &title, &plaintext)?;
    super::tasks::sync_from_checklist(conn, id, &plaintext)?;
    get(conn, id)
}

/// Duplicates a note (used by "duplicate" in the command palette).
pub fn duplicate(conn: &Connection, id: i64) -> Result<Note> {
    let note = get(conn, id)?;
    let now = now_millis();
    let title = format!("{} (نسخة)", note.title);
    conn.execute(
        "INSERT INTO notes(folder_id, title, content, content_format, color, direction, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            note.folder_id,
            title,
            note.content,
            note.content_format,
            note.color,
            note.direction,
            now
        ],
    )?;
    let new_id = conn.last_insert_rowid();
    refresh_search(conn, new_id, &title, &note.content)?;
    get(conn, new_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    fn db() -> Db {
        Db::open_in_memory().expect("db")
    }

    #[test]
    fn create_and_update_note() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        assert_eq!(note.title, "");
        let updated = update_content(db.conn(), note.id, "# عنوان\nنص الملاحظة").expect("update");
        assert_eq!(updated.title, "عنوان");
        assert!(updated.content.contains("نص الملاحظة"));
    }

    #[test]
    fn listing_is_sorted_by_update_and_pinned_first() {
        let db = db();
        let first = create(db.conn(), None).expect("create");
        update_content(db.conn(), first.id, "الأول").expect("update");
        let second = create(db.conn(), None).expect("create");
        update_content(db.conn(), second.id, "الثاني").expect("update");
        set_pinned(db.conn(), first.id, true).expect("pin");

        let notes = list(db.conn(), &ListQuery::default()).expect("list");
        assert_eq!(notes[0].id, first.id, "pinned note comes first");
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn search_is_arabic_normalised() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        update_content(db.conn(), note.id, "مُحَمَّد يكتب في المدرسة").expect("update");
        let hits = search(db.conn(), "محمد", None, 10).expect("search");
        assert_eq!(
            hits.len(),
            1,
            "tashkeel in the body must not hide the match"
        );
        let hits = search(db.conn(), "مدرسه", None, 10).expect("search");
        assert_eq!(hits.len(), 1, "ة and ه are equivalent");
    }

    #[test]
    fn search_respects_folder_filter() {
        let db = db();
        let folder = crate::folders::create(db.conn(), None, "عمل", "📁", "#fff").expect("folder");
        let inside = create(db.conn(), Some(folder.id)).expect("create");
        update_content(db.conn(), inside.id, "تقرير المشروع").expect("update");
        let outside = create(db.conn(), None).expect("create");
        update_content(db.conn(), outside.id, "تقرير شخصي").expect("update");

        let hits = search(db.conn(), "تقرير", Some(folder.id), 10).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].note.id, inside.id);
    }

    #[test]
    fn trash_flow_works() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        soft_delete(db.conn(), note.id).expect("delete");
        assert_eq!(trash_count(db.conn()).expect("count"), 1);
        let trashed = list(
            db.conn(),
            &ListQuery {
                trash: true,
                ..ListQuery::default()
            },
        )
        .expect("list");
        assert_eq!(trashed.len(), 1);
        restore(db.conn(), note.id).expect("restore");
        assert_eq!(trash_count(db.conn()).expect("count"), 0);
    }

    #[test]
    fn purge_expired_removes_old_trash() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        db.conn()
            .execute(
                "UPDATE notes SET deleted_at = ?1 WHERE id = ?2",
                params![now_millis() - 40 * 86_400_000, note.id],
            )
            .expect("age");
        let removed = purge_expired(db.conn(), 30).expect("purge");
        assert_eq!(removed, 1);
    }

    #[test]
    fn locked_note_roundtrip() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        update_content(db.conn(), note.id, "سري جداً").expect("update");
        let locked = lock(db.conn(), note.id, "pass123").expect("lock");
        assert!(locked.is_locked);
        assert_ne!(locked.content, "سري جداً");
        assert_eq!(
            unlock(db.conn(), note.id, "pass123").expect("unlock"),
            "سري جداً"
        );
        assert!(unlock(db.conn(), note.id, "wrong").is_err());
    }

    #[test]
    fn locked_note_saves_reencrypted() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        update_content(db.conn(), note.id, "أول").expect("update");
        lock(db.conn(), note.id, "pass").expect("lock");
        save_locked(db.conn(), note.id, "pass", "ثاني").expect("save");
        assert_eq!(unlock(db.conn(), note.id, "pass").expect("unlock"), "ثاني");
    }

    #[test]
    fn locked_note_cannot_be_updated_in_plaintext() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        lock(db.conn(), note.id, "pass").expect("lock");
        assert!(update_content(db.conn(), note.id, "نص").is_err());
    }

    #[test]
    fn duplicate_copies_body() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        update_content(db.conn(), note.id, "محتوى").expect("update");
        let copy = duplicate(db.conn(), note.id).expect("dup");
        assert!(copy.title.contains("نسخة"));
        assert_eq!(copy.content, "محتوى");
    }

    #[test]
    fn source_path_lookup_works() {
        let db = db();
        let note = create(db.conn(), None).expect("create");
        set_source(
            db.conn(),
            note.id,
            Some("C:/tmp/a.txt"),
            Some("UTF-8"),
            Some("CRLF"),
        )
        .expect("source");
        let found = by_source_path(db.conn(), "C:/tmp/a.txt").expect("lookup");
        assert_eq!(found.map(|n| n.id), Some(note.id));
    }
}
