//! The sections after the window list of an `.opj` file: the parameters, the note windows, the
//! folder tree and the project attribute records (spec sections 6, 13.5, 13.6, 13.10 and 13.11). Only
//! two things are kept: the folders with the windows they list, and the table of column formulas
//! that recalculate.
//!
//! Nothing here fails the import. A file whose tail cannot be read as described has no folders
//! and no recalculation table; when the folder tree was found but is damaged, a note says so.

use super::frame::{FrameError, FrameResult, Frames};
use super::strings::c_bytes;
use super::strings::decode_cp1252;
use std::io::{Read, Seek};

/// The first bytes of the marker block that precedes the folder tree.
const TREE_MARKER: [u8; 4] = [0xDE, 0x36, 0x10, 0x03];
/// The tag at the start of the recalculation table (attribute record 2) and of each of its entries.
const TABLE_TAG: [u8; 4] = [0x25, 0x11, 0x11, 0x11];
const ENTRY_TAG: [u8; 8] = [0x24, 0x11, 0x11, 0x11, 0x22, 0x11, 0x11, 0x11];
/// An entry is its size field plus this many bytes.
const ENTRY_OVERHEAD: usize = 68;
/// Folders nested deeper than this are not followed.
const MAX_DEPTH: usize = 64;

/// The kind word (the first four bytes of a folder entry) of a Notes window: its position counts in the
/// notes list instead of the window list (T470, measured in Origin 2016; 0 for the other windows).
pub(crate) const NOTES_ENTRY: u32 = 0x0010_0000;

/// A folder of the tree with the positions (in the window list) of the windows it lists, and the
/// positions (in the notes list) of the Notes windows it lists.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct TreeFolder {
    pub name: String,
    pub windows: Vec<usize>,
    pub notes: Vec<usize>,
    pub folders: Vec<TreeFolder>,
}

/// A Notes window of the file (T470): its name and its text, lines separated by `\n`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TreeNote {
    pub name: String,
    pub text: String,
}

