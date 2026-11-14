//! Finding the column records and the window records in the bytes of an `.opju` file.
//!
//! The file is a stream of tagged fields (spec section 3). This reader does not walk the whole
//! tree; it looks for the records it understands by their fixed shapes (spec section 8) and
//! reports what it leaves unread.

use super::numbers::{cut_implausible_tail, decode_strings, decode_values, read_varint};

/// What a column record holds.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnValues {
    /// The values, and how many values at the end were dropped because they cannot be data.
    Numeric(Vec<f64>, usize),
    Text(Vec<String>),
    /// A data block this reader does not decode; the text says which kind.
    Unsupported(&'static str),
}

/// One column (data set) record.
#[derive(Clone, Debug)]
pub struct ColumnRecord {
    /// The stored name, for example `Book1_B` or `Book1_B@2`.
    pub name: String,
    pub values: ColumnValues,
    /// The first bytes of the data block, for diagnostics.
    pub head: Vec<u8>,
}

/// A window record found after a preview picture.
#[derive(Clone, Debug)]
pub struct WindowRecord {
    pub name: String,
    pub long_name: String,
    /// The sheet names in order (workbook sheets or matrix sheets).
    pub sheets: Vec<String>,
    /// True when the block holds the mark of a workbook rather than of a graph.
    pub has_origin_mark: bool,
}

/// The end of a preview picture, the tag of the window record that follows it.
const PNG_END: &[u8] = b"IEND\xaeB`\x82\x0a\x80";
/// What follows the name of every sheet in a window record.
const SHEET_MARK: &[u8] = b"\x0a\x00\x10\x01\x09_SEPD";

fn is_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'@' | b'.' | b' ' | b'-' | b'(' | b')' | b'[' | b']')
}

/// The data-set name field that sits in the up to 120 bytes before a record.
fn find_name(b: &[u8], at: usize) -> Option<String> {
    let lo = at.saturating_sub(120);
    for j in (lo..at).rev() {
        if b[j] < 0x80 {
            continue;
        }
        let len = usize::from(*b.get(j + 1)?);
        if !(3..=100).contains(&len) || j + 2 + len > at {
            continue;
        }
        let s = &b[j + 2..j + 2 + len];
        if s.iter().all(|&c| is_name_byte(c)) && s.contains(&b'_') && s[0] != b'_' {
            return Some(String::from_utf8_lossy(s).into_owned());
        }
    }
    None
}

/// Where the data block of a column record ends: the position of the sentinel `CE FA`.
fn sentinel_after(b: &[u8], from: usize) -> usize {
    b[from..].windows(2).position(|w| w == [0xce, 0xfa]).map_or(b.len(), |p| from + p)
}

fn damaged() -> ColumnValues {
    ColumnValues::Unsupported("a damaged data block")
}

fn read_block(b: &[u8], d: usize, declared: u64) -> ColumnValues {
    // b[d] is the 0x0a that opens the block.
    if b.get(d + 1) != Some(&5) {
        return ColumnValues::Unsupported("a data block of another type");
    }
    let mut p = d + 2;
    let Some(rows) = read_varint(b, &mut p) else { return damaged() };
    let sentinel = sentinel_after(b, d);
    // The declared length is about the end of the numbers; a few bytes of trailer may follow.
    let numeric_end = usize::try_from(declared).ok().and_then(|l| d.checked_add(l)).map(|e| e.saturating_add(16));
    let dense = b.get(p..p + 2) == Some(&[0xff, 0xff][..]);
    if dense {
        p += 2;
        if read_varint(b, &mut p).is_none() {
            return damaged();
        }
        let Some(&flag) = b.get(p) else { return damaged() };
        p += 1;
        if flag != 0 {
            return ColumnValues::Unsupported("a text reference block");
        }
    }
    let mut q = p;
    let Some(count_code) = read_varint(b, &mut q) else { return damaged() };
    if b.get(q) == Some(&0x0c) {
        q += 1;
        // The one-value-per-tag form of the chart sources starts with a zero tag and a length tag.
        if !dense && b.get(q) == Some(&0) && matches!(b.get(q + 1), Some(5..=7)) {
            return ColumnValues::Unsupported("a chart-source copy of values");
        }
        let count = usize::try_from(count_code.div_ceil(2)).unwrap_or(usize::MAX);
        // The sentinel is not used here: its two bytes can occur inside the numbers.
        let end = numeric_end.unwrap_or(sentinel).min(b.len());
        return match decode_values(b, q, end, count) {
            Some((mut values, _)) => {
                let dropped = cut_implausible_tail(&mut values);
                ColumnValues::Numeric(values, dropped)
            }
            None => ColumnValues::Unsupported("numbers that do not fit their block"),
        };
    }
    if dense {
        return ColumnValues::Unsupported("a data block of another kind");
    }
    // Text: `rows` strings, each a length and its bytes, start right after the row count.
    let Ok(rows) = usize::try_from(rows) else { return damaged() };
    match decode_strings(b, p, sentinel, rows) {
        Some(strings) => ColumnValues::Text(strings),
        None => ColumnValues::Unsupported("text that does not fit its block"),
    }
}

