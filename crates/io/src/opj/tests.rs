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
        comments: Vec::new(),
        notes: Vec::new(),
        matrices: Vec::new(),
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
fn text_and_numbers_in_one_column_stay_a_text_and_numeric_column() {
    let mut b = FileBuilder::default();
    let mut cells = number_cell(1.5);
    cells.extend(text_cell("x", 10));
    b.dataset("Book1_A", 0x6121, 10, &cells);
    b.window("Book1", "", &[("S", &[(1, 0, "A", b"")])]);
    let imp = read(b.build()).unwrap();
    match &imp.workbooks[0].sheets[0].columns[0].data {
        ColumnData::Mixed { numbers, texts } => {
            assert_eq!(numbers[0], 1.5);
            assert!(numbers[1].is_nan());
            assert_eq!(texts[&1], "x");
        }
        _ => panic!("expected a text and numeric column"),
    }
    assert!(imp.warnings.iter().all(|w| !w.contains("both text and numbers")));
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
    if let Some(home) = std::env::var_os("ORIGIN_HOME") {
        collect(&Path::new(&home).join("Samples"), true, &mut files);
    }
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

#[test]
fn matrix_books_are_read_with_their_sheets_sizes_and_coordinates() {
    // T471: the layout measured in Origin 2016 (opj-format.md 13.16).
    let mut fb = FileBuilder::default();
    fb.dataset(
        "MBook1",
        0x6001,
        8,
        &doubles(&[101.0, 102.0, 103.0, 104.0, 201.0, 202.0, 203.0, 204.0, 301.0, 302.0, 303.0, 304.0]),
    )
    .dataset("MBook1@2", 0x6001, 8, &doubles(&[1.0, dataset::MISSING, 3.0, 4.0]))
    .dataset("Book1_A", 0x6021, 8, &doubles(&[1.0, 2.0]));
    fb.window("Book1", "", &[("Sheet1", &[(1, 3, "A", &[])])]);
    fb.matrix(
        "MBook1",
        "Surface",
        &[("MSheet1", 4, 3, &[]), ("Data", 2, 2, &[("X1", "11"), ("X2", "20"), ("Y1", "-0.5"), ("Y2", "1.5")])],
    );
    let imp = read(fb.build()).unwrap();
    assert_eq!(imp.workbooks.len(), 1);
    assert_eq!(imp.matrices.len(), 1);
    assert!(!imp.warnings.iter().any(|w| w.contains("not imported")), "{:?}", imp.warnings);
    let project = imp.into_project();
    let m = project.matrix_books().next().unwrap();
    assert_eq!((m.name.as_str(), m.long_name.as_str(), m.sheets.len()), ("MBook1", "Surface", 2));
    let s = &m.sheets[0];
    assert_eq!((s.name.as_str(), s.rows, s.cols), ("MSheet1", 3, 4));
    assert_eq!((s.x_from, s.x_to, s.y_from, s.y_to), (1.0, 10.0, 1.0, 10.0), "Origin's default range");
    assert_eq!((s.value(0, 1), s.value(2, 3)), (102.0, 304.0), "row by row, the first row first");
    let d = &m.sheets[1];
    assert_eq!((d.name.as_str(), d.rows, d.cols), ("Data", 2, 2));
    assert_eq!((d.x_from, d.x_to, d.y_from, d.y_to), (11.0, 20.0, -0.5, 1.5));
    assert!(d.value(0, 1).is_nan(), "Origin's missing value");
    assert_eq!(d.value(1, 1), 4.0);
}

#[test]
fn an_empty_matrix_has_missing_cells_and_no_fallback_workbook() {
    let mut fb = FileBuilder::default();
    fb.dataset("MBook2", 0x6001, 8, &doubles(&[5.0]));
    fb.matrix("MBook1", "", &[("MSheet1", 2, 2, &[])]).matrix("MBook2", "", &[("MSheet1", 1, 1, &[])]);
    let imp = read(fb.build()).unwrap();
    assert!(imp.workbooks.is_empty());
    assert!(imp.warnings.iter().all(|w| !w.contains("No workbook could be matched")), "{:?}", imp.warnings);
    assert_eq!(imp.matrices.len(), 2);
    assert!(imp.matrices[0].sheets[0].values().iter().all(|v| v.is_nan()));
    assert_eq!(imp.matrices[1].sheets[0].values(), [5.0]);
}

/// Origin 2016's own files with matrices (T471; `re-workspace/scratch/t471`, local only): `mj.opj` has
/// `MXA` (4 x 3, `i*100+j`, default coordinates) and `MXB` (long name `LongMXB`; a 2 x 3 sheet of `i+0.25`
/// whose third column is empty, and a 32 x 32 sheet of `1000*j`); `mh.opj` has Origin's sample matrix
/// `2D Gaussian.ogm` (10 x 10, sheet `Data`, X 11 to 20, Y 1 to 10); `m1.opj` a matrix with set coordinates.
#[test]
fn local_origin_files_give_their_matrices() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re-workspace/scratch/t471");
    if !dir.join("mj.opj").exists() || !dir.join("mh.opj").exists() {
        eprintln!("opj: {} not found; skipping", dir.display());
        return;
    }
    let p = read_opj(&dir.join("mj.opj")).unwrap().into_project();
    let a = p.matrix_books().find(|m| m.name == "MXA").unwrap();
    assert_eq!((a.sheets[0].rows, a.sheets[0].cols, a.sheets[0].value(1, 2)), (3, 4, 203.0));
    assert_eq!((a.sheets[0].x_from, a.sheets[0].x_to), (1.0, 10.0));
    let b = p.matrix_books().find(|m| m.name == "MXB").unwrap();
    assert_eq!((b.long_name.as_str(), b.sheets.len()), ("LongMXB", 2));
    assert_eq!((b.sheets[0].rows, b.sheets[0].cols, b.sheets[0].value(1, 0)), (2, 3, 2.25));
    assert!(b.sheets[0].value(0, 2).is_nan());
    assert_eq!((b.sheets[1].name.as_str(), b.sheets[1].rows, b.sheets[1].cols), ("MSheet2", 32, 32));
    assert_eq!(b.sheets[1].value(5, 31), 32000.0);
    assert_eq!(p.workbooks().count(), 2, "GSrc and Book1 (m2w's XYZ columns)");
    let p = read_opj(&dir.join("mh.opj")).unwrap().into_project();
    let g = p.matrix_books().find(|m| m.name == "MBook1").unwrap();
    let s = &g.sheets[0];
    assert_eq!((s.name.as_str(), s.rows, s.cols), ("Data", 10, 10));
    assert_eq!((s.x_from, s.x_to, s.y_from, s.y_to), (11.0, 20.0, 1.0, 10.0));
    // T473: `r.lname$`, `r.unit$`, `r.comment$` of a matrix range
    if let Ok(imp) = read_opj(&dir.join("ml.opj")) {
        let z = &imp.matrices[0].sheets[0].z;
        assert_eq!((z.long_name.as_str(), z.units.as_str(), z.comments.as_str()), ("ZQZLONG", "ZQZUNIT", "ZQZCOMM"));
    }
    // `matrix -ps XY 0.5 10 2.5 20` on a 4 x 3 matrix of `i*10+j`
    if let Ok(imp) = read_opj(&dir.join("m1.opj")) {
        let s = &imp.matrices[0].sheets[0];
        assert_eq!((s.rows, s.cols, s.x_from, s.x_to, s.y_from, s.y_to), (3, 4, 0.5, 2.5, 10.0, 20.0));
        assert_eq!((s.value(0, 0), s.value(2, 3)), (11.0, 34.0));
    }
}

