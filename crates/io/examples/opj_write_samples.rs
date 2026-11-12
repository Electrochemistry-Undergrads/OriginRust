//! Writes sample projects as Origin project files, for checking them in Origin itself with
//! `tools/origin-ref/opj-verify-written.ps1`.
//!
//! Usage: `cargo run -p sheaf-io --example opj_write_samples -- <output folder>`
//!
//! - `kinds.opj`: every kind of column (X, Y, Z, error columns, label, unused; numbers, text,
//!   missing values), long names, units, comments, three sheets in one workbook, a second workbook.
//! - `wide.opj`: 200 rows and 36 columns over three workbooks, with gaps, text, and names with
//!   accents and symbols.
//! - `limits.opj`: the longest names and texts the writer accepts, and awkward values.
//! - `empty.opj` (no window at all) and `edge.opj` (a sheet without columns).
//! - `big.opj`: three columns of 100000 rows.

use sheaf_core::Project;
use sheaf_core::column::{Column, ColumnData, Designation};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::path::Path;

fn col(short: &str, d: Designation, data: ColumnData, long: &str, units: &str, comments: &str) -> Column {
    Column::new(ColumnId(0), short, d, data).with_labels(long, units, comments)
}

fn nums(v: Vec<f64>) -> ColumnData {
    ColumnData::numeric(v)
}

