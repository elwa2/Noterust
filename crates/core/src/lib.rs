//! Domain models shared by every layer of NoteRust.
//!
//! Timestamps are Unix milliseconds (`i64`) and money is stored in minor units
//! (piastres for EGP) to avoid floating point drift.

pub mod error;
pub mod models;
pub mod text;

pub use error::{CoreError, Result};
pub use models::{
    Category, CategoryTotal, ChecklistItem, DayTotal, Folder, FolderSummary, MonthlySummary, Note,
    NoteSummary, NoteVersion, SearchHit, Snippet, SortBy, Tag, TaskItem, TextCounts,
    TransactionItem,
};
pub use text::{
    arabic_datetime, counts, derive_title, detect_direction, extract_checklist,
    format_arabic_digits, preview, strip_markdown, truncate_chars,
};

/// Returns the current time as Unix milliseconds.
pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Converts Unix milliseconds into an RFC 3339 string (used by exports).
pub fn to_rfc3339(millis: i64) -> String {
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}
