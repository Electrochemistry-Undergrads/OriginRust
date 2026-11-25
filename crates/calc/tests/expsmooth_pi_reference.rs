//! Damped-trend exponential smoothing and prediction intervals against numpy and scipy, made by
//! `tools/expsmooth-pi-ref.py` (T427; none of these numbers come from Origin). At fixed constants to 1e-9; with the
//! damping fitted, Sheaf's SSE no worse than scipy's best.

#[path = "fixtures/expsmooth_pi_reference.rs"]
mod fixtures;

use sheaf_calc::expsmooth::{Damping, SmoothModel, smooth_with};

fn model(name: &str) -> SmoothModel {
    match name {
        "simple" => SmoothModel::Simple,
        "holt" => SmoothModel::Holt,
        _ => SmoothModel::WintersAdditive,
    }
}

fn close(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}");
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= 1e-9 * y.abs().max(1.0), "{what}: {a:?} vs {b:?}");
    }
}

#[test]
fn damped_smoothing_and_intervals_match_numpy() {
    for c in fixtures::FIXED {
        let damping = if c.phi == 1.0 { Damping::None } else { Damping::Given(c.phi) };
        let h = c.forecast.len();
        let s = smooth_with(c.y, model(c.model), c.period, Some(c.constants), damping, h, c.level).unwrap();
        close(&s.fitted, c.fitted, &format!("{} fitted", c.name));
        close(&[s.sse], &[c.sse], &format!("{} sse", c.name));
        close(&s.forecast, c.forecast, &format!("{} forecast", c.name));
        close(&s.lower, c.lower, &format!("{} lower", c.name));
        close(&s.upper, c.upper, &format!("{} upper", c.name));
    }
}

#[test]
fn fitted_damping_is_no_worse_than_scipy() {
    for (name, y, m, period, best) in fixtures::FITTED {
        let s = smooth_with(y, model(m), *period, None, Damping::Fitted, 3, 0.95).unwrap();
        assert!(s.sse <= best * (1.0 + 1e-9), "{name}: {} vs scipy {best}", s.sse);
        assert!((0.8..=0.98).contains(&s.phi), "{name}: {}", s.phi);
    }
}
