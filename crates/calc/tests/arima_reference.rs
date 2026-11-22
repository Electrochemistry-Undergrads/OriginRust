//! ARIMA by conditional sum of squares against numpy and scipy, made by `tools/arima-ref.py` (T376; none of these
//! numbers come from Origin). At scipy's parameters the errors, forecasts and standard errors agree to 1e-10
//! relative; Sheaf's own fit must reach a sum of squares no more than 1e-8 (relative) above scipy's.

#[path = "fixtures/arima_reference.rs"]
mod fixtures;

use sheaf_calc::arima::{ArimaOrder, css, fit, forecast};

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= 1e-10 * b.abs().max(1.0), "{what}: {a} vs {b}");
}

#[test]
fn arima_matches_numpy_and_scipy() {
    for c in fixtures::CASES {
        let order = ArimaOrder { p: c.order.0, d: c.order.1, q: c.order.2, constant: c.order.3 };
        let (e, ss) = css(c.y, order, c.params);
        close(ss, c.ss, &format!("{} SS", c.name));
        for (i, (a, b)) in e.iter().zip(c.errors).enumerate() {
            close(*a, *b, &format!("{} error {i}", c.name));
        }
        let (fc, se) = forecast(c.y, order, c.params, c.sigma2, 6);
        for i in 0..6 {
            close(fc[i], c.forecast[i], &format!("{} forecast {i}", c.name));
            close(se[i], c.se[i], &format!("{} se {i}", c.name));
        }
        let f = fit(c.y, order, 6).unwrap();
        assert!(
            f.ss <= c.ss * (1.0 + 1e-8),
            "{}: Sheaf SS {} at {:?} vs scipy {} at {:?}",
            c.name,
            f.ss,
            f.params,
            c.ss,
            c.params
        );
    }
}
