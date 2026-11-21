//! Sheaf's Q-Q plot numbers against Origin 2016's Plot > Statistics > Q-Q Plot (T311).

#[path = "fixtures/qqplot_origin.rs"]
mod fixtures;

use sheaf_calc::distfit::{Family, Scores, prob_plot_params, qq_band, qq_line_ends, quantile};

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-300)
}

fn expected(c: &fixtures::Case) -> Vec<f64> {
    let p = prob_plot_params(c.family, c.data).unwrap();
    Scores::Blom.positions(c.data.len()).iter().map(|&q| quantile(c.family, &p, q)).collect()
}

#[test]
fn the_expected_values_are_the_fitted_quantiles_at_the_scores() {
    for c in &fixtures::CASES {
        for (k, (got, want)) in expected(c).iter().zip(c.expected).enumerate() {
            assert!(rel(*got, *want) < 1e-12, "{} {k}: {got} against {want}", c.probe);
        }
    }
}

#[test]
fn the_reference_line_ends_are_origins() {
    for c in &fixtures::CASES {
        let (lo, hi) = qq_line_ends(&expected(c));
        assert!(rel(lo + 1.0, c.line.0 + 1.0) < 1e-12 && rel(hi, c.line.1) < 1e-12, "{}: {lo} {hi}", c.probe);
    }
}

#[test]
fn the_band_and_its_grid_are_origins() {
    for c in &fixtures::CASES {
        let (lo, hi) = qq_line_ends(&expected(c));
        let grid: Vec<f64> = (1..=51).map(|k| lo + (hi - lo) * k as f64 / 51.0).collect();
        for (k, (g, w)) in grid.iter().zip(&c.grid).enumerate() {
            assert!(rel(*g, *w) < 1e-12, "{} grid {k}", c.probe);
        }
        let p = prob_plot_params(c.family, c.data).unwrap();
        let (lower, upper) = qq_band(c.family, &p, c.data, c.level, &grid).unwrap();
        let tol = match c.family {
            Family::Gamma => 1e-8,
            Family::Weibull => 1e-7,
            // z comes from the round trip p = F(x), z = quantile(p): about 1e-11.
            _ => 1e-10,
        };
        for k in 0..51 {
            assert!(rel(lower[k], c.lower[k]) < tol, "{} lower {k}: {} against {}", c.probe, lower[k], c.lower[k]);
            assert!(rel(upper[k], c.upper[k]) < tol, "{} upper {k}: {} against {}", c.probe, upper[k], c.upper[k]);
        }
    }
}
