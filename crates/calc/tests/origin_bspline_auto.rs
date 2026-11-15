//! Cubic B-spline interpolation with Origin's default smoothing factor 0, against Origin
//! (`interp1 method:=bspline option:=extrap`, OriginPro 2016).
//!
//! Reference scripts: `re-workspace/scratch/t16/gen1.py`, `gen3.py`, `gen4.py` (the data are ours:
//! impulses, random numbers, noisy sine, uneven and unsorted X, repeated X, missing values, 5 to 60
//! points). Origin's output at 15 positions per data set is in `fixtures/bspline_auto_origin.rs`.
//! The automatic factor is the nugget of the semivariogram (see `automatic_smoothing` and
//! `docs/spec/analysis-tools.md`, section 5); with it the Dierckx smoothing spline reproduces all
//! 236 measured data sets to 6e-15. Tolerance here: 1e-10 absolute (values are of order 1).

use sheaf_calc::{InterpMethod, InterpOptions, automatic_smoothing, interp1};

// The fixture holds measured positions such as 3.14, not approximations of pi.
#[allow(clippy::approx_constant)]
#[path = "fixtures/bspline_auto_origin.rs"]
mod fix;

fn default_bspline() -> InterpOptions {
    InterpOptions { method: InterpMethod::BSpline, ..Default::default() }
}

#[test]
fn the_default_factor_reproduces_origin_inside_and_outside_the_data() {
    for (name, x, y, at, want) in fix::CASES {
        let got = interp1(x, y, at, &default_bspline());
        for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            let tol = 1e-10 * w.abs().max(1.0);
            assert!((g - w).abs() <= tol, "{name} at {}: got {g:?}, Origin gave {w:?}", at[i]);
        }
    }
}

/// The factors Origin used, found by matching its curves with the Dierckx fit (to about 1e-9).
#[test]
fn automatic_factor_of_impulses_and_random_numbers() {
    let x11: Vec<f64> = (0..11).map(f64::from).collect();
    let impulse = |k: usize| -> Vec<f64> { (0..11).map(|i| f64::from(u8::from(i == k))).collect() };
    // m = 11: 1/(m - 5) in the middle, half of that at the first point, the asymmetric rest
    // (the same starts serve every lag, so the last points count less); 9 and 10 need the
    // absolute value (the intercept is negative there).
    let want = [
        1.0 / 12.0,
        127.0 / 1020.0,
        19.0 / 120.0,
        367.0 / 2040.0,
        47.0 / 255.0,
        1.0 / 6.0,
        1.0 / 12.0,
        43.0 / 1020.0,
        1.0 / 120.0,
        9.0 / 680.0,
        3.0 / 170.0,
    ];
    for (k, w) in want.iter().enumerate() {
        let s = automatic_smoothing(&x11, &impulse(k));
        assert!((s - w).abs() <= 1e-12, "impulse at {k}: {s} vs {w}");
    }
    // Spacing does not matter, the scale of y enters squared, a constant offset does not.
    let x2: Vec<f64> = x11.iter().map(|v| 2.0 * v).collect();
    assert!((automatic_smoothing(&x2, &impulse(4)) - 47.0 / 255.0).abs() < 1e-12);
    let y3: Vec<f64> = impulse(4).iter().map(|v| 3.0 * v + 10.0).collect();
    assert!((automatic_smoothing(&x11, &y3) - 9.0 * 47.0 / 255.0).abs() < 1e-11);
    // m = 5: four lags from the first point only (Origin 39/172 for the middle impulse).
    let x5 = [0.0, 1.0, 2.0, 3.0, 4.0];
    assert!((automatic_smoothing(&x5, &[0.0, 0.0, 1.0, 0.0, 0.0]) - 39.0 / 172.0).abs() < 1e-12);
    assert!((automatic_smoothing(&x5, &[0.0, 1.0, -1.0, 0.0, 0.0]) - 93.0 / 172.0).abs() < 1e-12);
    // constant data: no noise, the interpolating spline
    assert_eq!(automatic_smoothing(&x11, &[2.5; 11]), 0.0);
}

#[test]
fn the_factor_uses_worksheet_order_before_sorting() {
    // Origin computes the factor on the rows as they are and only then sorts: the same points in
    // another row order give another curve (measured, case "unsorted" of the fixture).
    let x = [3.0, 1.0, 4.0, 0.0, 5.0, 9.0, 2.0, 6.0, 8.0, 7.0, 10.0, 11.0];
    let y = [0.3, 1.2, 0.8, 0.4, 1.5, -0.2, 0.7, 0.6, 1.1, 0.2, 0.9, 0.5];
    let mut sorted: Vec<(f64, f64)> = x.iter().copied().zip(y.iter().copied()).collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (sx, sy): (Vec<f64>, Vec<f64>) = sorted.into_iter().unzip();
    let a = automatic_smoothing(&x, &y);
    let b = automatic_smoothing(&sx, &sy);
    assert!((a - 0.326_615_492).abs() < 1e-8 && (b - 0.215_727).abs() < 1e-5, "{a} {b}");
    let at = [2.5, 6.5];
    assert_ne!(interp1(&x, &y, &at, &default_bspline()), interp1(&sx, &sy, &at, &default_bspline()));
}
