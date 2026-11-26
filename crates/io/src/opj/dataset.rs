//! The data-set section of an `.opj` file: one header, one data block and one mask
//! block per column of every workbook (spec section 3), and the decoding of values.

use super::frame::{Extent, FrameError, Frames};
use super::strings::field_text;
use sheaf_core::column::ColumnData;
use std::io::{Read, Seek};

/// Offsets inside a data-set header block.
const OFF_TYPE: usize = 0x16;
const OFF_ROWS: usize = 0x19;
const OFF_FIRST: usize = 0x1D;
const OFF_LAST: usize = 0x21;
const OFF_CELL_SIZE: usize = 0x3D;
const OFF_FLAGS: usize = 0x3F;
const OFF_NAME: usize = 0x58;
const NAME_WIDTH: usize = 25;
/// The shortest header that still holds the whole name field.
const MIN_HEADER: usize = OFF_NAME + NAME_WIDTH - 1;

/// Bit of the type code that marks a cell that can hold text or a number.
const TYPE_MIXED: u16 = 0x0100;
/// Bit of the type code that marks an integer type.
const TYPE_INTEGER: u16 = 0x0800;
/// Flag bit "unsigned" (integers only).
const FLAG_UNSIGNED: u8 = 0x08;

/// The number Origin stores for "no value" in a double.
pub(crate) const MISSING: f64 = -1.234_567_89e-300;

/// What the header of one data set says; the values stay in the file until asked for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DataSetInfo {
    /// `Window_Column` with a `@n` suffix for sheets after the first.
    pub name: String,
    pub type_code: u16,
    /// Bytes per cell.
    pub cell_size: usize,
    pub unsigned: bool,
    /// Rows the header claims (the data block decides the real count).
    pub declared_rows: u32,
    pub data: Extent,
    /// Where the mask block is (empty when nothing of the column is masked).
    pub mask: Extent,
    /// The rows that hold values: the first and one past the last (`0x1D`, `0x21`; T476).
    pub used: (u32, u32),
}

/// Offset of the first row's byte in a mask block, and the bytes after the last row.
const MASK_ROWS_AT: usize = 40;
const MASK_TAIL: usize = 4;

/// The masked rows (0-based) of a mask block, measured on projects saved by Origin 2016 with
/// `wcellmask` (opj-format.md 3.2): 4 bytes that vary between saves, the block's own length as a
/// little-endian u32, 32 zero bytes, one byte per row (1 = masked) and 4 zero bytes. A block whose
/// length field disagrees with its size, or that is too short, gives no mask.
pub(crate) fn decode_mask(b: &[u8]) -> std::collections::BTreeSet<usize> {
    let mut rows = std::collections::BTreeSet::new();
    if b.len() < MASK_ROWS_AT + MASK_TAIL {
        return rows;
    }
    let stated = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
    if stated != b.len() {
        return rows;
    }
    for (r, &flag) in b[MASK_ROWS_AT..b.len() - MASK_TAIL].iter().enumerate() {
        if flag != 0 {
            rows.insert(r);
        }
    }
    rows
}

/// Reads the headers of all data sets; the data blocks are stepped over, not read.
/// Stops quietly at the first damaged block and says why in the second result.
pub(crate) fn scan<R: Read + Seek>(f: &mut Frames<R>) -> (Vec<DataSetInfo>, Option<String>) {
    let mut sets = Vec::new();
    loop {
        match scan_one(f) {
            Ok(Some(info)) => sets.push(info),
            Ok(None) => return (sets, None),
            Err(e) => {
                let why = format!("data set {}: {e}", sets.len() + 1);
                return (sets, Some(why));
            }
        }
    }
}

fn scan_one<R: Read + Seek>(f: &mut Frames<R>) -> Result<Option<DataSetInfo>, FrameError> {
    let header = f.block()?;
    if header.is_empty() {
        return Ok(None);
    }
    let data = f.skip_block()?;
    let mask = f.skip_block()?;
    Ok(Some(parse_header(&header, data, mask)))
}

