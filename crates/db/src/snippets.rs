//! Saved code snippets and prompts.

use noterust_core::{now_millis, Snippet};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

use crate::{DbError, Result};

/// Payload used when saving a snippet from the UI.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SnippetDraft {
    pub title: String,
    /// `code` or `prompt`; anything else falls back to `code`.
    pub kind: String,
    pub language: String,
    pub content: String,
    pub tags: String,
}

const SELECT: &str = "SELECT id, title, kind, language, content, tags, is_pinned, created_at,
        updated_at FROM snippets";

fn normalize_kind(kind: &str) -> &'static str {
    if kind == "prompt" {
        "prompt"
    } else {
        "code"
    }
}

fn map_snippet(row: &rusqlite::Row<'_>) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: row.get("id")?,
        title: row.get("title")?,
        kind: row.get("kind")?,
        language: row.get("language")?,
        content: row.get("content")?,
        tags: row.get("tags")?,
        is_pinned: row.get::<_, i64>("is_pinned")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// First non-empty line of `content`, trimmed to 80 characters.
fn title_from(content: &str) -> String {
    let line = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let trimmed = line.trim_start_matches('#').trim();
    trimmed.chars().take(80).collect()
}

/// Lists snippets, pinned first then newest, optionally filtered by kind and text.
pub fn list(conn: &Connection, query: Option<&str>, kind: Option<&str>) -> Result<Vec<Snippet>> {
    let pattern = query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));
    let kind = kind.map(normalize_kind);
    let sql = format!(
        "{SELECT} WHERE (?1 IS NULL OR kind = ?1)
           AND (?2 IS NULL OR title LIKE ?2 COLLATE NOCASE
                OR content LIKE ?2 COLLATE NOCASE
                OR tags LIKE ?2 COLLATE NOCASE)
         ORDER BY is_pinned DESC, updated_at DESC"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement
        .query_map(params![kind, pattern], map_snippet)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: i64) -> Result<Snippet> {
    let sql = format!("{SELECT} WHERE id = ?1");
    conn.query_row(&sql, params![id], map_snippet)
        .optional()?
        .ok_or(DbError::NotFound)
}

pub fn create(conn: &Connection, draft: &SnippetDraft) -> Result<Snippet> {
    let content = draft.content.clone();
    let title = if draft.title.trim().is_empty() {
        title_from(&content)
    } else {
        draft.title.trim().to_string()
    };
    if title.is_empty() && content.trim().is_empty() {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "اكتب كوداً أو برومبت أو عنواناً".into(),
        )));
    }
    let now = now_millis();
    conn.execute(
        "INSERT INTO snippets(title, kind, language, content, tags, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            title,
            normalize_kind(&draft.kind),
            draft.language.trim(),
            content,
            draft.tags.trim(),
            now
        ],
    )?;
    get(conn, conn.last_insert_rowid())
}

/// Updates the given fields; `None` leaves a field untouched.
pub fn update(
    conn: &Connection,
    id: i64,
    title: Option<&str>,
    kind: Option<&str>,
    language: Option<&str>,
    content: Option<&str>,
    tags: Option<&str>,
) -> Result<Snippet> {
    let current = get(conn, id)?;
    let title = match title {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => current.title.clone(),
    };
    let content = content.map(str::to_string).unwrap_or(current.content);
    let title = if title.trim().is_empty() && !content.trim().is_empty() {
        title_from(&content)
    } else {
        title
    };
    conn.execute(
        "UPDATE snippets SET title = ?1, kind = ?2, language = ?3, content = ?4, tags = ?5,
             updated_at = ?6
         WHERE id = ?7",
        params![
            title,
            kind.map(normalize_kind)
                .unwrap_or_else(|| normalize_kind(&current.kind)),
            language.map(str::trim).unwrap_or(current.language.as_str()),
            content,
            tags.map(str::trim).unwrap_or(current.tags.as_str()),
            now_millis(),
            id
        ],
    )?;
    get(conn, id)
}

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) -> Result<Snippet> {
    conn.execute(
        "UPDATE snippets SET is_pinned = ?1 WHERE id = ?2",
        params![pinned as i64, id],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn count(conn: &Connection) -> Result<i64> {
    let total = conn.query_row("SELECT COUNT(*) FROM snippets", [], |row| row.get(0))?;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    fn draft(title: &str, kind: &str, content: &str) -> SnippetDraft {
        SnippetDraft {
            title: title.to_string(),
            kind: kind.to_string(),
            language: "ts".to_string(),
            content: content.to_string(),
            tags: "tauri, rust".to_string(),
        }
    }

    #[test]
    fn create_and_list_snippets() {
        let db = Db::open_in_memory().expect("db");
        create(db.conn(), &draft("دالة", "code", "fn main() {}")).expect("code");
        create(db.conn(), &draft("برومبت", "prompt", "اكتب لي ملخصاً")).expect("prompt");
        assert_eq!(list(db.conn(), None, None).expect("list").len(), 2);
        assert_eq!(
            list(db.conn(), None, Some("prompt")).expect("list").len(),
            1
        );
        assert_eq!(count(db.conn()).expect("count"), 2);
    }

    #[test]
    fn search_matches_content_and_tags() {
        let db = Db::open_in_memory().expect("db");
        create(db.conn(), &draft("واحد", "code", "let x = 1;")).expect("one");
        create(db.conn(), &draft("اثنان", "code", "console.log(2)")).expect("two");
        assert_eq!(list(db.conn(), Some("let x"), None).expect("list").len(), 1);
        assert_eq!(list(db.conn(), Some("tauri"), None).expect("list").len(), 2);
    }

    #[test]
    fn untitled_snippet_takes_title_from_content() {
        let db = Db::open_in_memory().expect("db");
        let snippet =
            create(db.conn(), &draft("", "code", "دالة الحفظ\nثم بقية السطور")).expect("create");
        assert_eq!(snippet.title, "دالة الحفظ");
    }

    #[test]
    fn update_and_delete_snippet() {
        let db = Db::open_in_memory().expect("db");
        let snippet = create(db.conn(), &draft("أول", "code", "a")).expect("create");
        let updated = update(
            db.conn(),
            snippet.id,
            Some("ثانٍ"),
            Some("prompt"),
            Some("ar"),
            Some("b"),
            Some(""),
        )
        .expect("update");
        assert_eq!(updated.title, "ثانٍ");
        assert_eq!(updated.kind, "prompt");
        assert_eq!(updated.content, "b");
        set_pinned(db.conn(), snippet.id, true).expect("pin");
        assert!(get(db.conn(), snippet.id).expect("get").is_pinned);
        delete(db.conn(), snippet.id).expect("delete");
        assert!(get(db.conn(), snippet.id).is_err());
    }

    #[test]
    fn empty_snippet_is_rejected() {
        let db = Db::open_in_memory().expect("db");
        assert!(create(db.conn(), &draft("", "code", "   ")).is_err());
    }

    #[test]
    fn invalid_kind_falls_back_to_code() {
        let db = Db::open_in_memory().expect("db");
        let snippet = create(db.conn(), &draft("x", "weird", "y")).expect("create");
        assert_eq!(snippet.kind, "code");
    }
}
