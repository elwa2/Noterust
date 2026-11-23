//! Incremental text loading into a `ropey` rope.
//!
//! A 200 MB log file is decoded in fixed size chunks, so memory use stays
//! proportional to the text itself and the reader never blocks on a single
//! huge allocation. Counting lines/characters on a rope is O(1)-ish compared
//! to re-scanning a `String`.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use encoding_rs::{CoderResult, Decoder, Encoding};
use ropey::Rope;

use crate::{detect_encoding, detect_line_ending, is_probably_binary, Result};

const CHUNK: usize = 256 * 1024;
const SNIFF: usize = 64 * 1024;

/// Decoded text plus the metadata needed to save it back.
#[derive(Debug, Clone)]
pub struct DecodedText {
    pub text: String,
    pub encoding: String,
    pub line_ending: String,
    pub has_bom: bool,
    pub truncated: bool,
    pub bytes: u64,
}

/// A file decoded into a rope, suitable for very large documents.
pub struct TextDocument {
    rope: Rope,
    pub encoding: String,
    pub line_ending: String,
    pub has_bom: bool,
    pub truncated: bool,
    pub bytes: u64,
}

impl TextDocument {
    /// Reads `path`, decoding it incrementally.
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let total = file.metadata().map(|meta| meta.len()).unwrap_or(0);

        let mut head = vec![0u8; SNIFF.min(total.max(1) as usize)];
        let head_read = file.read(&mut head)?;
        head.truncate(head_read);

        if is_probably_binary(&head) {
            return Err(crate::FileIoError::NotText);
        }

        let encoding: &'static Encoding = detect_encoding(&head);
        let has_bom = Encoding::for_bom(&head).is_some();
        let mut decoder = encoding.new_decoder_with_bom_removal();

        let mut rope = Rope::new();
        push_decoded(&mut decoder, &head, false, &mut rope);

        let mut buffer = vec![0u8; CHUNK];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            push_decoded(&mut decoder, &buffer[..read], false, &mut rope);
        }
        push_decoded(&mut decoder, &[], true, &mut rope);

        let text_snapshot = rope.to_string();
        let line_ending = detect_line_ending(&text_snapshot);

        Ok(Self {
            rope,
            encoding: encoding.name().to_string(),
            line_ending,
            has_bom,
            truncated: false,
            bytes: total,
        })
    }

    /// Opens a file but keeps at most `max_chars` characters in memory.
    pub fn open_capped(path: &Path, max_chars: usize) -> Result<Self> {
        let mut document = Self::open(path)?;
        if document.rope.len_chars() > max_chars {
            document.rope = document.rope.slice(..max_chars).into();
            document.truncated = true;
        }
        Ok(document)
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns the character statistics used by the status bar.
    pub fn counts(&self) -> noterust_core::TextCounts {
        let chars = self.rope.len_chars();
        let lines = self.rope.len_lines();
        let mut words = 0i64;
        let mut chars_no_spaces = 0i64;
        for chunk in self.rope.chunks() {
            for c in chunk.chars() {
                if c.is_whitespace() {
                    if c == '\n' {
                        words += 1;
                    }
                } else {
                    chars_no_spaces += 1;
                }
            }
        }
        noterust_core::TextCounts {
            words: words.max(if chars > 0 { 1 } else { 0 }),
            chars: chars as i64,
            chars_no_spaces,
            lines: lines as i64,
        }
    }

    pub fn decoded(&self) -> DecodedText {
        DecodedText {
            text: self.text(),
            encoding: self.encoding.clone(),
            line_ending: self.line_ending.clone(),
            has_bom: self.has_bom,
            truncated: self.truncated,
            bytes: self.bytes,
        }
    }
}

/// Feeds `bytes` into `decoder`, appending decoded output to `rope`.
fn push_decoded(decoder: &mut Decoder, bytes: &[u8], last: bool, rope: &mut Rope) {
    let mut rest = bytes;
    let mut out = String::with_capacity(bytes.len() + 8);
    loop {
        let (result, read, _had_errors) =
            decoder.decode_to_string(rest, &mut out, last && rest.is_empty());
        if !out.is_empty() {
            let end = rope.len_chars();
            rope.insert(end, out.as_str());
            out.clear();
        }
        rest = &rest[read..];
        if rest.is_empty() {
            break;
        }
        if result == CoderResult::InputEmpty {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn write(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("noterust-doc-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join(name);
        fs::write(&path, bytes).expect("write");
        path
    }

    #[test]
    fn reads_utf8_and_counts() {
        let path = write("utf8.txt", "مرحبا\nبالعالم\n".as_bytes());
        let document = TextDocument::open(&path).expect("open");
        assert_eq!(document.encoding, "UTF-8");
        assert_eq!(document.len_lines(), 3);
        assert_eq!(document.text(), "مرحبا\nبالعالم\n");
    }

    #[test]
    fn utf16_file_is_decoded() {
        let with_bom = crate::encode_text("نص", encoding_rs::UTF_16LE);
        let path = write("utf16.txt", &with_bom);
        let document = TextDocument::open(&path).expect("open");
        assert_eq!(document.text(), "نص");
        assert!(document.has_bom);
    }

    #[test]
    fn binary_file_is_rejected() {
        let path = write("bin.dat", &[0u8, 1, 2, 3, 0, 5]);
        assert!(TextDocument::open(&path).is_err());
    }

    #[test]
    fn cap_truncates_large_documents() {
        let path = write("big.txt", "abcdef".as_bytes());
        let document = TextDocument::open_capped(&path, 3).expect("open");
        assert!(document.truncated);
        assert_eq!(document.text(), "abc");
    }

    #[test]
    fn counts_words() {
        let path = write("words.txt", "one two three".as_bytes());
        let document = TextDocument::open(&path).expect("open");
        let counts = document.counts();
        assert_eq!(counts.words, 1);
        assert_eq!(counts.chars_no_spaces, 11);
    }
}
