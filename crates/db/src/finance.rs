//! Income/expense tracking.

use chrono::{Datelike, Local, TimeZone};
use noterust_core::{
    now_millis, Category, CategoryTotal, DayTotal, MonthlySummary, TransactionItem,
};
use rusqlite::{params, Connection};
use serde::Deserialize;

use crate::{DbError, Result};

/// Payload used when adding a transaction.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDraft {
    pub kind: String,
    /// Amount in piastres (EGP * 100).
    pub amount_minor: i64,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub category_id: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
    pub occurred_at: i64,
}

/// All categories, grouped by kind in the UI.
pub fn categories(conn: &Connection) -> Result<Vec<Category>> {
    let mut statement = conn.prepare(
        "SELECT id, name, kind, color FROM categories ORDER BY kind, name COLLATE NOCASE",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                color: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn create_category(conn: &Connection, name: &str, kind: &str, color: &str) -> Result<Category> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "اسم الفئة مطلوب".into(),
        )));
    }
    if !matches!(kind, "income" | "expense") {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "نوع الفئة يجب أن يكون income أو expense".into(),
        )));
    }
    conn.execute(
        "INSERT OR IGNORE INTO categories(name, kind, color) VALUES (?1, ?2, ?3)",
        params![trimmed, kind, color],
    )?;
    conn.query_row(
        "SELECT id, name, kind, color FROM categories WHERE name = ?1 AND kind = ?2",
        params![trimmed, kind],
        |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                color: row.get(3)?,
            })
        },
    )
    .map_err(DbError::from)
}

/// Adds a transaction and returns the stored row.
pub fn add_transaction(conn: &Connection, draft: &TransactionDraft) -> Result<TransactionItem> {
    if !matches!(draft.kind.as_str(), "income" | "expense") {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "نوع المعاملة غير صالح".into(),
        )));
    }
    if draft.amount_minor <= 0 {
        return Err(DbError::Core(noterust_core::CoreError::Invalid(
            "المبلغ يجب أن يكون أكبر من صفر".into(),
        )));
    }
    let currency = draft.currency.clone().unwrap_or_else(|| "EGP".to_string());
    conn.execute(
        "INSERT INTO transactions(kind, amount_minor, currency, category_id, note, occurred_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            draft.kind,
            draft.amount_minor,
            currency,
            draft.category_id,
            draft.note,
            draft.occurred_at,
            now_millis()
        ],
    )?;
    get_transaction(conn, conn.last_insert_rowid())
}

pub fn get_transaction(conn: &Connection, id: i64) -> Result<TransactionItem> {
    conn.query_row(
        "SELECT t.id, t.kind, t.amount_minor, t.currency, t.category_id, c.name, c.color,
                t.note, t.occurred_at
         FROM transactions t LEFT JOIN categories c ON c.id = t.category_id
         WHERE t.id = ?1",
        params![id],
        map_transaction,
    )
    .map_err(DbError::from)
}

fn map_transaction(row: &rusqlite::Row<'_>) -> rusqlite::Result<TransactionItem> {
    Ok(TransactionItem {
        id: row.get(0)?,
        kind: row.get(1)?,
        amount_minor: row.get(2)?,
        currency: row.get(3)?,
        category_id: row.get(4)?,
        category_name: row.get(5)?,
        category_color: row.get(6)?,
        note: row.get(7)?,
        occurred_at: row.get(8)?,
    })
}

/// Lists transactions inside a time range, newest first.
pub fn list_transactions(
    conn: &Connection,
    from: i64,
    to: i64,
    kind: Option<&str>,
) -> Result<Vec<TransactionItem>> {
    let mut statement = conn.prepare(
        "SELECT t.id, t.kind, t.amount_minor, t.currency, t.category_id, c.name, c.color,
                t.note, t.occurred_at
         FROM transactions t LEFT JOIN categories c ON c.id = t.category_id
         WHERE t.occurred_at >= ?1 AND t.occurred_at < ?2 AND (?3 IS NULL OR t.kind = ?3)
         ORDER BY t.occurred_at DESC, t.id DESC",
    )?;
    let rows = statement
        .query_map(params![from, to, kind], map_transaction)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn delete_transaction(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM transactions WHERE id = ?1", params![id])?;
    Ok(())
}

/// Unix millisecond bounds of a month in local time.
pub fn month_range(year: i32, month: u32) -> (i64, i64) {
    let start = Local
        .with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0);
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let end = Local
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(start);
    (start, end)
}

