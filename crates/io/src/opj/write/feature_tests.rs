//! Tests of the column formulas and the folder tree of written projects: what is written reads
//! back the same, and the file keeps the layout described in spec sections 13.10 and 13.11.

use super::*;
use crate::opj::read_opj_from;
use sheaf_core::column::RecalcMode;
use sheaf_core::ids::{ColumnId, WindowId};
use std::io::Cursor;
use std::path::Path;

const TIME: f64 = 2_461_000.5;

fn numeric(short: &str, designation: Designation, values: &[f64]) -> Column {
    Column::new(ColumnId(0), short, designation, ColumnData::numeric(values.to_vec()))
}

fn book(name: &str, sheets: Vec<(&str, Vec<Column>)>) -> Workbook {
    let mut b = Workbook::new(WindowId(0), name);
    b.sheets = sheets
        .into_iter()
        .map(|(n, columns)| {
            let mut s = Worksheet::new(n);
            s.columns = columns;
            s
        })
        .collect();
    b
}

/// A workbook of columns A (X), B, C, D (Y), all with five rows.
fn abcd(name: &str) -> Workbook {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    book(
        name,
        vec![(
            "Data",
            vec![
                numeric("A", Designation::X, &a),
                numeric("B", Designation::Y, &[0.0; 5]),
                numeric("C", Designation::Y, &[0.0; 5]),
                numeric("D", Designation::Y, &[0.0; 5]),
            ],
        )],
    )
}

fn column_id(p: &Project, book: &str, sheet: usize, short: &str) -> ColumnId {
    let b = p.workbooks().find(|b| b.name == book).unwrap();
    b.sheets[sheet].columns.iter().find(|c| c.short_name == short).unwrap().id
}

fn set(
    p: &mut Project,
    book: &str,
    sheet: usize,
    col: &str,
    f: &str,
    rows: Option<std::ops::Range<usize>>,
    m: RecalcMode,
) {
    let id = column_id(p, book, sheet, col);
    p.set_column_values_with(id, f, rows, m).unwrap();
}

fn written(p: &Project) -> OpjWrite {
    write_named(p, "features", TIME).unwrap()
}

fn read_back(bytes: &[u8]) -> crate::opj::OpjImport {
    read_opj_from(Cursor::new(bytes.to_vec()), Path::new("written.opj")).unwrap()
}

/// (formula, recalculate mode, rows) of every column of the first sheet of workbook `name`.
fn formulas_of(
    imp: &crate::opj::OpjImport,
    name: &str,
    sheet: usize,
) -> Vec<(String, RecalcMode, Option<std::ops::Range<usize>>)> {
    let b = imp.workbooks.iter().find(|b| b.name == name).unwrap();
    b.sheets[sheet]
        .columns
        .iter()
        .map(|c| (c.formula.clone().unwrap_or_default(), c.recalc, c.formula_rows.clone()))
        .collect()
}

#[test]
fn formulas_of_every_recalculate_mode_read_back() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    set(&mut p, "Book1", 0, "B", "col(A)*2", None, RecalcMode::Auto);
    set(&mut p, "Book1", 0, "C", "col(A)+col(B)", None, RecalcMode::Manual);
    set(&mut p, "Book1", 0, "D", "col(A)-1", None, RecalcMode::None);
    let out = written(&p);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    let none = String::new();
    assert_eq!(
        formulas_of(&imp, "Book1", 0),
        [
            (none, RecalcMode::None, None),
            ("col(A)*2".into(), RecalcMode::Auto, None),
            ("col(A)+col(B)".into(), RecalcMode::Manual, None),
            ("col(A)-1".into(), RecalcMode::None, None),
        ]
    );
    // the values are still the values
    let b = &imp.workbooks[0].sheets[0].columns;
    assert_eq!(b[1].numeric().unwrap()[..3], [2.0, 4.0, 6.0]);
}

#[test]
fn rows_of_a_formula_read_back_and_a_second_sheet_and_workbook_keep_their_own() {
    let mut p = Project::new();
    let mut first = abcd("Book1");
    first.sheets.push({
        let mut s = Worksheet::new("Second");
        s.columns = vec![numeric("A", Designation::X, &[1.0, 2.0, 3.0]), numeric("B", Designation::Y, &[0.0; 3])];
        s
    });
    p.adopt_named_workbook(first);
    p.adopt_named_workbook(abcd("Book2"));
    set(&mut p, "Book1", 0, "B", "col(A)*3", Some(1..4), RecalcMode::Auto);
    set(&mut p, "Book1", 1, "B", "col(A)*5", None, RecalcMode::Manual);
    set(&mut p, "Book2", 0, "C", "col(B)+col(A)", None, RecalcMode::Auto);
    let imp = read_back(&written(&p).bytes);
    assert_eq!(formulas_of(&imp, "Book1", 0)[1], ("col(A)*3".into(), RecalcMode::Auto, Some(1..4)));
    assert_eq!(formulas_of(&imp, "Book1", 1)[1], ("col(A)*5".into(), RecalcMode::Manual, None));
    assert_eq!(formulas_of(&imp, "Book2", 0)[2], ("col(B)+col(A)".into(), RecalcMode::Auto, None));
    assert_eq!(formulas_of(&imp, "Book2", 0)[1].0, "", "a column without formula stays without");
}

#[test]
fn a_formula_chain_and_long_names_as_references_are_followed() {
    let mut p = Project::new();
    let mut b = abcd("Book1");
    b.sheets[0].columns[2].long_name = "Doubled".into();
    p.adopt_named_workbook(b);
    set(&mut p, "Book1", 0, "B", "col(A)*2", None, RecalcMode::Auto);
    set(&mut p, "Book1", 0, "D", "col(\"Doubled\")+col(2)", None, RecalcMode::Auto);
    let out = written(&p);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert_eq!(formulas_of(&imp, "Book1", 0)[3].1, RecalcMode::Auto);
}