/// Origin 2016's own file with every Date, Time, Month and Day of Week display (T474;
/// `re-workspace/scratch/t474/d1.opj`, local only): columns 1-20 Date subformats 1-20, 21-37 Time 1-17,
/// 38-40 Month 1-3, 41-43 Day of Week 1-3.
#[test]
fn local_origin_file_gives_date_and_time_displays() {
    use sheaf_core::datetime::{DatePattern, TimePattern};
    use sheaf_core::format::{NameStyle, NumberFormat as F};
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re-workspace/scratch/t474/d1.opj");
    if !path.exists() {
        eprintln!("opj: {} not found; skipping", path.display());
        return;
    }
    let imp = read_opj(&path).unwrap();
    assert!(imp.warnings.iter().all(|w| !w.contains("day numbers")), "{:?}", imp.warnings);
    let b = imp.workbooks.iter().find(|b| b.name == "Book1").unwrap();
    let f: Vec<F> = b.sheets[0].columns.iter().map(|c| c.format).collect();
    assert_eq!(f[0], F::Date(DatePattern::Mdy));
    assert_eq!(f[10], F::DateTime(DatePattern::Mdy, TimePattern::Hms));
    assert_eq!(f[19], F::Date(DatePattern::MdyDash));
    assert_eq!(f[20], F::Time(TimePattern::Hm));
    assert_eq!(f[36], F::Time(TimePattern::ElapsedDhmsMicro));
    assert_eq!((f[37], f[39]), (F::Month(NameStyle::Short), F::Month(NameStyle::Letter)));
    assert_eq!(f[41], F::Weekday(NameStyle::Full));
    assert_eq!(b.sheets[0].columns[0].numeric().unwrap()[0], 2_461_355.573_032_407_4);
}

