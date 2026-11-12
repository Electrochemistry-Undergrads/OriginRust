use super::testutil::{FileBuilder, number_cell, text_cell};
use super::*;

#[test]
fn into_project_keeps_window_names_and_drops_trailing_missing_cells() {
    use sheaf_core::{Column, ColumnData, ColumnId, Designation, WindowId, Workbook};
    let mut book = Workbook::new(WindowId(0), "Rates");
    let cols = &mut book.sheets[0].columns;
    cols.push(Column::new(
        ColumnId(1),
        "A",
        Designation::X,
        ColumnData::numeric(vec![1.0, f64::NAN, 3.0, f64::NAN, f64::NAN]),
    ));
    cols.push(Column::new(ColumnId(2), "B", Designation::Y, ColumnData::text(vec!["a".into(), String::new()])));
    cols.push(Column::new(ColumnId(3), "C", Designation::Y, ColumnData::numeric(vec![f64::NAN; 32])));
    let import = OpjImport {
        workbooks: vec![book],
        graphs: Vec::new(),
        folders: Vec::new(),
        version: "9.3".into(),
        signature: String::new(),
        warnings: Vec::new(),
    };
    let project = import.into_project();
    let b = project.workbooks().next().unwrap();
    assert_eq!(b.name, "Rates");
    let lens: Vec<usize> = b.sheets[0].columns.iter().map(|c| c.len()).collect();
    assert_eq!(lens, [3, 1, 0], "a missing value inside the data stays; the padding at the end goes");
}
use sheaf_core::column::ColumnData;
use std::io::Cursor;

const MISSING_BYTES: f64 = dataset::MISSING;

fn read(bytes: Vec<u8>) -> Result<OpjImport, IoError> {
    read_opj_from(Cursor::new(bytes), Path::new("synthetic.opj"))
}

fn doubles(values: &[f64]) -> Vec<u8> {
    values.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn sample_file() -> Vec<u8> {
    let mut b = FileBuilder::default();
    // Book1: sheet one has X (plain doubles) and Y (mixed cells with a missing value) ...
    b.dataset("Book1_A", 0x5001, 8, &doubles(&[1.0, 2.0, 3.0]));
    let mut y = number_cell(10.5);
    y.extend(number_cell(MISSING_BYTES));
    y.extend(number_cell(-0.25));
    b.dataset("Book1_B", 0x6121, 10, &y);
    // ... and sheet two has a text column; its data-set name carries a sheet suffix.
    let mut t = text_cell("alpha", 12);
    t.extend(text_cell("", 12));
    t.extend(text_cell("gamma", 12));
    b.dataset("Book1_C@2", 0x6121, 12, &t);
    // A graph's curve points at a data set of Book1; a matrix and a function have none of ours.
    b.dataset("Other_A", 0x6001, 8, &doubles(&[5.0]));
    b.window(
        "Book1",
        "First book\r\nsecond line",
        &[
            ("Sheet1", &[(1, 3, "A", b""), (2, 0, "B", b"Current\r\nmA\r\nraw@${[0|3|X|1|1]}junk")]),
            ("Second", &[(3, 4, "C", b"")]),
        ],
    );
    b.window("Graph1", "", &[("Layer1", &[(2, 96, "", b"")])]);
    b.window("Other", "", &[("Sheet1", &[(4, 3, "A", b"")])]);
    b.build()
}

#[test]
fn reads_workbooks_sheets_columns_and_values() {
    let imp = read(sample_file()).unwrap();
    assert_eq!(imp.version, "9.3");
    assert_eq!(imp.signature, "CPYA 4.3168 226 W64 #");
    assert_eq!(imp.workbooks.len(), 2, "the graph is not a workbook");
    let wb = &imp.workbooks[0];
    assert_eq!((wb.name.as_str(), wb.long_name.as_str()), ("Book1", "First book"));
    assert_eq!(wb.source.as_ref().unwrap().importer, "opj");
    assert_eq!(wb.sheets.len(), 2);
    assert_eq!(wb.sheets[0].name, "Sheet1");
    assert_eq!(wb.sheets[1].name, "Second");

    let a = &wb.sheets[0].columns[0];
    assert_eq!((a.short_name.as_str(), a.designation), ("A", Designation::X));
    assert_eq!(a.numeric().unwrap(), &[1.0, 2.0, 3.0]);
    let b = &wb.sheets[0].columns[1];
    assert_eq!((b.short_name.as_str(), b.designation), ("B", Designation::Y));
    assert_eq!((b.long_name.as_str(), b.units.as_str(), b.comments.as_str()), ("Current", "mA", "raw"));
    let v = b.numeric().unwrap();
    assert_eq!((v[0], v[2]), (10.5, -0.25));
    assert!(v[1].is_nan(), "the missing marker becomes NaN");

    let c = &wb.sheets[1].columns[0];
    assert_eq!((c.short_name.as_str(), c.designation), ("C", Designation::Label));
    assert_eq!(c.text().unwrap(), &["alpha".to_string(), String::new(), "gamma".to_string()]);

    assert_eq!(imp.workbooks[1].name, "Other");
    let ids: Vec<u64> =
        imp.workbooks.iter().flat_map(|w| w.sheets.iter()).flat_map(|s| s.columns.iter()).map(|c| c.id.0).collect();
    let unique: HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "column ids are distinct placeholders");
    assert!(imp.warnings.iter().any(|w| w.contains("not workbooks")), "{:?}", imp.warnings);
}

