//! Prints the structure of an Origin project file: workbooks, sheets and columns with
//! short name, long name, units, comments, role, type, row count and the first values.
//!
//! Usage: `cargo run -p sheaf-io --example opj_dump -- <file.opj> [--values N] [--summary] [--cells]`
//!
//! With `--cells` it prints instead a tab-separated listing in the same format that
//! `tools/origin-ref/opj-dump-cells.ps1` writes for Origin itself, so the two can be compared
//! line by line (`tools/origin-ref/opj-compare-cells.py`):
//!
//! ```text
//! BOOK  <window name>  <long name>
//! SHEET <sheet name>
//! COL   <index> <short name> <long name> <units> <comments> <designation> <used rows>
//! CELL  <index> <row> <N number | T text | M missing> <value>
//! ```
//!
//! Row and column indices start at 0. "Used rows" is the position of the last cell that is not
//! missing, plus one. The sampled rows are the first three, the last three and twenty spread
//! evenly over the used rows. In text, backslash, tab, line feed and carriage return are written
//! as a backslash followed by `\`, `t`, `n` or `r`. Numbers print with the shortest text that
//! reads back as the same double.

use sheaf_core::column::ColumnData;
use sheaf_io::opj::read_opj;
use std::path::Path;

fn main() {
    let mut path = None;
    let mut values = 3usize;
    let mut summary = false;
    let mut cells = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--values" => values = args.next().and_then(|n| n.parse().ok()).unwrap_or(3),
            "--summary" => summary = true,
            "--cells" => cells = true,
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
    if cells {
        print_cells(&imp);
        return;
    }
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

/// Rows to print for a column with `used` used rows: first 3, last 3 and 20 spread evenly.
fn sample_rows(used: usize) -> Vec<usize> {
    let mut rows = std::collections::BTreeSet::new();
    rows.extend(0..3.min(used));
    rows.extend(used.saturating_sub(3)..used);
    if used > 0 {
        rows.extend((0..20usize).map(|k| k * (used - 1) / 19));
    }
    rows.into_iter().collect()
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

fn print_cells(imp: &sheaf_io::opj::OpjImport) {
    for w in &imp.workbooks {
        println!("BOOK\t{}\t{}", escape(&w.name), escape(&w.long_name));
        for s in &w.sheets {
            println!("SHEET\t{}", escape(&s.name));
            for (i, c) in s.columns.iter().enumerate() {
                let used = match &c.data {
                    ColumnData::Numeric(v) => v.iter().rposition(|x| !x.is_nan()).map_or(0, |p| p + 1),
                    ColumnData::Text(v) => v.iter().rposition(|t| !t.is_empty()).map_or(0, |p| p + 1),
                };
                println!(
                    "COL\t{i}\t{}\t{}\t{}\t{}\t{:?}\t{used}",
                    escape(&c.short_name),
                    escape(&c.long_name),
                    escape(&c.units),
                    escape(&c.comments),
                    c.designation
                );
                for r in sample_rows(used) {
                    match &c.data {
                        ColumnData::Numeric(v) if v[r].is_nan() => println!("CELL\t{i}\t{r}\tM\t"),
                        ColumnData::Numeric(v) => println!("CELL\t{i}\t{r}\tN\t{:?}", v[r]),
                        ColumnData::Text(v) if v[r].is_empty() => println!("CELL\t{i}\t{r}\tM\t"),
                        ColumnData::Text(v) => println!("CELL\t{i}\t{r}\tT\t{}", escape(&v[r])),
                    }
                }
            }
        }
    }
}
