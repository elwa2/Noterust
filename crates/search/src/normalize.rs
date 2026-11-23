//! Arabic-aware text normalisation used for the FTS5 index and for queries.
//!
//! Normalising both sides (index and query) makes search forgiving about the
//! spelling variations that are common in Arabic text:
//!
//! * diacritics (tashkeel) and tatweel are removed;
//! * `أ إ آ ٱ` are folded to `ا`;
//! * `ى` is folded to `ي`;
//! * `ة` is folded to `ه`;
//! * `ؤ` / `ئ` are folded to `و` / `ي`;
//! * Arabic-Indic and Extended Arabic-Indic digits become ASCII digits;
//! * Latin letters are lower-cased and whitespace is collapsed.

/// Returns `true` for Arabic diacritics and Quranic annotation marks.
fn is_tashkeel(c: char) -> bool {
    matches!(c as u32,
        0x0610..=0x061A
        | 0x064B..=0x065F
        | 0x0670
        | 0x06D6..=0x06ED
        | 0x0640 // tatweel
    )
}

/// Removes diacritics and tatweel from Arabic text.
pub fn strip_tashkeel(input: &str) -> String {
    input.chars().filter(|c| !is_tashkeel(*c)).collect()
}

fn fold_char(c: char) -> char {
    match c {
        'أ' | 'إ' | 'آ' | 'ٱ' | 'ٲ' | 'ٳ' => 'ا',
        'ى' | 'ی' => 'ي',
        'ة' => 'ه',
        'ؤ' => 'و',
        'ئ' => 'ي',
        '٠'..='٩' => char::from_u32('0' as u32 + (c as u32 - '٠' as u32)).unwrap_or(c),
        '۰'..='۹' => char::from_u32('0' as u32 + (c as u32 - '۰' as u32)).unwrap_or(c),
        other => other,
    }
}

/// Normalises text so that equivalent Arabic spellings produce equal forms.
pub fn normalize_arabic(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_was_space = false;

    for c in input.chars() {
        if is_tashkeel(c) {
            continue;
        }
        for lowered in fold_char(c).to_lowercase() {
            if lowered.is_whitespace() {
                if !last_was_space && !out.is_empty() {
                    out.push(' ');
                    last_was_space = true;
                }
            } else {
                out.push(lowered);
                last_was_space = false;
            }
        }
    }

    strip_definite_article(out.trim_end())
}

/// Removes the Arabic definite article from the start of each word.
///
/// `المدرسة` and `مدرسة` then index (and match) identically, which is what a
/// reader expects from a search box. Short words are left alone so that words
/// such as `الله` keep their form.
fn strip_definite_article(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, word) in text.split(' ').enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let chars = word.chars().count();
        if chars >= 5 && word.starts_with("ال") {
            out.extend(word.chars().skip(2));
        } else {
            out.push_str(word);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tashkeel_is_removed() {
        assert_eq!(strip_tashkeel("مُحَمَّد"), "محمد");
        assert_eq!(normalize_arabic("كَتَبَ"), "كتب");
    }

    #[test]
    fn alef_forms_are_unified() {
        assert_eq!(normalize_arabic("أحمد"), normalize_arabic("احمد"));
        assert_eq!(normalize_arabic("إسلام"), "اسلام");
        assert_eq!(normalize_arabic("آمن"), "امن");
    }

    #[test]
    fn yaa_and_taa_marbuta_are_folded() {
        assert_eq!(normalize_arabic("على"), "علي");
        assert_eq!(normalize_arabic("مدرسة"), "مدرسه");
        assert_eq!(normalize_arabic("مؤمن"), "مومن");
        assert_eq!(normalize_arabic("رئيس"), "رييس");
    }

    #[test]
    fn arabic_indic_digits_are_ascii() {
        assert_eq!(normalize_arabic("٢٠٢٦"), "2026");
    }

    #[test]
    fn whitespace_is_collapsed() {
        assert_eq!(normalize_arabic("  نص   طويل \n"), "نص طويل");
    }

    #[test]
    fn latin_is_lowered() {
        assert_eq!(normalize_arabic("Hello World"), "hello world");
    }

    #[test]
    fn definite_article_is_folded() {
        assert_eq!(normalize_arabic("المدرسة"), normalize_arabic("مدرسة"));
        assert_eq!(normalize_arabic("المدرسة"), "مدرسه");
        // Short words keep their article.
        assert_eq!(normalize_arabic("الله"), "الله");
    }

    #[test]
    fn tatweel_is_dropped() {
        assert_eq!(normalize_arabic("مـــرحبا"), "مرحبا");
    }
}
