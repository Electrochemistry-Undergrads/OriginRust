//! Exponential smoothing against numpy and scipy, made by `tools/expsmooth-ref.py` (T374; none of these numbers come
//! from Origin). At fixed constants everything agrees to 1e-10 relative; with fitted constants Sheaf's SSE must be no
//! more than 1e-7 (relative) above scipy's best.

#[path = "fixtures/expsmooth_reference.rs"]
mod fixtures;

use sheaf_calc::expsmooth::{SmoothModel, smooth};

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= 1e-10 * b.abs().max(1.0), "{what}: {a} vs {b}");
}

#[test]
fn smoothing_matches_numpy_and_fits_match_scipy() {
    for c in fixtures::CASES {
        let model = match c.model {
            "simple" => SmoothModel::Simple,
            "holt" => SmoothModel::Holt,
            "add" => SmoothModel::WintersAdditive,
            _ => SmoothModel::WintersMultiplicative,
        };
        let s = smooth(c.y, model, c.period, Some(c.constants), 5).unwrap();
        for (i, (a, b)) in s.fitted.iter().zip(c.fitted).enumerate() {
            close(*a, *b, &format!("{} fitted {i}", c.name));
        }
        close(s.sse, c.sse, &format!("{} SSE", c.name));
        close(s.level, c.level, &format!("{} level", c.name));
        close(s.trend, c.trend, &format!("{} trend", c.name));
        for (i, (a, b)) in s.forecast.iter().zip(c.forecast).enumerate() {
            close(*a, *b, &format!("{} forecast {i}", c.name));
        }
        let f = smooth(c.y, model, c.period, None, 5).unwrap();
        assert!(
            f.sse <= c.best_sse * (1.0 + 1e-7) + 1e-12,
            "{}: fitted SSE {} (constants {:?}) vs scipy {} ({:?})",
            c.name,
            f.sse,
            f.constants,
            c.best_sse,
            c.best
        );
    }
}
