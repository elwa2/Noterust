//! Tag repository.

use noterust_core::Tag;
use rusqlite::{params, Connection, OptionalExtension};

use crate::Result;

/// Lists every tag with the number of notes using it.
pub fn all(conn: &Connection) -> Result<Vec<(Tag, i64)>> {
    let mut statement = conn.prepare(
        "SELECT t.id, t.name, t.color,
                (SELECT COUNT(*) FROM note_tags nt JOIN notes n ON n.id = nt.note_id
                  WHERE nt.tag_id = t.id AND n.deleted_at IS NULL)
         FROM tags t ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                },
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Finds a tag by name or creates it.
pub fn ensure(conn: &Connection, name: &str, color: &str) -> Result<Tag> {
    let trimmed = name.trim().trim_start_matches('#');
    if trimmed.is_empty() {
        return Err(crate::DbError::Core(noterust_core::CoreError::Invalid(
            "اسم الوسم مطلوب".into(),
        )));
    }
    if let Some(tag) = find(conn, trimmed)? {
        return Ok(tag);
    }
    conn.execute(
        "INSERT INTO tags(name, color) VALUES (?1, ?2)",
        params![trimmed, color],
    )?;
    find(conn, trimmed)?.ok_or(crate::DbError::NotFound)
}

pub fn find(conn: &Connection, name: &str) -> Result<Option<Tag>> {
    Ok(conn
        .query_row(
            "SELECT id, name, color FROM tags WHERE name = ?1 COLLATE NOCASE",
            params![name.trim()],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            },
        )
        .optional()?)
}

/// Tags attached to one note.
pub fn for_note(conn: &Connection, note_id: i64) -> Result<Vec<Tag>> {
    let mut statement = conn.prepare(
        "SELECT t.id, t.name, t.color
         FROM tags t JOIN note_tags nt ON nt.tag_id = t.id
         WHERE nt.note_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let tags = statement
        .query_map(params![note_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}

/// Replaces the tag set of a note, creating missing tags.
pub fn set_for_note(conn: &Connection, note_id: i64, names: &[String]) -> Result<Vec<Tag>> {
    conn.execute("DELETE FROM note_tags WHERE note_id = ?1", params![note_id])?;
    for name in names {
        let tag = ensure(conn, name, "#8ab4f8")?;
        conn.execute(
            "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES (?1, ?2)",
            params![note_id, tag.id],
        )?;
    }
    for_note(conn, note_id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn set_color(conn: &Connection, id: i64, color: &str) -> Result<()> {
    conn.execute(
        "UPDATE tags SET color = ?1 WHERE id = ?2",
        params![color, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    #[test]
    fn tags_are_created_once_and_linked() {
        let db = Db::open_in_memory().expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        let tags = set_for_note(db.conn(), note.id, &["عمل".into(), "مهم".into()]).expect("tags");
        assert_eq!(tags.len(), 2);

        let again = ensure(db.conn(), "عمل", "#fff").expect("ensure");
        assert_eq!(again.name, "عمل");
        let all = all(db.conn()).expect("all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].1, 1);
    }

    #[test]
    fn replacing_tags_removes_old_links() {
        let db = Db::open_in_memory().expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        set_for_note(db.conn(), note.id, &["أ".into()]).expect("set");
        let tags = set_for_note(db.conn(), note.id, &["ب".into()]).expect("set");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "ب");
    }

    #[test]
    fn empty_tag_is_rejected() {
        let db = Db::open_in_memory().expect("db");
        assert!(ensure(db.conn(), "   ", "#fff").is_err());
    }
}