/// Origin 2016's own file with two Notes windows (T470; `re-workspace/scratch/t470/n2.opj`, local only):
/// `MyNotes` and `Second`, both empty, listed in the root folder before the workbook `GSrc`.
#[test]
fn local_origin_file_gives_its_notes_windows() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re-workspace/scratch/t470/n2.opj");
    if !path.exists() {
        eprintln!("opj: {} not found; skipping", path.display());
        return;
    }
    let imp = read_opj(&path).unwrap();
    assert_eq!(imp.notes, [("MyNotes".to_string(), String::new()), ("Second".to_string(), String::new())]);
    assert!(imp.folders.is_empty());
    let project = imp.into_project();
    assert_eq!(project.notes().map(|n| n.name.as_str()).collect::<Vec<_>>(), ["MyNotes", "Second"]);
    assert_eq!(project.workbooks().count(), 1);
}

/// Origin 2016's own file with a sheet long name (T343; `re-workspace/scratch/t339/s.opj`, local only).
#[test]
fn local_origin_file_gives_the_sheet_long_name() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re-workspace/scratch/t339/s.opj");
    if !path.exists() {
        eprintln!("opj: {} not found; skipping", path.display());
        return;
    }
    let project = read_opj(&path).unwrap().into_project();
    let b = project.workbooks().find(|b| b.sheets.iter().any(|s| s.name == "Short1")).expect("the book");
    let s = b.sheets.iter().find(|s| s.name == "Short1").unwrap();
    assert_eq!(s.long_name, "QZLongSheet");
}

/// Origin 2016's own file with numeric displays (T337; `re-workspace/scratch/t337/f.opj`, local only).
#[test]
fn local_origin_file_gives_the_numeric_displays() {
    use sheaf_core::format::NumberFormat as F;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re-workspace/scratch/t337/f.opj");
    if !path.exists() {
        eprintln!("opj: {} not found; skipping", path.display());
        return;
    }
    let project = read_opj(&path).unwrap().into_project();
    let b = project.workbooks().find(|b| b.name == "Book1").unwrap();
    let formats: Vec<F> = b.sheets[0].columns.iter().map(|c| c.format).collect();
    assert_eq!(
        formats,
        [
            F::General,
            F::Decimal(3),
            F::Scientific(2),
            F::EngineeringAuto,
            F::Significant(4),
            F::DecimalGrouped(1),
            F::ScientificAuto,
            F::Decimal(0)
        ]
    );
}

/// Origin 2016's own files with sampling intervals (T303; `re-workspace/scratch/t303`, local only):
/// s1 sets B's interval to 3.5 / 0.125 beside the X column A, s2 sets 0 / 2 on B of a sheet without X.
#[test]
fn local_origin_files_give_the_sampling_intervals() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re-workspace/scratch/t303");
    let interval = |file: &str, book: &str, column: &str| {
        let project = read_opj(&dir.join(file)).unwrap().into_project();
        let b = project.workbooks().find(|b| b.name == book).unwrap();
        b.sheets[0].column_by_name(column).unwrap().sampling.as_ref().map(|s| (s.x0, s.inc))
    };
    if !dir.join("s1.opj").exists() || !dir.join("s2.opj").exists() {
        eprintln!("opj: {} not found; skipping", dir.display());
        return;
    }
    assert_eq!(interval("s1.opj", "SA", "B"), Some((3.5, 0.125)));
    assert_eq!(interval("s1.opj", "SA", "A"), None, "the X column");
    assert_eq!(interval("s2.opj", "SB", "B"), Some((0.0, 2.0)));
    assert_eq!(interval("s2.opj", "SB", "A"), None, "1 and 1 is no interval");
    assert_eq!(interval("s0.opj", "SA", "B"), None);
}

