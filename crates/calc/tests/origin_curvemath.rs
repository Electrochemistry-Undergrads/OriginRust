//! Curve arithmetic against numbers from Origin (`subtract_ref` and `mathtool`).
//!
//! Reference scripts: `tools/origin-ref/curvemath-suite1.lines.txt` and `curvemath-suite3.lines.txt`
//! (run with `run-labtalk-lines.ps1`). Suite 1: data x = 0..10 step 0.5, y = 2 x + 1; reference
//! x = 1.5..8.5 step 1, y = x^2 + 3 sin(x), so the data reach outside the reference on both
//! sides. Tolerance: relative 1e-10.

// Origin prints 17 significant digits; keep them as printed.
#![allow(clippy::excessive_precision)]

use sheaf_calc::{CurveOp, curve_math_const, curve_math_ref};

fn close(name: &str, got: f64, want: f64) {
    if want.is_nan() {
        assert!(got.is_nan(), "{name}: got {got:?}, Origin gave a missing value");
        return;
    }
    assert!((got - want).abs() <= 1e-10 * want.abs().max(1.0), "{name}: got {got:?}, Origin gave {want:?}");
}

fn suite1() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..21).map(|i| 0.5 * i as f64).collect();
    let y = x.iter().map(|v| 2.0 * v + 1.0).collect();
    let rx: Vec<f64> = (0..8).map(|i| 1.5 + i as f64).collect();
    let ry = rx.iter().map(|v| v * v + 3.0 * v.sin()).collect();
    (x, y, rx, ry)
}

fn check(name: &str, op: CurveOp, samples: &[(usize, f64)]) {
    let (x, y, rx, ry) = suite1();
    let r = curve_math_ref(&x, &y, &rx, &ry, op, false).unwrap();
    assert_eq!(r.x, x);
    for &(i, want) in samples {
        close(&format!("{name}[{i}]"), r.y[i], want);
    }
}

#[test]
fn subtract_reference_matches_origin_inside_and_outside_the_reference_range() {
    // index 0 and 20 are outside the reference: its end segments are continued as straight lines
    check(
        "sub",
        CurveOp::Subtract,
        &[
            (0, -0.038087751062604092),
            (1, -0.43955348731245714),
            (2, -0.84101922356231018),
            (4, -1.6439506960620163),
            (8, -5.2575299819679238),
            (12, -22.514369493776137),
            (20, -77.017653449189851),
        ],
    );
}

#[test]
fn the_other_operations_match_origin() {
    check("add", CurveOp::Add, &[(0, 2.0380877510626041), (4, 11.643950696062017)]);
    check("mul", CurveOp::Multiply, &[(0, 1.0380877510626041), (4, 33.219753480310082)]);
    check("div", CurveOp::Divide, &[(0, 0.96330969995203508), (20, 0.214247120401489)]);
    check(
        "pow",
        CurveOp::Power,
        &[(0, 1.0), (1, 5.4247380988734566), (2, 68.019261336531841), (4, 44047.599658478088)],
    );
}

#[test]
fn common_range_only_matches_origin() {
    let (x, y, rx, ry) = suite1();
    let r = curve_math_ref(&x, &y, &rx, &ry, CurveOp::Subtract, true).unwrap();
    assert_eq!(r.x.len(), 15);
    assert_eq!((r.x[0], r.x[14]), (1.5, 8.5));
    for (i, want) in [(0, -1.2424849598121632), (1, -1.6439506960620163), (5, -5.2575299819679238)] {
        close(&format!("common[{i}]"), r.y[i], want);
    }
    close("common[14]", r.y[14], -56.645461337870472);
    let d = curve_math_ref(&x, &y, &rx, &ry, CurveOp::Divide, true).unwrap();
    close("common div[0]", d.y[0], 0.76299694337002333);
    close("common div[14]", d.y[14], 0.24113991229186654);
}

#[test]
fn constants_match_origin() {
    let (_, y, _, _) = suite1();
    let at = |op, c: f64, i: usize| curve_math_const(&y, op, c)[i];
    close("add", at(CurveOp::Add, 3.5, 0), 4.5);
    close("add", at(CurveOp::Add, 3.5, 20), 24.5);
    close("sub", at(CurveOp::Subtract, 3.5, 0), -2.5);
    close("mul", at(CurveOp::Multiply, 3.5, 20), 73.5);
    close("div", at(CurveOp::Divide, 4.0, 0), 0.25);
    close("div", at(CurveOp::Divide, 4.0, 20), 5.25);
    close("pow", at(CurveOp::Power, 2.0, 20), 441.0);
    // Origin refuses to divide by the constant 0; the library returns missing values.
    assert!(curve_math_const(&y, CurveOp::Divide, 0.0).iter().all(|v| v.is_nan()));
}

#[test]
fn division_by_a_reference_that_crosses_zero_gives_missing_values() {
    // Origin, suite 2: data y = x + 1 at x = 0..10, reference (2,0) (4,2) (6,0) (8,4) (10,5).
    let x: Vec<f64> = (0..11).map(f64::from).collect();
    let y: Vec<f64> = x.iter().map(|v| v + 1.0).collect();
    let r = curve_math_ref(&x, &y, &[2.0, 4.0, 6.0, 8.0, 10.0], &[0.0, 2.0, 0.0, 4.0, 5.0], CurveOp::Divide, false)
        .unwrap();
    for (i, want) in [(0, -0.5), (1, -2.0), (2, f64::NAN), (3, 4.0), (4, 2.5), (6, f64::NAN), (7, 4.0), (10, 2.2)] {
        close(&format!("div0[{i}]"), r.y[i], want);
    }
}

#[test]
fn a_sorted_reference_with_a_repeated_x_matches_origin() {
    // Origin, suite 3: data x = 0, 1, 9, 10, 5, 4 (y = 100); reference sorted by x with two
    // points at 5.5: (1.5, 2.25) (3.5, 12.25) (5.5, 30.25) (5.5, 99) (8.5, 72.25).
    let x = [0.0, 1.0, 9.0, 10.0, 5.0, 4.0];
    let y = [100.0; 6];
    let r = curve_math_ref(
        &x,
        &y,
        &[1.5, 3.5, 5.5, 5.5, 8.5],
        &[2.25, 12.25, 30.25, 99.0, 72.25],
        CurveOp::Subtract,
        false,
    )
    .unwrap();
    for (i, want) in [105.25, 100.25, 32.208333333333329, 41.125, 74.25, 83.25].into_iter().enumerate() {
        close(&format!("dup[{i}]"), r.y[i], want);
    }
}
