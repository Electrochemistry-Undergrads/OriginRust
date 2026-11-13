use super::*;
use crate::opj::{OpjImport, read_opj_from};
use sheaf_core::column::ColumnData;
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::project::Window;
use std::io::Cursor;
use std::path::Path;

const TIME: f64 = 2_461_000.5;

fn column(short: &str, designation: Designation, data: ColumnData) -> Column {
    Column::new(ColumnId(0), short, designation, data)
}

fn nums(v: &[f64]) -> ColumnData {
    ColumnData::numeric(v.to_vec())
}

fn texts(v: &[&str]) -> ColumnData {
    ColumnData::text(v.iter().map(|s| s.to_string()).collect())
}

fn sheet(name: &str, columns: Vec<Column>) -> Worksheet {
    let mut s = Worksheet::new(name);
    s.columns = columns;
    s
}

fn book(name: &str, long: &str, sheets: Vec<Worksheet>) -> Workbook {
    let mut b = Workbook::new(WindowId(0), name);
    b.long_name = long.to_string();
    b.sheets = sheets;
    b
}

fn project_of(books: Vec<Workbook>) -> Project {
    let mut p = Project::new();
    for b in books {
        p.adopt_named_workbook(b);
    }
    p
}

/// A project with every kind of column the writer supports. Also the source of the fixture.
fn sample_project() -> Project {
    let x = column("A", Designation::X, nums(&[0.0, 0.25, 0.5, 0.75, 1.0])).with_labels("Potential", "V", "scan 1");
    let y = column("B", Designation::Y, nums(&[1.5, f64::NAN, -2.25e-9, 3.0e12, 5.0])).with_labels("Current", "A", "");
    let e = column("C", Designation::YError, nums(&[0.1, 0.1, 0.1, 0.1, 0.1])).with_labels("", "A", "");
    let z = column("D", Designation::Z, nums(&[1.0, 2.0, 3.0])).with_labels("Depth", "", "only three");
    let l = column("E", Designation::Label, texts(&["a", "", "caf\u{e9}", "5\u{b0}C", "\u{b5}m"]))
        .with_labels("Tag", "", "");
    let d = column("F", Designation::Disregard, nums(&[7.0])).with_labels("", "", "first line\nsecond line");
    let xe = column("G", Designation::XError, nums(&[0.5])).with_labels("x err", "V", "");
    let first = sheet("Data", vec![x, y, e, z, l, d, xe]);
    let second = sheet(
        "Second run",
        vec![
            column("A", Designation::X, nums(&[f64::NAN, 10.0, 20.0])),
            column("B", Designation::Y, nums(&[f64::NAN, 1.0, 2.0])).with_labels("Counts", "cps", "after a gap"),
        ],
    );
    let other = sheet("Sheet1", vec![column("A", Designation::Y, texts(&["only text", "x"]))]);
    project_of(vec![book("Book1", "Cyclic voltammetry", vec![first, second]), book("Notes", "", vec![other])])
}

fn read_back(bytes: &[u8]) -> OpjImport {
    read_opj_from(Cursor::new(bytes.to_vec()), Path::new("written.opj")).unwrap()
}

/// The value of cell `i` as the reader gives it, or missing past the end.
fn cell(c: &Column, i: usize) -> Option<String> {
    match &c.data {
        ColumnData::Numeric(v) => v.get(i).filter(|x| !x.is_nan()).map(|x| format!("{x:?}")),
        ColumnData::Text(v) => v.get(i).filter(|t| !t.is_empty()).cloned(),
        ColumnData::Mixed { numbers, .. } => match c.data.text_cell(i) {
            Some(t) => Some(t.to_string()),
            None => numbers.get(i).filter(|x| !x.is_nan()).map(|x| format!("{x:?}")),
        },
    }
}

