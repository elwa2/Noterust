//! Task repository, including the checklist mirror.

use noterust_core::{extract_checklist, now_millis, TaskItem};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

use crate::{DbError, Result};

/// Payload used when creating a task from the UI.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TaskDraft {
    pub note_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub due_at: Option<i64>,
    /// 1 = مرتفعة, 2 = متوسطة, 3 = منخفضة
    pub priority: i64,
    pub repeat_rule: Option<String>,
}

/// Marker stored in `repeat_rule` for tasks mirrored from note checklists.
pub const CHECKLIST_MARKER: &str = "checklist";

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskItem> {
    let repeat_rule: Option<String> = row.get("repeat_rule")?;
    Ok(TaskItem {
        id: row.get("id")?,
        note_id: row.get("note_id")?,
        note_title: row.get("note_title").ok(),
        title: row.get("title")?,
        description: row.get("description")?,
        due_at: row.get("due_at")?,
        priority: row.get("priority")?,
        is_done: row.get::<_, i64>("is_done")? != 0,
        from_checklist: repeat_rule.as_deref() == Some(CHECKLIST_MARKER),
        repeat_rule,
        created_at: row.get("created_at")?,
        completed_at: row.get("completed_at")?,
    })
}

const TASK_SELECT: &str = "SELECT t.id, t.note_id, t.title, t.description, t.due_at, t.priority,
        t.is_done, t.repeat_rule, t.created_at, t.completed_at, n.title AS note_title
     FROM tasks t LEFT JOIN notes n ON n.id = t.note_id";

/// Lists tasks, newest priority first.
pub fn list(conn: &Connection, include_done: bool) -> Result<Vec<TaskItem>> {
    let sql = format!(
        "{TASK_SELECT} WHERE (?1 = 1 OR t.is_done = 0) AND n.deleted_at IS NULL
         ORDER BY t.is_done, t.priority, t.due_at IS NULL, t.due_at, t.created_at DESC"
    );
    let mut statement = conn.prepare(&sql)?;
    let tasks = statement
        .query_map(params![include_done as i64], map_task)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tasks)
}

pub fn get(conn: &Connection, id: i64) -> Result<TaskItem> {
    let sql = format!("{TASK_SELECT} WHERE t.id = ?1");
    conn.query_row(&sql, params![id], map_task)
        .optional()?
        .ok_or(DbError::NotFound)
}

/// Number of open tasks (used by the sidebar badge).
pub fn open_count(conn: &Connection) -> Result<i64> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM tasks t LEFT JOIN notes n ON n.id = t.note_id
         WHERE t.is_done = 0 AND n.deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn create(conn: &Connection, draft: &TaskDraft) -> Result<TaskItem> {
    let title = draft.title.trim();
    if title.is_empty() {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "عنوان المهمة مطلوب".into(),
        )));
    }
    let priority = if (1..=3).contains(&draft.priority) {
        draft.priority
    } else {
        2
    };
    conn.execute(
        "INSERT INTO tasks(note_id, title, description, due_at, priority, repeat_rule, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            draft.note_id,
            title,
            draft.description.trim(),
            draft.due_at,
            priority,
            draft.repeat_rule,
            now_millis()
        ],
    )?;
    get(conn, conn.last_insert_rowid())
}

/// Updates the editable fields of a task.
pub fn update(
    conn: &Connection,
    id: i64,
    title: Option<&str>,
    description: Option<&str>,
    due_at: Option<Option<i64>>,
    priority: Option<i64>,
    repeat_rule: Option<Option<String>>,
) -> Result<TaskItem> {
    let current = get(conn, id)?;
    let title = match title {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => current.title,
    };
    let description = description
        .map(|value| value.to_string())
        .unwrap_or(current.description);
    let due = due_at.unwrap_or(current.due_at);
    let priority = priority
        .filter(|value| (1..=3).contains(value))
        .unwrap_or(current.priority);
    let repeat = repeat_rule.unwrap_or(current.repeat_rule);

    conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, due_at = ?3, priority = ?4, repeat_rule = ?5
         WHERE id = ?6",
        params![title, description, due, priority, repeat, id],
    )?;
    get(conn, id)
}

