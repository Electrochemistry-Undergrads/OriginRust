//! The byte layout of what a written project holds beyond plain workbooks: the "set column values"
//! formulas of columns (spec section 13.10) and the part after the windows with the folder tree
//! (spec sections 13.5 and 13.6).

use super::layout::{framed, put, put_name};
use crate::opj::strings::encode_cp1252;

// ----- column formulas --------------------------------------------------------------------------

/// The width of the name field of a sheet annotation, terminator included.
const ANNOTATION_NAME_WIDTH: usize = 28;

/// The annotation that holds the "set column values" text of one column: a 131-byte header that
/// carries the column's short name, the text with its terminator in the first block (at least 20
/// bytes) and two empty blocks. Returns the four blocks, framed.
pub(super) fn formula_annotation(column: &str, text: &[u8]) -> Vec<u8> {
    let size = (text.len() + 1).max(20);
    let mut h = vec![0u8; 131];
    h[0x02] = 0x06;
    put(
        &mut h,
        0x13,
        &[0x85, 0xeb, 0x51, 0xb8, 0x1e, 0x85, 0xbb, 0xbf, 0x92, 0x24, 0x49, 0x92, 0x24, 0x49, 0xc2, 0xbf],
    );
    put(&mut h, 0x23, &(size as u32).to_le_bytes());
    put(&mut h, 0x28, &[0x10, 0x01, 0x3e, 0x00, 0x3e, 0x00, 0xc2, 0xff, 0xc2, 0xff]);
    put_name(&mut h, 0x46, ANNOTATION_NAME_WIDTH, column);
    h[0x71] = 0x08;
    let mut body = vec![0u8; size];
    body[..text.len()].copy_from_slice(text);
    let mut out = framed(&h);
    out.extend(framed(&body));
    out.extend(framed(&[]));
    out.extend(framed(&[]));
    out
}

/// A formula column that recalculates: the columns it reads, the column it fills, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Dependency {
    /// Object numbers of the columns the formula reads (at least one).
    pub inputs: Vec<u32>,
    pub output: u32,
    /// True for recalculate mode Auto, false for Manual.
    pub auto: bool,
    /// The number of the formula itself.
    pub id: u32,
}

/// The creator name written in a recalculation entry, with its terminator.
const CREATOR: &[u8] = b"Sheaf\0";