fn assert_same_books(project: &Project, imp: &OpjImport) {
    let books: Vec<&Workbook> = project.workbooks().collect();
    assert_eq!(imp.workbooks.len(), books.len());
    for (want, got) in books.iter().zip(&imp.workbooks) {
        assert_eq!((&got.name, &got.long_name), (&want.name, &want.long_name));
        assert_eq!(got.sheets.len(), want.sheets.len(), "sheets of {}", want.name);
        for (ws, gs) in want.sheets.iter().zip(&got.sheets) {
            assert_eq!(gs.name, ws.name);
            assert_eq!(gs.columns.len(), ws.columns.len(), "columns of {}/{}", want.name, ws.name);
            for (wc, gc) in ws.columns.iter().zip(&gs.columns) {
                let what = format!("{}/{}/{}", want.name, ws.name, wc.short_name);
                assert_eq!(gc.short_name, wc.short_name, "{what}");
                assert_eq!(gc.designation, wc.designation, "{what}");
                assert_eq!(gc.long_name, wc.long_name, "{what}");
                assert_eq!(gc.units, wc.units, "{what}");
                // the reader keeps the CR LF that Origin puts between lines of a comment
                assert_eq!(gc.comments.replace("\r\n", "\n"), wc.comments, "{what}");
                assert_eq!(gc.data.is_numeric(), wc.data.is_numeric(), "{what}: kind of data");
                let rows = wc.len().max(gc.len());
                for i in 0..rows {
                    assert_eq!(cell(gc, i), cell(wc, i), "{what} row {i}");
                }
            }
        }
    }
}

fn written(project: &Project) -> OpjWrite {
    write_named(project, "sample", TIME).unwrap()
}

#[test]
fn a_project_with_every_kind_of_column_reads_back_unchanged() {
    let project = sample_project();
    let out = written(&project);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert_eq!(imp.version, "9.3");
    assert_eq!(imp.signature, "CPYA 4.3168 226 W64 #");
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    assert_same_books(&project, &imp);
}

#[test]
fn a_text_and_numeric_column_is_written_with_both_kinds_of_cell() {
    let texts_at = [(1, "<LOD".to_string()), (4, "caf\u{e9} 5\u{b0}C".to_string())].into_iter().collect();
    let mixed = ColumnData::mixed(vec![1.5, f64::NAN, 2.5e-9, f64::NAN, f64::NAN, f64::NAN, 7.0], texts_at);
    let project = project_of(vec![book(
        "Mixed",
        "",
        vec![sheet("S", vec![column("A", Designation::Y, mixed), column("B", Designation::Y, nums(&[1.0]))])],
    )]);
    let out = written(&project);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    let got = &imp.workbooks[0].sheets[0].columns[0];
    assert!(got.data.is_mixed());
    assert_eq!(got.numeric().unwrap()[0], 1.5);
    assert_eq!(got.numeric().unwrap()[6], 7.0);
    assert_eq!(got.cell_text(1), "<LOD");
    assert_eq!(got.cell_text(4), "caf\u{e9} 5\u{b0}C");
    assert_same_books(&project, &imp);
}

#[test]
fn numbers_keep_every_bit_and_missing_values_stay_missing() {
    let values =
        [0.1, -0.0, f64::MIN_POSITIVE, 1.0 / 3.0, f64::MAX, f64::NAN, 1e-300, f64::INFINITY, f64::NEG_INFINITY];
    let project =
        project_of(vec![book("Bits", "", vec![sheet("S", vec![column("A", Designation::Y, nums(&values))])])]);
    let imp = read_back(&written(&project).bytes);
    let got = imp.workbooks[0].sheets[0].columns[0].numeric().unwrap().to_vec();
    assert_eq!(got.len(), values.len());
    for (g, w) in got.iter().zip(values) {
        if w.is_nan() {
            assert!(g.is_nan());
        } else {
            assert_eq!(g.to_bits(), w.to_bits());
        }
    }
}

