//! Sheaf's probability-plot estimates and confidence band against Origin 2016's `plot_prob` (T308).

#[path = "fixtures/probplot_origin.rs"]
mod fixtures;

use sheaf_calc::distfit::{Family, Scores, confidence_band, prob_plot_params};

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-300)
}

#[test]
fn the_estimates_are_origins() {
    for c in &fixtures::CASES {
        let p = prob_plot_params(c.family, c.data).unwrap();
        for (k, (got, want)) in p.iter().zip(c.params).enumerate() {
            // Origin prints 14 significant digits.
            assert!(rel(*got, *want) < 1e-12, "{} parameter {k}: {got} against {want}", c.probe);
        }
    }
}

#[test]
fn the_confidence_band_is_origins_for_every_family() {
    for c in &fixtures::CASES {
        let p = prob_plot_params(c.family, c.data).unwrap();
        let (lower, upper) = confidence_band(c.family, &p, c.data, c.level).unwrap();
        // Gamma takes a numerical derivative of its quantile in the shape; Origin's Weibull band is
        // 1.4e-8 from the exact observed information (Origin's own derivatives, presumably).
        let tol = match c.family {
            Family::Gamma => 1e-8,
            Family::Weibull => 1e-7,
            _ => 1e-12,
        };
        for k in 0..27 {
            assert!(rel(lower[k], c.lower[k]) < tol, "{} lower {k}: {} against {}", c.probe, lower[k], c.lower[k]);
            assert!(rel(upper[k], c.upper[k]) < tol, "{} upper {k}: {} against {}", c.probe, upper[k], c.upper[k]);
        }
    }
}

#[test]
#[allow(clippy::excessive_precision)] // Origin's numbers as it printed them
fn origins_score_methods() {
    // Measured: the first of 10 values at these percents.
    let first = |s: Scores| s.positions(10)[0] * 100.0;
    for (s, want) in [
        (Scores::Blom, 6.0975609756097562),
        (Scores::Benard, 6.7307692307692308),
        (Scores::Hazen, 5.0),
        (Scores::VanDerWaerden, 9.0909090909090917),
        (Scores::KaplanMeier, 10.0),
    ] {
        assert!(rel(first(s), want) < 1e-14, "{s:?}");
    }
    assert_eq!(Scores::KaplanMeier.positions(10)[9], 1.0, "Kaplan-Meier puts the last at 100 %");
}
