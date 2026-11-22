//! The byte layout of the blocks of a written project (spec section 13).
//!
//! Every block starts as zero bytes. The fields that are known or were observed in projects written
//! by Origin are then filled in, either from a value that depends on the data (a name, a count)
//! or from a short table of constants whose meaning is not known. Offsets are into the block body.

use super::super::strings::encode_cp1252;

/// The first line of the file: the signature of Origin, build counter included.
pub(super) const SIGNATURE: &[u8] = b"CPYA 4.3168 226 W64 #\n";

/// The Origin version stored in the header block (9.3 is Origin).
const VERSION: f64 = 9.300_226;

/// A field of constant bytes: offset and value. Meanings that are known are in the comments; the
/// others were seen with these values in every project written by Origin.
pub(super) type Fields = &'static [(usize, &'static [u8])];

pub(super) fn fill(buf: &mut [u8], fields: Fields) {
    for (at, bytes) in fields {
        buf[*at..*at + bytes.len()].copy_from_slice(bytes);
    }
}

pub(super) fn put(buf: &mut [u8], at: usize, bytes: &[u8]) {
    buf[at..at + bytes.len()].copy_from_slice(bytes);
}

/// A block with its size field and delimiters (spec section 2.2).
pub(super) fn framed(body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(body.len() + 6);
    v.extend_from_slice(&(body.len() as u32).to_le_bytes());
    v.push(b'\n');
    if !body.is_empty() {
        v.extend_from_slice(body);
        v.push(b'\n');
    }
    v
}

/// Where Origin keeps a name: NUL-terminated Windows-1252 in a field of `width` bytes.
pub(super) fn put_name(buf: &mut [u8], at: usize, width: usize, text: &str) {
    let (bytes, _) = encode_cp1252(text);
    let n = bytes.len().min(width - 1);
    put(buf, at, &bytes[..n]);
}

// ----- file header ------------------------------------------------------------------------------

const HEADER_FIELDS: Fields = &[
    (0x02, &[0x82]),
    (0x04, &[0x82]),
    (0x06, &[0x22, 0x06, 0x73, 0x03]),
    (0x0e, &[0x58]),
    (0x17, &[0x01]),
    (0x25, &[0x20]),
    (0x27, &[0x01]),
    (0x2b, &[0x7c, 0x49]),
    (0x2f, &[0xd4, 0x6b]),
    (0x33, &[0xf4, 0x53]),
    (0x37, &[0x4c, 0x53]),
    (0x3b, &[0x89, 0x7e]),
    (0x3f, &[0xb7, 0x4f]),
    (0x43, &[0x6f, 0x37]),
];

/// The version header block (111 bytes, spec section 2.3).
pub(super) fn header() -> Vec<u8> {
    let mut h = vec![0u8; 111];
    fill(&mut h, HEADER_FIELDS);
    put(&mut h, 0x1B, &VERSION.to_le_bytes());
    h
}

// ----- data sets --------------------------------------------------------------------------------

/// What a data-set header says about one column.
pub(super) struct DatasetSpec<'a> {
    /// `Window_Column`, with `@n` for sheets after the first.
    pub name: &'a str,
    pub type_code: u16,
    /// Bytes per cell.
    pub cell_size: u8,
    pub rows: u32,
    /// Position of the first and one past the last cell that holds a value (0 and 0 when none).
    pub first: u32,
    pub last: u32,
    /// 1-based position of the column this one belongs to (its X, or for error, Z and label columns
    /// its Y); 0 for none.
    pub parent: u8,
    /// The first X and the step of a sampling interval (T303); Origin's default is 1 and 1.
    pub sampling: (f64, f64),
}

const DATASET_FIELDS: Fields = &[(0x02, &[0x01]), (0x06, &[0xfa]), (0x0a, &[0xa0, 0x0f, 0x01]), (0x71, &[0xca, 0x10])];

/// A data-set header block (147 bytes, spec section 3.1).
pub(super) fn dataset_header(s: &DatasetSpec) -> Vec<u8> {
    let mut h = vec![0u8; 147];
    fill(&mut h, DATASET_FIELDS);
    put(&mut h, 0x16, &s.type_code.to_le_bytes());
    h[0x18] = if s.last > 0 { 3 } else { 1 };
    put(&mut h, 0x19, &s.rows.to_le_bytes());
    put(&mut h, 0x1d, &s.first.to_le_bytes());
    put(&mut h, 0x21, &s.last.to_le_bytes());
    put(&mut h, 0x25, &s.sampling.0.to_le_bytes());
    put(&mut h, 0x2d, &s.sampling.1.to_le_bytes());
    h[0x3d] = s.cell_size;
    h[0x3e] = 0x14;
    h[0x52] = s.parent;
    put_name(&mut h, 0x58, 25, s.name);
    h
}

// ----- windows ----------------------------------------------------------------------------------

/// What a window header says.
pub(super) struct WindowSpec<'a> {
    pub name: &'a str,
    /// The long name; may be empty.
    pub label: &'a str,
    /// 0-based position among the windows written, which places the window on the screen.
    pub ordinal: usize,
    pub active_sheet: usize,
    /// Creation and change time as a Julian day number.
    pub time: f64,
    /// Where the window was in the workspace; the cascade is used when unknown.
    pub place: Option<sheaf_core::project::WindowPlace>,
}

