//! Folder repository.

use noterust_core::{now_millis, Folder, FolderSummary};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{DbError, Result};

/// A folder plus its direct note count.
pub struct FolderRow {
    pub folder: Folder,
    pub note_count: i64,
}

/// Creates a folder under `parent_id`.
pub fn create(
    conn: &Connection,
    parent_id: Option<i64>,
    name: &str,
    icon: &str,
    color: &str,
) -> Result<Folder> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "اسم المجلد مطلوب".into(),
        )));
    }
    if let Some(parent) = parent_id {
        get(conn, parent)?;
    }
    let sort_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM folders WHERE parent_id IS ?1",
        params![parent_id],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO folders(parent_id, name, icon, color, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![parent_id, trimmed, icon, color, sort_order, now_millis()],
    )?;
    get(conn, conn.last_insert_rowid())
}

/// Loads a single folder.
pub fn get(conn: &Connection, id: i64) -> Result<Folder> {
    conn.query_row(
        "SELECT id, parent_id, name, icon, color, sort_order, created_at
         FROM folders WHERE id = ?1",
        params![id],
        |row| {
            Ok(Folder {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                icon: row.get(3)?,
                color: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or(DbError::NotFound)
}

/// Lists all folders ordered by position.
pub fn list(conn: &Connection) -> Result<Vec<FolderRow>> {
    let mut statement = conn.prepare(
        "SELECT f.id, f.parent_id, f.name, f.icon, f.color, f.sort_order, f.created_at,
                (SELECT COUNT(*) FROM notes n
                  WHERE n.folder_id = f.id AND n.deleted_at IS NULL AND n.is_archived = 0)
         FROM folders f
         ORDER BY f.parent_id IS NOT NULL, f.sort_order, f.name COLLATE NOCASE",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(FolderRow {
                folder: Folder {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    name: row.get(2)?,
                    icon: row.get(3)?,
                    color: row.get(4)?,
                    sort_order: row.get(5)?,
                    created_at: row.get(6)?,
                },
                note_count: row.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Builds the nested folder tree used by the sidebar.
pub fn tree(conn: &Connection) -> Result<Vec<FolderSummary>> {
    let rows = list(conn)?;
    Ok(build_tree(&rows, None, 0))
}

fn build_tree(rows: &[FolderRow], parent: Option<i64>, depth: usize) -> Vec<FolderSummary> {
    if depth > 8 {
        return Vec::new();
    }
    rows.iter()
        .filter(|row| row.folder.parent_id == parent)
        .map(|row| FolderSummary {
            folder: row.folder.clone(),
            note_count: row.note_count,
            children: build_tree(rows, Some(row.folder.id), depth + 1),
        })
        .collect()
}

/// Updates a folder's name, icon, colour or parent.
pub fn update(
    conn: &Connection,
    id: i64,
    name: Option<&str>,
    icon: Option<&str>,
    color: Option<&str>,
    parent_id: Option<Option<i64>>,
) -> Result<Folder> {
    let current = get(conn, id)?;
    let name = match name {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => current.name,
    };
    let icon = icon.unwrap_or(&current.icon).to_string();
    let color = color.unwrap_or(&current.color).to_string();

    if let Some(Some(new_parent)) = parent_id {
        if new_parent == id {
            return Err(DbError::Core(noterust_core::CoreError::Invalid(
                "لا يمكن نقل المجلد إلى نفسه".into(),
            )));
        }
        if is_descendant(conn, new_parent, id)? {
            return Err(DbError::Core(noterust_core::CoreError::Invalid(
                "لا يمكن نقل المجلد إلى أحد مجلداته الفرعية".into(),
            )));
        }
    }

    let resolved_parent = parent_id.unwrap_or(current.parent_id);
    conn.execute(
        "UPDATE folders SET name = ?1, icon = ?2, color = ?3, parent_id = ?4 WHERE id = ?5",
        params![name, icon, color, resolved_parent, id],
    )?;
    get(conn, id)
}

/// True when `candidate` is inside `ancestor`'s subtree.
pub fn is_descendant(conn: &Connection, candidate: i64, ancestor: i64) -> Result<bool> {
    let mut current = Some(candidate);
    let mut guard = 0;
    while let Some(id) = current {
        guard += 1;
        if guard > 32 {
            return Ok(true);
        }
        if id == ancestor {
            return Ok(true);
        }
        current = conn
            .query_row(
                "SELECT parent_id FROM folders WHERE id = ?1",
                params![id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();
    }
    Ok(false)
}

/// Moves a folder up or down among its siblings.
pub fn reorder(conn: &Connection, id: i64, new_index: i64) -> Result<()> {
    let folder = get(conn, id)?;
    let mut siblings = list(conn)?;
    siblings.retain(|row| row.folder.parent_id == folder.parent_id);
    // The list is global-ordered; re-sort siblings by their own sort_order.
    siblings.sort_by_key(|row| row.folder.sort_order);

    let Some(position) = siblings.iter().position(|row| row.folder.id == id) else {
        return Err(DbError::NotFound);
    };
    let target = new_index.clamp(0, siblings.len() as i64 - 1) as usize;
    let mut order: Vec<i64> = siblings.iter().map(|row| row.folder.id).collect();
    let moved = order.remove(position);
    order.insert(target, moved);

    for (index, folder_id) in order.iter().enumerate() {
        conn.execute(
            "UPDATE folders SET sort_order = ?1 WHERE id = ?2",
            params![index as i64, folder_id],
        )?;
    }
    Ok(())
}

/// Deletes a folder; notes inside move to "all notes" and children are removed.
pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    get(conn, id)?;
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
    Ok(())
}
