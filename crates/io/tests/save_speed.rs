//! Save and load time of a large mixed project (T426, the "faster project save" target). Ignored by
//! default; run with `cargo test -p sheaf-io --test save_speed -- --ignored --nocapture`.

use sheaf_core::{ColumnData, Designation, Project};
use std::collections::BTreeMap;
use std::time::Instant;

/// 8 books of 3 sheets... kept simple: 8 books, 24 numeric columns of 50 000 rows each (measurement-like
/// values with noise), 4 text columns and 4 Text & Numeric columns per book.
fn big_project() -> Project {
    let mut p = Project::new();
    let rows = 50_000;
    for b in 0..8 {
        let book = p.add_workbook(&format!("Book{b}"));
        for c in 0..24 {
            let v: Vec<f64> = (0..rows)
                .map(|i| {
                    let x = i as f64 * 0.001;
                    (x * (c + 1) as f64).sin() * 1.5 + 0.001 * ((i * 7919 + c * 104_729) % 1000) as f64
                })
                .collect();
            p.add_column(book, 0, if c == 0 { Designation::X } else { Designation::Y }, ColumnData::numeric(v))
                .unwrap();
        }
        for c in 0..4 {
            let t: Vec<String> = (0..rows / 10).map(|i| format!("sample {b}-{c}-{i}")).collect();
            p.add_column(book, 0, Designation::Label, ColumnData::text(t)).unwrap();
        }
        for c in 0..4 {
            let numbers: Vec<f64> = (0..rows).map(|i| (i * (c + 3)) as f64 * 0.5).collect();
            let texts: BTreeMap<usize, String> = (0..rows).step_by(97).map(|i| (i, "n.d.".to_string())).collect();
            p.add_column(book, 0, Designation::Y, ColumnData::mixed(numbers, texts)).unwrap();
        }
    }
    p
}

#[test]
#[ignore = "timing; run by hand"]
fn save_and_load_time_of_a_large_mixed_project() {
    let p = big_project();
    let dir = std::env::temp_dir().join(format!("sheaf-save-speed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.sheaf");
    let mut times = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        sheaf_io::project_file::save(&p, &path).unwrap();
        times.push(t.elapsed().as_secs_f64());
    }
    let size = std::fs::metadata(&path).unwrap().len();
    let t = Instant::now();
    let back = sheaf_io::project_file::load(&path).unwrap();
    let load = t.elapsed().as_secs_f64();
    assert_eq!(back.workbooks().count(), 8);
    println!(
        "save {:.3} s (best of 3: {:?}), load {load:.3} s, file {:.1} MB",
        times.iter().copied().fold(f64::INFINITY, f64::min),
        times.iter().map(|t| format!("{t:.3}")).collect::<Vec<_>>(),
        size as f64 / 1e6
    );
    let _ = std::fs::remove_dir_all(&dir);
}