const WINDOW_FIELDS: Fields = &[
    (0x2e, &[0x13]),
    (0x30, &[0x13]),
    (0x32, &[0x50]),
    (0x37, &[0x20]),
    (0x3e, &[0x05]),
    (0x40, &[0x04]),
    (0x42, &[0x02, 0x11, 0x11]),
    (0x4f, &[0x2c, 0x01, 0x46]),
    (0x54, &[0x64]),
    (0x57, &[0x58, 0x02, 0x58, 0x02]),
    (0x67, &[0x01]),
    (0x69, &[0x13]),
    (0x6f, &[0xc3]),
    (0x87, &[0xc0]),
    (0x89, &[0x10]),
    (0x8b, &[0x58, 0x02, 0x58, 0x02, 0xff, 0xff, 0xff, 0xff]),
    (0x97, &[0x11]),
    (0x9b, &[0xfc, 0xff, 0xff, 0xff]),
    (0xa0, &[0x39]),
];

/// The offset of the window label in a window header.
const LABEL_AT: usize = 0xC3;

/// A window header block (spec section 4.1): the short name, the position and size on the screen,
/// the template name `ORIGIN`, the times and the label.
pub(super) fn window_header(w: &WindowSpec) -> Vec<u8> {
    let (label, _) = encode_cp1252(w.label);
    let mut h = vec![0u8; LABEL_AT + label.len() + 1];
    fill(&mut h, WINDOW_FIELDS);
    put_name(&mut h, 0x02, 25, w.name);
    // left, top, right, bottom of the window and the size of the screen it was made on; each window
    // sits 26 pixels right of and below the one before it
    let shift = 26 * (w.ordinal % 10) as i16;
    let rect = window_rect(w.place).unwrap_or([20 + shift, 20 + shift, 416 + shift, 342 + shift]);
    for (i, v) in rect.into_iter().chain([1920, 1080]).enumerate() {
        put(&mut h, 0x1B + 2 * i, &v.to_le_bytes());
    }
    set_window_state(&mut h, w.place);
    h[0x29] = w.active_sheet.min(255) as u8;
    put(&mut h, 0x45, b"ORIGIN");
    put(&mut h, 0x73, &w.time.to_le_bytes());
    put(&mut h, 0x7b, &w.time.to_le_bytes());
    put(&mut h, LABEL_AT, &label);
    h
}

// ----- layers (sheets) --------------------------------------------------------------------------

/// What a layer header says about one sheet.
pub(super) struct LayerSpec<'a> {
    pub name: &'a str,
    pub columns: usize,
    /// 0-based position of the sheet in its workbook.
    pub index_in_book: usize,
    /// 0-based position of the sheet among all sheets of the project.
    pub ordinal: usize,
}

const LAYER_FIELDS: Fields = &[
    (0x02, &[0x5e]),
    (0x25, &[0xf0, 0x3f, 0x08]),
    (0x2d, &[0x40, 0x01]),
    (0x37, &[0x01]),
    (0x48, &[0x22, 0x40]),
    (0x50, &[0xf0, 0x3f, 0x0e]),
    (0x56, &[0x06]),
    (0x58, &[0x40, 0x01]),
    (0x62, &[0x01]),
    (0x65, &[0x10, 0x10]),
    (0x6d, &[0x0f]),
    (0x71, &[0x56]),
    (0x73, &[0x50]),
    (0x75, &[0x67, 0x01, 0x06, 0x01]),
    (0x81, &[0x20, 0x28, 0x56, 0x80]),
    (0x87, &[0x05, 0x01, 0x01, 0x07]),
    (0x91, &[0xf0, 0x3f]),
    (0xa4, &[0x1c]),
    (0xa7, &[0x03, 0x12, 0x12]),
    (0xae, &[0x80]),
    (0xb8, &[0x08]),
    (0xc0, &[0x0a, 0xd7, 0xa3, 0x3d, 0x0a, 0xd7, 0xa3, 0x3d, 0x0a]),
    (0xca, &[0x14]),
    (0xd0, b"Pd"),
    (0xf7, &[0xb8, 0x0b]),
    (0xfb, &[0x02, 0x01]),
    (0xfe, &[0x04]),
    (0x105, &[0x12]),
    (0x109, &[0xfc, 0xff, 0xff, 0xff]),
    (0x119, &[0xfc, 0xff, 0xff, 0xff]),
    (0x121, &[0xfc, 0xff, 0xff, 0xff]),
    (0x129, &[0x06]),
    (0x131, &[0x07]),
    (0x13d, &[0x7b, 0x14, 0xae, 0x47, 0xe1, 0x7a, 0xb4, 0x3f]),
];

/// The width of the sheet name field of a layer header, terminator included.
pub(super) const SHEET_NAME_WIDTH: usize = 32;