/// Origin 2016's own file with window comments (T283; made by `opj-make-from-lines.ps1` from
/// `re-workspace/scratch/t282/probe3.lines.txt`, local only).
#[test]
fn local_origin_file_gives_the_window_comments() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re-workspace/scratch/t282/wc3.opj");
    if !path.exists() {
        eprintln!("opj: {} not found; skipping", path.display());
        return;
    }
    let project = read_opj(&path).unwrap().into_project();
    let comments = |name: &str| {
        let w = project.windows.iter().find(|w| w.name() == name).unwrap_or_else(|| panic!("no window {name}"));
        project.window_comments(w.id()).to_string()
    };
    assert_eq!(comments("BKA"), "ZQXbookA");
    assert_eq!(comments("BKB"), "ZQXbookB");
    assert_eq!(comments("BKC"), "ZQXbookC line1");
    assert_eq!(comments("GY"), "ZQXg1");
    assert_eq!(comments("GX"), "ZQXg2");
    assert_eq!(comments("GSrc"), "");
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
    // Origin reports a gap of 40 percent between the bars of this graph.
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
    let Some(home) = std::env::var_os("ORIGIN_HOME") else {
        eprintln!("opj: ORIGIN_HOME not set; skipping installed sample");
        return;
    };
    let path = Path::new(&home).join("Samples/COM Server and Client/Simple XY.opj");
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("opj: sample Simple XY.opj not found; skipping");
        return;
    };
    let imp = read(bytes).unwrap();
    assert_eq!(imp.graphs.len(), 1);
    assert_eq!(imp.graphs[0].name, "Graph1");
    assert!(!imp.warnings.iter().any(|w| w.contains("plot type 0")), "{:?}", imp.warnings);
}

#[test]
fn into_project_renumbers_the_stack_offsets_with_the_columns() {
    use sheaf_core::graph::{PlotOffset, Stack};
    use sheaf_core::{Column, ColumnData, ColumnId, Designation, WindowId, Workbook};
    let mut book = Workbook::new(WindowId(0), "XRD");
    let cols = &mut book.sheets[0].columns;
    cols.push(Column::new(ColumnId(1), "A", Designation::X, ColumnData::numeric(vec![1.0, 2.0])));
    cols.push(Column::new(ColumnId(2), "B", Designation::Y, ColumnData::numeric(vec![3.0, 4.0])));
    cols.push(Column::new(ColumnId(3), "C", Designation::Y, ColumnData::numeric(vec![5.0, 6.0])));
    let mut graph = sheaf_core::template::Template::Line.new_graph(WindowId(0), "Stacked");
    let layer = &mut graph.layers[0];
    for y in [2, 3] {
        layer.plots.push(sheaf_core::graph::Plot {
            x: Some(ColumnId(1)),
            y: ColumnId(y),
            y_error: None,
            rows: None,
            kind: sheaf_core::graph::PlotKind::Line,
            line: layer.default_line.clone(),
            symbol: layer.default_symbol.clone(),
            label: None,
            visible: true,
            auto_color: true,
            group: None,
            x_error: None,
            y_base: None,
            error: sheaf_core::graph::ErrorStyle::default(),
            hist: Default::default(),
            ridge: Default::default(),
            hexbin: Default::default(),
            parallel: Default::default(),
            treemap: Default::default(),
            indexed: None,
            stat: None,
            sunburst: Default::default(),
            sankey: Default::default(),
            size_from: None,
            color_from: None,
            decor: Default::default(),
        });
    }
    layer.stack = Stack::Individual;
    layer.plot_offsets = vec![
        PlotOffset::new(ColumnId(2)),
        PlotOffset { y: 250.0, ..PlotOffset::new(ColumnId(3)) },
        // An entry whose column is not in the file is dropped.
        PlotOffset { y: 7.0, ..PlotOffset::new(ColumnId(99)) },
    ];
    let import = OpjImport {
        workbooks: vec![book],
        graphs: vec![graph],
        folders: Vec::new(),
        version: "9.3".into(),
        signature: String::new(),
        warnings: Vec::new(),
        comments: Vec::new(),
        notes: Vec::new(),
        matrices: Vec::new(),
    };
    let project = import.into_project();
    let layer = &project.graphs().next().unwrap().layers[0];
    let ys: Vec<ColumnId> = layer.plots.iter().map(|p| p.y).collect();
    assert_eq!(layer.plot_offsets.len(), 2);
    assert_eq!(layer.plot_offset(ys[1]).map(|o| o.y), Some(250.0), "the offset follows its curve's new id");
    assert_eq!(layer.plot_offset(ys[0]).map(|o| o.y), Some(0.0));
    assert_eq!(layer.stack, Stack::Individual);
}