/// Every column record of the file, in file order.
pub fn find_columns(b: &[u8]) -> Vec<ColumnRecord> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 27 < b.len() {
        // tag, 0x18, then three 64-bit numbers: 24, the block length, a check value; then 0x0a.
        if b[i] >= 0x80 && b[i + 1] == 0x18 && b[i + 2..i + 10] == [0x18, 0, 0, 0, 0, 0, 0, 0] && b[i + 26] == 0x0a {
            let declared = u64::from_le_bytes(b[i + 10..i + 18].try_into().expect("8 bytes"));
            if let Some(name) = find_name(b, i) {
                let values = read_block(b, i + 26, declared);
                let head = b[i + 26..(i + 26 + 24).min(b.len())].to_vec();
                out.push(ColumnRecord { name, values, head });
            }
            i += 27;
        } else {
            i += 1;
        }
    }
    out
}

/// The window records, in file order.
pub fn find_windows(b: &[u8]) -> Vec<WindowRecord> {
    let mut starts = Vec::new();
    let mut i = 0usize;
    while let Some(pos) = b[i..].windows(PNG_END.len()).position(|w| w == PNG_END) {
        let at = i + pos + PNG_END.len();
        // two bytes, then two zero bytes, then the window name
        let p = at + 2;
        if b.get(p..p + 2) == Some(&[0, 0][..]) {
            let p = p + 2;
            let name: Vec<u8> = b[p..].iter().copied().take_while(|c| (0x20..0x7f).contains(c)).collect();
            if !name.is_empty() && name.len() <= 60 {
                starts.push((at, String::from_utf8_lossy(&name).into_owned()));
            }
        }
        i = at;
    }
    let mut out = Vec::new();
    for (k, (start, name)) in starts.iter().enumerate() {
        let end = starts.get(k + 1).map_or(b.len(), |s| s.0);
        out.push(window_from_block(name, &b[*start..end]));
    }
    out
}

fn window_from_block(name: &str, blk: &[u8]) -> WindowRecord {
    let mut sheets = Vec::new();
    let mut i = 0usize;
    while let Some(pos) = blk[i..].windows(SHEET_MARK.len()).position(|w| w == SHEET_MARK) {
        let at = i + pos;
        // tag, length, name
        for len in 1..=60usize {
            if at >= len + 2 && blk[at - len - 2] >= 0x80 && usize::from(blk[at - len - 1]) == len {
                let s = &blk[at - len..at];
                if s.iter().all(|c| (0x20..0x7f).contains(c)) {
                    sheets.push(String::from_utf8_lossy(s).into_owned());
                    break;
                }
            }
        }
        i = at + SHEET_MARK.len();
    }
    let mut long_name = String::new();
    let head = &blk[..blk.len().min(600)];
    if let Some(pos) = head.windows(4).position(|w| w == [0xc0, 0xff, 0x02, 0x00]) {
        // one byte, 0x9f, the length with the end mark, the text
        let q = pos + 4;
        if head.get(q + 1) == Some(&0x9f)
            && let Some(&len) = head.get(q + 2)
        {
            let len = usize::from(len);
            if len >= 1
                && let Some(text) = head.get(q + 3..q + 2 + len)
            {
                long_name = String::from_utf8_lossy(text).into_owned();
            }
        }
    }
    let has_origin_mark = blk[..blk.len().min(400)].windows(6).any(|w| w == b"ORIGIN");
    WindowRecord { name: name.to_string(), long_name, sheets, has_origin_mark }
}