/// Next occurrence of a repeating task, advanced past `now` in whole steps.
fn next_due(due: i64, rule: &str) -> Option<i64> {
    let now = now_millis();
    let mut at = due;
    let mut guard = 0;
    while at <= now && guard < 500 {
        let dt = chrono::DateTime::from_timestamp_millis(at)?.with_timezone(&chrono::Local);
        let next = match rule {
            "daily" => dt.checked_add_days(chrono::Days::new(1)),
            "weekly" => dt.checked_add_days(chrono::Days::new(7)),
            "monthly" => dt.checked_add_months(chrono::Months::new(1)),
            "yearly" => dt.checked_add_months(chrono::Months::new(12)),
            _ => return None,
        }?;
        at = next.timestamp_millis();
        guard += 1;
    }
    (at > now).then_some(at)
}

pub fn set_done(conn: &Connection, id: i64, done: bool) -> Result<TaskItem> {
    let task = get(conn, id)?;
    // Completing a recurring task rolls it forward instead of closing it.
    if done && !task.from_checklist {
        if let (Some(rule), Some(due)) = (task.repeat_rule.as_deref(), task.due_at) {
            if let Some(next) = next_due(due, rule) {
                conn.execute(
                    "UPDATE tasks SET due_at = ?1, is_done = 0, completed_at = ?2 WHERE id = ?3",
                    params![next, Some(now_millis()), id],
                )?;
                return get(conn, id);
            }
        }
    }
    let completed_at = if done { Some(now_millis()) } else { None };
    conn.execute(
        "UPDATE tasks SET is_done = ?1, completed_at = ?2 WHERE id = ?3",
        params![done as i64, completed_at, id],
    )?;
    // Keep the checklist line in the note in sync when the task came from one.
    if task.from_checklist {
        if let Some(note_id) = task.note_id {
            let note = crate::notes::get(conn, note_id)?;
            let updated = toggle_checklist_line(&note.content, &task.title, done);
            if updated != note.content {
                crate::notes::update_content(conn, note_id, &updated)?;
            }
        }
    }
    get(conn, id)
}

