//! File reading/writing for the "open external file" mode.
//!
//! Responsibilities:
//! * encoding detection (BOM first, then `chardetng`) and decoding for
//!   UTF-8/UTF-16/Windows-1256 and other legacy Notepad encodings;
//! * line-ending preservation (`CRLF` / `LF` / `CR`);
//! * incremental decoding into a `ropey` rope so very large files do not
//!   freeze the UI;
//! * atomic writes (temporary file + rename);
//! * filesystem watching for the "external file" mode.

pub mod doc;
pub mod watcher;

pub use doc::{DecodedText, TextDocument};
pub use watcher::{watch_path, FileWatcher};

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
use thiserror::Error;

/// Errors produced by the file layer.
#[derive(Debug, Error)]
pub enum FileIoError {
    #[error("خطأ في الملف: {0}")]
    Io(#[from] io::Error),
    #[error("الملف ليس ملفاً نصياً")]
    NotText,
    #[error("ترميز غير مدعوم: {0}")]
    UnsupportedEncoding(String),
    #[error("تعذر مراقبة الملف: {0}")]
    Watch(String),
}

pub type Result<T> = std::result::Result<T, FileIoError>;

/// A file loaded into memory, with the metadata needed to save it back
/// unchanged.
#[derive(Debug, Clone)]
pub struct LoadedFile {
    pub text: String,
    /// Canonical encoding name, e.g. `UTF-8` or `windows-1256`.
    pub encoding: String,
    /// `CRLF`, `LF` or `CR`.
    pub line_ending: String,
    pub has_bom: bool,
}

/// Detects the encoding of `bytes`, honouring a BOM when present.
pub fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    if let Some((encoding, _bom_len)) = Encoding::for_bom(bytes) {
        return encoding;
    }
    if let Some(encoding) = utf16_lane(bytes) {
        return encoding;
    }
    // Only files with NUL bytes need the slower decoding probe.
    if bytes.contains(&0) {
        if let Some(encoding) = sniff_utf16(bytes) {
            return encoding;
        }
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    detector.guess(None, true)
}

/// Share of characters that never appear in ordinary text (control codes and
/// C1 range), used to score a UTF-16 decoding attempt.
/// Scores one UTF-16 byte order: `Some((bad, familiar))`, the share of control
/// characters and of characters in ranges ordinary text actually uses.
fn utf16_quality(bytes: &[u8], encoding: &'static Encoding) -> Option<(f64, f64)> {
    let even = bytes.len() - bytes.len() % 2;
    if even < 8 {
        return None;
    }
    // A trailing split surrogate is normal when the sample was cut mid-file.
    let (text, _had_errors) = encoding.decode_without_bom_handling(&bytes[..even]);
    let total = text.chars().count();
    if total == 0 {
        return None;
    }
    let mut odd = 0usize;
    let mut familiar = 0usize;
    for c in text.chars() {
        let code = c as u32;
        let is_control =
            (code < 0x20 && c != '\t' && c != '\n' && c != '\r') || (0x7f..0xa0).contains(&code);
        if is_control {
            odd += 1;
        }
        let known = matches!(code,
            0x20..=0x7e
            | 0x00a0..=0x024f
            | 0x0600..=0x06ff
            | 0x0750..=0x077f
            | 0x2000..=0x206f
            | 0x20aa | 0x20ac
            | 0xfb50..=0xfdff
            | 0xfe70..=0xfeff
        );
        if known {
            familiar += 1;
        }
    }
    Some((odd as f64 / total as f64, familiar as f64 / total as f64))
}

/// BOM-less UTF-16 that only contains Arabic text has almost no NUL bytes, so
/// the lane pattern above misses it. Decoding both byte orders and keeping the
/// one that reads like human text catches those files — while random binary
/// bytes (images, archives) score near zero on familiar characters.
pub fn sniff_utf16(bytes: &[u8]) -> Option<&'static Encoding> {
    let sample = &bytes[..bytes.len().min(8192)];
    if sample.len() < 8 {
        return None;
    }
    let usable = |quality: Option<(f64, f64)>| match quality {
        Some((bad, familiar)) if bad < 0.02 && familiar > 0.5 => Some(bad - familiar),
        _ => None,
    };
    let le = usable(utf16_quality(sample, UTF_16LE));
    let be = usable(utf16_quality(sample, UTF_16BE));
    match (le, be) {
        (Some(left), Some(right)) if left <= right => Some(UTF_16LE),
        (Some(_), Some(_)) => Some(UTF_16BE),
        (Some(_), None) => Some(UTF_16LE),
        (None, Some(_)) => Some(UTF_16BE),
        (None, None) => None,
    }
}