#[test]
fn many_rows_and_many_columns() {
    let rows = 20_000;
    let x: Vec<f64> = (0..rows).map(|i| f64::from(i) * 0.001).collect();
    let y: Vec<f64> = (0..rows).map(|i| if i % 7 == 3 { f64::NAN } else { (f64::from(i) * 0.01).sin() }).collect();
    let words: Vec<String> = (0..rows).map(|i| format!("row {i}")).collect();
    let mut columns = vec![column("T", Designation::X, nums(&x)), column("Y1", Designation::Y, nums(&y))];
    columns.push(column("W", Designation::Label, ColumnData::text(words)));
    for k in 0..40 {
        columns.push(column(&format!("C{k}"), Designation::Y, nums(&x[..100 + k])));
    }
    let project = project_of(vec![book("Big", "many rows", vec![sheet("S", columns)])]);
    let out = written(&project);
    assert_same_books(&project, &read_back(&out.bytes));
}

#[test]
fn names_with_accents_and_symbols_survive() {
    let c = column("\u{b5}x", Designation::Y, nums(&[1.0])).with_labels(
        "Temp\u{e9}rature 5\u{b0}C",
        "\u{b5}m/s\u{b2}",
        "caf\u{e9} \u{20ac}",
    );
    let project = project_of(vec![book("B\u{e9}ta", "\u{c9}tude \u{2013} 1", vec![sheet("Feuille \u{e9}", vec![c])])]);
    let imp = read_back(&written(&project).bytes);
    assert_same_books(&project, &imp);
}