#[test]
fn unsupported_columns_are_skipped_with_a_warning() {
    let mut b = FileBuilder::default();
    b.dataset("Book1_A", 0x6001, 8, &doubles(&[1.0]));
    b.dataset("Book1_B", 0x0201, 16, &[0u8; 32]);
    b.window("Book1", "", &[("S", &[(1, 3, "A", b""), (2, 0, "B", b"")])]);
    let imp = read(b.build()).unwrap();
    assert_eq!(imp.workbooks[0].sheets[0].columns.len(), 1);
    assert!(imp.warnings.iter().any(|w| w.contains("complex")), "{:?}", imp.warnings);
}

#[test]
fn a_column_record_pointing_nowhere_is_reported() {
    let mut b = FileBuilder::default();
    b.dataset("Book1_A", 0x6001, 8, &doubles(&[1.0]));
    b.window("Book1", "", &[("S", &[(1, 3, "A", b""), (9, 0, "B", b""), (0, 0, "C", b"")])]);
    let imp = read(b.build()).unwrap();
    assert_eq!(imp.workbooks[0].sheets[0].columns.len(), 1);
    assert!(imp.warnings.iter().any(|w| w.contains("could not be matched")), "{:?}", imp.warnings);
}

#[test]
fn without_a_usable_window_list_columns_are_grouped_by_name() {
    let mut b = FileBuilder::default();
    b.dataset("Run_A", 0x6001, 8, &doubles(&[1.0, 2.0]));
    b.dataset("Run_B", 0x6001, 8, &doubles(&[3.0, 4.0]));
    b.dataset("Run_A@2", 0x6001, 8, &doubles(&[5.0]));
    // no windows at all
    let imp = read(b.build()).unwrap();
    assert_eq!(imp.workbooks.len(), 1);
    assert_eq!(imp.workbooks[0].sheets.len(), 2);
    assert_eq!(imp.workbooks[0].sheets[0].columns[0].designation, Designation::X);
    assert_eq!(imp.workbooks[0].sheets[0].columns[1].short_name, "B");
    assert!(imp.warnings.iter().any(|w| w.contains("grouped by their stored names")));
}

#[test]
fn text_and_numbers_in_one_column_become_text_with_a_warning() {
    let mut b = FileBuilder::default();
    let mut cells = number_cell(1.5);
    cells.extend(text_cell("x", 10));
    b.dataset("Book1_A", 0x6121, 10, &cells);
    b.window("Book1", "", &[("S", &[(1, 0, "A", b"")])]);
    let imp = read(b.build()).unwrap();
    match &imp.workbooks[0].sheets[0].columns[0].data {
        ColumnData::Text(t) => assert_eq!(&t[..], &["1.5".to_string(), "x".to_string()]),
        ColumnData::Numeric(_) => panic!("expected text"),
    }
    assert!(imp.warnings.iter().any(|w| w.contains("both text and numbers")));
}

#[test]
fn other_files_are_refused_with_a_clear_message() {
    let e = read(b"PK\x03\x04 zip".to_vec()).unwrap_err().to_string();
    assert!(e.contains("CPYA"), "{e}");
    let e = read(b"CPYUA 5.0 #\n".to_vec()).unwrap_err().to_string();
    assert!(e.contains("Unicode"), "{e}");
    assert!(read(Vec::new()).is_err());
    let e = read_opj(Path::new("definitely/not/here.opj")).unwrap_err();
    assert!(matches!(e, IoError::Io { .. }));
}

