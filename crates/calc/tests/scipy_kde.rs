//! The kernel density estimate against numbers from scipy (scipy-checked; none of these numbers come from Origin).
//!
//! The inputs and the scipy numbers are in `fixtures/scipy_kde.rs`, made by `tools/kde-ref.py` with
//! `scipy.stats.gaussian_kde` (a scalar bandwidth factor `w / SD`, so scipy evaluates exactly the Gaussian estimate
//! with bandwidth `w`). The bandwidth rules are Origin's documented formulas; the fixture writes them out in numpy,
//! so the Scott and Silverman bandwidths are checked against an independent transcription, not against scipy's own
//! (different) Scott and Silverman factors, which are checked here as the k x SD rule.
//!
//! Tolerance: 1e-12 relative (1e-300 absolute: the far tails are tiny but still compared relatively).

#[path = "fixtures/scipy_kde.rs"]
mod fixtures;

use fixtures::*;
use sheaf_calc::kde::{Bandwidth, GRID_POINTS, bandwidth, density_at, kernel_density};

fn near(what: &str, got: f64, want: f64, rel: f64) {
    assert!((got - want).abs() <= rel * want.abs() + 1e-300, "{what}: got {got:?}, scipy gave {want:?}");
}

fn rule(c: &Case) -> Bandwidth {
    match c.rule {
        0 => Bandwidth::Scott,
        1 => Bandwidth::Silverman,
        _ => Bandwidth::Sd(c.k),
    }
}

#[test]
fn the_density_matches_scipy_on_the_grid() {
    assert!(CASES.len() >= 15);
    for c in CASES {
        let what = |s: &str| format!("{} rule {} k {} {s}", c.name, c.rule, c.k);
        let w = bandwidth(c.data, rule(c)).unwrap();
        near(&what("bandwidth"), w, c.bandwidth, 1e-12);
        let d = kernel_density(c.data, rule(c), c.extend_pct).unwrap();
        assert_eq!(d.grid.len(), GRID_POINTS);
        assert_eq!(c.grid.len(), GRID_POINTS);
        for (i, ((g, f), (wg, wf))) in d.grid.iter().zip(&d.density).zip(c.grid.iter().zip(c.density)).enumerate() {
            near(&what(&format!("grid[{i}]")), *g, *wg, 1e-12);
            near(&what(&format!("density[{i}]")), *f, *wf, 1e-12);
        }
    }
}

#[test]
fn scipys_own_scott_and_silverman_factors_are_the_k_times_sd_rule() {
    for c in FACTORS {
        let n = c.data.len() as f64;
        // Scott's factor n^(-1/5); Silverman's (n (d + 2) / 4)^(-1/(d + 4)) with d = 1 is (3n/4)^(-1/5).
        near("scott factor", c.scott, n.powf(-0.2), 1e-12);
        near("silverman factor", c.silverman, (n * 3.0 / 4.0).powf(-0.2), 1e-12);
        for (k, want) in [(c.scott, c.scott_density), (c.silverman, c.silverman_density)] {
            let w = bandwidth(c.data, Bandwidth::Sd(k)).unwrap();
            near("density at the mean", density_at(c.data, w, c.at), want, 1e-12);
        }
    }
}
