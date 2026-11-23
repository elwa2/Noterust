//! Note version history.

use noterust_core::{now_millis, Note, NoteVersion};
use rusqlite::{params, Connection};

use crate::Result;

/// How old the newest snapshot must be before another one is stored, so that
/// autosave does not create hundreds of versions per minute.
const MIN_INTERVAL_MS: i64 = 60_000;
/// Maximum snapshots kept per note.
const MAX_VERSIONS: i64 = 60;

/// Stores a snapshot of `note` when it differs from the newest one.
pub fn snapshot(conn: &Connection, note: &Note) -> Result<()> {
    if note.is_locked {
        return Ok(());
    }
    let newest: Option<(String, i64)> = conn
        .query_row(
            "SELECT content, saved_at FROM note_versions WHERE note_id = ?1
             ORDER BY saved_at DESC LIMIT 1",
            params![note.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let now = now_millis();
    if let Some((content, saved_at)) = newest {
        if content == note.content || now - saved_at < MIN_INTERVAL_MS {
            return Ok(());
        }
    }

    conn.execute(
        "INSERT INTO note_versions(note_id, content, title, saved_at) VALUES (?1, ?2, ?3, ?4)",
        params![note.id, note.content, note.title, now],
    )?;
    conn.execute(
        "DELETE FROM note_versions WHERE note_id = ?1 AND id NOT IN (
             SELECT id FROM note_versions WHERE note_id = ?1
             ORDER BY saved_at DESC LIMIT ?2)",
        params![note.id, MAX_VERSIONS],
    )?;
    Ok(())
}

/// All versions of a note, newest first.
pub fn list(conn: &Connection, note_id: i64) -> Result<Vec<NoteVersion>> {
    let mut statement = conn.prepare(
        "SELECT id, note_id, content, title, saved_at FROM note_versions
         WHERE note_id = ?1 ORDER BY saved_at DESC",
    )?;
    let versions = statement
        .query_map(params![note_id], |row| {
            Ok(NoteVersion {
                id: row.get(0)?,
                note_id: row.get(1)?,
                content: row.get(2)?,
                title: row.get(3)?,
                saved_at: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(versions)
}

/// Restores a version, snapshotting the current text first.
pub fn restore(conn: &Connection, version_id: i64) -> Result<i64> {
    let (note_id, content): (i64, String) = conn.query_row(
        "SELECT note_id, content FROM note_versions WHERE id = ?1",
        params![version_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let current = crate::notes::get(conn, note_id)?;
    snapshot(conn, &current)?;
    crate::notes::update_content(conn, note_id, &content)?;
    Ok(note_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    #[test]
    fn snapshots_are_throttled_and_restorable() {
        let db = Db::open_in_memory().expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        crate::notes::update_content(db.conn(), note.id, "الإصدار الأول").expect("update");

        // Age the last snapshot so a new one is allowed.
        db.conn()
            .execute("UPDATE note_versions SET saved_at = saved_at - 120000", [])
            .expect("age");
        crate::notes::update_content(db.conn(), note.id, "الإصدار الثاني").expect("update");

        let versions = list(db.conn(), note.id).expect("list");
        assert!(!versions.is_empty());

        let oldest = versions
            .iter()
            .find(|version| version.content == "الإصدار الأول")
            .expect("first revision")
            .clone();
        restore(db.conn(), oldest.id).expect("restore");
        let restored = crate::notes::get(db.conn(), note.id).expect("get");
        assert_eq!(restored.content, "الإصدار الأول");
    }

    #[test]
    fn unchanged_content_creates_no_version() {
        let db = Db::open_in_memory().expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        crate::notes::update_content(db.conn(), note.id, "ثابت").expect("update");
        let current = crate::notes::get(db.conn(), note.id).expect("get");
        // First explicit snapshot is stored, a second identical one is not.
        snapshot(db.conn(), &current).expect("snapshot");
        let before = list(db.conn(), note.id).expect("list").len();
        snapshot(db.conn(), &current).expect("snapshot");
        assert_eq!(list(db.conn(), note.id).expect("list").len(), before);
    }
}