#[test]
fn a_formula_that_cannot_recalculate_is_stored_with_mode_none_and_reported() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    // set directly: the core refuses some of these through `set_column_values_with`
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    for (i, f) in [(1, "5"), (2, "col(Z)*2"), (3, "this is (not a formula")] {
        b.sheets[0].columns[i].formula = Some(f.into());
        b.sheets[0].columns[i].recalc = RecalcMode::Auto;
    }
    let out = written(&p);
    let all = out.warnings.join(" | ");
    assert!(all.contains("recalculate mode None"), "{all}");
    let imp = read_back(&out.bytes);
    let got = formulas_of(&imp, "Book1", 0);
    assert_eq!(
        got[1..],
        [
            ("5".into(), RecalcMode::None, None),
            ("col(Z)*2".into(), RecalcMode::None, None),
            ("this is (not a formula".into(), RecalcMode::None, None),
        ]
    );
}

#[test]
fn a_formula_state_that_the_file_cannot_hold_is_reported() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    set(&mut p, "Book1", 0, "B", "col(A)*2", None, RecalcMode::Manual);
    // changing the input marks the manual column as out of date
    let a = column_id(&p, "Book1", 0, "A");
    p.column_mut(a).unwrap().numeric_mut().unwrap()[0] = 9.0;
    p.refresh_formulas(&[a]);
    assert_ne!(
        p.column(column_id(&p, "Book1", 0, "B")).unwrap().formula_state,
        sheaf_core::column::FormulaState::Current
    );
    let out = written(&p);
    assert!(out.warnings.join(" | ").contains("out of date"), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert_eq!(formulas_of(&imp, "Book1", 0)[1].1, RecalcMode::Manual);
}

#[test]
fn formula_text_that_the_file_cannot_hold_is_reported() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let b = match &mut p.windows[0] {
        Window::Workbook(b) => b,
        Window::Graph(_) | Window::Notes(_) | Window::Matrix(_) | Window::Layout(_) => unreachable!(),
    };
    b.sheets[0].columns[1].formula = Some("col(A)\0".into());
    b.sheets[0].columns[2].formula = Some("col(A);\r\n#col(B)=1".into());
    b.sheets[0].columns[3].formula = Some(format!("col(A){}", " ".repeat(70_000)));
    let out = written(&p);
    assert!(out.warnings.join(" | ").contains("3 column formula(s) could not be stored"), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert!(formulas_of(&imp, "Book1", 0).iter().all(|f| f.0.is_empty()));
}

#[test]
fn a_formula_with_a_script_in_front_reads_back_whole_and_is_reported() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let b = match &mut p.windows[0] {
        Window::Workbook(b) => b,
        Window::Graph(_) | Window::Notes(_) | Window::Matrix(_) | Window::Layout(_) => unreachable!(),
    };
    b.sheets[0].columns[1].formula = Some("col(A)*2".into());
    let mut bytes = written(&p).bytes;
    // an Origin file stores a "before formula" script after the tail mark: patch the stored text
    let needle = b"col(A)*2\0";
    let at = bytes.windows(needle.len()).position(|w| w == needle).unwrap();
    bytes[at + 8..at + 16].copy_from_slice(b";\r\n#c=7\0");
    let imp = read_back(&bytes);
    assert_eq!(formulas_of(&imp, "Book1", 0)[1].0, "col(A)*2;\r\n#c=7");
    assert!(imp.warnings.iter().any(|w| w.contains("script")), "{:?}", imp.warnings);
}

#[test]
fn a_range_with_nonsense_numbers_is_kept_as_text() {
    for tail in ["$[0:3]", "$[4:2]", "$[a:b]", "$[2:4", "$2:4]"] {
        let text = format!("col(A);\r\n#{tail}");
        let (formula, rows, script) = crate::opj::split_formula(&text);
        assert_eq!((rows, script), (None, true), "{tail}");
        assert_eq!(formula, text);
    }
    let (formula, rows, script) = crate::opj::split_formula("col(A)*2;\r\n#$[2:4]");
    assert_eq!((formula, rows, script), ("col(A)*2", Some(1..4), false));
}

// ----- the layout of the file ---------------------------------------------------------------------

#[test]
fn a_recalculating_formula_writes_the_table_in_attribute_record_2() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    set(&mut p, "Book1", 0, "B", "col(A)*2", None, RecalcMode::Auto);
    set(&mut p, "Book1", 0, "C", "col(A)+col(B)", None, RecalcMode::Manual);
    let out = written(&p);
    // walk the eight attribute records from the front of the first one
    let bytes = &out.bytes;
    let marker = [8u8, 0, 0, 0, b'\n', 0, 0x10, 0, 0, 8, 0, 0, 0, b'\n'];
    let at = bytes.windows(marker.len()).rposition(|w| w == marker).unwrap() + marker.len();
    let mut records = Vec::new();
    let mut i = at;
    while i < bytes.len() {
        let size = u32::from_le_bytes(bytes[i + 8..i + 12].try_into().unwrap()) as usize;
        records.push(bytes[i + 29..i + 29 + size].to_vec());
        i += 30 + size;
    }
    assert_eq!(records.len(), 8);
    assert_eq!(records[0], b"<OriginStorage></OriginStorage>", "record 0 is not empty when formulas recalculate");
    let table = &records[2];
    assert_eq!(&table[..8], [0x25, 0x11, 0x11, 0x11, 0, 0, 1, 0]);
    assert_eq!(u32::from_le_bytes(table[8..12].try_into().unwrap()), 2);
    // entry 1 reads one column (284 - 1 bytes with the shorter creator name), entry 2 reads two (4 more)
    let first = u32::from_le_bytes(table[16 + 12..16 + 16].try_into().unwrap()) as usize + 68;
    let second = u32::from_le_bytes(table[16 + first + 12..16 + first + 16].try_into().unwrap()) as usize + 68;
    assert_eq!(second, first + 4);
    assert_eq!(table.len(), 16 + first + second + 4);
    // both entries start with the tags
    assert_eq!(&table[16..24], [0x24, 0x11, 0x11, 0x11, 0x22, 0x11, 0x11, 0x11]);
    assert_eq!(&table[16 + first..16 + first + 8], [0x24, 0x11, 0x11, 0x11, 0x22, 0x11, 0x11, 0x11]);
    // the column records of A, B and C carry their numbers in the records; D takes no part
    let ids: Vec<u16> = (0..4)
        .map(|k| {
            let rec = column_record_bytes(bytes, k);
            u16::from_le_bytes([rec[0x1e2], rec[0x1e3]])
        })
        .collect();
    assert!(ids[0] != 0 && ids[1] != 0 && ids[2] != 0 && ids[3] == 0, "{ids:?}");
    assert!(ids.iter().filter(|i| **i != 0).collect::<HashSet<_>>().len() == 3, "numbers are distinct: {ids:?}");
    // a project without recalculating formulas keeps record 0 and the table empty
    let plain = written(&Project::new());
    let plain_at = plain.bytes.windows(marker.len()).rposition(|w| w == marker).unwrap() + marker.len();
    assert_eq!(u32::from_le_bytes(plain.bytes[plain_at + 8..plain_at + 12].try_into().unwrap()), 0);
}

