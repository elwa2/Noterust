use serde::{Deserialize, Serialize};

/// A note folder; folders can be nested through `parent_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub sort_order: i64,
    pub created_at: i64,
}

impl Folder {
    /// Nesting depth below the root, computed from a flat folder list.
    pub fn depth(folders: &[Folder], id: i64) -> usize {
        let mut depth = 0;
        let mut current = folders
            .iter()
            .find(|f| f.id == id)
            .and_then(|f| f.parent_id);
        let mut guard = 0;
        while let Some(parent) = current {
            depth += 1;
            guard += 1;
            if guard > 32 {
                break;
            }
            current = folders
                .iter()
                .find(|f| f.id == parent)
                .and_then(|f| f.parent_id);
        }
        depth
    }
}

/// Folder row plus the counters shown in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSummary {
    #[serde(flatten)]
    pub folder: Folder,
    pub note_count: i64,
    pub children: Vec<FolderSummary>,
}

/// A full note, including its body and tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub content: String,
    pub content_format: String,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub is_locked: bool,
    pub color: Option<String>,
    pub direction: String,
    pub source_path: Option<String>,
    pub encoding: Option<String>,
    pub line_ending: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub tags: Vec<Tag>,
}

/// Lightweight note row used by the middle column.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub preview: String,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub is_archived: bool,
    pub color: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub word_count: i64,
    pub tags: Vec<Tag>,
}

/// A search result: a summary plus the highlighted FTS5 snippet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    #[serde(flatten)]
    pub note: NoteSummary,
    pub snippet: String,
}

/// A tag that can be attached to notes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
}

/// One entry of a note's version history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteVersion {
    pub id: i64,
    pub note_id: i64,
    pub content: String,
    pub title: String,
    pub saved_at: i64,
}

/// A task, either standalone or linked to a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    pub id: i64,
    pub note_id: Option<i64>,
    pub note_title: Option<String>,
    pub title: String,
    pub description: String,
    pub due_at: Option<i64>,
    pub priority: i64,
    pub is_done: bool,
    pub repeat_rule: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    /// True for tasks mirrored from `- [ ]` checklist lines in a note.
    pub from_checklist: bool,
}

/// A saved code snippet or prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: i64,
    pub title: String,
    /// `code` or `prompt`.
    pub kind: String,
    pub language: String,
    pub content: String,
    /// Free-form comma separated tags.
    pub tags: String,
    pub is_pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A income/expense transaction. `amount_minor` is in piastres.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionItem {
    pub id: i64,
    pub kind: String,
    pub amount_minor: i64,
    pub currency: String,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub category_color: Option<String>,
    pub note: Option<String>,
    pub occurred_at: i64,
}

/// An income/expense category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub color: String,
}

/// Totals for one month plus a per-category breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySummary {
    pub year: i32,
    pub month: u32,
    pub income_minor: i64,
    pub expense_minor: i64,
    pub balance_minor: i64,
    pub currency: String,
    pub by_category: Vec<CategoryTotal>,
    pub by_day: Vec<DayTotal>,
}

/// Aggregate for a single category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTotal {
    pub category_id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub color: String,
    pub total_minor: i64,
}

/// Aggregate for a single day of the month.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayTotal {
    pub day: u32,
    pub income_minor: i64,
    pub expense_minor: i64,
}

/// A checklist entry parsed out of note content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

/// Word/character/line counters for the status bar.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextCounts {
    pub words: i64,
    pub chars: i64,
    pub chars_no_spaces: i64,
    pub lines: i64,
}

/// Sort order for the note list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SortBy {
    #[default]
    Updated,
    Created,
    Title,
}

impl SortBy {
    pub fn from_name(value: &str) -> Self {
        match value {
            "created" => SortBy::Created,
            "title" => SortBy::Title,
            _ => SortBy::Updated,
        }
    }

    pub fn as_sql(self) -> &'static str {
        match self {
            SortBy::Updated => "n.updated_at DESC",
            SortBy::Created => "n.created_at DESC",
            SortBy::Title => "n.title COLLATE NOCASE ASC",
        }
    }
}