/// Heuristic: text interleaved with NUL bytes in a single byte lane is UTF-16
/// without a BOM. The lane that holds the NUL bytes tells the byte order:.
fn utf16_lane(bytes: &[u8]) -> Option<&'static Encoding> {
    // BOM-less UTF-16 is only guessed for reasonably large samples: a short
    // byte run with a zero in every other position is far more likely binary.
    if bytes.len() < 8 {
        return None;
    }
    let sample = &bytes[..bytes.len().min(4096)];
    let half = sample.len() / 2;
    if half == 0 {
        return None;
    }
    let nul_even = sample.iter().step_by(2).filter(|b| **b == 0).count() as f64 / half as f64;
    let nul_odd = sample
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|b| **b == 0)
        .count() as f64
        / half as f64;
    // Most characters are ASCII: one lane is almost all NUL. NUL bytes in the
    // odd lane mean the low byte of each little-endian unit is the text.
    if nul_odd > 0.4 && nul_even < 0.05 {
        return Some(UTF_16LE);
    }
    if nul_even > 0.4 && nul_odd < 0.05 {
        return Some(UTF_16BE);
    }
    // Arabic (and other non-Latin) UTF-16 only leaves NULs where the text is
    // ASCII, so the ratio is low — but the NULs still sit in one lane only.
    if nul_odd > 0.02 && nul_even == 0.0 {
        return Some(UTF_16LE);
    }
    if nul_even > 0.02 && nul_odd == 0.0 {
        return Some(UTF_16BE);
    }
    None
}

/// Resolves a user supplied encoding name to an `encoding_rs` encoding.
pub fn encoding_by_name(name: &str) -> Option<&'static Encoding> {
    let normalised = name.trim().to_ascii_lowercase().replace('_', "-");
    let encoding = match normalised.as_str() {
        "utf-8" | "utf8" => UTF_8,
        "utf-16le" | "utf16le" | "unicode" => UTF_16LE,
        "utf-16be" | "utf16be" => UTF_16BE,
        other => Encoding::for_label(other.as_bytes())?,
    };
    Some(encoding)
}

/// Large values of decoded text; used to decide whether to show a warning.
pub const LARGE_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// Detects the dominant line ending in `text`.
pub fn detect_line_ending(text: &str) -> String {
    let crlf = text.matches("\r\n").count();
    let lf = text.matches('\n').count().saturating_sub(crlf);
    let cr = text.matches('\r').count().saturating_sub(crlf);
    if crlf >= lf && crlf >= cr && crlf > 0 {
        "CRLF".to_string()
    } else if cr > lf {
        "CR".to_string()
    } else {
        "LF".to_string()
    }
}

/// Converts `text` to `LF` line endings for editing.
pub fn normalise_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Applies the requested line ending to `LF` normalised text.
pub fn apply_line_ending(text: &str, line_ending: &str) -> String {
    let normalised = normalise_newlines(text);
    match line_ending {
        "CRLF" => normalised.replace('\n', "\r\n"),
        "CR" => normalised.replace('\n', "\r"),
        _ => normalised,
    }
}

/// True when the byte sample contains NUL bytes outside a UTF-16 pattern.
pub fn is_probably_binary(bytes: &[u8]) -> bool {
    // A BOM is conclusive: text may legitimately contain NUL bytes when it is
    // UTF-16 with only ASCII characters.
    if Encoding::for_bom(bytes).is_some() || utf16_lane(bytes).is_some() {
        return false;
    }
    let sample = &bytes[..bytes.len().min(4096)];
    if !sample.contains(&0) {
        return false;
    }
    // Arabic or CJK UTF-16 keeps its high byte non-zero, so a file that is
    // mostly Arabic still contains NUL bytes from its ASCII parts. Give such a
    // file a chance to prove it is text before calling it binary.
    if sniff_utf16(sample).is_some() {
        return false;
    }
    true
}

