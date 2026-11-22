//! Linear quantile regression against exact linear-programme solutions from `scipy.optimize.linprog`
//! (HiGHS), made by `tools/quantreg-ref.py` (T342; none of these numbers come from Origin).
//!
//! Tolerances: coefficients 1e-9 relative to 1 + their size, the loss 1e-9 relative. The data have no ties,
//! so the optimal vertex is unique and both programs must land on it.

#[path = "fixtures/scipy_quantreg.rs"]
mod fixtures;

use sheaf_calc::quantreg::quantile_regression;

#[test]
fn coefficients_and_losses_match_the_exact_linear_programme() {
    for case in fixtures::CASES {
        for &(tau, want, loss) in case.fits {
            let got = quantile_regression(case.xs, case.y, tau, case.intercept).unwrap();
            assert_eq!(got.coefficients.len(), want.len(), "{} tau {tau}", case.name);
            for (g, w) in got.coefficients.iter().zip(want) {
                assert!(
                    (g - w).abs() <= 1e-9 * (1.0 + w.abs()),
                    "{} tau {tau}: {:?} vs {want:?}",
                    case.name,
                    got.coefficients
                );
            }
            assert!(
                (got.loss - loss).abs() <= 1e-9 * loss.max(1.0),
                "{} tau {tau}: loss {} vs {loss}",
                case.name,
                got.loss
            );
            assert!(got.iterations < 60, "{} tau {tau}: {} iterations", case.name, got.iterations);
        }
    }
}
