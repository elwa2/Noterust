use crate::models::{ChecklistItem, TextCounts};

/// Converts Western digits into Arabic-Indic digits (٠١٢٣٤٥٦٧٨٩).
pub fn format_arabic_digits(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32('٠' as u32 + (c as u32 - '0' as u32)).unwrap_or(c),
            _ => c,
        })
        .collect()
}

const ARABIC_MONTHS: [&str; 12] = [
    "يناير",
    "فبراير",
    "مارس",
    "أبريل",
    "مايو",
    "يونيو",
    "يوليو",
    "أغسطس",
    "سبتمبر",
    "أكتوبر",
    "نوفمبر",
    "ديسمبر",
];

/// Formats a timestamp the way the note header shows it, e.g.
/// `٢٣ نوفمبر ٢٠٢٦ في ٢:١١ م`.
pub fn arabic_datetime(millis: i64) -> String {
    let Some(dt) = chrono::DateTime::from_timestamp_millis(millis) else {
        return String::new();
    };
    use chrono::{Datelike, Timelike};

    let local = dt.with_timezone(&chrono::Local);
    let (hour24, minute) = (local.hour(), local.minute());
    let suffix = if hour24 < 12 { "ص" } else { "م" };
    let hour12 = match hour24 % 12 {
        0 => 12,
        other => other,
    };
    let day = local.day();
    let month = ARABIC_MONTHS[(local.month() as usize).saturating_sub(1).min(11)];
    let year = local.year();
    format_arabic_digits(&format!(
        "{day} {month} {year} في {hour12}:{minute:02} {suffix}"
    ))
}

/// Splits text into words on Unicode whitespace.
fn words_of(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
}

/// Text statistics for the status bar.
pub fn counts(text: &str) -> TextCounts {
    TextCounts {
        words: words_of(text).count() as i64,
        chars: text.chars().count() as i64,
        chars_no_spaces: text.chars().filter(|c| !c.is_whitespace()).count() as i64,
        lines: if text.is_empty() {
            1
        } else {
            text.lines().count() as i64
        },
    }
}

/// True when the text is mostly right-to-left script.
pub fn is_rtl(text: &str) -> bool {
    let mut rtl = 0usize;
    let mut ltr = 0usize;
    for c in text.chars() {
        let cp = c as u32;
        let is_arabic = (0x0600..=0x06FF).contains(&cp)
            || (0x0750..=0x077F).contains(&cp)
            || (0x08A0..=0x08FF).contains(&cp)
            || (0xFB50..=0xFDFF).contains(&cp)
            || (0xFE70..=0xFEFF).contains(&cp)
            || (0x0590..=0x05FF).contains(&cp);
        if is_arabic {
            rtl += 1;
        } else if c.is_alphabetic() {
            ltr += 1;
        }
    }
    rtl > ltr
}

/// Picks `rtl` or `ltr` for a note, honouring an explicit override.
pub fn detect_direction(direction_setting: &str, text: &str) -> &'static str {
    match direction_setting {
        "rtl" => "rtl",
        "ltr" => "ltr",
        _ => {
            if is_rtl(text) {
                "rtl"
            } else {
                "ltr"
            }
        }
    }
}

/// Removes the leading Markdown block marker from a line.
fn strip_block_marker(line: &str) -> &str {
    let trimmed = line.trim_start();
    let without_heading = trimmed.trim_start_matches('#').trim_start();
    let without_quote = without_heading.trim_start_matches('>').trim_start();
    let without_bullet = without_quote
        .strip_prefix("- [ ] ")
        .or_else(|| without_quote.strip_prefix("- [x] "))
        .or_else(|| without_quote.strip_prefix("- [X] "))
        .or_else(|| without_quote.strip_prefix("- "))
        .or_else(|| without_quote.strip_prefix("* "))
        .or_else(|| without_quote.strip_prefix("+ "))
        .unwrap_or(without_quote);
    without_bullet.trim()
}