fn texts(v: Vec<String>) -> ColumnData {
    ColumnData::text(v)
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

fn project(books: Vec<Workbook>) -> Project {
    let mut p = Project::new();
    for b in books {
        p.adopt_named_workbook(b);
    }
    p
}

fn kinds() -> Project {
    let nan = f64::NAN;
    let first = sheet(
        "Data",
        vec![
            col("A", Designation::X, nums(vec![0.0, 0.25, 0.5, 0.75, 1.0]), "Potential", "V", "scan 1"),
            col("B", Designation::Y, nums(vec![1.5, nan, -2.25e-9, 3.0e12, 5.0]), "Current", "A", ""),
            col("C", Designation::YError, nums(vec![0.1; 5]), "", "A", ""),
            col("D", Designation::Z, nums(vec![1.0, 2.0, 3.0]), "Depth", "", "only three"),
            col(
                "E",
                Designation::Label,
                texts(["a", "", "caf\u{e9}", "5\u{b0}C", "\u{b5}m"].map(String::from).to_vec()),
                "Tag",
                "",
                "",
            ),
            col("F", Designation::Disregard, nums(vec![7.0]), "", "", "first line\nsecond line"),
            col("G", Designation::XError, nums(vec![0.5]), "x err", "V", ""),
        ],
    );
    let second = sheet(
        "Second run",
        vec![
            col("A", Designation::X, nums(vec![nan, 10.0, 20.0]), "", "", ""),
            col("B", Designation::Y, nums(vec![nan, 1.0, 2.0]), "Counts", "cps", "after a gap"),
        ],
    );
    let third = sheet(
        "Third",
        vec![
            col("X1", Designation::X, nums(vec![1.0, 2.0, 3.0, 4.0]), "", "s", ""),
            col("Y1", Designation::Y, nums(vec![0.5, 0.25, 0.125, nan]), "", "", ""),
            col("Y2", Designation::Y, nums(vec![nan, nan, 9.0, 8.0]), "", "", ""),
            col("E2", Designation::YError, nums(vec![nan, nan, 0.5, 0.5]), "", "", ""),
        ],
    );
    let notes =
        sheet("Sheet1", vec![col("A", Designation::Y, texts(vec!["only text".into(), "x".into()]), "", "", "")]);
    project(vec![book("Book1", "Cyclic voltammetry", vec![first, second, third]), book("Notes", "", vec![notes])])
}

fn wide() -> Project {
    let rows = 200usize;
    let mut books = Vec::new();
    for b in 0..3 {
        let mut columns = vec![col(
            "T",
            Designation::X,
            nums((0..rows).map(|i| i as f64 * 0.01 + b as f64).collect()),
            "Time",
            "s",
            "",
        )];
        for k in 0..11 {
            let data: Vec<f64> = (0..rows)
                .map(|i| match (i + k) % 11 {
                    4 => f64::NAN,
                    _ => ((i * (k + 1)) as f64 * 0.017).sin() * 10f64.powi(k as i32 - 5),
                })
                .collect();
            let name = format!("Y{k}");
            columns.push(col(&name, Designation::Y, nums(data), &format!("Signal {k}"), "\u{b5}A", "generated"));
        }
        let words: Vec<String> =
            (0..rows).map(|i| if i % 9 == 0 { String::new() } else { format!("r\u{e9}p {i} \u{b0}C") }).collect();
        columns.push(col("Tag", Designation::Label, texts(words), "Tag", "", ""));
        books.push(book(&format!("Run{b}"), &format!("Run number {b}"), vec![sheet("Sheet1", columns)]));
    }
    project(books)
}

fn limits() -> Project {
    // data-set name Window_Column@n is at most 24 bytes: 11 + 1 + 12
    let long_book = "LongBookName"; // 12
    let long_col = "ColumnName1"; // 11
    let sheet_name = "S".repeat(31);
    let text252 = "x".repeat(252);
    let values = vec![0.1, -0.0, f64::MIN_POSITIVE, 1.0 / 3.0, f64::MAX, f64::NAN, 1e-300, 123_456_789.012_345_6];
    let a = sheet(
        &sheet_name,
        vec![
            col(
                long_col,
                Designation::X,
                nums(values),
                "name with a long text ".repeat(8).trim_end(),
                "u",
                &"c".repeat(300),
            ),
            col(
                "Txt",
                Designation::Y,
                texts(vec![text252, "short".into(), String::new(), "\u{20ac} \u{e9}".into()]),
                "",
                "",
                "",
            ),
            col("with space", Designation::Y, nums(vec![1.0, 2.0]), "", "", ""),
        ],
    );
    let accent = sheet("Feuille \u{e9}", vec![col("\u{b5}x", Designation::Y, nums(vec![1.0, 2.0, 3.0]), "", "", "")]);
    let empty = sheet(
        "Empty",
        vec![col("A", Designation::X, nums(vec![]), "", "", ""), col("B", Designation::Y, nums(vec![]), "", "", "")],
    );
    project(vec![
        book(long_book, &"L".repeat(100), vec![a]),
        book("B\u{e9}ta", "accented window name", vec![accent]),
        book("Blank", "", vec![empty]),
    ])
}

fn big() -> Project {
    let rows = 100_000usize;
    let t: Vec<f64> = (0..rows).map(|i| i as f64 * 0.001).collect();
    let y: Vec<f64> = (0..rows).map(|i| if i % 1000 == 999 { f64::NAN } else { (i as f64 * 0.0123).cos() }).collect();
    let z: Vec<f64> = (0..rows).map(|i| (i % 977) as f64 * 1e-6).collect();
    project(vec![book(
        "Long",
        "100000 rows",
        vec![sheet(
            "Sheet1",
            vec![
                col("T", Designation::X, nums(t), "Time", "s", ""),
                col("Y", Designation::Y, nums(y), "Signal", "V", ""),
                col("Z", Designation::Y, nums(z), "Offset", "V", ""),
            ],
        )],
    )])
}

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: opj_write_samples <output folder>");
        std::process::exit(2);
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("the output folder can be made");
    let edge = || {
        project(vec![
            book(
                "NoColumns",
                "",
                vec![
                    sheet("Empty one", vec![]),
                    sheet("Full", vec![col("A", Designation::X, nums(vec![1.0, 2.0]), "", "", "")]),
                ],
            ),
            book("Plain", "", vec![sheet("Sheet1", vec![col("A", Designation::Y, nums(vec![3.0]), "", "", "")])]),
        ])
    };
    for (name, project) in [
        ("kinds", kinds()),
        ("wide", wide()),
        ("limits", limits()),
        ("empty", Project::new()),
        ("edge", edge()),
        ("big", big()),
    ] {
        let path = dir.join(format!("{name}.opj"));
        match sheaf_io::opj::write_file(&project, &path) {
            Ok(warnings) => {
                println!("wrote {}", path.display());
                for w in warnings {
                    println!("  warning: {w}");
                }
            }
            Err(e) => println!("{name}: {e}"),
        }
    }
}