/// A layer header block (361 bytes, spec section 4.2).
pub(super) fn layer_header(l: &LayerSpec) -> Vec<u8> {
    let mut h = vec![0u8; 361];
    fill(&mut h, LAYER_FIELDS);
    put_name(&mut h, 0xD2, SHEET_NAME_WIDTH, l.name);
    // the number of columns, which Origin shows as the visible ones (at most 4) in a double
    let n = l.columns.min(u16::MAX as usize) as u16;
    put(&mut h, 0x17, &f64::from(n.min(4)).to_le_bytes());
    put(&mut h, 0x2b, &n.to_le_bytes());
    put(&mut h, 0x151, &n.to_le_bytes());
    put(&mut h, 0x101, &(802 + l.ordinal.min(60_000) as u16).to_le_bytes());
    h[0x12d] = l.index_in_book.min(255) as u8;
    h
}

// ----- column records ---------------------------------------------------------------------------

/// What a column record says.
pub(super) struct ColumnSpec<'a> {
    /// 1-based position of the data set in the data-set list.
    pub dataset: u16,
    /// The role code (spec section 4.3).
    pub designation: u8,
    pub short_name: &'a str,
    pub type_code: u16,
    pub first: u32,
    pub last: u32,
    pub parent: u16,
    /// Position of the column in its sheet; the first two get the default width.
    pub index: usize,
    /// The column's own width in characters (T334), written in tenths at `0x4A`.
    pub width: Option<u16>,
    /// The number that ties the column to the recalculation records (0 for a column that takes
    /// no part in one; spec section 13.10).
    pub object_id: u16,
    /// A sampling interval (first X, step) and whether the sheet has an X column it would else
    /// belong to (T303: Origin then writes 0xFFFF at `0x33`).
    pub sampling: Option<(f64, f64, bool)>,
}

const COLUMN_FIELDS: Fields =
    &[(0x00, &[0x10]), (0x06, &[0x0b]), (0x1e, &[0x09]), (0x33, &[0x01]), (0x39, &[0x20]), (0x1e9, &[0x08])];

/// A column record block (515 bytes, spec section 4.3).
pub(super) fn column_record(c: &ColumnSpec) -> Vec<u8> {
    let mut h = vec![0u8; 515];
    fill(&mut h, COLUMN_FIELDS);
    put(&mut h, 0x04, &c.dataset.to_le_bytes());
    put(&mut h, 0x07, &c.first.to_le_bytes());
    put(&mut h, 0x0b, &c.last.to_le_bytes());
    h[0x11] = c.designation;
    put_name(&mut h, 0x12, 12, c.short_name);
    // error columns carry a marker that tells the X error (0x13) from the Y error (0x03)
    match c.designation {
        2 => h[0x20] = 0x03,
        6 => h[0x20] = 0x13,
        _ => {}
    }
    put(&mut h, 0x23, &c.parent.to_le_bytes());
    put(&mut h, 0x25, &c.type_code.to_le_bytes());
    if c.index < 2 {
        h[0x4a] = 0x43;
    }
    // T334, measured in Origin 2016: wks.col2.width=14 stored 140 here.
    if let Some(w) = c.width {
        put(&mut h, 0x4a, &w.saturating_mul(10).to_le_bytes());
    }
    let (x0, inc) = c.sampling.map_or((1.0, 1.0), |(x0, inc, _)| (x0, inc));
    put(&mut h, 0x1d2, &x0.to_le_bytes());
    put(&mut h, 0x1da, &inc.to_le_bytes());
    if c.sampling.is_some_and(|(_, _, beside_x)| beside_x) {
        put(&mut h, 0x33, &0xFFFFu16.to_le_bytes());
    }
    put(&mut h, 0x1e2, &c.object_id.to_le_bytes());
    h
}

/// The note block of a column: long name, units and comments joined by CR LF, then a NUL.
/// Empty trailing parts are left out; with nothing to say the block is empty.
pub(super) fn column_note(long: &str, units: &str, comments: &str) -> Vec<u8> {
    let mut text = format!("{long}\r\n{units}\r\n{comments}");
    while text.ends_with("\r\n") {
        text.truncate(text.len() - 2);
    }
    if text.is_empty() {
        return Vec::new();
    }
    let (mut bytes, _) = encode_cp1252(&text);
    bytes.push(0);
    bytes
}

/// The window rectangle (left, top, right, bottom in workspace pixels) of a known place, spec 13.14.
pub(super) fn window_rect(place: Option<sheaf_core::project::WindowPlace>) -> Option<[i16; 4]> {
    let p = place?;
    let v = |x: f64| x.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
    if ![p.x, p.y, p.width, p.height].iter().all(|v| v.is_finite()) || p.width < 1.0 || p.height < 1.0 {
        return None;
    }
    Some([v(p.x), v(p.y), v(p.x + p.width), v(p.y + p.height)])
}

/// The state bits of a window header (byte 0x32, spec 13.14).
pub(super) fn set_window_state(h: &mut [u8], place: Option<sheaf_core::project::WindowPlace>) {
    use sheaf_core::project::WindowState;
    match place.map(|p| p.state) {
        Some(WindowState::Minimised) => h[0x32] |= 0x01,
        Some(WindowState::Maximised) => h[0x32] |= 0x02,
        _ => {}
    }
}
