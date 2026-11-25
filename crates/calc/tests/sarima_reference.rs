//! Seasonal ARIMA against numpy and scipy, made by `tools/sarima-ref.py` (T446; none of these numbers come from
//! Origin). At given coefficients the errors, sum of squares, forecasts and bands to 1e-9, the standard errors (central
//! differences against the complex step) to 1e-6; the fitted sum of squares no worse than scipy's.

#[path = "fixtures/sarima_reference.rs"]
mod fixtures;

use sheaf_calc::arima::{ArimaOrder, Seasonal, css, fit, forecast, standard_errors};

fn close(a: &[f64], b: &[f64], tol: f64, what: &str) {
    assert_eq!(a.len(), b.len(), "{what}");
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= tol * y.abs().max(1.0), "{what}: {a:?} vs {b:?}");
    }
}

#[test]
fn seasonal_arima_matches_numpy_and_scipy() {
    for c in fixtures::CASES {
        let (p, d, q, constant, sp, sd, sq, period) = c.order;
        let order = ArimaOrder { p, d, q, constant, seasonal: Seasonal { p: sp, d: sd, q: sq, period } };
        let (e, ss) = css(c.y, order, c.params);
        let start = p + sp * period;
        close(&e[start..], c.errors, 1e-9, &format!("{} errors", c.name));
        close(&[ss], &[c.ss], 1e-9, &format!("{} ss", c.name));
        let (fc, fse) = forecast(c.y, order, c.params, c.sigma2, c.forecast.len());
        close(&fc, c.forecast, 1e-9, &format!("{} forecast", c.name));
        close(&fse, c.forecast_se, 1e-9, &format!("{} bands", c.name));
        close(&standard_errors(c.y, order, c.params, c.sigma2), c.se, 1e-6, &format!("{} se", c.name));
        let m = fit(c.y, order, 1).unwrap();
        assert!(m.ss <= c.best_ss * (1.0 + 1e-9), "{}: {} vs scipy {}", c.name, m.ss, c.best_ss);
    }
}
