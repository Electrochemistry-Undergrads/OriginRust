//! The byte layout of the blocks of a written project (spec section 13).
//!
//! Every block starts as zero bytes. The fields that are known or were observed in projects written
//! by Origin 2016 are then filled in, either from a value that depends on the data (a name, a count)
//! or from a short table of constants whose meaning is not known. Offsets are into the block body.

use super::super::strings::encode_cp1252;

/// The first line of the file: the signature of Origin 2016, build counter included.
pub(super) const SIGNATURE: &[u8] = b"CPYA 4.3168 226 W64 #\n";

/// The Origin version stored in the header block (9.3 is Origin 2016).
const VERSION: f64 = 9.300_226;

/// A field of constant bytes: offset and value. Meanings that are known are in the comments; the
/// others were seen with these values in every project written by Origin 2016.
type Fields = &'static [(usize, &'static [u8])];

fn fill(buf: &mut [u8], fields: Fields) {
    for (at, bytes) in fields {
        buf[*at..*at + bytes.len()].copy_from_slice(bytes);
    }
}

fn put(buf: &mut [u8], at: usize, bytes: &[u8]) {
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
fn put_name(buf: &mut [u8], at: usize, width: usize, text: &str) {
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
    put(&mut h, 0x25, &1.0f64.to_le_bytes());
    put(&mut h, 0x2d, &1.0f64.to_le_bytes());
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
    for (i, v) in [20 + shift, 20 + shift, 416 + shift, 342 + shift, 1920, 1080].into_iter().enumerate() {
        put(&mut h, 0x1B + 2 * i, &v.to_le_bytes());
    }
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
    put(&mut h, 0x1d2, &1.0f64.to_le_bytes());
    put(&mut h, 0x1da, &1.0f64.to_le_bytes());
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

// ----- the part after the windows ---------------------------------------------------------------

/// What the tail of the file says.
pub(super) struct TailSpec<'a> {
    /// The project name, shown as the root folder.
    pub project_name: &'a str,
    /// How many windows the folder lists.
    pub windows: usize,
    pub time: f64,
}

/// The sections after the window list (spec section 13.6): the parameter list (empty), the note
/// windows (only the results log), the project folder tree and the project attribute records.
pub(super) fn tail(t: &TailSpec) -> Vec<u8> {
    let mut v = vec![0x00, b'\n'];
    // notes: the results log that every project has
    v.extend(framed(&[]));
    v.extend(framed(&[0u8; 80]));
    v.extend(framed(b"ResultsLog\0"));
    v.extend(framed(&[0]));
    v.extend(framed(&[]));

    // the folder tree: one root folder that holds every window, closed by the marker block
    let mut tree = Vec::new();
    tree.extend(framed(&[0xde, 0x36, 0x10, 0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    let mut times = vec![0u8; 32];
    times[2] = 1;
    times[16..24].copy_from_slice(&t.time.to_le_bytes());
    times[24..32].copy_from_slice(&t.time.to_le_bytes());
    tree.extend(framed(&times));
    tree.extend(framed(&[]));
    let (name, _) = encode_cp1252(t.project_name);
    let mut name_block = name;
    name_block.push(0);
    tree.extend(framed(&name_block));
    tree.extend_from_slice(&[2, 0, 0, 0, b'\n']);
    let mut a = vec![0u8; 36];
    a[..5].copy_from_slice(&[0x47, 0x11, 0x11, 0x11, 0x01]);
    tree.extend(framed(&a));
    let mut b = vec![0u8; 28];
    b[..5].copy_from_slice(&[0x4d, 0x11, 0x11, 0x11, 0x01]);
    tree.extend(framed(&b));
    tree.extend(framed(&(t.windows as u32).to_le_bytes()));
    // one entry per window, in file order: an empty block, the position, an empty block
    for i in 0..t.windows {
        tree.extend(framed(&[]));
        let mut entry = [0u8; 8];
        entry[4..].copy_from_slice(&(i as u32).to_le_bytes());
        tree.extend(framed(&entry));
        tree.extend(framed(&[]));
    }
    // no sub-folders
    tree.extend(framed(&0u32.to_le_bytes()));
    tree.extend(framed(&[]));
    tree.extend(framed(&[0x00, 0x10, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00]));
    // the size block in front of the tree counts everything after it except 4 bytes
    let size = (tree.len() - 4) as u32;
    v.extend(framed(&size.to_le_bytes()));
    v.extend(tree);

    // project attribute records: type 0x1000, an index, a size and a body
    let records: [(u32, &[u8]); 8] = [
        (0, &[]),
        (1, &[]),
        (2, &[0x25, 0x11, 0x11, 0x11, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        (3, &[0x28, 0x11, 0x11, 0x11, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        (4, &[]),
        (5, &[0x3e, 0x11, 0x11, 0x11, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        (6, &[0, 0, 0, 0]),
        (
            7,
            &[
                0x5d, 0x11, 0x11, 0x11, 0x01, 0, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ],
        ),
    ];
    for (index, body) in records {
        let mut head = vec![0u8; 28];
        head[..4].copy_from_slice(&0x1000u32.to_le_bytes());
        head[4..8].copy_from_slice(&index.to_le_bytes());
        head[8..12].copy_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend_from_slice(&head);
        v.push(b'\n');
        v.extend_from_slice(body);
        v.push(b'\n');
    }
    v
}