#[test]
fn a_project_without_workbooks_is_read_but_says_so() {
    let mut b = FileBuilder::default();
    b.window("Graph1", "", &[("Layer1", &[])]);
    let imp = read(b.build()).unwrap();
    assert!(imp.workbooks.is_empty());
    assert!(imp.warnings.iter().any(|w| w.contains("no workbook data")), "{:?}", imp.warnings);
}

#[test]
fn truncated_and_random_input_never_panics() {
    let good = sample_file();
    // every prefix of a good file
    for n in (0..good.len()).step_by(7).chain([good.len() - 1]) {
        let _ = read(good[..n].to_vec());
    }
    // a good file with single bytes damaged
    for at in (0..good.len()).step_by(13) {
        for value in [0u8, 0xFF, b'\n'] {
            let mut bad = good.clone();
            bad[at] = value;
            let _ = read(bad);
        }
    }
    // pseudo-random bytes, with and without a valid signature
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for round in 0..300 {
        let len = (next() % 400) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        if round % 2 == 0 {
            let mut head = b"CPYA 4.3168 1 W64 #\n".to_vec();
            head.extend(bytes);
            bytes = head;
        }
        let _ = read(bytes);
    }
}

#[test]
fn huge_size_fields_do_not_allocate() {
    let mut v = b"CPYA 4.3168 226 W64 #\n".to_vec();
    v.extend_from_slice(&u32::MAX.to_le_bytes());
    v.push(b'\n');
    v.extend_from_slice(&[0; 50]);
    assert!(read(v).is_err());
}