/// The k-th column record (515 bytes, size block 0x203) of the file.
fn column_record_bytes(bytes: &[u8], k: usize) -> Vec<u8> {
    let head = [0x03u8, 0x02, 0, 0, b'\n'];
    bytes
        .windows(5)
        .enumerate()
        .filter(|(_, w)| *w == head)
        .map(|(i, _)| i)
        .nth(k)
        .map(|i| bytes[i + 5..i + 5 + 515].to_vec())
        .unwrap()
}

#[test]
fn the_annotations_of_a_sheet_hold_the_formulas_last_column_first() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    set(&mut p, "Book1", 0, "B", "col(A)*2", None, RecalcMode::None);
    set(&mut p, "Book1", 0, "D", "col(A)-1", Some(0..2), RecalcMode::Auto);
    let bytes = written(&p).bytes;
    let find = |needle: &[u8]| bytes.windows(needle.len()).position(|w| w == needle);
    let b = find(b"col(A)*2\0").unwrap();
    let d = find(b"col(A)-1;\r\n#$[1:2]\0").unwrap();
    assert!(d < b, "the formula of the later column comes first");
    // the block holding the text is at least 20 bytes
    let size = u32::from_le_bytes(bytes[b - 5..b - 1].try_into().unwrap());
    assert_eq!(size, 20);
}

// ----- folders ------------------------------------------------------------------------------------

fn leaf(name: &str, windows: Vec<WindowId>) -> Folder {
    Folder { name: name.into(), windows, ..Folder::default() }
}

/// Six workbooks Book1..Book6 with Book1 in the root, Book2 in F1, Book3 in F1/F1a, Book4 in
/// F1/F1a/F1b, Book5 and Book6 in F2, and an empty folder F3.
fn six_in_folders() -> Project {
    let mut p = Project::new();
    let ids: Vec<WindowId> = (1..=6)
        .map(|n| {
            p.adopt_named_workbook(book(
                &format!("Book{n}"),
                vec![("Sheet1", vec![numeric("A", Designation::X, &[f64::from(n)])])],
            ))
        })
        .collect();
    let mut f1b = leaf("F1b", vec![ids[3]]);
    f1b.folders = Vec::new();
    let mut f1a = leaf("F1a", vec![ids[2]]);
    f1a.folders.push(f1b);
    let mut f1 = leaf("F1", vec![ids[1]]);
    f1.folders.push(f1a);
    p.root.folders = vec![f1, leaf("F2", vec![ids[4], ids[5]]), leaf("F3", Vec::new())];
    p
}

/// The folder tree as text: `name[windows](children)`.
fn shape(f: &Folder, p: &Project) -> String {
    let windows: Vec<String> =
        f.windows.iter().map(|id| p.window(*id).map_or("?".to_string(), |w| w.name().to_string())).collect();
    let children: Vec<String> = f.folders.iter().map(|c| shape(c, p)).collect();
    format!("{}[{}]({})", f.name, windows.join(","), children.join(" "))
}

#[test]
fn nested_folders_and_their_windows_read_back() {
    let p = six_in_folders();
    let out = written(&p);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    let q = imp.into_project();
    let roots: Vec<String> = q.root.folders.iter().map(|f| shape(f, &q)).collect();
    assert_eq!(roots, ["F1[Book2](F1a[Book3](F1b[Book4]()))", "F2[Book5,Book6]()", "F3[]()"]);
    assert!(q.root.windows.is_empty(), "windows of the root are not listed");
    assert_eq!(roots, p.root.folders.iter().map(|f| shape(f, &p)).collect::<Vec<_>>());
}

#[test]
fn windows_no_folder_lists_stay_in_the_root_and_a_double_listing_keeps_the_first() {
    let mut p = six_in_folders();
    let ids: Vec<WindowId> = p.windows.iter().map(Window::id).collect();
    // Book1 is in no folder: it stays in the root and no folder lists it
    let q = read_back(&written(&p).bytes).into_project();
    assert!(q.root.folders.iter().all(|f| !shape(f, &q).contains("Book1")));
    // Book2 is listed by F1 and by F3 after it, and Book1 is listed by F2: windows of a folder are
    // listed by their position in the file
    p.root.folders[1].windows.push(ids[0]);
    p.root.folders[2].windows.push(ids[1]);
    let q = read_back(&written(&p).bytes).into_project();
    let roots: Vec<String> = q.root.folders.iter().map(|f| shape(f, &q)).collect();
    assert_eq!(roots, ["F1[Book2](F1a[Book3](F1b[Book4]()))", "F2[Book1,Book5,Book6]()", "F3[]()"]);
}

#[test]
fn graphs_are_listed_in_their_folders_too() {
    let mut p = Project::new();
    let b = p.adopt_named_workbook(abcd("Book1"));
    let g = p.add_graph(sheaf_core::template::Template::Line);
    p.root.folders = vec![leaf("Plots", vec![g]), leaf("Data", vec![b])];
    let q = read_back(&written(&p).bytes).into_project();
    let roots: Vec<String> = q.root.folders.iter().map(|f| shape(f, &q)).collect();
    assert_eq!(roots, ["Plots[Graph1]()", "Data[Book1]()"]);
}