/// Derives a note title from the first meaningful line.
pub fn derive_title(content: &str) -> String {
    for line in content.lines() {
        let candidate = strip_block_marker(line);
        if !candidate.is_empty() && candidate != "---" {
            return truncate_chars(candidate, 120);
        }
    }
    String::new()
}

/// Builds a one-line plain-text preview for the note list.
pub fn preview(content: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let candidate = strip_block_marker(line);
        if candidate.is_empty() || candidate == "---" {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(candidate);
        if out.chars().count() > 140 {
            break;
        }
    }
    truncate_chars(&out, 140)
}

/// Removes common Markdown syntax, used for plain-text export.
pub fn strip_markdown(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_code = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let stripped = strip_block_marker(line)
            .replace("**", "")
            .replace("__", "")
            .replace(['*', '`'], "");
        out.push_str(&stripped);
        out.push('\n');
    }
    out.trim_end().to_string()
}

/// Truncates on character (not byte) boundaries and appends an ellipsis.
pub fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit).collect();
    out.push('…');
    out
}

/// Extracts `- [ ]` / `- [x]` checklist lines from note content.
pub fn extract_checklist(content: &str) -> Vec<ChecklistItem> {
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        let rest = trimmed
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed.strip_prefix("* [ ] "))
            .or_else(|| trimmed.strip_prefix("- [x] "))
            .or_else(|| trimmed.strip_prefix("* [x] "))
            .or_else(|| trimmed.strip_prefix("- [X] "))
            .or_else(|| trimmed.strip_prefix("* [X] "))
            .or_else(|| trimmed.strip_prefix("☐ "))
            .or_else(|| trimmed.strip_prefix("☑ "));
        if let Some(rest) = rest {
            let done =
                trimmed.contains("[x]") || trimmed.contains("[X]") || trimmed.starts_with('☑');
            let text = rest.trim().to_string();
            if !text.is_empty() {
                items.push(ChecklistItem { text, done });
            }
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_digits_are_converted() {
        assert_eq!(format_arabic_digits("2026"), "٢٠٢٦");
        assert_eq!(format_arabic_digits("abc 12"), "abc ١٢");
    }

    #[test]
    fn datetime_is_arabic() {
        let millis = 1_800_000_000_000;
        let formatted = arabic_datetime(millis);
        assert!(formatted.contains('ف'));
        assert!(!formatted.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn counts_words_lines_and_chars() {
        let counts = counts("سطر أول\nسطر ثانٍ");
        assert_eq!(counts.words, 4);
        assert_eq!(counts.lines, 2);
        assert_eq!(counts.chars, 16);
    }

    #[test]
    fn rtl_detection_prefers_arabic() {
        assert!(is_rtl("مرحبا بالعالم"));
        assert!(!is_rtl("hello world"));
        assert_eq!(detect_direction("ltr", "مرحبا"), "ltr");
        assert_eq!(detect_direction("auto", "مرحبا"), "rtl");
    }

    #[test]
    fn title_comes_from_first_line() {
        assert_eq!(derive_title("# عنوان الملاحظة\nنص"), "عنوان الملاحظة");
        assert_eq!(derive_title("\n\n- [ ] مهمة\n"), "مهمة");
    }

    #[test]
    fn checklist_lines_are_parsed() {
        let items = extract_checklist("- [ ] أولاً\n- [x] ثانياً\nنص عادي");
        assert_eq!(items.len(), 2);
        assert!(!items[0].done);
        assert!(items[1].done);
        assert_eq!(items[1].text, "ثانياً");
    }

    #[test]
    fn preview_skips_fences() {
        assert_eq!(preview("```\ncode\n```\nمرحبا"), "مرحبا");
    }

    #[test]
    fn truncation_respects_chars() {
        assert_eq!(truncate_chars("مرحبا", 2), "مر…");
    }
}
