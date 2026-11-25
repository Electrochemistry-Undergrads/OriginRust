//! ARIMA coefficient standard errors and automatic order choice against numpy and scipy, made by
//! `tools/arima-se-ref.py` (T428; none of these numbers come from Origin). Standard errors at given coefficients to
//! 1e-9; the order with the smallest AIC exactly.

#[path = "fixtures/arima_se_reference.rs"]
mod fixtures;

use sheaf_calc::arima::{ArimaOrder, fit_auto, standard_errors};

#[test]
fn standard_errors_match_the_complex_step() {
    for (name, y, p, d, q, constant, params, sigma2, want) in fixtures::SE {
        let order = ArimaOrder { p: *p, d: *d, q: *q, constant: *constant, ..Default::default() };
        let got = standard_errors(y, order, params, *sigma2);
        for (a, b) in got.iter().zip(*want) {
            assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{name}: {got:?} vs {want:?}");
        }
    }
}

#[test]
fn the_order_with_the_smallest_aic_is_chosen() {
    for (name, y, d, p, q) in fixtures::AUTO {
        let m = fit_auto(y, *d, 2, 2, true, Default::default(), 1).unwrap();
        assert_eq!((m.order.p, m.order.q), (*p, *q), "{name}");
    }
}
