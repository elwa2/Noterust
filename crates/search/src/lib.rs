//! Text normalisation for Arabic search plus FTS5 query building.

mod normalize;

pub use normalize::{normalize_arabic, strip_tashkeel};

/// Builds a safe FTS5 `MATCH` expression from free user input.
///
/// The input is normalised and every term is quoted, so characters that are
/// significant to FTS5 (`-`, `*`, `:`, `^`, `"`) can never break the query.
/// Returns `None` when the query contains no searchable term.
pub fn build_match_query(input: &str) -> Option<String> {
    let normalised = normalize_arabic(input);
    let terms: Vec<String> = normalised
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        // Every term must match, and the last one may be a prefix (`كتا` -> كتاب).
        let last = terms.len() - 1;
        let mut clauses = terms;
        if let Some(last_term) = clauses.get_mut(last) {
            if !last_term.ends_with('"') {
                last_term.push('"');
            }
            last_term.insert(last_term.len() - 1, '*');
        }
        Some(clauses.join(" AND "))
    }
}

/// Turns the FTS5 snippet markers into something the UI can render safely.
pub fn clean_snippet(snippet: &str) -> String {
    snippet
        .replace("<b>", "\u{2}")
        .replace("</b>", "\u{3}")
        .replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_quotes_terms_and_adds_prefix() {
        assert_eq!(build_match_query("كتاب"), Some("\"كتاب*\"".to_string()));
        assert_eq!(
            build_match_query("كتاب جديد"),
            Some("\"كتاب\" AND \"جديد*\"".to_string())
        );
    }

    #[test]
    fn query_escapes_fts_syntax() {
        let query = build_match_query("a-b AND c:d").unwrap_or_default();
        assert!(query.contains("\"a-b\""));
        assert!(query.contains("\"c:d*\""));
    }

    #[test]
    fn empty_query_returns_none() {
        assert_eq!(build_match_query("   "), None);
    }

    #[test]
    fn snippet_markers_become_control_chars() {
        assert_eq!(clean_snippet("a<b>مهم</b>c"), "a\u{2}مهم\u{3}c");
    }
}