#[test]
fn a_project_without_folders_has_none_after_reading() {
    let q = read_back(&written(&six_in_folders_flat()).bytes).into_project();
    assert!(q.root.folders.is_empty());
}

fn six_in_folders_flat() -> Project {
    let mut p = six_in_folders();
    p.root.folders.clear();
    p
}

fn refusal(p: &Project) -> String {
    match write_named(p, "x", TIME) {
        Err(IoError::OpjWrite { reason }) => reason,
        other => panic!("expected a refusal, got {:?}", other.map(|w| w.warnings)),
    }
}

#[test]
fn folder_names_that_origin_cannot_store_are_refused() {
    let named = |name: &str| {
        let mut p = six_in_folders();
        p.root.folders[2].name = name.into();
        p
    };
    assert!(refusal(&named("a/b")).contains("slash"));
    assert!(refusal(&named("a\\b")).contains("slash"));
    assert!(refusal(&named("  ")).contains("no name"));
    assert!(refusal(&named("")).contains("no name"));
    assert!(refusal(&named("\u{3a9}")).contains("Windows-1252"));
    assert!(refusal(&named("tab\there")).contains("control"));
    assert!(refusal(&named(&"x".repeat(MAX_FOLDER_NAME + 1))).contains("longer"));
    assert!(write_named(&named(&"x".repeat(MAX_FOLDER_NAME)), "x", TIME).is_ok());
    // two folders in one place with names that differ in case only
    let mut p = six_in_folders();
    p.root.folders[2].name = "f1".into();
    assert!(refusal(&p).contains("distinct folder names"));
    // the same name in different places is fine
    let mut p = six_in_folders();
    p.root.folders[2].name = "F1a".into();
    assert!(write_named(&p, "x", TIME).is_ok());
}

#[test]
fn folders_nested_too_deep_are_refused() {
    let mut inner = leaf("L", Vec::new());
    for i in 0..MAX_FOLDER_DEPTH {
        let mut f = leaf(&format!("L{i}"), Vec::new());
        f.folders.push(inner);
        inner = f;
    }
    let mut p = six_in_folders();
    p.root.folders = vec![inner];
    assert!(refusal(&p).contains("nested more than"));
    // exactly the limit is fine
    let mut inner = leaf("L", Vec::new());
    for i in 0..MAX_FOLDER_DEPTH - 1 {
        let mut f = leaf(&format!("L{i}"), Vec::new());
        f.folders.push(inner);
        inner = f;
    }
    p.root.folders = vec![inner];
    assert!(write_named(&p, "x", TIME).is_ok());
}

#[test]
fn the_size_in_front_of_the_tree_matches_what_follows_for_nested_folders() {
    let bytes = written(&six_in_folders()).bytes;
    // the marker block (16 bytes) starts the tree; the size block is the 10 bytes before it
    let marker = [16u8, 0, 0, 0, b'\n', 0xde, 0x36, 0x10, 0x03];
    let start = bytes.windows(marker.len()).rposition(|w| w == marker).unwrap();
    let size = u32::from_le_bytes(bytes[start - 5..start - 1].try_into().unwrap()) as usize;
    assert_eq!(&bytes[start - 10..start - 5], [4, 0, 0, 0, b'\n']);
    let closer = [8u8, 0, 0, 0, b'\n', 0, 0x10, 0, 0, 8, 0, 0, 0, b'\n'];
    let end = bytes.windows(closer.len()).rposition(|w| w == closer).unwrap() + closer.len();
    assert_eq!(end - start - 4, size);
}

#[test]
fn every_truncation_of_a_written_file_reads_without_panicking() {
    let mut p = six_in_folders();
    p.windows.truncate(6);
    set(&mut p, "Book1", 0, "A", "5", None, RecalcMode::None);
    let bytes = written(&p).bytes;
    let from = bytes.windows(10).rposition(|w| w == b"ResultsLog").unwrap() - 120;
    for end in (from..bytes.len()).step_by(7) {
        // Whatever the cut, reading returns either an error or an import; it must not panic.
        let _ = read_opj_from(Cursor::new(bytes[..end].to_vec()), Path::new("cut.opj"));
    }
}

#[test]
fn a_tail_that_is_damaged_gives_a_note_and_no_folders() {
    let mut bytes = written(&six_in_folders()).bytes;
    // break the number of sub-folders of the root: find the root's name and corrupt what follows
    let at = bytes.windows(10).rposition(|w| w == b"ResultsLog").unwrap();
    let marker = bytes[at..].windows(4).position(|w| w == [0xde, 0x36, 0x10, 0x03]).unwrap() + at;
    // the times block of the root starts 5 bytes (head) + 16 + 1 + 1 after the marker start
    bytes.truncate(marker + 300);
    let imp = read_back(&bytes);
    assert!(imp.folders.is_empty());
    assert_eq!(imp.workbooks.len(), 6, "the data is still read");
    assert!(imp.warnings.iter().any(|w| w.contains("folder tree could not be read")), "{:?}", imp.warnings);
}

#[test]
fn a_sheet_long_name_is_written_as_origins_longname_annotation() {
    // T343: Origin 2016 keeps it as a worksheet-layer annotation named "__LONGNAME".
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].long_name = "Scan 1".into();
    let out = written(&p);
    assert!(out.bytes.windows(10).any(|w| w == b"__LONGNAME"));
    let back = read_back(&out.bytes).into_project();
    let Window::Workbook(b) = &back.windows[0] else { unreachable!() };
    assert_eq!(b.sheets[0].long_name, "Scan 1");
}