/// The owner's XRD project (git-ignored, only on his machine): its stacked graphs keep Origin's
/// offsets, so the patterns are drawn apart and in the order of the labels.
#[test]
fn local_xrd_project_keeps_the_offsets_of_its_stacked_graphs() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../XRD of all dopants for Pittcon.opj");
    if !path.exists() {
        eprintln!("opj: {} not found; skipping", path.display());
        return;
    }
    let project = read_opj(&path).unwrap().into_project();
    let graph = project.graphs().find(|g| g.name == "AllDopantsXRD").expect("the graph is read");
    let layer = &graph.layers[0];
    assert_eq!(layer.stack, sheaf_core::graph::Stack::Individual);
    let offsets: Vec<f64> = layer.plots.iter().map(|p| layer.plot_offset(p.y).map_or(f64::NAN, |o| o.y)).collect();
    assert_eq!(offsets.len(), 6);
    assert_eq!(offsets[0], 0.0);
    assert!(offsets.windows(2).all(|w| w[1] > w[0]), "each curve above the one before: {offsets:?}");
    // A graph whose curves are not moved has no stack.
    let plain = project.graphs().find(|g| g.name == "LMOafter100cycle").expect("the graph is read");
    assert_eq!(plain.layers[0].stack, sheaf_core::graph::Stack::None);
    assert!(plain.layers[0].plot_offsets.is_empty());
}

#[test]
fn masked_cells_are_read_from_the_mask_block() {
    let mut b = FileBuilder::default();
    let values: Vec<u8> = (0..6).flat_map(|i| number_cell(f64::from(i))).collect();
    b.dataset_masked("Book1_A", 0x6121, 10, &values, &[1, 4]);
    b.dataset("Book1_B", 0x6121, 10, &values);
    b.window("Book1", "", &[("S", &[(1, 0, "A", b""), (2, 0, "B", b"")])]);
    let imp = read(b.build()).unwrap();
    let cols = &imp.workbooks[0].sheets[0].columns;
    assert_eq!(cols[0].mask.iter().copied().collect::<Vec<_>>(), vec![1, 4]);
    assert!(cols[1].mask.is_empty(), "an empty mask block masks nothing");
}

/// Projects saved by Origin 2016 with `wcellmask` (re-workspace/scratch/opjmask, local only):
/// `SHEAF_OPJ_MASK_DIR=<that folder> cargo test -p sheaf-io -- --ignored origin_written_masks`.
#[test]
#[ignore]
fn origin_written_masks() {
    let Ok(dir) = std::env::var("SHEAF_OPJ_MASK_DIR") else { return };
    let rows = |file: &str, book: &str, col: &str| -> Vec<usize> {
        let imp = read_opj(&std::path::Path::new(&dir).join(file)).unwrap();
        let b = imp.workbooks.iter().find(|b| b.name == book).unwrap();
        b.sheets[0].columns.iter().find(|c| c.short_name == col).unwrap().mask.iter().copied().collect()
    };
    assert!(rows("m0.opj", "GSrc", "B").is_empty());
    assert_eq!(rows("m1.opj", "GSrc", "B"), vec![2]);
    assert_eq!(rows("m2.opj", "GSrc", "B"), vec![0, 1, 4]);
    assert_eq!(rows("m2.opj", "GSrc", "C"), vec![20]);
    assert_eq!(rows("m3.opj", "GSrc", "B"), (2..10).collect::<Vec<_>>());
    assert_eq!(rows("m4.opj", "Book1", "A"), vec![0, 49]);
}