/// Monthly totals, per-category and per-day breakdowns.
pub fn monthly_summary(conn: &Connection, year: i32, month: u32) -> Result<MonthlySummary> {
    let (from, to) = month_range(year, month);

    let mut income = 0i64;
    let mut expense = 0i64;
    {
        let mut statement = conn.prepare(
            "SELECT kind, COALESCE(SUM(amount_minor), 0) FROM transactions
             WHERE occurred_at >= ?1 AND occurred_at < ?2 GROUP BY kind",
        )?;
        let rows = statement.query_map(params![from, to], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (kind, total) = row?;
            if kind == "income" {
                income = total;
            } else {
                expense = total;
            }
        }
    }

    let by_category = {
        let mut statement = conn.prepare(
            "SELECT t.category_id, COALESCE(c.name, 'غير مصنف'), t.kind, COALESCE(c.color, '#8ab4f8'),
                    SUM(t.amount_minor)
             FROM transactions t LEFT JOIN categories c ON c.id = t.category_id
             WHERE t.occurred_at >= ?1 AND t.occurred_at < ?2
             GROUP BY t.category_id, t.kind
             ORDER BY SUM(t.amount_minor) DESC",
        )?;
        let rows = statement
            .query_map(params![from, to], |row| {
                Ok(CategoryTotal {
                    category_id: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    color: row.get(3)?,
                    total_minor: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let mut by_day: Vec<DayTotal> = Vec::new();
    {
        let mut statement = conn.prepare(
            "SELECT occurred_at, kind, SUM(amount_minor) FROM transactions
             WHERE occurred_at >= ?1 AND occurred_at < ?2 GROUP BY occurred_at, kind",
        )?;
        let rows = statement.query_map(params![from, to], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (at, kind, total) = row?;
            let Some(dt) = chrono::DateTime::from_timestamp_millis(at) else {
                continue;
            };
            let day = dt.with_timezone(&Local).day();
            let entry = by_day.iter_mut().find(|item| item.day == day);
            match (entry, kind.as_str()) {
                (Some(entry), "income") => entry.income_minor += total,
                (Some(entry), _) => entry.expense_minor += total,
                (None, "income") => by_day.push(DayTotal {
                    day,
                    income_minor: total,
                    expense_minor: 0,
                }),
                (None, _) => by_day.push(DayTotal {
                    day,
                    income_minor: 0,
                    expense_minor: total,
                }),
            }
        }
    }
    by_day.sort_by_key(|item| item.day);

    let currency = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'default_currency'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "EGP".to_string());

    Ok(MonthlySummary {
        year,
        month,
        income_minor: income,
        expense_minor: expense,
        balance_minor: income - expense,
        currency,
        by_category,
        by_day,
    })
}

/// Exports transactions of a month as CSV (UTF-8 with BOM for Excel).
pub fn export_csv(conn: &Connection, from: i64, to: i64) -> Result<String> {
    let rows = list_transactions(conn, from, to, None)?;
    let mut csv = String::from("\u{feff}التاريخ,النوع,المبلغ,العملة,الفئة,ملاحظة\n");
    for row in rows {
        let date = chrono::DateTime::from_timestamp_millis(row.occurred_at)
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let kind = if row.kind == "income" {
            "دخل"
        } else {
            "مصروف"
        };
        let amount = format!("{:.2}", row.amount_minor as f64 / 100.0);
        let note = row.note.unwrap_or_default().replace(',', "،");
        csv.push_str(&format!(
            "{date},{kind},{amount},{},{},{note}\n",
            row.currency,
            row.category_name.unwrap_or_else(|| "غير مصنف".into())
        ));
    }
    Ok(csv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    fn draft(kind: &str, amount: i64) -> TransactionDraft {
        TransactionDraft {
            kind: kind.to_string(),
            amount_minor: amount,
            currency: None,
            category_id: None,
            note: None,
            occurred_at: crate::finance::month_range(2026, 11).0 + 3_600_000,
        }
    }

    #[test]
    fn summary_totals_income_and_expense() {
        let db = Db::open_in_memory().expect("db");
        add_transaction(db.conn(), &draft("income", 50_000)).expect("income");
        add_transaction(db.conn(), &draft("expense", 12_050)).expect("expense");
        let summary = monthly_summary(db.conn(), 2026, 11).expect("summary");
        assert_eq!(summary.income_minor, 50_000);
        assert_eq!(summary.expense_minor, 12_050);
        assert_eq!(summary.balance_minor, 50_000 - 12_050);
        assert_eq!(summary.currency, "EGP");
    }

    #[test]
    fn invalid_transactions_are_rejected() {
        let db = Db::open_in_memory().expect("db");
        assert!(add_transaction(db.conn(), &draft("expense", 0)).is_err());
        assert!(add_transaction(db.conn(), &draft("other", 100)).is_err());
    }

    #[test]
    fn filtering_by_kind_works() {
        let db = Db::open_in_memory().expect("db");
        add_transaction(db.conn(), &draft("income", 100)).expect("income");
        add_transaction(db.conn(), &draft("expense", 50)).expect("expense");
        let (from, to) = month_range(2026, 11);
        assert_eq!(
            list_transactions(db.conn(), from, to, Some("income"))
                .expect("list")
                .len(),
            1
        );
    }

    #[test]
    fn csv_export_has_bom_and_rows() {
        let db = Db::open_in_memory().expect("db");
        add_transaction(db.conn(), &draft("expense", 100)).expect("add");
        let (from, to) = month_range(2026, 11);
        let csv = export_csv(db.conn(), from, to).expect("csv");
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.lines().count() >= 2);
    }

    #[test]
    fn categories_can_be_added() {
        let db = Db::open_in_memory().expect("db");
        let category = create_category(db.conn(), "قهوة", "expense", "#ff0000").expect("create");
        assert_eq!(category.name, "قهوة");
        assert!(create_category(db.conn(), "قهوة", "expense", "#ff0000").is_ok());
        assert!(create_category(db.conn(), "x", "wrong", "#fff").is_err());
    }

    #[test]
    fn deleting_transaction_removes_it() {
        let db = Db::open_in_memory().expect("db");
        let tx = add_transaction(db.conn(), &draft("expense", 100)).expect("add");
        delete_transaction(db.conn(), tx.id).expect("delete");
        let summary = monthly_summary(db.conn(), 2026, 11).expect("summary");
        assert_eq!(summary.expense_minor, 0);
    }
}
