//! Simulate Curve against Origin 2016's Simulate Curve window (fitting.md 13.1).
//!
//! `tests/data/origin_simulate_gauss.tsv` is the sheet Origin wrote for Gauss with the window's
//! defaults (probe `re-workspace/scratch/t47/d6`, local only).

use sheaf_fit::library;
use sheaf_fit::simulate::{Spacing, default_range, initial_params, simulate, x_values};

#[test]
fn gauss_with_origins_window_defaults_gives_origins_sheet() {
    let rows: Vec<(f64, f64)> = include_str!("data/origin_simulate_gauss.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let mut f = l.split('\t').map(|v| v.parse::<f64>().unwrap());
            (f.next().unwrap(), f.next().unwrap())
        })
        .collect();
    let gauss = library::get("Gauss").unwrap();
    let params = initial_params(&gauss);
    assert_eq!(params, [1.0, 2.0, 1.5, 5.0], "Origin's y0, xc, w, A");
    let (from, to, points) = default_range(&gauss);
    assert_eq!((from, to, points), (-1.0, 5.0, 200));
    let xs = x_values(from, to, points, Spacing::Linear).unwrap();
    let ys = simulate(&gauss, &params, &xs).unwrap();
    assert_eq!(rows.len(), xs.len());
    for (i, (&(ox, oy), (x, y))) in rows.iter().zip(xs.iter().zip(&ys)).enumerate() {
        assert!((x - ox).abs() <= 1e-14 * ox.abs().max(1.0), "row {i}: X {x} against Origin's {ox}");
        assert!((y - oy).abs() <= 1e-12 * oy.abs().max(1.0), "row {i}: Y {y} against Origin's {oy}");
    }
}