/// Opens every project found in the repository root and in Origin's sample folder,
/// if there are any. Prints a summary; asserts only that parsing worked.
#[test]
fn local_projects_parse() {
    let mut files = Vec::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    collect(&root, false, &mut files);
    collect(Path::new(r"C:\Program Files\OriginLab\Origin2016\Samples"), true, &mut files);
    if files.is_empty() {
        eprintln!("opj: no local .opj files found; skipping");
        return;
    }
    files.sort();
    let mut failures = Vec::new();
    for path in &files {
        let started = std::time::Instant::now();
        match read_opj(path) {
            Ok(imp) => {
                let sheets: usize = imp.workbooks.iter().map(|w| w.sheets.len()).sum();
                let mut columns = 0usize;
                let mut rows = 0usize;
                let (mut cells, mut missing) = (0usize, 0usize);
                for c in imp.workbooks.iter().flat_map(|w| w.sheets.iter()).flat_map(|s| s.columns.iter()) {
                    columns += 1;
                    rows += c.len();
                    if let Some(v) = c.numeric() {
                        cells += v.len();
                        missing += v.iter().filter(|x| x.is_nan()).count();
                    }
                }
                eprintln!(
                    "opj: {:<60} v{:<5} workbooks {:>3} sheets {:>3} columns {:>4} cells {:>9} missing {:>5.1}% warnings {} ({:.2?})",
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                        .chars()
                        .take(60)
                        .collect::<String>(),
                    imp.version,
                    imp.workbooks.len(),
                    sheets,
                    columns,
                    rows,
                    if cells == 0 { 0.0 } else { 100.0 * missing as f64 / cells as f64 },
                    imp.warnings.len(),
                    started.elapsed()
                );
                // A few of Origin's own samples hold only graphs or matrices.
                let dataless = imp.warnings.iter().any(|w| w.contains("no workbook data"));
                if (imp.workbooks.is_empty() || columns == 0) && !dataless {
                    failures.push(format!("{}: no workbook with a column", path.display()));
                }
                if dataless {
                    eprintln!("opj:   (no workbook data in this project; accepted)");
                }
            }
            Err(e) => {
                eprintln!("opj: {} -> {e}", path.display());
                failures.push(format!("{}: {e}", path.display()));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

fn collect(dir: &Path, recurse: bool, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if recurse {
                collect(&p, true, out);
            }
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("opj")) {
            out.push(p);
        }
    }
}

/// The owner's figure project (git-ignored, only on his machine): its one graph is a column plot
/// with error bars whose data come from the workbook next to it.
#[test]
fn local_figure_project_gives_a_column_graph() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Final Project Figures.opj");
    if !path.exists() {
        eprintln!("opj: {} not found; skipping", path.display());
        return;
    }
    let project = read_opj(&path).unwrap().into_project();
    let graphs: Vec<_> = project.graphs().collect();
    assert_eq!(graphs.len(), 1);
    let layer = &graphs[0].layers[0];
    assert_eq!(layer.plots.len(), 1, "the error column is attached to the bars");
    let plot = &layer.plots[0];
    assert_eq!(plot.kind, sheaf_core::graph::PlotKind::Column);
    assert!(plot.y_error.is_some() && project.numeric(plot.y).is_some());
    assert_eq!((layer.y_axis.min, layer.y_axis.max), (Some(0.0), Some(3.25)));
    assert!(layer.labels.len() >= 6);
    // Origin 2016 reports a gap of 40 percent between the bars of this graph.
    assert_eq!(layer.bars.gap_pct, 40.0);
}

/// A project with one workbook `Book1` (two columns) and optionally one graph `Graph1` whose
/// curve points at the second column. Every layer carries six axis blocks in each group, as
/// Origin 8.x writes them for workbooks too.
fn project_with_axis_blocks(version: f64, with_graph: bool) -> Vec<u8> {
    let mut b = FileBuilder { version, ..FileBuilder::default() };
    b.dataset("Book1_A", 0x5001, 8, &doubles(&[1.0, 2.0, 3.0]));
    b.dataset("Book1_B", 0x5001, 8, &doubles(&[4.0, 5.0, 6.0]));
    b.window_with("Book1", "", &[("Sheet1", &[(1, 3, "A", b""), (2, 0, "B", b"")])], false, 6);
    if with_graph {
        b.window_with("Graph1", "", &[("Layer1", &[(2, 96, "", b"")])], true, 6);
    }
    b.build()
}

#[test]
fn a_pre_9_workbook_with_axis_blocks_is_not_read_as_a_graph() {
    let imp = read(project_with_axis_blocks(8.1, false)).unwrap();
    assert_eq!(imp.version, "8.1");
    assert_eq!(imp.workbooks.len(), 1);
    assert!(imp.graphs.is_empty(), "the workbook must not be read a second time as a graph");
    assert!(!imp.warnings.iter().any(|w| w.contains("Graph1")), "{:?}", imp.warnings);
    assert!(
        imp.warnings.iter().any(|w| w.contains("before version 9") && w.contains("missing")),
        "graphs the reader misses must not go unmentioned: {:?}",
        imp.warnings
    );
}

#[test]
fn a_pre_9_graph_is_read_with_a_warning_that_it_was_not_verified() {
    let imp = read(project_with_axis_blocks(8.1, true)).unwrap();
    assert_eq!(imp.workbooks.len(), 1);
    assert_eq!(imp.graphs.len(), 1);
    assert_eq!(imp.graphs[0].name, "Graph1");
    assert!(
        imp.warnings.iter().any(|w| w.contains("before version 9") && w.contains("not verified")),
        "{:?}",
        imp.warnings
    );
}

#[test]
fn a_9x_graph_is_read_without_the_version_warning() {
    let imp = read(project_with_axis_blocks(9.3, true)).unwrap();
    assert_eq!(imp.workbooks.len(), 1);
    assert_eq!(imp.graphs.len(), 1);
    assert_eq!(imp.graphs[0].layers[0].plots.len(), 1);
    assert!(!imp.warnings.iter().any(|w| w.contains("before version 9")), "{:?}", imp.warnings);
}

/// Origin's installed sample "Simple XY.opj" (Origin 8.1) holds one workbook and one graph, and
/// Origin itself reports one graph for it. Skipped when the sample is not installed.
#[test]
fn local_pre_9_sample_has_one_graph() {
    let path = Path::new(r"C:\Program Files\OriginLab\Origin2016\Samples\COM Server and Client\Simple XY.opj");
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("opj: sample Simple XY.opj not found; skipping");
        return;
    };
    let imp = read(bytes).unwrap();
    assert_eq!(imp.graphs.len(), 1);
    assert_eq!(imp.graphs[0].name, "Graph1");
    assert!(!imp.warnings.iter().any(|w| w.contains("plot type 0")), "{:?}", imp.warnings);
}
