//! Key/value settings store.

use std::collections::BTreeMap;

use rusqlite::{params, Connection};

use crate::Result;

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
    let value = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .ok();
    Ok(value)
}

pub fn get_or(conn: &Connection, key: &str, fallback: &str) -> Result<String> {
    Ok(get(conn, key)?.unwrap_or_else(|| fallback.to_string()))
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn all(conn: &Connection) -> Result<BTreeMap<String, String>> {
    let mut statement = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut map = BTreeMap::new();
    for row in rows {
        let (key, value) = row?;
        map.insert(key, value);
    }
    Ok(map)
}

pub fn get_i64(conn: &Connection, key: &str, fallback: i64) -> Result<i64> {
    Ok(get(conn, key)?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(fallback))
}

pub fn get_bool(conn: &Connection, key: &str, fallback: bool) -> Result<bool> {
    Ok(get(conn, key)?
        .map(|value| value == "true" || value == "1")
        .unwrap_or(fallback))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    #[test]
    fn set_and_get_roundtrip() {
        let db = Db::open_in_memory().expect("db");
        set(db.conn(), "theme", "light").expect("set");
        assert_eq!(get(db.conn(), "theme").expect("get"), Some("light".into()));
        set(db.conn(), "theme", "system").expect("set");
        assert_eq!(get(db.conn(), "theme").expect("get"), Some("system".into()));
    }

    #[test]
    fn typed_helpers_parse_values() {
        let db = Db::open_in_memory().expect("db");
        set(db.conn(), "font_size", "20").expect("set");
        set(db.conn(), "word_wrap", "false").expect("set");
        assert_eq!(get_i64(db.conn(), "font_size", 16).expect("i64"), 20);
        assert!(!get_bool(db.conn(), "word_wrap", true).expect("bool"));
        assert_eq!(get_i64(db.conn(), "missing", 7).expect("i64"), 7);
    }

    #[test]
    fn all_returns_sorted_map() {
        let db = Db::open_in_memory().expect("db");
        let map = all(db.conn()).expect("all");
        assert!(map.contains_key("theme"));
        let keys: Vec<&String> = map.keys().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }
}