#[test]
fn numeric_displays_are_written_where_origin_keeps_them_and_read_back() {
    // T337: Origin 2016 keeps the display at 0x1E (high nibble) and the digits at 0x1F of the column
    // record of a Numeric-format column.
    use sheaf_core::format::NumberFormat as F;
    for format in [
        F::Decimal(3),
        F::Decimal(0),
        F::Significant(4),
        F::Scientific(2),
        F::ScientificAuto,
        F::EngineeringSi(1),
        F::EngineeringSig(3),
        F::EngineeringAuto,
        F::DecimalGrouped(2),
    ] {
        let mut p = Project::new();
        p.adopt_named_workbook(abcd("Book1"));
        let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
        b.sheets[0].columns[1].format = format;
        let out = written(&p);
        assert!(!out.warnings.iter().any(|w| w.contains("display format")), "{format:?}: {:?}", out.warnings);
        let back = read_back(&out.bytes).into_project();
        let Window::Workbook(b) = &back.windows[0] else { unreachable!() };
        assert_eq!(b.sheets[0].columns[1].format, format);
        assert_eq!(b.sheets[0].columns[1].numeric(), p.workbooks().next().unwrap().sheets[0].columns[1].numeric());
        assert_eq!(b.sheets[0].columns[0].format, F::General, "{format:?}: the other columns keep the default");
    }
}

#[test]
fn a_column_width_is_written_where_origin_keeps_it_and_read_back() {
    // T334: Origin 2016 keeps the width in tenths of a character at 0x4A of the column record.
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].columns[1].width = Some(14);
    let back = read_back(&written(&p).bytes).into_project();
    let Window::Workbook(b) = &back.windows[0] else { unreachable!() };
    assert_eq!(b.sheets[0].columns[1].width, Some(14));
    assert_eq!(b.sheets[0].columns[0].width, None, "the default width stays the default");
    assert_eq!(b.sheets[0].columns[2].width, None);
}

#[test]
fn group_and_subject_columns_keep_their_roles_in_the_opj() {
    // T330: Origin 2016 stores Group (LabTalk type 8) as role 7 and Subject (9) as 8; Origin read
    // both back from a file written here (re-workspace/scratch/t330/sheaf-gs.opj).
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].columns[2].designation = sheaf_core::Designation::Group;
    b.sheets[0].columns[3].designation = sheaf_core::Designation::Subject;
    let out = written(&p);
    let back = read_back(&out.bytes).into_project();
    let Window::Workbook(b) = &back.windows[0] else { unreachable!() };
    assert_eq!(b.sheets[0].columns[2].designation, sheaf_core::Designation::Group);
    assert_eq!(b.sheets[0].columns[3].designation, sheaf_core::Designation::Subject);
}

#[test]
fn a_before_formula_script_is_named_in_the_warnings() {
    // T307: the .opj written here has no place for the script.
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].columns[2].formula = Some("col(A) - m".into());
    b.sheets[0].columns[2].before_formula = Some("m = mean(col(B))".into());
    let all = written(&p).warnings.join(" | ");
    assert!(all.contains("1 column formula(s) use Before Formula Scripts"), "{all}");
}

#[test]
fn a_sampling_interval_is_written_where_origin_keeps_it_and_read_back() {
    // T303: Origin 2016 keeps xinit/xinc in the column record (0x1D2, 0x1DA) and the data-set header
    // (0x25, 0x2D), with the column belonging to no X column.
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].columns[2].sampling = Some(sheaf_core::sampling::SamplingInterval {
        x0: 3.5,
        inc: 0.125,
        long_name: String::new(),
        units: String::new(),
    });
    let out = written(&p);
    assert!(!out.warnings.iter().any(|w| w.contains("sampling")), "{:?}", out.warnings);
    let back = read_back(&out.bytes).into_project();
    let Window::Workbook(b) = &back.windows[0] else { unreachable!() };
    let s = b.sheets[0].columns[2].sampling.as_ref().expect("the interval comes back");
    assert_eq!((s.x0, s.inc), (3.5, 0.125));
    assert!(b.sheets[0].columns[1].sampling.is_none() && b.sheets[0].columns[3].sampling.is_none());
}

#[test]
fn hidden_columns_and_column_widths_are_named_in_the_warnings() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].columns[1].hidden = true;
    b.sheets[0].columns[2].width = Some(14);
    b.sheets[0].columns[2].sampling =
        Some(sheaf_core::sampling::SamplingInterval { x0: 0.0, inc: 0.5, long_name: "Time".into(), units: "s".into() });
    b.sheets[0].columns[1].data_range = Some(sheaf_core::data_range::DataRange { begin: 1, end: Some(2) });
    b.sheets[0].columns[0].mask.insert(0);
    b.sheets[0].columns[3].designation = sheaf_core::Designation::Group;
    b.sheets[0].filter =
        Some(sheaf_core::rowfilter::RowFilter { condition: "col(A) > 0".into(), enabled: true, keep: vec![true] });
    b.sheets[0].frozen = sheaf_core::Frozen { rows: 1, columns: 0 };
    b.sheets[0].long_name = "Scan".into();
    b.sheets[0].view.units = false;
    b.sheets[0].cond_formats = vec![sheaf_core::condformat::CondFormat {
        name: "C1R1-C1R0".into(),
        range: sheaf_core::condformat::CondRange::whole_columns(vec![b.sheets[0].columns[0].id]),
        rule: sheaf_core::condformat::Rule::min_max(),
    }];
    let first = b.sheets[0].columns[0].id;
    b.sheets[0].set_note(first, 0, "a note");
    b.sheets[0].apply_cell_format(
        sheaf_core::cellformat::FormatTarget::whole_columns(vec![first]),
        sheaf_core::cellformat::CellFormat { bold: Some(true), ..Default::default() },
    );
    let all = written(&p).warnings.join(" | ");
    assert!(all.contains("1 sheet(s) have cell formatting"), "{all}");
    assert!(all.contains("1 cell note(s) are not written"), "{all}");
    assert!(all.contains("1 sheet(s) have frozen panes"), "{all}");
    assert!(!all.contains("sheet(s) have a long name"), "a sheet long name is written now (T343): {all}");
    assert!(all.contains("1 sheet(s) hide label rows"), "{all}");
    assert!(all.contains("1 sheet(s) have conditional formatting"), "{all}");
    assert!(all.contains("1 hidden column(s)"), "{all}");
    assert!(!all.contains("their own width"), "widths are written now (T334): {all}");
    assert!(all.contains("1 column(s) have a sampling interval"), "{all}");
    assert!(all.contains("1 column(s) have a data range"), "{all}");
    assert!(!all.contains("masked cells"), "masks are written now: {all}");
    assert!(!all.contains("Group or Subject"), "Group and Subject are written now (T330): {all}");
    assert!(all.contains("1 sheet(s) have a worksheet filter"), "{all}");
}