fn toggle_checklist_line(content: &str, title: &str, done: bool) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.lines() {
        let trimmed = line.trim_start();
        let is_unchecked = trimmed.starts_with("- [ ] ");
        let is_checked = trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ");
        if is_unchecked || is_checked {
            let text = trimmed
                .trim_start_matches("- [ ] ")
                .trim_start_matches("- [x] ")
                .trim_start_matches("- [X] ")
                .trim();
            if text == title {
                let prefix = if done { "- [x] " } else { "- [ ] " };
                out.push_str(prefix);
                out.push_str(text);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if content.ends_with('\n') {
        out
    } else {
        out.trim_end_matches('\n').to_string()
    }
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(())
}

/// Rebuilds the mirror tasks for `- [ ]` lines in a note.
pub fn sync_from_checklist(conn: &Connection, note_id: i64, content: &str) -> Result<()> {
    let existing: Vec<(i64, String, i64)> = {
        let mut statement = conn.prepare(
            "SELECT id, title, is_done FROM tasks WHERE note_id = ?1 AND repeat_rule = ?2",
        )?;
        let rows = statement
            .query_map(params![note_id, CHECKLIST_MARKER], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let items = extract_checklist(content);
    let mut kept: Vec<i64> = Vec::new();
    for item in items {
        if let Some((id, _, done)) = existing.iter().find(|(_, title, _)| title == &item.text) {
            kept.push(*id);
            let is_done = if item.done { 1 } else { 0 };
            if *done != is_done {
                conn.execute(
                    "UPDATE tasks SET is_done = ?1, completed_at = ?2 WHERE id = ?3",
                    params![
                        is_done,
                        if item.done { Some(now_millis()) } else { None },
                        id
                    ],
                )?;
            }
            continue;
        }
        conn.execute(
            "INSERT INTO tasks(note_id, title, priority, is_done, repeat_rule, created_at, completed_at)
             VALUES (?1, ?2, 2, ?3, ?4, ?5, ?6)",
            params![
                note_id,
                item.text,
                item.done as i64,
                CHECKLIST_MARKER,
                now_millis(),
                if item.done { Some(now_millis()) } else { None }
            ],
        )?;
        kept.push(conn.last_insert_rowid());
    }

    for (id, _, _) in existing {
        if !kept.contains(&id) {
            conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    fn draft(title: &str) -> TaskDraft {
        TaskDraft {
            title: title.to_string(),
            ..TaskDraft::default()
        }
    }

    #[test]
    fn create_list_and_complete() {
        let db = Db::open_in_memory().expect("db");
        let task = create(db.conn(), &draft("مراجعة")).expect("create");
        assert_eq!(task.priority, 2);
        assert_eq!(open_count(db.conn()).expect("count"), 1);
        set_done(db.conn(), task.id, true).expect("done");
        assert_eq!(open_count(db.conn()).expect("count"), 0);
        assert!(list(db.conn(), false).expect("list").is_empty());
        assert_eq!(list(db.conn(), true).expect("list").len(), 1);
    }

    #[test]
    fn checklist_lines_become_tasks() {
        let db = Db::open_in_memory().expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        crate::notes::update_content(db.conn(), note.id, "- [ ] أولاً\n- [x] ثانياً").expect("update");
        let tasks = list(db.conn(), true).expect("list");
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|task| task.from_checklist));
    }

    #[test]
    fn removing_checklist_line_removes_task() {
        let db = Db::open_in_memory().expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        crate::notes::update_content(db.conn(), note.id, "- [ ] مهمة").expect("update");
        assert_eq!(list(db.conn(), true).expect("list").len(), 1);
        crate::notes::update_content(db.conn(), note.id, "لا مهام").expect("update");
        assert!(list(db.conn(), true).expect("list").is_empty());
    }

    #[test]
    fn completing_from_tasks_updates_note_text() {
        let db = Db::open_in_memory().expect("db");
        let note = crate::notes::create(db.conn(), None).expect("note");
        crate::notes::update_content(db.conn(), note.id, "- [ ] اشتري خبز").expect("update");
        let task = list(db.conn(), true).expect("list").remove(0);
        set_done(db.conn(), task.id, true).expect("done");
        let refreshed = crate::notes::get(db.conn(), note.id).expect("get");
        assert!(refreshed.content.contains("- [x] اشتري خبز"));
    }

    #[test]
    fn task_update_and_delete() {
        let db = Db::open_in_memory().expect("db");
        let task = create(db.conn(), &draft("مهمة")).expect("create");
        let updated = update(
            db.conn(),
            task.id,
            Some("مهمة معدلة"),
            None,
            Some(Some(5)),
            Some(1),
            None,
        )
        .expect("update");
        assert_eq!(updated.title, "مهمة معدلة");
        assert_eq!(updated.priority, 1);
        delete(db.conn(), task.id).expect("delete");
        assert!(get(db.conn(), task.id).is_err());
    }

    #[test]
    fn completing_a_recurring_task_moves_it_forward() {
        let db = Db::open_in_memory().expect("db");
        let due = crate::finance::month_range(2026, 11).0;
        let mut draft = draft("تمرين");
        draft.due_at = Some(due);
        draft.repeat_rule = Some("daily".to_string());
        let task = create(db.conn(), &draft).expect("create");
        let after = set_done(db.conn(), task.id, true).expect("done");
        assert!(!after.is_done, "recurring task stays open");
        assert!(after.due_at.expect("due") > now_millis());
    }

    #[test]
    fn completing_a_plain_task_closes_it() {
        let db = Db::open_in_memory().expect("db");
        let mut draft = draft("عادية");
        draft.due_at = Some(crate::finance::month_range(2026, 11).0);
        let task = create(db.conn(), &draft).expect("create");
        let after = set_done(db.conn(), task.id, true).expect("done");
        assert!(after.is_done);
    }

    #[test]
    fn empty_title_is_rejected() {
        let db = Db::open_in_memory().expect("db");
        assert!(create(db.conn(), &draft("   ")).is_err());
    }
}
