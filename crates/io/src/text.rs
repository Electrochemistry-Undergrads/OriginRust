//! Reading text files whose encoding is not known in advance.

use crate::error::IoError;
use std::path::Path;

/// Reads a whole text file.
///
/// Accepted encodings: UTF-8 with or without a byte-order mark, UTF-16 (LE or
/// BE) with a byte-order mark, and, for any other bytes that are not valid
/// UTF-8, Windows-1252 (what instrument software on Windows usually writes).
/// Line ends are normalised: CRLF and a lone CR both become LF.
pub fn read_text_file(path: &Path) -> Result<String, IoError> {
    let bytes = std::fs::read(path).map_err(|e| IoError::io(path, e))?;
    decode_text(&bytes, path)
}

/// Decodes bytes as described in [`read_text_file`]. `path` is only used in
/// error messages.
pub fn decode_text(bytes: &[u8], path: &Path) -> Result<String, IoError> {
    let not_text = |reason: &str| IoError::NotText { path: path.to_path_buf(), reason: reason.to_string() };
    let text = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(rest.to_vec())
            .map_err(|_| not_text("it has a UTF-8 byte-order mark but invalid UTF-8 content"))?
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        decode_utf16(rest, u16::from_le_bytes).ok_or_else(|| not_text("it is not valid UTF-16 (little endian)"))?
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        decode_utf16(rest, u16::from_be_bytes).ok_or_else(|| not_text("it is not valid UTF-16 (big endian)"))?
    } else if bytes.contains(&0) {
        return Err(not_text("it contains NUL bytes, so it is probably a binary file"));
    } else {
        match String::from_utf8(bytes.to_vec()) {
            Ok(s) => s,
            Err(_) => bytes.iter().map(|&b| windows_1252(b)).collect(),
        }
    };
    Ok(normalize_newlines(&text))
}

fn decode_utf16(bytes: &[u8], make: fn([u8; 2]) -> u16) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|c| make([c[0], c[1]])).collect();
    String::from_utf16(&units).ok()
}

/// CRLF and a lone CR both become LF.
pub fn normalize_newlines(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Windows-1252 code points for the bytes 0x80..=0x9F; 0 marks the five
/// bytes Windows leaves undefined (they map to the control character with the
/// same number).
const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0, 0x017D, 0, 0,
    0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0, 0x017E, 0x0178,
];

fn windows_1252(b: u8) -> char {
    if (0x80..=0x9F).contains(&b) {
        let cp = CP1252_HIGH[(b - 0x80) as usize];
        if cp != 0 {
            return char::from_u32(cp as u32).unwrap_or('\u{FFFD}');
        }
    }
    char::from(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(bytes: &[u8]) -> Result<String, IoError> {
        decode_text(bytes, Path::new("x.txt"))
    }

    #[test]
    fn utf8_with_and_without_bom() {
        assert_eq!(dec("a\u{e9}\n".as_bytes()).unwrap(), "a\u{e9}\n");
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice(b"abc");
        assert_eq!(dec(&b).unwrap(), "abc");
    }

    #[test]
    fn utf16_both_orders() {
        let le: Vec<u8> =
            [0xFF, 0xFE].into_iter().chain("h\u{e9}\r\n".encode_utf16().flat_map(u16::to_le_bytes)).collect();
        assert_eq!(dec(&le).unwrap(), "h\u{e9}\n");
        let be: Vec<u8> = [0xFE, 0xFF].into_iter().chain("hi".encode_utf16().flat_map(u16::to_be_bytes)).collect();
        assert_eq!(dec(&be).unwrap(), "hi");
        assert!(dec(&[0xFF, 0xFE, 0x41]).is_err(), "odd length");
    }

    #[test]
    fn windows_1252_fallback_and_newlines() {
        // 0xB0 = degree sign, 0x93/0x94 = curly quotes, lone CR and CRLF.
        let bytes = [b'2', b'5', 0xB0, b'C', b'\r', 0x93, b'x', 0x94, b'\r', b'\n'];
        assert_eq!(dec(&bytes).unwrap(), "25\u{b0}C\n\u{201c}x\u{201d}\n");
    }

    #[test]
    fn binary_is_refused() {
        let e = dec(&[1, 2, 0, 3]).unwrap_err();
        assert!(e.to_string().contains("x.txt"));
    }
}
