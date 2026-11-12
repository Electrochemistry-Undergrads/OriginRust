//! Inputs at the edge of what the linear and polynomial fits can handle. These check our own
//! rules (refusing a bad confidence level, flagging a constant y); Origin was not run on them.

use sheaf_fit::FitError;
use sheaf_fit::linear::{LinearOptions, PolyOptions, fit_linear, fit_polynomial};

const X: [f64; 4] = [1.0, 2.0, 3.0, 4.0];

#[test]
fn confidence_level_outside_zero_one_is_refused() {
    let y = [1.0, 3.0, 2.0, 5.0];
    for level in [0.0, 1.0, 1.5, -0.2, f64::NAN] {
        let lin = LinearOptions { confidence_level: level, ..Default::default() };
        assert!(matches!(fit_linear(&X, &y, &lin), Err(FitError::Failed(_))), "linear, level {level}");
        let poly = PolyOptions { confidence_level: level, ..Default::default() };
        assert!(matches!(fit_polynomial(&X, &y, 2, &poly), Err(FitError::Failed(_))), "poly, level {level}");
    }
    // A valid level still gives finite limits.
    let ok = fit_linear(&X, &y, &LinearOptions { confidence_level: 0.99, ..Default::default() }).unwrap();
    assert!(ok.params.iter().all(|p| p.lcl.is_finite() && p.ucl.is_finite()));
}

#[test]
fn constant_y_gives_undefined_r_squared_and_a_warning() {
    let y = [2.0; 4];
    let r = fit_linear(&X, &y, &LinearOptions::default()).unwrap();
    assert!(r.stats.r_squared.is_nan());
    assert!(r.stats.adj_r_squared.is_nan());
    assert!(r.stats.pearson_r.unwrap().is_nan());
    assert!(!r.warnings.is_empty());
    let p = fit_polynomial(&X, &y, 2, &PolyOptions::default()).unwrap();
    assert!(p.stats.r_squared.is_nan());
    assert!(!p.warnings.is_empty());
    // y that varies is unaffected.
    let r = fit_linear(&X, &[1.0, 2.0, 3.0, 4.5], &LinearOptions::default()).unwrap();
    assert!(r.stats.r_squared.is_finite() && r.warnings.is_empty());
}
