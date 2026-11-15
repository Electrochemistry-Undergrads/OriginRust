//! Hexagonal binning against matplotlib (matplotlib-checked; none of these numbers come from Origin).
//!
//! The inputs and the counts are in `fixtures/mpl_hexbin.rs`, made by `tools/hexbin-ref.py` with
//! `Axes.hexbin(gridsize=(nx, ny), extent=data range, mincnt=1)`, which the script also checks against a
//! numpy brute force over the same lattice. matplotlib's lattice is Sheaf's with `nx + 1` centres in an even
//! row and a row spacing of `yrange / (2 ny)`, so the cases set Bins(nx + 1) along X and the frame aspect
//! that derives exactly that row spacing.

#[path = "fixtures/mpl_hexbin.rs"]
mod fixtures;

use fixtures::*;
use sheaf_calc::hexbin::{HexAxisSetup, HexSetup, HexStep, hexbin};

fn setup(c: &Case) -> HexSetup {
    HexSetup {
        x: HexAxisSetup { step: HexStep::Bins(c.nx + 1), ..Default::default() },
        y: HexAxisSetup::default(),
        derive_y_from_x: true,
        prefer_regular: true,
        // dy = sx * a * (yr / xr) * sqrt(3) / 2 = yr / (2 ny)  for  a = nx / (sqrt(3) ny).
        frame_aspect: c.nx as f64 / (3f64.sqrt() * c.ny as f64),
    }
}

#[test]
fn the_counts_match_matplotlib_hexbin_cell_by_cell() {
    assert!(CASES.len() >= 4);
    for c in CASES {
        let h = hexbin(c.x, c.y, &setup(c)).unwrap();
        let (ymin, ymax) = c.y.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| (a.min(v), b.max(v)));
        assert_eq!(h.lattice.rows, 2 * c.ny + 1, "{} rows", c.name);
        assert_eq!(h.lattice.cols, c.nx + 1, "{} cols", c.name);
        assert!((h.lattice.dy - (ymax - ymin) / (2.0 * c.ny as f64)).abs() < 1e-12, "{} dy", c.name);
        assert_eq!((h.counted, h.outside), (c.x.len(), 0), "{}", c.name);
        let got: Vec<(usize, usize, usize)> = h.bins.iter().map(|b| (b.row, b.col, b.count)).collect();
        assert_eq!(got, c.cells.to_vec(), "{}", c.name);
    }
}
