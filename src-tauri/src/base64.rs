//! Minimal Base64 for pasted images: the webview sends the clipboard image as
//! Base64 text and reads stored files back the same way (as a `data:` URL).
//! Kept local so the app needs no extra dependency.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes bytes as standard Base64 with padding.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let value = (a << 16) | (b << 8) | c;
        out.push(ALPHABET[(value >> 18) as usize & 63] as char);
        out.push(ALPHABET[(value >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(value >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[value as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Decodes standard Base64, ignoring whitespace and a trailing newline.
pub fn decode(text: &str) -> Result<Vec<u8>, String> {
    let cleaned: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let cleaned = match cleaned.split_last() {
        Some((b'=', rest)) => rest,
        _ => &cleaned[..],
    };
    let cleaned = match cleaned.split_last() {
        Some((b'=', rest)) => rest,
        _ => cleaned,
    };
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for byte in cleaned {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("بيانات الصورة غير صالحة".to_string()),
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn round_trip() {
        for sample in [
            b"".as_slice(),
            b"a".as_slice(),
            b"ab".as_slice(),
            b"abc".as_slice(),
            b"image bytes \x00\xff\x10".as_slice(),
        ] {
            let text = encode(sample);
            assert_eq!(decode(&text).expect("decode"), sample, "sample {text}");
        }
    }

    #[test]
    fn known_values() {
        assert_eq!(encode(b"hello"), "aGVsbG8=");
        assert_eq!(decode("aGVsbG8=").expect("decode"), b"hello");
        assert_eq!(decode("aGVs\nbG8=").expect("decode"), b"hello");
        assert!(decode("not base64!").is_err());
    }
}
