//! Text fields of an `.opj` file: fixed-width NUL-padded names, the Windows-1252
//! code page, and the "long name / units / comments" layout of a column note.

/// Code points of the bytes 0x80..=0x9F in Windows-1252. The five bytes the code page
/// leaves undefined map to the control character with the same number.
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{02C6}',
    '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8D}', '\u{017D}', '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}',
    '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
    '\u{0153}', '\u{9D}', '\u{017E}', '\u{0178}',
];

/// Decodes Windows-1252 bytes (every byte value is defined).
pub(crate) fn decode_cp1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => CP1252_HIGH[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}

/// The Windows-1252 byte of a character, or `None` when the code page has no such character.
pub(crate) fn encode_cp1252_char(c: char) -> Option<u8> {
    match u32::from(c) {
        n @ (0x00..=0x7F | 0xA0..=0xFF) => Some(n as u8),
        // The bytes 0x80..=0x9F: the table gives the character each one decodes to, which also
        // covers the five bytes the code page leaves undefined (they decode to a control character).
        _ => CP1252_HIGH.iter().position(|&h| h == c).map(|i| 0x80 + i as u8),
    }
}

/// Encodes text as Windows-1252. The flag is true when a character had no byte and was replaced by `?`.
pub(crate) fn encode_cp1252(text: &str) -> (Vec<u8>, bool) {
    let mut lossy = false;
    let bytes = text
        .chars()
        .map(|c| {
            encode_cp1252_char(c).unwrap_or_else(|| {
                lossy = true;
                b'?'
            })
        })
        .collect();
    (bytes, lossy)
}

/// The bytes before the first NUL (or all of them).
pub(crate) fn c_bytes(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(n) => &bytes[..n],
        None => bytes,
    }
}

/// A fixed-width, NUL-terminated name field read from `bytes[start..start + width]`
/// (clipped to what exists), decoded as Windows-1252.
pub(crate) fn field_text(bytes: &[u8], start: usize, width: usize) -> String {
    let end = start.saturating_add(width).min(bytes.len());
    if start >= end {
        return String::new();
    }
    decode_cp1252(c_bytes(&bytes[start..end]))
}

/// Marker that starts the machine-readable storage appended to labels and comments.
const STORAGE_MARK: &str = "@${";

/// The human-readable part of a label or comment: everything before the storage mark.
pub(crate) fn visible_part(text: &str) -> &str {
    match text.find(STORAGE_MARK) {
        Some(n) => &text[..n],
        None => text,
    }
}

/// Splits the note of a column into (long name, units, comments).
/// The note is `long name CRLF units CRLF comments`, optionally followed by storage.
pub(crate) fn column_labels(note: &[u8]) -> (String, String, String) {
    let text = decode_cp1252(c_bytes(note));
    let visible = visible_part(&text);
    let mut parts = visible.splitn(3, "\r\n");
    let long = parts.next().unwrap_or("").to_string();
    let units = parts.next().unwrap_or("").to_string();
    let comments = parts.next().unwrap_or("").to_string();
    (long, units, comments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp1252_maps_the_special_range_and_latin1() {
        assert_eq!(decode_cp1252(b"abc"), "abc");
        assert_eq!(decode_cp1252(&[0xB0, b'C']), "\u{b0}C");
        assert_eq!(decode_cp1252(&[0x80, 0x93, 0x94, 0xE9]), "\u{20AC}\u{201C}\u{201D}\u{e9}");
        assert_eq!(decode_cp1252(&[0x81]), "\u{81}");
    }

    #[test]
    fn cp1252_encoding_is_the_inverse_of_decoding_and_marks_what_does_not_fit() {
        for b in 0..=255u8 {
            let c = decode_cp1252(&[b]).chars().next().unwrap();
            assert_eq!(encode_cp1252_char(c), Some(b), "byte {b:#04x}");
        }
        assert_eq!(
            encode_cp1252("5\u{b0}C \u{b5}m caf\u{e9}"),
            (vec![b'5', 0xB0, b'C', b' ', 0xB5, b'm', b' ', b'c', b'a', b'f', 0xE9], false)
        );
        assert_eq!(encode_cp1252("\u{20AC}"), (vec![0x80], false));
        assert_eq!(encode_cp1252("a\u{394}b"), (b"a?b".to_vec(), true));
        assert_eq!(encode_cp1252("\u{1F600}"), (b"?".to_vec(), true));
    }

    #[test]
    fn fixed_fields_stop_at_nul_and_never_overrun() {
        let b = b"xxAB\0\0CD\0\0";
        assert_eq!(field_text(b, 2, 6), "AB");
        assert_eq!(field_text(b, 2, 100), "AB");
        assert_eq!(field_text(b, 50, 5), "");
        assert_eq!(field_text(b, usize::MAX, 5), "");
        assert_eq!(field_text(b, 0, 2), "xx");
    }

    #[test]
    fn column_notes_split_into_three_and_drop_the_storage_tail() {
        assert_eq!(
            column_labels(b"Time\r\ns\r\nfrom the logger"),
            ("Time".into(), "s".into(), "from the logger".into())
        );
        assert_eq!(column_labels(b"Only a name"), ("Only a name".into(), String::new(), String::new()));
        assert_eq!(column_labels(b"\r\n\r\nnote"), (String::new(), String::new(), "note".into()));
        assert_eq!(
            column_labels(b"Name@${[0|3|SYSTEM|32|1]}[44]\n151=\"\"\n"),
            ("Name".into(), String::new(), String::new())
        );
        assert_eq!(column_labels(b"A\r\nB\r\nC\r\nD\0junk"), ("A".into(), "B".into(), "C\r\nD".into()));
        assert_eq!(column_labels(b""), (String::new(), String::new(), String::new()));
        assert_eq!(column_labels(b"\0"), (String::new(), String::new(), String::new()));
    }
}
