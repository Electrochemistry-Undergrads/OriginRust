//! Prints the structure of an Origin project file: workbooks, sheets and columns with
//! short name, long name, units, comments, role, type, row count and the first values.
//!
//! Usage: `cargo run -p sheaf-io --example opj_dump -- <file.opj> [--values N] [--summary]`

use sheaf_core::column::ColumnData;
use sheaf_io::opj::read_opj;
use std::path::Path;

fn main() {
    let mut path = None;
    let mut values = 3usize;
    let mut summary = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--values" => values = args.next().and_then(|n| n.parse().ok()).unwrap_or(3),
            "--summary" => summary = true,
            _ => path = Some(a),
        }
    }
    let Some(path) = path else {
        eprintln!("usage: opj_dump <file.opj> [--values N] [--summary]");
        std::process::exit(2);
    };
    let started = std::time::Instant::now();
    let imp = match read_opj(Path::new(&path)) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    println!("{path}");
    println!("signature: {}   origin version: {}   read in {:.2?}", imp.signature, imp.version, started.elapsed());
    for w in &imp.workbooks {
        println!("workbook '{}'  long name '{}'  sheets {}", w.name, w.long_name, w.sheets.len());
        for s in &w.sheets {
            println!("  sheet '{}'  columns {}  rows {}", s.name, s.columns.len(), s.row_count());
            if summary {
                continue;
            }
            for c in &s.columns {
                let (kind, missing, first) = match &c.data {
                    ColumnData::Numeric(v) => (
                        "number",
                        v.iter().filter(|x| x.is_nan()).count(),
                        v.iter()
                            .take(values)
                            .map(|x| if x.is_nan() { "--".to_string() } else { format!("{x}") })
                            .collect::<Vec<_>>(),
                    ),
                    ColumnData::Text(v) => (
                        "text",
                        v.iter().filter(|x| x.is_empty()).count(),
                        v.iter().take(values).map(|x| format!("{x:?}")).collect(),
                    ),
                };
                println!(
                    "    {:<8} long '{}' units '{}' comments '{}' role {:?} {} rows {} missing {} first [{}]",
                    c.short_name,
                    c.long_name,
                    c.units,
                    c.comments.replace(['\r', '\n'], " "),
                    c.designation,
                    kind,
                    c.len(),
                    missing,
                    first.join(", ")
                );
            }
        }
    }
    for w in &imp.warnings {
        println!("warning: {w}");
    }
}