/// Reads and decodes a text file, preserving its detected metadata.
pub fn read_file(path: &Path) -> Result<LoadedFile> {
    let document = TextDocument::open(path)?;
    let text = document.text();
    Ok(LoadedFile {
        text,
        encoding: document.encoding.clone(),
        line_ending: document.line_ending.clone(),
        has_bom: document.has_bom,
    })
}

/// Encodes `text` in `encoding`, producing the exact bytes to store.
///
/// `encoding_rs` cannot encode to UTF-16 (its encoder output for UTF-16 labels
/// is UTF-8), so UTF-16 is encoded by hand here — otherwise "save" would
/// silently rewrite a UTF-16 file as UTF-8.
pub fn encode_text(text: &str, encoding: &'static Encoding) -> Vec<u8> {
    if encoding == UTF_16LE {
        let mut bytes = Vec::with_capacity(2 + text.len() * 2);
        bytes.extend_from_slice(&[0xFF, 0xFE]);
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes
    } else if encoding == UTF_16BE {
        let mut bytes = Vec::with_capacity(2 + text.len() * 2);
        bytes.extend_from_slice(&[0xFE, 0xFF]);
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        bytes
    } else {
        encoding.encode(text).0.into_owned()
    }
}

/// Writes `text` atomically: the data is written to a temporary file in the
/// same directory, flushed, and then renamed over the destination. A crash can
/// therefore never leave a half-written file behind.
pub fn write_atomic(path: &Path, text: &str, encoding_name: &str, line_ending: &str) -> Result<()> {
    let encoding = encoding_by_name(encoding_name)
        .ok_or_else(|| FileIoError::UnsupportedEncoding(encoding_name.to_string()))?;
    let with_endings = apply_line_ending(text, line_ending);
    let encoded = encode_text(&with_endings, encoding);
    if encoded.is_empty() && !with_endings.is_empty() {
        return Err(FileIoError::UnsupportedEncoding(format!(
            "النص يحتوي محارف لا يمكن تمثيلها في {encoding_name}"
        )));
    }

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;

    let temp_name = format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("noterust"),
        std::process::id()
    );
    let temp_path = parent.join(temp_name);

    {
        use std::io::Write;
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(&encoded)?;
        file.flush()?;
        file.sync_all()?;
    }

    match fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&temp_path);
            Err(FileIoError::Io(err))
        }
    }
}

/// Formats bytes as a short human readable size.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["بايت", "ك.ب", "م.ب", "ج.ب"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[0])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// File extensions the app treats as text when importing folders.
pub const TEXT_EXTENSIONS: [&str; 25] = [
    "txt", "log", "md", "markdown", "json", "xml", "csv", "tsv", "ini", "cfg", "conf", "bat",
    "cmd", "ps1", "psm1", "sh", "rs", "toml", "yaml", "yml", "html", "css", "js", "ts", "sql",
];

/// Collects text files under `path` (recursively for directories).
pub fn collect_text_files(path: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_into(path, &mut found, 0);
    found
}

fn collect_into(path: &Path, found: &mut Vec<PathBuf>, depth: usize) {
    if depth > 16 || found.len() > 5000 {
        return;
    }
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.is_file() {
        found.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let child = entry.path();
        if child
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'))
        {
            continue;
        }
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect_into(&child, found, depth + 1);
        } else {
            found.push(child);
        }
    }
}