#[test]
fn text_outside_windows_1252_is_replaced_and_reported() {
    let c = column("A", Designation::Y, texts(&["\u{394}E", "ok"])).with_labels("\u{3a9} resistance", "", "");
    let project = project_of(vec![book("Greek", "", vec![sheet("S", vec![c])])]);
    let out = written(&project);
    assert!(out.warnings.iter().any(|w| w.contains("Windows-1252")), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    let col = &imp.workbooks[0].sheets[0].columns[0];
    assert_eq!(col.text().unwrap(), ["?E".to_string(), "ok".to_string()]);
    assert_eq!(col.long_name, "? resistance");
}

#[test]
fn sheets_get_their_own_data_set_suffix_and_keep_their_order() {
    let a = sheet("One", vec![column("A", Designation::X, nums(&[1.0])), column("B", Designation::Y, nums(&[2.0]))]);
    let b = sheet("Two", vec![column("A", Designation::X, nums(&[3.0]))]);
    let c = sheet("Three", vec![column("A", Designation::X, nums(&[4.0]))]);
    let project = project_of(vec![book("Multi", "", vec![a, b, c])]);
    let bytes = written(&project).bytes;
    let text = String::from_utf8_lossy(&bytes);
    for name in ["Multi_A\0", "Multi_B\0", "Multi_A@2\0", "Multi_A@3\0"] {
        assert!(text.contains(name), "{name}");
    }
    assert_same_books(&project, &read_back(&bytes));
}

#[test]
fn empty_cases_are_written() {
    let empty_sheet = sheet("Empty", Vec::new());
    let empty_columns =
        sheet("Columns", vec![column("A", Designation::X, nums(&[])), column("B", Designation::Y, texts(&[]))]);
    let project = project_of(vec![
        book("Nothing", "", vec![empty_sheet, empty_columns]),
        book("Blank", "", vec![sheet("S", vec![])]),
    ]);
    let imp = read_back(&written(&project).bytes);
    assert_eq!(imp.workbooks.len(), 1, "the reader skips workbooks without any column");
    assert_eq!(imp.workbooks[0].sheets.len(), 2);
    // a project with no windows at all is a valid file too
    let out = write_named(&Project::new(), "empty", TIME).unwrap();
    let _ = read_back(&out.bytes);
}

#[test]
fn text_cells_use_the_widest_text_and_tell_empty_from_present() {
    let project = project_of(vec![book(
        "T",
        "",
        vec![sheet("S", vec![column("A", Designation::Y, texts(&["", "x", "a longer piece of text", ""]))])],
    )]);
    let imp = read_back(&written(&project).bytes);
    let col = &imp.workbooks[0].sheets[0].columns[0];
    assert_eq!(col.text().unwrap(), ["", "x", "a longer piece of text", ""]);
}

#[test]
fn a_column_after_a_gap_remembers_where_its_values_start() {
    let project = project_of(vec![book(
        "Gap",
        "",
        vec![sheet(
            "S",
            vec![
                column("A", Designation::X, nums(&[f64::NAN, f64::NAN, 5.0, 6.0])),
                column("B", Designation::Y, nums(&[1.0])),
            ],
        )],
    )]);
    let bytes = written(&project).bytes;
    assert_same_books(&project, &read_back(&bytes));
}

#[test]
fn the_roles_of_columns_map_to_the_codes_origin_uses() {
    let codes: Vec<u8> = Designation::ALL.iter().map(|&d| designation_code(d)).collect();
    assert_eq!(codes, [3, 0, 5, 2, 6, 4, 1]);
}

#[test]
fn what_cannot_be_written_is_refused_or_reported() {
    let reason = |p: &Project| write_named(p, "p", TIME).unwrap_err().to_string();
    let one = |name: &str, col: &str, sheet_name: &str| {
        project_of(vec![book(name, "", vec![sheet(sheet_name, vec![column(col, Designation::Y, nums(&[1.0]))])])])
    };
    assert!(reason(&one("Book1", "A@1", "S")).contains('@'));
    assert!(reason(&one("Book1", "", "S")).contains("empty"));
    assert!(reason(&one("Book1", "\u{394}", "S")).contains("Windows-1252"));
    assert!(reason(&one("Book1", "A", "")).contains("empty"));
    assert!(reason(&one("A_very_long_workbook", "a_long_column", "S")).contains("too long"));
    assert!(reason(&one("Book1", "A\tB", "S")).contains("control"));
    assert!(reason(&one("Book1", "A", &"s".repeat(40))).contains("longer"));
    // two windows or sheets or columns with one name
    let twice = project_of(vec![book("Same", "", vec![]), book("Same", "", vec![])]);
    assert_eq!(twice.workbooks().count(), 2, "adopt_named_workbook renames, so build the clash by hand");
    let mut clash = twice.clone();
    for w in &mut clash.windows {
        if let Window::Workbook(b) = w {
            b.name = "Same".into();
        }
    }
    assert!(reason(&clash).contains("two windows"));
    let two_sheets = project_of(vec![book("B", "", vec![sheet("S", vec![]), sheet("s", vec![])])]);
    assert!(reason(&two_sheets).contains("two sheets"));
    let two_columns = project_of(vec![book(
        "B",
        "",
        vec![sheet("S", vec![column("A", Designation::X, nums(&[])), column("a", Designation::Y, nums(&[]))])],
    )]);
    assert!(reason(&two_columns).contains("two columns"));
    // text that does not fit a cell, and labels that would be read as storage
    let long_cell = project_of(vec![book(
        "B",
        "",
        vec![sheet("S", vec![column("A", Designation::Y, texts(&[&"x".repeat(300)]))])],
    )]);
    assert!(reason(&long_cell).contains("at most 252"));
    let with_nul =
        project_of(vec![book("B", "", vec![sheet("S", vec![column("A", Designation::Y, texts(&["a\0b"]))])])]);
    assert!(reason(&with_nul).contains("NUL"));
    let storage = project_of(vec![book(
        "B",
        "",
        vec![sheet("S", vec![column("A", Designation::Y, nums(&[])).with_labels("x@${y}", "", "")])],
    )]);
    assert!(reason(&storage).contains("stored data"));
}

#[test]
fn line_breaks_are_reported_and_graphs_are_written() {
    let mut project = sample_project();
    project.add_graph(sheaf_core::template::Template::Line);
    if let Some(Window::Workbook(b)) = project.windows.first_mut() {
        b.sheets[0].columns[0].formula = Some("col(B)*2".into());
        b.sheets[0].columns[0].long_name = "two
lines"
            .into();
    }
    let out = written(&project);
    let all = out.warnings.join(" | ");
    assert!(all.contains("line breaks"), "{all}");
    assert!(!all.contains("formula") && !all.contains("folder"), "formulas and folders are stored: {all}");
    let imp = read_back(&out.bytes);
    assert_eq!(imp.workbooks.len(), 2);
    assert_eq!(imp.workbooks[0].sheets[0].columns[0].long_name, "two lines");
    assert_eq!(imp.workbooks[0].sheets[0].columns[0].formula.as_deref(), Some("col(B)*2"));
    // the empty graph is written, not skipped
    assert_eq!(imp.graphs.len(), 1);
}

#[test]
fn the_file_ends_with_the_project_tree_and_the_attribute_records() {
    let project = sample_project();
    let bytes = written(&project).bytes;
    // the eight attribute records are the last 8 * (28 + 1 + 1) bytes plus their bodies; walk back from the front
    let marker = [8u8, 0, 0, 0, b'\n', 0, 0x10, 0, 0, 8, 0, 0, 0, b'\n'];
    let at = bytes.windows(marker.len()).rposition(|w| w == marker).expect("the tree ends with its marker block");
    let mut i = at + marker.len();
    let mut indices = Vec::new();
    while i < bytes.len() {
        let head = &bytes[i..i + 28];
        assert_eq!(u32::from_le_bytes(head[..4].try_into().unwrap()), 0x1000);
        indices.push(u32::from_le_bytes(head[4..8].try_into().unwrap()));
        let size = u32::from_le_bytes(head[8..12].try_into().unwrap()) as usize;
        assert_eq!(bytes[i + 28], b'\n');
        assert_eq!(bytes[i + 29 + size], b'\n');
        i += 30 + size;
    }
    assert_eq!(i, bytes.len());
    assert_eq!(indices, (0..8).collect::<Vec<u32>>());
    // the folder lists both windows
    let text = String::from_utf8_lossy(&bytes[..at]);
    assert!(text.contains("sample\0"));
    assert!(text.contains("ResultsLog\0"));
}

#[test]
fn the_size_in_front_of_the_tree_matches_what_follows() {
    for n in [0usize, 1, 2, 5] {
        let root = tail::FolderOut { name: "p".into(), windows: (0..n as u32).collect(), folders: Vec::new() };
        let tail = tail::tail(&tail::TailSpec { root: &root, dependencies: &[], time: TIME });
        // skip "\0\n", the empty block, the note (three blocks) and the empty block that closes the notes
        let mut i = 2 + 5;
        for _ in 0..3 {
            let size = u32::from_le_bytes(tail[i..i + 4].try_into().unwrap()) as usize;
            i += 5 + size + 1;
        }
        i += 5;
        assert_eq!(u32::from_le_bytes(tail[i..i + 4].try_into().unwrap()), 4);
        let size = u32::from_le_bytes(tail[i + 5..i + 9].try_into().unwrap()) as usize;
        let after = i + 10;
        let marker_end =
            tail.windows(14).rposition(|w| w == [8, 0, 0, 0, b'\n', 0, 0x10, 0, 0, 8, 0, 0, 0, b'\n']).unwrap() + 14;
        assert_eq!(marker_end - after - 4, size, "{n} windows");
    }
}

#[test]
fn the_writer_is_deterministic_and_reads_through_the_public_functions() {
    let project = sample_project();
    assert_eq!(written(&project).bytes, written(&project).bytes);
    let dir = std::env::temp_dir().join(format!("sheaf-opj-write-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Lab data.opj");
    let warnings = write_file(&project, &path).unwrap();
    assert!(warnings.is_empty());
    let imp = crate::opj::read_opj(&path).unwrap();
    assert_same_books(&project, &imp);
    // the project is named after the file; and into_project gives a usable project again
    let bytes = std::fs::read(&path).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("Lab data\0"));
    assert_eq!(imp.into_project().workbooks().count(), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn damaged_copies_of_a_written_file_never_panic_the_reader() {
    let good = written(&sample_project()).bytes;
    for n in (0..good.len()).step_by(11) {
        let _ = read_opj_from(Cursor::new(good[..n].to_vec()), Path::new("x.opj"));
    }
    for at in (0..good.len()).step_by(17) {
        let mut bad = good.clone();
        bad[at] ^= 0xFF;
        let _ = read_opj_from(Cursor::new(bad), Path::new("x.opj"));
    }
}

/// The fixture `tests/fixtures/sheaf-written.opj` is what this writer makes of [`sample_project`]
/// at a fixed time. It was never written by Origin. Set `SHEAF_UPDATE_FIXTURE=1` to rewrite it.
#[test]
fn the_fixture_is_what_the_writer_makes_and_reads_back() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sheaf-written.opj");
    let bytes = write_named(&sample_project(), "sheaf-written", TIME).unwrap().bytes;
    if std::env::var_os("SHEAF_UPDATE_FIXTURE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
    }
    let on_disk = std::fs::read(&path).expect("run the test once with SHEAF_UPDATE_FIXTURE=1 to create the fixture");
    assert_eq!(
        on_disk, bytes,
        "the writer changed; rewrite the fixture with SHEAF_UPDATE_FIXTURE=1 and re-check it in Origin"
    );
    assert_same_books(&sample_project(), &read_back(&on_disk));
}

#[test]
fn label_columns_without_a_y_column_are_written_as_unused() {
    // Origin opens such a label column as unused (checked in Origin), so the writer says so
    let only_labels = sheet("S", vec![column("A", Designation::Label, texts(&["a", "b"]))]);
    let x_label = sheet(
        "T",
        vec![column("A", Designation::X, nums(&[1.0, 2.0])), column("B", Designation::Label, texts(&["a", "b"]))],
    );
    let out = written(&project_of(vec![book("B", "", vec![only_labels, x_label])]));
    assert!(out.warnings.iter().any(|w| w.contains("2 label column(s)")), "{:?}", out.warnings);
    let imp = read_back(&out.bytes);
    let sheets = &imp.workbooks[0].sheets;
    assert_eq!(sheets[0].columns[0].designation, Designation::Disregard);
    assert_eq!(sheets[1].columns[1].designation, Designation::Disregard);
    // with a Y column in the sheet the label stays a label, without a warning
    let with_y =
        sheet("S", vec![column("A", Designation::Label, texts(&["a"])), column("B", Designation::Y, nums(&[1.0]))]);
    let out = written(&project_of(vec![book("B", "", vec![with_y])]));
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert_eq!(read_back(&out.bytes).workbooks[0].sheets[0].columns[0].designation, Designation::Label);
}

/// An analysis result that recalculates from its source is not stored in the file, so the writer
/// names it (and the save keeps the `.opj.sheaf` companion, ADR 0008).
#[test]
fn analysis_links_are_named_in_the_warnings() {
    use sheaf_core::Designation;
    use sheaf_core::analysis::{OutputTarget, Request, RowRange, Tool};
    use sheaf_core::column::RecalcMode;
    let mut p = Project::new();
    let book = p.add_workbook("Book1");
    p.add_column(book, 0, Designation::X, ColumnData::numeric((0..20).map(f64::from).collect())).unwrap();
    let y = p
        .add_column(book, 0, Designation::Y, ColumnData::numeric((0..20).map(|i| f64::from(i % 4)).collect()))
        .unwrap();
    assert!(write(&p).unwrap().warnings.iter().all(|w| !w.contains("analysis result")));
    use sheaf_core::analysis::{Boundary, SmoothMethod, SmoothOptions};
    let smooth = SmoothOptions {
        method: SmoothMethod::AdjacentAveraging { weighted: false },
        npts: 3,
        boundary: Boundary::None,
    };
    let tool = Tool::Smooth(smooth);
    p.analyze_linked(
        &Request { columns: vec![y], rows: RowRange::All, tool },
        &OutputTarget::NextToInput,
        RecalcMode::Manual,
    )
    .unwrap();
    let out = write(&p).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("1 analysis result")), "{:?}", out.warnings);
}
