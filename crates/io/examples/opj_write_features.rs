//! Writes sample projects with column formulas and with nested folders as Origin project files,
//! for checking them in Origin itself with `tools/origin-ref/opj-verify-features.ps1`.
//!
//! Usage: `cargo run -p sheaf-io --example opj_write_features -- <output folder>`
//!
//! - `formulas.opj`: formula columns of every recalculate mode (None, Auto, Manual), a formula
//!   that reads two columns, one that fills only some rows, one that reads another formula column,
//!   formulas in a second sheet and in a second workbook.
//! - `many.opj`: twelve workbooks, each with one Auto formula column (the case in which Origin
//!   ignored the table when the numbers of the objects were too small, spec section 13.10).
//! - `folders.opj`: six workbooks in the root and in folders nested three levels deep, a folder
//!   with two windows and an empty folder.
//! - `deep.opj`: folders nested twelve levels deep with a window in the deepest, and a folder with
//!   a name of 100 characters holding another window.

use sheaf_core::Project;
use sheaf_core::column::{Column, ColumnData, Designation, RecalcMode};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::project::Folder;
use sheaf_core::worksheet::{Workbook, Worksheet};
use std::ops::Range;
use std::path::Path;

fn column(short: &str, d: Designation, values: &[f64]) -> Column {
    Column::new(ColumnId(0), short, d, ColumnData::numeric(values.to_vec()))
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

/// Sets the formula of column `col` in sheet `sheet` of workbook `name`.
fn set(p: &mut Project, name: &str, sheet: usize, col: &str, f: &str, rows: Option<Range<usize>>, m: RecalcMode) {
    let b = p.workbooks().find(|b| b.name == name).expect("workbook");
    let id = b.sheets[sheet].columns.iter().find(|c| c.short_name == col).expect("column").id;
    p.set_column_values_with(id, f, rows, m).expect("formula");
}

fn formulas() -> Project {
    let zeros = [0.0; 5];
    let mut p = Project::new();
    p.adopt_named_workbook(book(
        "Book1",
        vec![
            (
                "Data",
                vec![
                    column("A", Designation::X, &[1.0, 2.0, 3.0, 4.0, 5.0]),
                    column("B", Designation::Y, &zeros),
                    column("C", Designation::Y, &zeros),
                    column("D", Designation::Y, &zeros),
                    column("E", Designation::Y, &[10.0, 20.0, 30.0, 40.0, 50.0]),
                    column("F", Designation::Y, &zeros),
                ],
            ),
            ("Second", vec![column("A", Designation::X, &[1.0, 2.0, 3.0]), column("B", Designation::Y, &[0.0; 3])]),
        ],
    ));
    p.adopt_named_workbook(book(
        "Book2",
        vec![("Sheet1", vec![column("A", Designation::X, &[1.0, 2.0, 3.0]), column("B", Designation::Y, &[0.0; 3])])],
    ));
    set(&mut p, "Book1", 0, "B", "col(A)*2", None, RecalcMode::Auto);
    set(&mut p, "Book1", 0, "C", "col(A)+col(B)", None, RecalcMode::Manual);
    set(&mut p, "Book1", 0, "D", "col(A)+1", None, RecalcMode::None);
    set(&mut p, "Book1", 0, "E", "col(A)*3", Some(1..4), RecalcMode::Auto);
    set(&mut p, "Book1", 0, "F", "col(B)-col(A)", None, RecalcMode::Auto);
    set(&mut p, "Book1", 1, "B", "col(A)*5", None, RecalcMode::Auto);
    set(&mut p, "Book2", 0, "B", "col(A)*7", None, RecalcMode::Manual);
    p
}

fn many(n: usize) -> Project {
    let mut p = Project::new();
    for k in 1..=n {
        let name = format!("Book{k}");
        let sheet = vec![column("A", Designation::X, &[1.0, 2.0, 3.0]), column("B", Designation::Y, &[0.0; 3])];
        p.adopt_named_workbook(book(&name, vec![("Sheet1", sheet)]));
        set(&mut p, &name, 0, "B", "col(A)*2", None, RecalcMode::Auto);
    }
    p
}

fn leaf(name: &str, windows: Vec<WindowId>) -> Folder {
    Folder { name: name.into(), windows, ..Folder::default() }
}

/// Workbooks `Book1` to `Book6` spread over folders: Book1 in the root, Book2 in F1, Book3 in
/// F1/F1a, Book4 in F1/F1a/F1b, Book5 and Book6 in F2, and an empty folder F3.
fn folders() -> Project {
    let mut p = Project::new();
    let ids: Vec<WindowId> = (1..=6)
        .map(|n| {
            let a = column("A", Designation::X, &[f64::from(n), f64::from(n) + 0.5]);
            p.adopt_named_workbook(book(&format!("Book{n}"), vec![("Sheet1", vec![a])]))
        })
        .collect();
    let mut f1a = leaf("F1a", vec![ids[2]]);
    f1a.folders.push(leaf("F1b", vec![ids[3]]));
    let mut f1 = leaf("F1", vec![ids[1]]);
    f1.folders.push(f1a);
    p.root.folders = vec![f1, leaf("F2", vec![ids[4], ids[5]]), leaf("F3", Vec::new())];
    p
}

fn deep() -> Project {
    let mut p = Project::new();
    let id = p.adopt_named_workbook(book("Deep1", vec![("Sheet1", vec![column("A", Designation::X, &[1.0, 2.0])])]));
    let top = p.adopt_named_workbook(book("Top1", vec![("Sheet1", vec![column("A", Designation::X, &[3.0])])]));
    let mut inner = leaf("Level 12 with a long name of 45 characters", vec![id]);
    for level in (1..12).rev() {
        let mut f = leaf(&format!("Level {level}"), Vec::new());
        f.folders.push(inner);
        inner = f;
    }
    p.root.folders = vec![inner, leaf(&"x".repeat(100), vec![top])];
    p
}

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: opj_write_features <output folder>");
        std::process::exit(2);
    };
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("create the output folder");
    let all = [("formulas", formulas()), ("many", many(12)), ("folders", folders()), ("deep", deep())];
    for (name, project) in all {
        let path = dir.join(format!("{name}.opj"));
        match sheaf_io::opj::write_file(&project, &path) {
            Ok(warnings) => {
                println!("{} written, {} warning(s)", path.display(), warnings.len());
                for w in warnings {
                    println!("  {w}");
                }
            }
            Err(e) => {
                eprintln!("{}: {e}", path.display());
                std::process::exit(1);
            }
        }
    }
}
