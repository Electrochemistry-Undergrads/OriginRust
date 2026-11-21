//! Sheaf's 2D kernel density (`sheaf_calc::kde::density_2d_with`) against the matrices Origin 2016's
//! Plot > Statistics > 2D Kernel Density made for 20 points (T289).

#[path = "fixtures/kde2d_origin.rs"]
mod fixtures;

use sheaf_calc::kde::{Density2d, GRID_2D, Kde2dMethod, density_2d_with};

fn sheaf(method: Kde2dMethod) -> Density2d {
    density_2d_with(&fixtures::X, &fixtures::Y, method).unwrap()
}

/// The largest difference relative to the matrix's peak.
fn worst(d: &Density2d, origin: &[[f64; 32]; 32]) -> f64 {
    let peak = origin.iter().flatten().fold(0.0f64, |a, b| a.max(*b));
    (0..GRID_2D)
        .flat_map(|r| (0..GRID_2D).map(move |c| (r, c)))
        .map(|(r, c)| (d.z[r * GRID_2D + c] - origin[r][c]).abs() / peak)
        .fold(0.0, f64::max)
}

#[test]
fn the_grid_is_origins_32_by_32_quarter_range_extent() {
    let d = sheaf(Kde2dMethod::Bivariate);
    assert_eq!((d.x.len(), d.y.len()), (32, 32));
    for (v, from, to) in [(&d.x, fixtures::X_FROM, fixtures::X_TO), (&d.y, fixtures::Y_FROM, fixtures::Y_TO)] {
        for (k, g) in v.iter().enumerate() {
            let want = from + (to - from) * k as f64 / 31.0;
            assert!((g - want).abs() < 1e-12, "{k}: {g} against {want}");
        }
    }
}

#[test]
fn rule_of_thumb_matches_origin() {
    let d = sheaf(Kde2dMethod::RuleOfThumb);
    // Origin's bandwidths, fitted to its matrix: 0.412003 and 0.386052.
    assert!((d.wx - 0.412003).abs() < 2e-6 && (d.wy - 0.386052).abs() < 2e-6, "{} {}", d.wx, d.wy);
    let e = worst(&d, &fixtures::RULE_OF_THUMB);
    assert!(e < 1e-12, "worst difference {e} of the peak");
}

#[test]
fn bivariate_comes_close_to_origin() {
    let d = sheaf(Kde2dMethod::Bivariate);
    // Origin's bandwidths, fitted to its matrix: 0.732066 and 0.662596; the published diffusion
    // estimator gives 0.7493 and 0.6653 (Origin's exact rule is not known), 1.3 % of the peak at worst.
    assert!((d.wx / 0.732066 - 1.0).abs() < 0.03, "wx {}", d.wx);
    assert!((d.wy / 0.662596 - 1.0).abs() < 0.03, "wy {}", d.wy);
    let e = worst(&d, &fixtures::BIVARIATE);
    assert!(e < 0.03, "worst difference {e} of the peak");
}