/// What follows the creator name in an entry, as Origin writes it. The meaning of these bytes is
/// not known; they were the same in every entry of every file examined.
const ENTRY_AFTER_CREATOR: [u8; 189] = [
    0x03, 0x00, 0x00, 0x00, 0x55, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x2c, 0x13, 0x1c, 0xfe, 0x74, 0xaa, 0x81,
    0x08, 0x00, 0x00, 0x00, 0x55, 0x6e, 0x6b, 0x6e, 0x6f, 0x77, 0x6e, 0x00, 0x08, 0x00, 0x00, 0x00, 0x55, 0x6e, 0x6b,
    0x6e, 0x6f, 0x77, 0x6e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x2f, 0x11, 0x11,
    0x11, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x66, 0x11, 0x11, 0x11, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x24, 0x11, 0x11, 0x11, 0x02, 0x00, 0x01,
    0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// The creator name of the entries that Origin wrote when the sizes below were measured, with its
/// terminator (a name of six letters).
const MEASURED_CREATOR_LEN: usize = 7;

fn id_list(ids: &[u32], out: &mut Vec<u8>) {
    out.extend(((ids.len() * 4) as u32).to_le_bytes());
    for id in ids {
        out.extend(id.to_le_bytes());
    }
    out.extend(0u32.to_le_bytes());
}

/// The bytes of the entry in front of its size field: three tags that start every entry.
const ENTRY_HEAD: [u8; 12] = [0x24, 0x11, 0x11, 0x11, 0x22, 0x11, 0x11, 0x11, 0x00, 0x00, 0x01, 0x00];

/// The size of an entry apart from the part that the size field counts.
pub(super) const ENTRY_OVERHEAD: usize = 68;

/// One entry of the recalculation record: 284 bytes for a formula that reads one column and 4 more
/// for every further column (with the measured creator name length).
fn dependency_entry(d: &Dependency, time: f64) -> Vec<u8> {
    let mut e = ENTRY_HEAD.to_vec();
    e.extend(0u32.to_le_bytes()); // the size, filled in below
    e.extend([0x23, 0x11, 0x11, 0x11, 0x06, 0x00, 0x01, 0x00]);
    id_list(&d.inputs, &mut e);
    id_list(&[d.output], &mut e);
    e.extend(if d.auto { 2u32 } else { 1u32 }.to_le_bytes());
    e.extend(d.id.to_le_bytes());
    e.extend(0u32.to_le_bytes());
    let delta = CREATOR.len() as i32 - MEASURED_CREATOR_LEN as i32;
    e.extend(5u32.to_le_bytes());
    e.extend((0x60 + delta).to_le_bytes());
    e.extend([0x2d, 0x11, 0x11, 0x11, 0x01, 0x00, 0x01, 0x00]);
    e.extend(time.to_le_bytes());
    e.extend((CREATOR.len() as u32).to_le_bytes());
    e.extend(CREATOR);
    e.extend(ENTRY_AFTER_CREATOR);
    let size = (e.len() - ENTRY_OVERHEAD) as u32;
    put(&mut e, 12, &size.to_le_bytes());
    e
}

/// The body of attribute record 2, which lists the formulas that recalculate (spec section 13.10).
pub(super) fn dependency_record(deps: &[Dependency], time: f64) -> Vec<u8> {
    let mut v = vec![0x25, 0x11, 0x11, 0x11, 0x00, 0x00, 0x01, 0x00];
    v.extend((deps.len() as u32).to_le_bytes());
    v.extend(0u32.to_le_bytes());
    for d in deps {
        v.extend(dependency_entry(d, time));
    }
    v.extend(0u32.to_le_bytes());
    v
}

// ----- the part after the windows ---------------------------------------------------------------

/// A folder of the project tree as it is written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct FolderOut {
    pub name: String,
    /// Positions of the windows in the window list of the file, ascending.
    pub windows: Vec<u32>,
    /// Positions of the Notes windows in the notes list of the file, ascending (T470).
    pub notes: Vec<u32>,
    pub folders: Vec<FolderOut>,
}

/// A Notes window as it is written (T470): its name and its text in Windows-1252, lines separated by
/// CR LF.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NoteOut {
    pub name: String,
    pub text: Vec<u8>,
}

/// What the tail of the file says.
pub(super) struct TailSpec<'a> {
    /// The root folder, named after the project.
    pub root: &'a FolderOut,
    pub dependencies: &'a [Dependency],
    pub time: f64,
    /// The Notes windows, written in front of the results log (T470).
    pub notes: &'a [NoteOut],
}

/// A line of the tree: a 4-byte number and a newline, with no body.
fn line(n: u32) -> Vec<u8> {
    let mut v = n.to_le_bytes().to_vec();
    v.push(b'\n');
    v
}