pub(crate) fn parse_header(h: &[u8], data: Extent, mask: Extent) -> DataSetInfo {
    let u16_at = |o: usize| h.get(o..o + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| h.get(o..o + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let mut cell_size = usize::from(h.get(OFF_CELL_SIZE).copied().unwrap_or(0));
    if cell_size == 0 {
        cell_size = 8;
    }
    DataSetInfo {
        name: if h.len() >= MIN_HEADER { field_text(h, OFF_NAME, NAME_WIDTH) } else { String::new() },
        type_code: u16_at(OFF_TYPE),
        cell_size,
        unsigned: h.get(OFF_FLAGS).is_some_and(|f| f & FLAG_UNSIGNED != 0),
        declared_rows: u32_at(OFF_ROWS),
        data,
        mask,
        used: (u32_at(OFF_FIRST), u32_at(OFF_LAST)),
    }
}

/// A decoded column plus facts the caller may want to report.
#[derive(Debug, PartialEq)]
pub(crate) struct Decoded {
    pub data: ColumnData,
    /// The cells held both text and numbers, so everything became text.
    /// Bytes left over after the last whole cell.
    pub stray_bytes: usize,
}

/// Turns the bytes of a data block into a column. The error says in plain words why
/// the type is not supported.
pub(crate) fn decode(info: &DataSetInfo, bytes: &[u8]) -> Result<Decoded, String> {
    let t = info.type_code;
    let low = (t & 0xFF) as u8;
    let size = info.cell_size;
    let stray = bytes.len() % size;
    let body = &bytes[..bytes.len() - stray];
    let numeric = |v: Vec<f64>| Decoded { data: ColumnData::numeric(v), stray_bytes: stray };

    if t & TYPE_MIXED != 0 {
        if size < 10 {
            return Err(format!("text-and-number cells of {size} bytes"));
        }
        return Ok(decode_mixed(body, size, stray));
    }
    if t & TYPE_INTEGER != 0 {
        return match (size, info.unsigned) {
            (1, false) => Ok(numeric(body.iter().map(|&b| f64::from(b as i8)).collect())),
            (1, true) => Ok(numeric(body.iter().map(|&b| f64::from(b)).collect())),
            (2, false) => Ok(numeric(each::<2>(body, |c| f64::from(i16::from_le_bytes(c))))),
            (2, true) => Ok(numeric(each::<2>(body, |c| f64::from(u16::from_le_bytes(c))))),
            (4, false) => Ok(numeric(each::<4>(body, |c| f64::from(i32::from_le_bytes(c))))),
            (4, true) => Ok(numeric(each::<4>(body, |c| f64::from(u32::from_le_bytes(c))))),
            _ => Err(format!("integers of {size} bytes")),
        };
    }
    if low & 0xF0 == 0x20 {
        // Fixed-width text, NUL padded.
        let v = body.chunks_exact(size).map(|c| field_text(c, 0, size)).collect();
        return Ok(Decoded { data: ColumnData::text(v), stray_bytes: stray });
    }
    if low < 0x10 {
        match size {
            8 => {
                let v = each::<8>(body, |c| {
                    let x = f64::from_le_bytes(c);
                    if x.to_bits() == MISSING.to_bits() { f64::NAN } else { x }
                });
                return Ok(numeric(v));
            }
            4 => return Ok(numeric(each::<4>(body, |c| f64::from(f32::from_le_bytes(c))))),
            16 => return Err("complex numbers".to_string()),
            _ => {}
        }
    }
    Err(format!("a data type that is not supported (code {t:#06x}, {size} bytes per cell)"))
}

/// Applies `f` to every whole `N`-byte group of `bytes`.
fn each<const N: usize>(bytes: &[u8], f: impl Fn([u8; N]) -> f64) -> Vec<f64> {
    bytes.as_chunks::<N>().0.iter().map(|c| f(*c)).collect()
}

/// Cells of `size` bytes: a kind byte (1 = text), one more byte, then 8 bytes of
/// double or `size - 2` bytes of NUL-padded text.
fn decode_mixed(body: &[u8], size: usize, stray: usize) -> Decoded {
    let rows = body.len() / size;
    let mut numbers = Vec::with_capacity(rows);
    let mut texts: std::collections::BTreeMap<usize, String> = std::collections::BTreeMap::new();
    let mut any_number = false;
    for (row, cell) in body.chunks_exact(size).enumerate() {
        if cell[0] == 1 {
            let s = field_text(cell, 2, size - 2);
            if !s.is_empty() {
                texts.insert(row, s);
            }
            numbers.push(f64::NAN);
        } else {
            let x = f64::from_le_bytes([cell[2], cell[3], cell[4], cell[5], cell[6], cell[7], cell[8], cell[9]]);
            if x.to_bits() == MISSING.to_bits() || x.is_nan() {
                numbers.push(f64::NAN);
            } else {
                numbers.push(x);
                any_number = true;
            }
        }
    }
    if !texts.is_empty() && !any_number {
        // Only text: a text column.
        let mut strings = vec![String::new(); rows];
        for (row, s) in texts {
            strings[row] = s;
        }
        return Decoded { data: ColumnData::text(strings), stray_bytes: stray };
    }
    // Origin stores every column as text-or-number cells and calls the format Text & Numeric.
    Decoded { data: ColumnData::mixed(numbers, texts), stray_bytes: stray }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(type_code: u16, cell_size: usize, unsigned: bool) -> DataSetInfo {
        DataSetInfo {
            name: "Book1_A".into(),
            type_code,
            cell_size,
            unsigned,
            declared_rows: 0,
            data: Extent { pos: 0, len: 0 },
            mask: Extent { pos: 0, len: 0 },
            used: (0, 0),
        }
    }

    fn mixed_number(x: f64) -> Vec<u8> {
        let mut c = vec![0u8, 0];
        c.extend_from_slice(&x.to_le_bytes());
        c
    }

    fn mixed_text(s: &str, size: usize) -> Vec<u8> {
        let mut c = vec![1u8, 0];
        c.extend_from_slice(s.as_bytes());
        c.resize(size, 0);
        c
    }

    fn numbers(d: &Decoded) -> Vec<f64> {
        match &d.data {
            ColumnData::Numeric(v) | ColumnData::Mixed { numbers: v, .. } => v.to_vec(),
            ColumnData::Text(_) => panic!("expected numbers"),
        }
    }

    #[test]
    fn plain_doubles_with_the_missing_marker() {
        let mut b = Vec::new();
        for x in [1.5, MISSING, -2.0, f64::NAN] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        let d = decode(&info(0x6001, 8, false), &b).unwrap();
        let v = numbers(&d);
        assert_eq!(v[0], 1.5);
        assert!(v[1].is_nan());
        assert_eq!(v[2], -2.0);
        assert!(v[3].is_nan());
        assert_eq!(d.stray_bytes, 0);
    }

    #[test]
    fn floats_and_integers_of_every_width() {
        let f: Vec<u8> = [0.5f32, -3.0].iter().flat_map(|x| x.to_le_bytes()).collect();
        assert_eq!(numbers(&decode(&info(0x6003, 4, false), &f).unwrap()), vec![0.5, -3.0]);
        let i4: Vec<u8> = (-7i32).to_le_bytes().into_iter().chain(9i32.to_le_bytes()).collect();
        assert_eq!(numbers(&decode(&info(0x6801, 4, false), &i4).unwrap()), vec![-7.0, 9.0]);
        assert_eq!(numbers(&decode(&info(0x6801, 4, true), &(u32::MAX).to_le_bytes()).unwrap()), vec![4294967295.0]);
        let i2: Vec<u8> = (-2i16).to_le_bytes().into_iter().chain(300i16.to_le_bytes()).collect();
        assert_eq!(numbers(&decode(&info(0x6803, 2, false), &i2).unwrap()), vec![-2.0, 300.0]);
        assert_eq!(numbers(&decode(&info(0x6803, 2, true), &65535u16.to_le_bytes()).unwrap()), vec![65535.0]);
        assert_eq!(numbers(&decode(&info(0x6821, 1, false), &[0xFF, 5]).unwrap()), vec![-1.0, 5.0]);
        assert_eq!(numbers(&decode(&info(0x6821, 1, true), &[0xFF, 5]).unwrap()), vec![255.0, 5.0]);
        assert!(decode(&info(0x6801, 3, false), &[0; 6]).is_err());
    }

    #[test]
    fn stray_bytes_after_the_last_whole_cell_are_ignored_and_counted() {
        let mut b = 1.0f64.to_le_bytes().to_vec();
        b.extend_from_slice(&[1, 2, 3]);
        let d = decode(&info(0x6001, 8, false), &b).unwrap();
        assert_eq!(numbers(&d), vec![1.0]);
        assert_eq!(d.stray_bytes, 3);
    }

    #[test]
    fn mixed_cells_that_are_all_numbers_give_a_text_and_numeric_column() {
        let mut b = mixed_number(2.5);
        b.extend(mixed_number(MISSING));
        b.extend(mixed_number(-1.0));
        let d = decode(&info(0x6121, 10, false), &b).unwrap();
        let v = numbers(&d);
        assert_eq!(v.len(), 3);
        assert_eq!((v[0], v[2]), (2.5, -1.0));
        assert!(v[1].is_nan());
    }

    #[test]
    fn mixed_cells_that_are_all_text_give_a_text_column() {
        let mut b = mixed_text("ab", 12);
        b.extend(mixed_text("", 12));
        let mut tail = mixed_text("caf", 12);
        tail[2 + 4] = 0xE9; // a byte after the NUL terminator is ignored
        b.extend(tail);
        let d = decode(&info(0x5121, 12, false), &b).unwrap();
        match d.data {
            ColumnData::Text(t) => assert_eq!(&t[..], &["ab".to_string(), String::new(), "caf".to_string()]),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn mixed_text_decodes_windows_1252() {
        let mut c = vec![1u8, 0, b'5', 0xB0, b'C'];
        c.resize(10, 0);
        let d = decode(&info(0x6121, 10, false), &c).unwrap();
        match d.data {
            ColumnData::Text(t) => assert_eq!(t[0], "5\u{b0}C"),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn a_column_with_text_and_numbers_keeps_both() {
        let mut b = mixed_number(3.0);
        b.extend(mixed_text("n/a", 10));
        b.extend(mixed_number(MISSING));
        b.extend(mixed_number(0.25));
        let d = decode(&info(0x6121, 10, false), &b).unwrap();
        match d.data {
            ColumnData::Mixed { numbers, texts } => {
                assert_eq!((numbers[0], numbers[3]), (3.0, 0.25));
                assert!(numbers[1].is_nan() && numbers[2].is_nan());
                assert_eq!(texts.len(), 1);
                assert_eq!(texts[&1], "n/a");
            }
            _ => panic!("expected a text and numeric column"),
        }
    }

    #[test]
    fn empty_text_cells_count_as_missing_numbers() {
        let mut b = mixed_text("", 10);
        b.extend(mixed_number(4.0));
        let d = decode(&info(0x6121, 10, false), &b).unwrap();
        let v = numbers(&d);
        assert!(v[0].is_nan());
        assert_eq!(v[1], 4.0);
    }

    #[test]
    fn fixed_width_text_columns() {
        let mut b = Vec::new();
        for s in ["one", "", "three"] {
            let mut c = s.as_bytes().to_vec();
            c.resize(6, 0);
            b.extend(c);
        }
        let d = decode(&info(0x6021, 6, false), &b).unwrap();
        match d.data {
            ColumnData::Text(t) => assert_eq!(&t[..], &["one".to_string(), String::new(), "three".into()]),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn unsupported_types_say_why() {
        assert!(decode(&info(0x0201, 16, false), &[0; 32]).unwrap_err().contains("complex"));
        assert!(decode(&info(0x6081, 100, false), &[0; 100]).is_err());
        assert!(decode(&info(0x0121, 4, false), &[0; 8]).is_err());
        assert!(decode(&info(0x6001, 8, false), &[]).unwrap().data.is_empty());
    }

    #[test]
    fn headers_are_read_by_offset_and_short_headers_do_not_panic() {
        let mut h = vec![0u8; 147];
        h[0x16] = 0x21;
        h[0x17] = 0x61;
        h[0x19..0x1D].copy_from_slice(&7u32.to_le_bytes());
        h[0x3D] = 10;
        h[0x3F] = 8;
        h[0x58..0x58 + 7].copy_from_slice(b"Book1_A");
        let i = parse_header(&h, Extent { pos: 9, len: 70 }, Extent { pos: 0, len: 0 });
        assert_eq!(
            (i.name.as_str(), i.type_code, i.cell_size, i.declared_rows, i.unsigned),
            ("Book1_A", 0x6121, 10, 7, true)
        );
        assert_eq!(i.data, Extent { pos: 9, len: 70 });
        let short = parse_header(&h[..0x30], Extent { pos: 0, len: 0 }, Extent { pos: 0, len: 0 });
        assert_eq!((short.name.as_str(), short.cell_size), ("", 8));
        let none = parse_header(&[], Extent { pos: 0, len: 0 }, Extent { pos: 0, len: 0 });
        assert_eq!(none.type_code, 0);
    }

    #[test]
    fn mask_blocks_hold_one_byte_per_row_after_a_40_byte_head() {
        // As Origin 2016 wrote it for 21 rows with rows 1, 2 and 5 masked (0-based 0, 1, 4).
        let mut b = vec![0u8; 65];
        b[..4].copy_from_slice(&[0xc0, 0x09, 0x41, 0x0f]);
        b[4] = 65;
        for r in [0, 1, 4] {
            b[40 + r] = 1;
        }
        assert_eq!(decode_mask(&b).into_iter().collect::<Vec<_>>(), vec![0, 1, 4]);
        // a length field that disagrees, or a short block, gives nothing
        let mut wrong = b.clone();
        wrong[4] = 64;
        assert!(decode_mask(&wrong).is_empty());
        assert!(decode_mask(&b[..30]).is_empty());
        assert!(decode_mask(&[]).is_empty());
    }
}