#[test]
fn masked_cells_are_written_in_origins_layout_and_read_back() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].columns[1].mask.extend([0, 2]);
    let w = written(&p);
    assert!(w.warnings.iter().all(|x| !x.contains("mask")), "{:?}", w.warnings);
    let imp = read_back(&w.bytes);
    let cols = &imp.workbooks[0].sheets[0].columns;
    assert_eq!(cols[1].mask.iter().copied().collect::<Vec<_>>(), vec![0, 2]);
    assert!(cols[0].mask.is_empty());
}

/// Writes `sheaf-masked.opj` (column B rows 1 and 3 masked, 1-based) into `SHEAF_OPJ_MASK_DIR`, so
/// Origin can be asked which cells it sees masked (opj-format.md 3.2):
/// `SHEAF_OPJ_MASK_DIR=<folder> cargo test -p sheaf-io -- --ignored write_masked_project_for_origin`.
#[test]
#[ignore]
fn write_masked_project_for_origin() {
    let Ok(dir) = std::env::var("SHEAF_OPJ_MASK_DIR") else { return };
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let Window::Workbook(b) = &mut p.windows[0] else { unreachable!() };
    b.sheets[0].columns[1].mask.extend([0, 2]);
    std::fs::write(Path::new(&dir).join("sheaf-masked.opj"), written(&p).bytes).unwrap();
}