/// Reads the first bytes of a file without consuming the whole thing.
pub fn read_head(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; limit];
    let read = file.read(&mut buffer)?;
    buffer.truncate(read);
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("noterust-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    #[test]
    fn utf8_with_bom_is_detected() {
        let bytes = b"\xEF\xBB\xBFhello";
        assert_eq!(detect_encoding(bytes).name(), "UTF-8");
    }

    #[test]
    fn utf16_le_is_detected_from_bom() {
        let bytes = b"\xFF\xFEh\x00i\x00";
        assert_eq!(detect_encoding(bytes).name(), "UTF-16LE");
    }

    #[test]
    fn windows_1256_arabic_is_detected() {
        let (encoded, _, _) = encoding_rs::WINDOWS_1256.encode("مرحبا بالعالم");
        let encoding = detect_encoding(&encoded);
        let (decoded, _, _) = encoding.decode(&encoded);
        assert_eq!(decoded, "مرحبا بالعالم");
    }

    #[test]
    fn line_endings_are_detected_and_applied() {
        assert_eq!(detect_line_ending("a\r\nb\r\n"), "CRLF");
        assert_eq!(detect_line_ending("a\nb\n"), "LF");
        assert_eq!(detect_line_ending("a\rb\r"), "CR");
        assert_eq!(apply_line_ending("a\nb", "CRLF"), "a\r\nb");
    }

    #[test]
    fn atomic_write_replaces_file() {
        let path = temp_path("write.txt");
        write_atomic(&path, "first\n", "UTF-8", "LF").expect("write");
        write_atomic(&path, "second\n", "UTF-8", "LF").expect("write");
        let loaded = read_file(&path).expect("read");
        assert_eq!(loaded.text, "second\n");
    }

    #[test]
    fn utf16_write_keeps_utf16_bytes() {
        let path = temp_path("utf16-out.txt");
        write_atomic(&path, "نص عربي", "UTF-16LE", "CRLF").expect("write");
        let bytes = fs::read(&path).expect("bytes");
        assert_eq!(&bytes[..2], &[0xFF, 0xFE]);
        let loaded = read_file(&path).expect("read");
        assert_eq!(loaded.text, "نص عربي");
        assert_eq!(loaded.encoding, "UTF-16LE");
    }

    #[test]
    fn read_write_roundtrip_preserves_crlf() {
        let path = temp_path("crlf.txt");
        fs::write(&path, b"a\r\nb\r\n").expect("seed");
        let loaded = read_file(&path).expect("read");
        assert_eq!(loaded.line_ending, "CRLF");
        write_atomic(&path, &loaded.text, &loaded.encoding, &loaded.line_ending).expect("write");
        assert_eq!(fs::read(&path).expect("bytes"), b"a\r\nb\r\n");
    }

    #[test]
    fn binary_files_are_flagged() {
        assert!(is_probably_binary(&[0, 1, 2, 3]));
        assert!(!is_probably_binary("نص".as_bytes()));
        // Random bytes with NULs must stay binary even though they decode as
        // syntactically valid UTF-16 code units.
        let noise: Vec<u8> = (0..512u32)
            .map(|i| (i.wrapping_mul(97) % 256) as u8)
            .collect();
        assert!(is_probably_binary(&noise));
    }

    #[test]
    fn bomless_utf16_arabic_is_text() {
        // Mixed Arabic + ASCII in UTF-16LE without a BOM: the ASCII parts leave
        // NUL bytes behind, which used to look like a binary file.
        let text = "المكتب admin 192.168.1.79\nالمكتب admin 10.0.0.1\n";
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        assert!(!is_probably_binary(&bytes));
        assert_eq!(detect_encoding(&bytes).name(), "UTF-16LE");
        let (decoded, _) = detect_encoding(&bytes).decode_without_bom_handling(&bytes);
        assert_eq!(decoded, text);
    }

    #[test]
    fn encoding_lookup_accepts_aliases() {
        assert_eq!(encoding_by_name("utf8").map(|e| e.name()), Some("UTF-8"));
        assert_eq!(
            encoding_by_name("windows-1256").map(|e| e.name()),
            Some("windows-1256")
        );
        assert!(encoding_by_name("not-a-real-encoding").is_none());
    }

    #[test]
    fn folder_import_collects_nested_files() {
        let dir = std::env::temp_dir().join(format!("noterust-import-{}", std::process::id()));
        fs::create_dir_all(dir.join("nested")).expect("dirs");
        let mut file = fs::File::create(dir.join("nested/a.txt")).expect("file");
        file.write_all(b"x").expect("write");
        fs::File::create(dir.join("b.md")).expect("file");
        let files = collect_text_files(&dir);
        assert!(files.len() >= 2);
    }

    #[test]
    fn size_formatting_is_arabic() {
        assert_eq!(format_size(512), "512 بايت");
        assert!(format_size(2048).contains("ك.ب"));
    }
}