/// One formula that recalculates: the numbers of the columns it reads and of the column it fills,
/// and the recalculate mode (2 Auto, 1 Manual).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Recalc {
    pub inputs: Vec<u32>,
    pub output: u32,
    pub mode: u32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Tail {
    /// The root folder; `None` when the file has no readable tree.
    pub root: Option<TreeFolder>,
    pub recalcs: Vec<Recalc>,
    /// The Notes windows in file order (T470); `None` for the results log, so that the positions the
    /// folders give stay right.
    pub notes: Vec<Option<TreeNote>>,
    /// What was wrong, when the tree was found but could not be read to its end.
    pub problem: Option<String>,
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// A block of exactly four bytes holding a count.
fn count<R: Read + Seek>(f: &mut Frames<R>) -> FrameResult<usize> {
    let b = f.block()?;
    let n = u32_at(&b, 0).filter(|_| b.len() == 4).ok_or(FrameError::BadDelimiter { at: f.pos() })?;
    Ok(n as usize)
}

/// Reads the tail of the file from where the window list ended. `windows` is the number of windows
/// in the list; positions beyond it are dropped.
pub(crate) fn scan<R: Read + Seek>(f: &mut Frames<R>, windows: usize) -> Tail {
    let mut tail = Tail::default();
    // Without the markers of the tree this is not a tail we know: say nothing.
    let Some(notes) = skip_to_tree(f) else {
        return tail;
    };
    let note_count = notes.len();
    tail.notes = notes;
    match folder(f, 0, (windows, note_count), &mut Vec::new(), &mut Vec::new()) {
        Ok(root) => {
            tail.root = Some(root);
            // the empty block, the block that closes the tree, then the attribute records
            let closed = f.block().is_ok() && f.block().is_ok();
            if closed {
                tail.recalcs = recalc_table(f);
            } else {
                tail.problem = Some("the folder tree is not closed properly".to_string());
            }
        }
        Err(e) => tail.problem = Some(e.to_string()),
    }
    tail
}

/// Steps over the parameters and the notes, the size block and the marker block, and returns the
/// notes in file order (`None` for the results log, which keeps its place in the numbering). `None`
/// when the bytes do not look like that.
fn skip_to_tree<R: Read + Seek>(f: &mut Frames<R>) -> Option<Vec<Option<TreeNote>>> {
    for _ in 0..100_000 {
        let name = f.line(512).ok()?;
        if name == [0] {
            break;
        }
        f.raw(8).ok()?;
    }
    // an empty block, then notes (an 80-byte header, the name, the text) up to an empty block; the
    // results log is the last one (T470, measured in Origin 2016)
    f.block().ok()?;
    let mut notes = Vec::new();
    for _ in 0..100_000 {
        if f.block().ok()?.is_empty() {
            break;
        }
        let name = decode_cp1252(c_bytes(&f.block().ok()?));
        let text = decode_cp1252(c_bytes(&f.block().ok()?)).replace("\r\n", "\n");
        notes.push((name != "ResultsLog").then_some(TreeNote { name, text }));
    }
    let size = f.block().ok()?;
    let marker = f.block().ok()?;
    (size.len() == 4 && marker.len() == 16 && marker.starts_with(&TREE_MARKER)).then_some(notes)
}

/// One folder and everything inside it.
fn folder<R: Read + Seek>(
    f: &mut Frames<R>,
    depth: usize,
    lists: (usize, usize),
    seen: &mut Vec<usize>,
    seen_notes: &mut Vec<usize>,
) -> FrameResult<TreeFolder> {
    let (windows, note_count) = lists;
    if depth > MAX_DEPTH {
        return Err(FrameError::BadDelimiter { at: f.pos() });
    }
    let times = f.block()?;
    let empty = f.block()?;
    let name = f.block()?;
    let _two = f.head()?;
    let (a, b) = (f.block()?, f.block()?);
    if times.len() != 32 || !empty.is_empty() || a.len() < 5 || b.len() < 5 {
        return Err(FrameError::BadDelimiter { at: f.pos() });
    }
    let listed = count(f)?;
    // every window entry takes at least 19 bytes, so a count beyond the file is damage
    if listed as u64 * 19 > f.remaining() {
        return Err(FrameError::Truncated { at: f.pos() });
    }
    let mut out = TreeFolder { name: decode_cp1252(c_bytes(&name)), ..TreeFolder::default() };
    for _ in 0..listed {
        let (before, entry, after) = (f.block()?, f.block()?, f.block()?);
        if !before.is_empty() || !after.is_empty() || entry.len() != 8 {
            return Err(FrameError::BadDelimiter { at: f.pos() });
        }
        let position = u32_at(&entry, 4).unwrap_or(u32::MAX) as usize;
        // a Notes window counts in the notes list (T470)
        if u32_at(&entry, 0) == Some(NOTES_ENTRY) {
            if position < note_count && !seen_notes.contains(&position) {
                seen_notes.push(position);
                out.notes.push(position);
            }
            continue;
        }
        // a position outside the window list, or a window another folder listed first, is dropped
        if position < windows && !seen.contains(&position) {
            seen.push(position);
            out.windows.push(position);
        }
    }
    let children = count(f)?;
    if children as u64 * 60 > f.remaining() {
        return Err(FrameError::Truncated { at: f.pos() });
    }
    for _ in 0..children {
        out.folders.push(folder(f, depth + 1, lists, seen, seen_notes)?);
    }
    Ok(out)
}

/// Reads the attribute records and returns the table of record 2. Damage gives what was read.
fn recalc_table<R: Read + Seek>(f: &mut Frames<R>) -> Vec<Recalc> {
    for _ in 0..16 {
        let Ok(head) = f.raw(28) else { return Vec::new() };
        if u32_at(&head, 0) != Some(0x1000) {
            return Vec::new();
        }
        let (Some(index), Some(size)) = (u32_at(&head, 4), u32_at(&head, 8)) else { return Vec::new() };
        let Ok(body) = f.raw(size) else { return Vec::new() };
        if index == 2 {
            return parse_table(&body);
        }
    }
    Vec::new()
}

/// The entries of the recalculation table: a 16-byte head with the number of entries, then the
/// entries one after the other.
fn parse_table(body: &[u8]) -> Vec<Recalc> {
    let mut out = Vec::new();
    if body.get(..4) != Some(&TABLE_TAG[..]) {
        return out;
    }
    let n = u32_at(body, 8).unwrap_or(0) as usize;
    let mut at = 16;
    for _ in 0..n.min(body.len() / ENTRY_OVERHEAD) {
        let Some(size) = u32_at(body, at + 12) else { break };
        let len = size as usize + ENTRY_OVERHEAD;
        let Some(entry) = body.get(at..at + len).filter(|e| e.starts_with(&ENTRY_TAG)) else { break };
        if let Some(r) = parse_entry(entry) {
            out.push(r);
        }
        at += len;
    }
    out
}

/// An id list: the size in bytes, the ids, and a zero. Returns the ids and the position after.
fn id_list(entry: &[u8], at: usize) -> Option<(Vec<u32>, usize)> {
    let bytes = u32_at(entry, at)? as usize;
    if !bytes.is_multiple_of(4) || bytes > entry.len() {
        return None;
    }
    let ids = (0..bytes / 4).map(|k| u32_at(entry, at + 4 + 4 * k)).collect::<Option<Vec<u32>>>()?;
    Some((ids, at + 4 + bytes + 4))
}

fn parse_entry(entry: &[u8]) -> Option<Recalc> {
    // 12 bytes of tags, the size, then the tag 0x11111123 and 06 00 01 00
    let (inputs, next) = id_list(entry, 24)?;
    let (outputs, next) = id_list(entry, next)?;
    let mode = u32_at(entry, next)?;
    (outputs.len() == 1 && !inputs.is_empty()).then(|| Recalc { inputs, output: outputs[0], mode })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(inputs: &[u32], output: u32, mode: u32) -> Vec<u8> {
        let mut e = ENTRY_TAG.to_vec();
        e.extend([0, 0, 1, 0]);
        e.extend(0u32.to_le_bytes());
        e.extend([0x23, 0x11, 0x11, 0x11, 6, 0, 1, 0]);
        for ids in [inputs, &[output][..]] {
            e.extend(((ids.len() * 4) as u32).to_le_bytes());
            for id in ids {
                e.extend(id.to_le_bytes());
            }
            e.extend(0u32.to_le_bytes());
        }
        e.extend(mode.to_le_bytes());
        e.extend([0u8; 60]);
        let size = (e.len() - ENTRY_OVERHEAD) as u32;
        e[12..16].copy_from_slice(&size.to_le_bytes());
        e
    }

    fn table(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut t = TABLE_TAG.to_vec();
        t.extend([0, 0, 1, 0]);
        t.extend((entries.len() as u32).to_le_bytes());
        t.extend(0u32.to_le_bytes());
        for e in entries {
            t.extend(e);
        }
        t.extend(0u32.to_le_bytes());
        t
    }

    #[test]
    fn the_table_gives_inputs_output_and_mode() {
        let t = table(&[entry(&[5], 7, 2), entry(&[5, 7], 9, 1)]);
        let r = parse_table(&t);
        assert_eq!(
            r,
            [Recalc { inputs: vec![5], output: 7, mode: 2 }, Recalc { inputs: vec![5, 7], output: 9, mode: 1 }]
        );
    }

    #[test]
    fn a_damaged_table_gives_what_was_read() {
        let mut t = table(&[entry(&[1], 2, 2), entry(&[1], 3, 2)]);
        t.truncate(t.len() - 30);
        assert_eq!(parse_table(&t).len(), 1);
        assert!(parse_table(&[1, 2, 3]).is_empty());
        let mut huge = table(&[entry(&[1], 2, 2)]);
        huge[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_table(&huge).len(), 1);
    }
}
