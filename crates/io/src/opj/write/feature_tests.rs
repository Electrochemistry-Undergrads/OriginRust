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
        Window::Graph(_) => unreachable!(),
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
        Window::Graph(_) => unreachable!(),
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