#[test]
fn notes_windows_are_written_and_read_back_with_their_folders() {
    // T470: Notes windows go into the notes list after the window list, as Origin 2016 writes them.
    let mut p = Project::new();
    let book = p.adopt_named_workbook(abcd("Book1"));
    let a = p.add_notes();
    let b = p.add_notes();
    p.notes_mut(a).unwrap().text = "first line\nsecond line".into();
    p.notes_mut(b).unwrap().name = "Diary".into();
    p.notes_mut(b).unwrap().text = "x".into();
    // the book and the second notes window in a folder, the first notes window in the root
    p.root.folders.push(sheaf_core::project::Folder {
        name: "Sub".into(),
        windows: vec![b, book],
        ..Default::default()
    });
    let out = written(&p);
    assert!(out.warnings.iter().all(|w| !w.contains("Notes")), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert_eq!(
        imp.notes,
        [("Notes1".to_string(), "first line\nsecond line".to_string()), ("Diary".to_string(), "x".to_string())]
    );
    assert_eq!(imp.folders.len(), 1);
    assert_eq!(imp.folders[0].windows, ["Book1", "Diary"], "a notes entry is not taken for window 0");
    let back = imp.into_project();
    assert_eq!(back.notes().count(), 2);
    let diary = back.notes().find(|n| n.name == "Diary").unwrap();
    assert_eq!(back.root.folders[0].windows.len(), 2);
    assert!(back.root.folders[0].windows.contains(&diary.id));
    // the text is stored with CR LF line breaks and a terminator
    let crlf = b"first line\r\nsecond line\0";
    assert!(out.bytes.windows(crlf.len()).any(|w| w == crlf));
}

#[test]
fn a_notes_window_is_three_blocks_in_front_of_the_results_log() {
    // T470, measured in Origin 2016: an 80-byte header (window rectangle, 00 C0 CF 54 at 0x10, 1 at 0x18,
    // the two times at 0x20 and 0x28), the name and the text, each with a terminator.
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let n = p.add_notes();
    p.notes_mut(n).unwrap().name = "MyNotes".into();
    let bytes = written(&p).bytes;
    let at = bytes.windows(8).rposition(|w| w == b"MyNotes\0").unwrap();
    // the header block ends 5 bytes before the name block's size field
    let name_block = at - 5;
    let header = &bytes[name_block - 81..name_block - 1];
    assert_eq!(&bytes[name_block - 86..name_block - 81], &[80, 0, 0, 0, b'\n']);
    let i32_at = |k: usize| i32::from_le_bytes(header[k..k + 4].try_into().unwrap());
    assert_eq!((i32_at(0), i32_at(4), i32_at(8), i32_at(12)), (26, 26, 551, 266), "as Origin placed its first one");
    assert_eq!(&header[0x10..0x14], &[0x00, 0xc0, 0xcf, 0x54]);
    assert_eq!(header[0x18], 1);
    assert_eq!(f64::from_le_bytes(header[0x20..0x28].try_into().unwrap()), TIME);
    // the empty text, then the results log
    assert_eq!(&bytes[at + 9..at + 9 + 7], &[1, 0, 0, 0, b'\n', 0, b'\n']);
    assert_eq!(&bytes[at + 16..at + 21], &[80, 0, 0, 0, b'\n']);
    assert_eq!(&bytes[at + 21 + 81 + 5..at + 21 + 81 + 5 + 10], b"ResultsLog");
}

/// Writes `sheaf-notes.opj` (Book1, then Notes1 with two lines and Diary in folder Sub) into
/// `SHEAF_OPJ_NOTES_DIR`, so Origin can be asked to open it (T470):
/// `SHEAF_OPJ_NOTES_DIR=<folder> cargo test -p sheaf-io -- --ignored write_notes_project_for_origin`.
#[test]
#[ignore]
fn write_notes_project_for_origin() {
    let Ok(dir) = std::env::var("SHEAF_OPJ_NOTES_DIR") else { return };
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let a = p.add_notes();
    let b = p.add_notes();
    p.notes_mut(a).unwrap().text = "first line\nsecond line".into();
    p.notes_mut(b).unwrap().name = "Diary".into();
    p.notes_mut(b).unwrap().text = "in a folder".into();
    p.root.folders.push(sheaf_core::project::Folder { name: "Sub".into(), windows: vec![b], ..Default::default() });
    std::fs::write(Path::new(&dir).join("sheaf-notes.opj"), written(&p).bytes).unwrap();
}

/// One Y column per Date, Time, Month and Day of Week display of Origin's lists (T474), after an X column.
fn date_book() -> (Project, Vec<sheaf_core::format::NumberFormat>) {
    use sheaf_core::datetime::TimePattern;
    use sheaf_core::format::{NameStyle, NumberFormat as F};
    let mut formats: Vec<F> = crate::opj::DATE_SUBFORMATS.to_vec();
    formats.extend(TimePattern::ALL.iter().map(|t| F::Time(*t)));
    for s in [NameStyle::Short, NameStyle::Full, NameStyle::Letter] {
        formats.push(F::Month(s));
    }
    for s in [NameStyle::Short, NameStyle::Full, NameStyle::Letter] {
        formats.push(F::Weekday(s));
    }
    let mut columns = vec![numeric("A", Designation::X, &[1.0, 2.0])];
    for (i, f) in formats.iter().enumerate() {
        let mut c = numeric(&format!("C{}", i + 1), Designation::Y, &[2_461_355.573_032_407_4, 0.573_032_407_4]);
        c.format = *f;
        columns.push(c);
    }
    let mut p = Project::new();
    p.adopt_named_workbook(book("Book1", vec![("Sheet1", columns)]));
    (p, formats)
}

#[test]
fn date_and_time_displays_are_written_and_read_back() {
    // T474, measured in Origin 2016: the value type at 0x1E (3 Date, 2 Time, 4 Month, 5 Day of Week) and
    // the subformat minus 1 at 0x1F, with Origin's Numeric cells.
    let (p, formats) = date_book();
    let out = written(&p);
    assert!(out.warnings.iter().all(|w| !w.contains("format")), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert!(imp.warnings.iter().all(|w| !w.contains("day numbers")), "{:?}", imp.warnings);
    let cols = &imp.workbooks[0].sheets[0].columns;
    let read: Vec<sheaf_core::format::NumberFormat> = cols[1..].iter().map(|c| c.format).collect();
    assert_eq!(read, formats);
    assert_eq!(cols[1].numeric().unwrap()[0], 2_461_355.573_032_407_4, "the day number itself");
}

/// Writes `sheaf-dates.opj` (the book of `date_book`) into `SHEAF_OPJ_DATES_DIR`, so Origin can be asked
/// what it shows (T474):
/// `SHEAF_OPJ_DATES_DIR=<folder> cargo test -p sheaf-io -- --ignored write_dates_project_for_origin`.
#[test]
#[ignore]
fn write_dates_project_for_origin() {
    let Ok(dir) = std::env::var("SHEAF_OPJ_DATES_DIR") else { return };
    std::fs::write(Path::new(&dir).join("sheaf-dates.opj"), written(&date_book().0).bytes).unwrap();
}

#[test]
fn a_date_column_record_holds_type_and_subformat() {
    use sheaf_core::datetime::DatePattern;
    use sheaf_core::format::NumberFormat as F;
    let mut p = Project::new();
    let mut c = numeric("A", Designation::X, &[2_461_355.0]);
    c.format = F::Date(DatePattern::MdyDash);
    let mut iso = numeric("B", Designation::Y, &[2_461_355.0]);
    iso.format = F::Date(DatePattern::Iso);
    p.adopt_named_workbook(book("Book1", vec![("Sheet1", vec![c, iso])]));
    let out = written(&p);
    // the first column record: Date (3), subformat 20 (19), the X type word of Numeric cells
    // the first block of 515 bytes is the first column record (a project without graphs)
    let at = out.bytes.windows(5).position(|w| w == [0x03, 0x02, 0, 0, b'\n']).unwrap() + 5;
    let rec = &out.bytes[at..at + 515];
    assert_eq!((rec[0x1E], rec[0x1F]), (0x03, 19));
    assert_eq!(u16::from_le_bytes([rec[0x25], rec[0x26]]), 0x5001);
    // yyyy-MM-dd is not in Origin's Date list: the column keeps its numbers and the warning says so
    assert!(out.warnings.iter().any(|w| w.contains("display format")), "{:?}", out.warnings);
}

fn matrix_project() -> Project {
    use sheaf_core::matrix::MatrixSheet;
    let mut p = Project::new();
    let first = MatrixSheet::from_values(
        "MSheet1",
        0,
        3,
        4,
        vec![101.0, 102.0, 103.0, 104.0, 201.0, f64::NAN, 203.0, 204.0, 301.0, 302.0, 303.0, 304.0],
        (0.5, 2.5),
        (10.0, 20.0),
    )
    .unwrap();
    let second = MatrixSheet::from_values("Data", 0, 2, 2, vec![1.0, 2.0, 3.0, 4.0], (1.0, 10.0), (1.0, 10.0)).unwrap();
    let mut first = first;
    first.z =
        sheaf_core::matrix::MatrixLabel { long_name: "Height".into(), units: "nm".into(), comments: "scan 1".into() };
    let id = p.adopt_matrix_book("Surface", vec![first, second]);
    p.matrix_book_mut(id).unwrap().name = "MBook1".into();
    p.adopt_named_workbook(abcd("Book1"));
    p.add_matrix_book("");
    p
}

#[test]
fn matrix_books_are_written_and_read_back() {
    // T472: as Origin 2016 stores them (opj-format.md 13.16).
    let p = matrix_project();
    let out = written(&p);
    assert!(out.warnings.iter().all(|w| !w.contains("matri")), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert_eq!(imp.matrices.len(), 2);
    assert_eq!(imp.workbooks.len(), 1, "the workbook's columns are not taken for the matrix's");
    let q = imp.into_project();
    let books: Vec<&sheaf_core::matrix::MatrixBook> = q.matrix_books().collect();
    assert_eq!((books[0].name.as_str(), books[0].long_name.as_str(), books[0].sheets.len()), ("MBook1", "Surface", 2));
    let (a, b) = (&books[0].sheets[0], &books[0].sheets[1]);
    let before = p.matrix_books().next().unwrap();
    assert_eq!((a.rows, a.cols, a.x_from, a.x_to, a.y_from, a.y_to), (3, 4, 0.5, 2.5, 10.0, 20.0));
    assert_eq!(
        a.values().iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        before.sheets[0].values().iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!((b.name.as_str(), b.rows, b.cols, b.values()), ("Data", 2, 2, &[1.0, 2.0, 3.0, 4.0][..]));
    assert_eq!((b.x_from, b.x_to), (1.0, 10.0));
    // an empty 32 x 32 book: every cell missing
    assert_eq!((books[1].sheets[0].rows, books[1].sheets[0].cols), (32, 32));
    assert!(books[1].sheets[0].values().iter().all(|v| v.is_nan()));
    let book1 = &q.workbooks().next().unwrap().sheets[0];
    assert_eq!(book1.columns[0].numeric().unwrap(), &[1.0, 2.0, 3.0, 4.0, 5.0][..]);
}

#[test]
fn a_matrix_sheet_is_written_as_origin_writes_one() {
    let p = matrix_project();
    let bytes = written(&p).bytes;
    let has = |pat: &[u8]| bytes.windows(pat.len()).any(|w| w == pat);
    // the coordinates as text annotations, only for the sheet whose range is not 1 to 10
    assert!(has(b"X1\0") && has(b"Y2\0"));
    assert!(has(b"0.5\0") && has(b"2.5\0"));
    assert_eq!(bytes.windows(3).filter(|w| w == b"X1\0").count(), 1);
    // data sets named after the book, then @2 for the second sheet
    assert!(has(b"MBook1@2\0"));
    // the layer header: the matrix flag, 4 columns and 3 rows
    let at = bytes.windows(9).position(|w| w == b"PdMSheet1").unwrap() - 0xD0;
    assert_eq!(bytes[at + 2], 0xDE);
    assert_eq!(u16::from_le_bytes([bytes[at + 0x2B], bytes[at + 0x2C]]), 4);
    assert_eq!(u16::from_le_bytes([bytes[at + 0x52], bytes[at + 0x53]]), 3);
}

#[test]
fn the_z_labels_of_a_matrix_are_written_in_its_column_note() {
    // T473, measured in Origin 2016: `r.lname$`, `r.unit$`, `r.comment$` of a matrix range are stored as
    // `long CR LF units CR LF comments` in the note of the sheet's column record.
    let mut p = matrix_project();
    let id = p.matrix_books().next().unwrap().id;
    let z = &mut p.matrix_book_mut(id).unwrap().sheets[0].z;
    z.long_name = "Height".into();
    z.units = "nm".into();
    z.comments = "scan 1\nleft side".into();
    let out = written(&p);
    assert!(out.warnings.iter().all(|w| !w.contains("matrix sheet")), "{:?}", out.warnings);
    let note = b"Height\r\nnm\r\nscan 1\r\nleft side\0";
    assert!(out.bytes.windows(note.len()).any(|w| w == note));
    let q = read_back(&out.bytes).into_project();
    let z = &q.matrix_books().next().unwrap().sheets[0].z;
    assert_eq!((z.long_name.as_str(), z.units.as_str(), z.comments.as_str()), ("Height", "nm", "scan 1\nleft side"));
}

#[test]
fn a_matrix_with_x_labels_warns_and_a_long_book_name_is_refused() {
    let mut p = matrix_project();
    let id = p.matrix_books().next().unwrap().id;
    p.matrix_book_mut(id).unwrap().sheets[0].x.long_name = "Width".into();
    let out = written(&p);
    assert!(out.warnings.iter().any(|w| w.contains("1 matrix sheet(s) have X or Y labels")), "{:?}", out.warnings);
    p.matrix_book_mut(id).unwrap().name = "M".repeat(23);
    assert!(refusal(&p).contains("too long for its 2 sheets"), "{}", refusal(&p));
}

/// Writes `sheaf-matrix.opj` (the books of `matrix_project`) into `SHEAF_OPJ_MATRIX_DIR`, so Origin can
/// be asked to open it (T472):
/// `SHEAF_OPJ_MATRIX_DIR=<folder> cargo test -p sheaf-io -- --ignored write_matrix_project_for_origin`.
#[test]
#[ignore]
fn write_matrix_project_for_origin() {
    let Ok(dir) = std::env::var("SHEAF_OPJ_MATRIX_DIR") else { return };
    std::fs::write(Path::new(&dir).join("sheaf-matrix.opj"), written(&matrix_project()).bytes).unwrap();
}

#[test]
fn a_notes_window_with_a_long_name_warns() {
    let mut p = Project::new();
    p.adopt_named_workbook(abcd("Book1"));
    let n = p.add_notes();
    p.notes_mut(n).unwrap().long_name = "Lab diary".into();
    let out = written(&p);
    assert!(out.warnings.iter().any(|w| w.contains("1 Notes window(s) have a long name")), "{:?}", out.warnings);
}

#[test]
fn notes_sheets_are_left_out_with_a_warning() {
    // T188: a workbook's Notes sheet is not written; its other sheets are.
    let mut p = Project::new();
    let id = p.adopt_named_workbook(abcd("Book1"));
    let b = p.workbook_mut(id).unwrap();
    let s = b.add_notes_sheet();
    b.sheets[s].notes_text = Some("hello".into());
    let out = written(&p);
    assert!(out.warnings.iter().any(|w| w.contains("1 Notes or graph sheet(s) were left out")), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert_eq!(imp.workbooks.len(), 1);
    assert_eq!(imp.workbooks[0].sheets.len(), 1, "only the data sheet");
}