/// One folder with everything inside it (spec section 13.5). The root folder carries the flag of
/// the folder that is open, and the tree starts with a marker block in front of it.
fn folder(f: &FolderOut, is_root: bool, time: f64, out: &mut Vec<u8>) {
    if is_root {
        out.extend(framed(&[0xde, 0x36, 0x10, 0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    }
    let mut times = vec![0u8; 32];
    times[2] = u8::from(is_root);
    times[16..24].copy_from_slice(&time.to_le_bytes());
    times[24..32].copy_from_slice(&time.to_le_bytes());
    out.extend(framed(&times));
    out.extend(framed(&[]));
    let (mut name, _) = encode_cp1252(&f.name);
    name.push(0);
    out.extend(framed(&name));
    out.extend(line(2));
    let mut a = vec![0u8; 36];
    a[..5].copy_from_slice(&[0x47, 0x11, 0x11, 0x11, 0x01]);
    out.extend(framed(&a));
    let mut b = vec![0u8; 28];
    b[..5].copy_from_slice(&[0x4d, 0x11, 0x11, 0x11, 0x01]);
    out.extend(framed(&b));
    out.extend(framed(&((f.notes.len() + f.windows.len()) as u32).to_le_bytes()));
    // one entry per window: an empty block, the kind and the position, an empty block; the Notes
    // windows first, as Origin 2016 lists them (T470)
    let kinds = f.notes.iter().map(|n| (NOTES_ENTRY, *n)).chain(f.windows.iter().map(|w| (0, *w)));
    for (kind, at) in kinds {
        out.extend(framed(&[]));
        let mut entry = [0u8; 8];
        entry[..4].copy_from_slice(&kind.to_le_bytes());
        entry[4..].copy_from_slice(&at.to_le_bytes());
        out.extend(framed(&entry));
        out.extend(framed(&[]));
    }
    out.extend(framed(&(f.folders.len() as u32).to_le_bytes()));
    for child in &f.folders {
        folder(child, false, time, out);
    }
}

/// The kind word of a folder entry that lists a Notes window (T470).
const NOTES_ENTRY: u32 = 0x0010_0000;

/// The 80-byte header of a Notes window, as Origin 2016 writes one (T470, measured): the window
/// rectangle (left, top, right, bottom; each new window 26 pixels lower and to the right), four bytes
/// Origin always wrote, a 1 at 0x18, and the creation and modification times.
fn note_header(index: usize, time: f64) -> [u8; 80] {
    let mut h = [0u8; 80];
    let step = 26 * (index as i32 % 10 + 1);
    for (k, v) in [step, step, step + 525, step + 240].into_iter().enumerate() {
        h[4 * k..4 * k + 4].copy_from_slice(&v.to_le_bytes());
    }
    h[0x10..0x14].copy_from_slice(&[0x00, 0xc0, 0xcf, 0x54]);
    h[0x18] = 1;
    h[0x20..0x28].copy_from_slice(&time.to_le_bytes());
    h[0x28..0x30].copy_from_slice(&time.to_le_bytes());
    h
}

/// The sections after the window list (spec section 13.6): the parameter list (empty), the note
/// windows (the Notes windows, then the results log), the project folder tree and the project
/// attribute records.
pub(super) fn tail(t: &TailSpec) -> Vec<u8> {
    let mut v = vec![0x00, b'\n'];
    // notes: each Notes window (header, name, text), then the results log that every project has
    v.extend(framed(&[]));
    for (k, n) in t.notes.iter().enumerate() {
        v.extend(framed(&note_header(k, t.time)));
        let (mut name, _) = encode_cp1252(&n.name);
        name.push(0);
        v.extend(framed(&name));
        let mut text = n.text.clone();
        text.push(0);
        v.extend(framed(&text));
    }
    v.extend(framed(&[0u8; 80]));
    v.extend(framed(b"ResultsLog\0"));
    v.extend(framed(&[0]));
    v.extend(framed(&[]));

    // the folder tree, closed by the empty block and the marker block
    let mut tree = Vec::new();
    folder(t.root, true, t.time, &mut tree);
    tree.extend(framed(&[]));
    tree.extend(framed(&[0x00, 0x10, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00]));
    // the size block in front of the tree counts everything after it except 4 bytes
    let size = (tree.len() - 4) as u32;
    v.extend(framed(&size.to_le_bytes()));
    v.extend(tree);

    // project attribute records: type 0x1000, an index, a size and a body
    let dependencies = dependency_record(t.dependencies, t.time);
    // Origin does not recalculate the columns of a loaded file when record 0 is empty, so a project
    // with recalculating formulas gets the shortest storage text there.
    let storage: &[u8] = if t.dependencies.is_empty() { &[] } else { b"<OriginStorage></OriginStorage>" };
    let records: [(u32, &[u8]); 8] = [
        (0, storage),
        (1, &[]),
        (2, &dependencies),
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
