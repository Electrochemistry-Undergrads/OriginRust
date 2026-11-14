//! Prints what the `.opju` reader finds in the files of a folder (or in one file).
//!
//! `cargo run -p sheaf-io --example opju_dump -- <folder or file> [--cells]`

use sheaf_io::opju;
use std::path::{Path, PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let target = PathBuf::from(args.next().expect("a folder or an .opju file"));
    let cells = args.any(|a| a == "--cells");
    let mut files: Vec<PathBuf> = if target.is_dir() {
        std::fs::read_dir(&target)
            .expect("folder")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("opju")))
            .collect()
    } else {
        vec![target.clone()]
    };
    files.sort();
    for f in files {
        report(&f, cells);
    }
}

fn report(path: &Path, cells: bool) {
    let bytes = std::fs::read(path).expect("read");
    let s = opju::summarize(&bytes);
    println!("{}", path.file_name().unwrap().to_string_lossy());
    println!(
        "  windows {} (workbooks {} + {} rebuilt, matrices {}, graphs/other {}); column records {}, read {}, unmatched {}, skipped {:?}",
        s.windows,
        s.workbook_windows,
        s.windowless_workbooks,
        s.matrix_windows,
        s.graph_windows,
        s.column_records,
        s.columns_read,
        s.columns_unmatched,
        s.columns_skipped
    );
    println!("  cut columns {} (values dropped {})", s.columns_cut, s.values_dropped);
    if cells {
        for e in &s.examples {
            println!("    {e}");
        }
    }
    match opju::read_opju_bytes(&bytes, path) {
        Ok(imp) => {
            for wb in &imp.workbooks {
                let cols: usize = wb.sheets.iter().map(|s| s.columns.len()).sum();
                println!("  book {} [{}] sheets {} columns {}", wb.name, wb.long_name, wb.sheets.len(), cols);
                if cells {
                    for sh in &wb.sheets {
                        for c in &sh.columns {
                            println!("    {}.{} len {}", sh.name, c.short_name, c.len());
                            if let Some(want) = std::env::var_os("OPJU_VALUES")
                                && c.short_name == want.to_string_lossy()
                                && let Some(v) = c.numeric()
                            {
                                println!("      {:?}", &v[..v.len().min(60)]);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => println!("  ERROR {e}"),
    }
}
