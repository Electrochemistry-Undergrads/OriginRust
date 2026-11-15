//! ROC curve against scipy (not Origin; Origin's ROC tool has no scriptable X-Function, see spec section 19).
//! Numbers from `tools/roc-ref.py` (scipy 1.17: Mann-Whitney U for the area, DeLong's standard error by the
//! pairwise kernel, `scipy.stats.norm` for the interval and the p-value). Tolerance 1e-12 relative (1e-8 for the p-value: the normal tail of `statrs` differs from scipy's in the 11th digit).

#![allow(clippy::too_many_arguments)]

use sheaf_calc::{RocOptions, roc};

fn check(
    case: &str,
    values: &[f64],
    state: &[u8],
    auc: f64,
    se: f64,
    p: f64,
    lcl: f64,
    best: f64,
    curve: &[(f64, f64)],
) {
    let positive: Vec<bool> = state.iter().map(|&s| s == 1).collect();
    let r = roc(values, &positive, &RocOptions::default()).unwrap();
    let close =
        |a: f64, b: f64, what: &str| assert!((a - b).abs() <= 1e-12 * b.abs().max(1e-3), "{case} {what}: {a} vs {b}");
    close(r.auc, auc, "auc");
    close(r.se, se, "se");
    assert!((r.p - p).abs() <= 1e-8 * p.max(1e-3), "{case} p: {} vs {p}", r.p);
    close(r.lcl, lcl, "lower limit");
    assert_eq!(r.points[r.best].cutoff, best, "{case}: best cut-off");
    assert_eq!(r.points.len(), curve.len(), "{case}: points");
    for (pt, (x, y)) in r.points.iter().zip(curve) {
        close(1.0 - pt.specificity, *x, "1 - specificity");
        close(pt.sensitivity, *y, "sensitivity");
    }
}

#[test]
fn ties() {
    let values = [0.1, 0.35, 0.4, 0.8, 0.2, 0.7, 0.55, 0.9, 0.6, 0.3, 0.4, 0.7, 0.35, 0.6, 0.8, 0.2];
    let state = [0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0];
    let curve = [
        (0.0, 0.0),
        (0.0, 0.125),
        (0.0, 0.375),
        (0.0, 0.625),
        (0.125, 0.75),
        (0.25, 0.75),
        (0.375, 0.875),
        (0.5, 1.0),
        (0.625, 1.0),
        (0.875, 1.0),
        (1.0, 1.0),
    ];
    check(
        "ties",
        &values,
        &state,
        0.8984375,
        0.07643256101903541,
        1.8588223620186696e-07,
        0.7486324331565306,
        0.7,
        &curve,
    );
}

#[test]
fn separated() {
    let values = [1.5, 2.5, 3.5, 0.5, 0.9, 1.1, 4.0];
    let state = [1, 1, 1, 0, 0, 0, 1];
    let curve = [
        (0.0, 0.0),
        (0.0, 0.25),
        (0.0, 0.5),
        (0.0, 0.75),
        (0.0, 1.0),
        (0.3333333333333333, 1.0),
        (0.6666666666666666, 1.0),
        (1.0, 1.0),
    ];
    check("separated", &values, &state, 1.0, 0.0, 0.0, 1.0, 1.5, &curve);
}

#[test]
fn overlap() {
    let values = [2.1, 3.3, 1.7, 4.8, 2.9, 3.1, 4.2, 1.1, 2.5, 3.9, 0.7, 3.6, 2.2, 4.4, 1.9, 3.0, 2.8, 3.7];
    let state = [0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 1, 1];
    let curve = [
        (0.0, 0.0),
        (0.0, 0.1111111111111111),
        (0.1111111111111111, 0.1111111111111111),
        (0.1111111111111111, 0.2222222222222222),
        (0.1111111111111111, 0.3333333333333333),
        (0.1111111111111111, 0.4444444444444444),
        (0.1111111111111111, 0.5555555555555556),
        (0.1111111111111111, 0.6666666666666666),
        (0.2222222222222222, 0.6666666666666666),
        (0.2222222222222222, 0.7777777777777778),
        (0.3333333333333333, 0.7777777777777778),
        (0.3333333333333333, 0.8888888888888888),
        (0.3333333333333333, 1.0),
        (0.4444444444444444, 1.0),
        (0.5555555555555556, 1.0),
        (0.6666666666666666, 1.0),
        (0.7777777777777778, 1.0),
        (0.8888888888888888, 1.0),
        (1.0, 1.0),
    ];
    check(
        "overlap",
        &values,
        &state,
        0.8395061728395061,
        0.10673837324562087,
        0.00146903396841778,
        0.6303028055096955,
        2.5,
        &curve,
    );
}
