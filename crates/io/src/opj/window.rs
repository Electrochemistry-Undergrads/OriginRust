//! The window section of an `.opj` file: windows, their layers (sheets), and the
//! column records of each layer (spec section 4). Only the parts that workbooks need
//! are kept; annotations, axis blocks and everything else are stepped over.

use super::frame::{FrameError, FrameResult, Frames};
use super::strings::{field_text, visible_part};
use std::io::{Read, Seek};

const WIN_NAME_AT: usize = 0x02;
const WIN_NAME_WIDTH: usize = 25;
const WIN_LABEL_AT: usize = 0xC3;
const LAYER_NAME_AT: usize = 0xD2;
const LAYER_NAME_WIDTH: usize = 32;
const COL_DATASET_AT: usize = 0x04;
const COL_DESIGNATION_AT: usize = 0x11;
const COL_NAME_AT: usize = 0x12;
const COL_NAME_WIDTH: usize = 12;
const COL_TYPE_AT: usize = 0x1E;
/// A column record shorter than this cannot be read.
const COL_MIN_LEN: usize = 0x20;

/// Sizes (block 1, block 2) that mark an annotation as a group whose first member
/// follows directly.
const GROUP_BLOCK1_SIZES: [u32; 2] = [0x5E, 0x0A];

/// One column record of a layer: a worksheet column or a plotted curve.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WinColumn {
    /// 1-based position in the data-set list (0 when absent).
    pub dataset: usize,
    pub designation: u8,
    /// The short name stored here is cut to 11 characters; the data-set name is longer-lived.
    pub short_name: String,
    pub value_type: u8,
    /// The column note: long name, units and comments.
    pub note: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WinLayer {
    pub name: String,
    pub columns: Vec<WinColumn>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Window {
    pub name: String,
    /// First line of the window label (the long name).
    pub label: String,
    pub layers: Vec<WinLayer>,
}

/// Reads windows up to the empty block that ends the list. A damaged window ends the
/// scan; the windows before it are kept and the reason is returned.
pub(crate) fn scan<R: Read + Seek>(f: &mut Frames<R>) -> (Vec<Window>, Option<String>) {
    let mut windows = Vec::new();
    loop {
        match scan_one(f) {
            Ok(Some(w)) => windows.push(w),
            Ok(None) => return (windows, None),
            Err(e) => {
                let why = format!("window {}: {e}", windows.len() + 1);
                return (windows, Some(why));
            }
        }
    }
}

fn scan_one<R: Read + Seek>(f: &mut Frames<R>) -> FrameResult<Option<Window>> {
    let header = f.block()?;
    if header.is_empty() {
        return Ok(None);
    }
    if header.len() < WIN_NAME_AT + 2 {
        return Err(FrameError::BadDelimiter { at: f.pos() });
    }
    let name = field_text(&header, WIN_NAME_AT, WIN_NAME_WIDTH);
    let label = window_label(&header);
    let mut layers = Vec::new();
    loop {
        let layer_header = f.block()?;
        if layer_header.is_empty() {
            break;
        }
        let name = field_text(&layer_header, LAYER_NAME_AT, LAYER_NAME_WIDTH);
        // Annotations, then column records, then one extra block, then three groups
        // of axis blocks, each group closed by an empty block.
        while f.skip_block()?.len != 0 {
            skip_annotation(f)?;
        }
        let mut columns = Vec::new();
        loop {
            let record = f.block()?;
            if record.is_empty() {
                break;
            }
            let note = f.block()?;
            columns.push(parse_column(&record, note));
        }
        f.skip_block()?;
        for _ in 0..3 {
            while f.skip_block()?.len != 0 {}
        }
        layers.push(WinLayer { name, columns });
    }
    Ok(Some(Window { name, label, layers }))
}

/// The long name of a window: the first line of its label, without the storage tail.
fn window_label(header: &[u8]) -> String {
    if header.len() <= WIN_LABEL_AT {
        return String::new();
    }
    let text = field_text(header, WIN_LABEL_AT, header.len() - WIN_LABEL_AT);
    let visible = visible_part(&text);
    visible.split(['\r', '\n']).next().unwrap_or("").to_string()
}

/// Steps over the rest of an annotation whose header block has been read: two blocks
/// and, unless the annotation is a group, a third. A group announces the size of its
/// first member's header in its second block; that header follows unframed and the
/// member's own blocks come next, so a group is followed in a loop.
fn skip_annotation<R: Read + Seek>(f: &mut Frames<R>) -> FrameResult<()> {
    loop {
        let first = f.skip_block()?;
        let second = f.skip_block()?;
        if GROUP_BLOCK1_SIZES.contains(&first.len) && second.len == 4 {
            let size = f.read_extent(second)?;
            let n = u32::from_le_bytes([size[0], size[1], size[2], size[3]]);
            f.raw(n)?;
            continue;
        }
        f.skip_block()?;
        return Ok(());
    }
}

fn parse_column(record: &[u8], note: Vec<u8>) -> WinColumn {
    let get = |at: usize| record.get(at).copied().unwrap_or(0);
    let long_enough = record.len() >= COL_MIN_LEN;
    WinColumn {
        dataset: if long_enough {
            usize::from(u16::from_le_bytes([get(COL_DATASET_AT), get(COL_DATASET_AT + 1)]))
        } else {
            0
        },
        designation: get(COL_DESIGNATION_AT),
        short_name: if long_enough { field_text(record, COL_NAME_AT, COL_NAME_WIDTH) } else { String::new() },
        value_type: get(COL_TYPE_AT),
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opj::testutil::{annotation, column_record, framed, layer_header, window_header};
    use std::io::Cursor;

    fn scan_bytes(bytes: Vec<u8>) -> (Vec<Window>, Option<String>) {
        let mut f = Frames::new(Cursor::new(bytes)).unwrap();
        scan(&mut f)
    }

    fn one_layer(columns: &[(u16, u8, &str, &[u8])]) -> Vec<u8> {
        let mut b = framed(&layer_header("Data"));
        b.extend(annotation(false));
        b.extend(framed(&[]));
        for (idx, des, name, note) in columns {
            b.extend(framed(&column_record(*idx, *des, name)));
            b.extend(framed(note));
        }
        b.extend(framed(&[]));
        b.extend(framed(&[]));
        for _ in 0..3 {
            b.extend(framed(&[]));
        }
        b.extend(framed(&[]));
        b
    }

    #[test]
    fn a_workbook_window_with_two_columns() {
        let mut bytes = framed(&window_header("Book1", "My data\r\nsecond line"));
        bytes.extend(one_layer(&[(1, 3, "A", b"Time\r\ns"), (2, 0, "B", b"")]));
        bytes.extend(framed(&[]));
        let (w, err) = scan_bytes(bytes);
        assert_eq!(err, None);
        assert_eq!(w.len(), 1);
        assert_eq!((w[0].name.as_str(), w[0].label.as_str()), ("Book1", "My data"));
        assert_eq!(w[0].layers.len(), 1);
        assert_eq!(w[0].layers[0].name, "Data");
        let c = &w[0].layers[0].columns;
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].dataset, c[0].designation, c[0].short_name.as_str()), (1, 3, "A"));
        assert_eq!(c[0].note, b"Time\r\ns");
        assert_eq!((c[1].dataset, c[1].designation), (2, 0));
    }

    #[test]
    fn annotations_including_groups_are_stepped_over() {
        let mut bytes = framed(&window_header("Graph1", ""));
        let mut layer = framed(&layer_header("S"));
        layer.extend(annotation(false));
        layer.extend(annotation(true));
        layer.extend(annotation(false));
        layer.extend(framed(&[])); // end of annotations
        layer.extend(framed(&column_record(5, 0, "Y")));
        layer.extend(framed(b"note"));
        layer.extend(framed(&[])); // end of columns
        layer.extend(framed(&[])); // extra block
        for _ in 0..3 {
            layer.extend(framed(&[1, 2, 3]));
            layer.extend(framed(&[])); // axis group ends
        }
        layer.extend(framed(&[])); // end of layers
        bytes.extend(layer);
        bytes.extend(framed(&[])); // end of windows
        let (w, err) = scan_bytes(bytes);
        assert_eq!(err, None);
        assert_eq!(w[0].layers[0].columns.len(), 1);
        assert_eq!(w[0].layers[0].columns[0].dataset, 5);
    }

    #[test]
    fn two_windows_and_a_window_without_layers() {
        let mut bytes = framed(&window_header("A", ""));
        bytes.extend(framed(&[])); // no layers
        bytes.extend(framed(&window_header("B", "")));
        bytes.extend(one_layer(&[(1, 0, "X", b"")]));
        bytes.extend(framed(&[]));
        let (w, err) = scan_bytes(bytes);
        assert_eq!(err, None);
        assert_eq!(w.len(), 2);
        assert!(w[0].layers.is_empty());
        assert_eq!(w[1].name, "B");
    }

    #[test]
    fn damage_keeps_the_windows_before_it() {
        let mut bytes = framed(&window_header("Good", ""));
        bytes.extend(one_layer(&[(1, 0, "A", b"")]));
        bytes.extend(framed(&window_header("Bad", "")));
        bytes.extend_from_slice(&[9, 9, 9]); // junk where a layer should start
        let (w, err) = scan_bytes(bytes);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].name, "Good");
        assert!(err.unwrap().contains("window 2"));
    }

    #[test]
    fn short_records_do_not_panic() {
        let c = parse_column(&[1, 2, 3], Vec::new());
        assert_eq!((c.dataset, c.short_name.as_str()), (0, ""));
        let c = parse_column(&[], Vec::new());
        assert_eq!(c.designation, 0);
        assert_eq!(window_label(&[0; 10]), "");
        assert_eq!(window_label(&[]), "");
    }
}
