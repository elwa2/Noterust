//! AI provider connections (local servers such as Ollama / LM Studio and
//! cloud APIs such as OpenAI, Gemini and Anthropic).

use noterust_core::now_millis;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{DbError, Result};

/// A saved AI endpoint. The key is stored locally in the app database.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnection {
    pub id: i64,
    pub name: String,
    /// `openai` (any OpenAI-compatible server or API), `gemini` or `anthropic`.
    pub kind: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Payload used when saving a connection from the UI.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiConnectionDraft {
    pub id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

const SELECT: &str = "SELECT id, name, kind, base_url, api_key, model, created_at, updated_at
        FROM ai_connections";

fn map_connection(row: &rusqlite::Row<'_>) -> rusqlite::Result<AiConnection> {
    Ok(AiConnection {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        base_url: row.get("base_url")?,
        api_key: row.get("api_key")?,
        model: row.get("model")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn normalize_kind(kind: &str) -> &'static str {
    match kind {
        "gemini" => "gemini",
        "anthropic" => "anthropic",
        _ => "openai",
    }
}

/// Connection name fallback: the host, or the provider name.
fn fallback_name(kind: &str, base_url: &str) -> String {
    let host = base_url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .filter(|host| !host.is_empty());
    match host {
        Some(host) => host.to_string(),
        None => match kind {
            "gemini" => "Google Gemini".to_string(),
            "anthropic" => "Anthropic".to_string(),
            _ => "OpenAI-compatible".to_string(),
        },
    }
}

/// Lists connections oldest first.
pub fn list(conn: &Connection) -> Result<Vec<AiConnection>> {
    let mut statement = conn.prepare(&format!("{SELECT} ORDER BY created_at ASC"))?;
    let rows = statement.query_map([], map_connection)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Reads one connection.
pub fn get(conn: &Connection, id: i64) -> Result<AiConnection> {
    let found = conn
        .query_row(
            &format!("{SELECT} WHERE id = ?1"),
            params![id],
            map_connection,
        )
        .optional()?;
    found.ok_or(DbError::NotFound)
}

/// Creates or updates a connection and returns the stored row.
pub fn save(conn: &Connection, draft: &AiConnectionDraft) -> Result<AiConnection> {
    let kind = normalize_kind(&draft.kind);
    let base_url = draft.base_url.trim().trim_end_matches('/').to_string();
    let name = if draft.name.trim().is_empty() {
        fallback_name(kind, &base_url)
    } else {
        draft.name.trim().to_string()
    };
    let model = draft.model.trim().to_string();
    let api_key = draft.api_key.trim().to_string();
    let now = now_millis();

    if let Some(id) = draft.id {
        let changed = conn.execute(
            "UPDATE ai_connections
                SET name = ?1, kind = ?2, base_url = ?3, api_key = ?4, model = ?5, updated_at = ?6
              WHERE id = ?7",
            params![name, kind, base_url, api_key, model, now, id],
        )?;
        if changed == 0 {
            return Err(DbError::NotFound);
        }
        return get(conn, id);
    }

    conn.execute(
        "INSERT INTO ai_connections (name, kind, base_url, api_key, model, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![name, kind, base_url, api_key, model, now],
    )?;
    get(conn, conn.last_insert_rowid())
}

/// Deletes a connection.
pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    let changed = conn.execute("DELETE FROM ai_connections WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    fn draft(base_url: &str, kind: &str) -> AiConnectionDraft {
        AiConnectionDraft {
            id: None,
            name: String::new(),
            kind: kind.to_string(),
            base_url: base_url.to_string(),
            api_key: "sk-test".to_string(),
            model: "llama3.2".to_string(),
        }
    }

    #[test]
    fn save_list_update_delete() -> Result<()> {
        let db = Db::open_in_memory()?;
        let saved = save(db.conn(), &draft("http://127.0.0.1:11434/v1/", "openai"))?;
        // Trailing slash is trimmed and a host based name is used.
        assert_eq!(saved.name, "127.0.0.1:11434");
        assert_eq!(saved.base_url, "http://127.0.0.1:11434/v1");
        assert_eq!(saved.kind, "openai");
        assert_eq!(list(db.conn())?.len(), 1);

        let updated = save(
            db.conn(),
            &AiConnectionDraft {
                id: Some(saved.id),
                name: "My Ollama".to_string(),
                kind: "weird".to_string(),
                ..draft("http://127.0.0.1:11434/v1", "openai")
            },
        )?;
        assert_eq!(updated.name, "My Ollama");
        // Unknown kinds fall back to the OpenAI-compatible shape.
        assert_eq!(updated.kind, "openai");
        assert_eq!(get(db.conn(), saved.id)?.model, "llama3.2");

        delete(db.conn(), saved.id)?;
        assert!(list(db.conn())?.is_empty());
        assert!(matches!(get(db.conn(), saved.id), Err(DbError::NotFound)));
        Ok(())
    }
}
