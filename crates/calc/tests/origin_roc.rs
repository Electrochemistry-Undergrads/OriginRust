//! ROC curve against Origin 2016's Statistics > ROC Curve (statistics.md 19.5).
//!
//! Origin's tool is a menu dialog without a scriptable X-Function; it was run through its window with
//! `tools/origin-ref/origin-dialog.ps1` (`re-workspace/scratch/t29roc`, local only) and the numbers below
//! were read from its report sheet ("ROC Curve1") and curve sheet ("ROCValues1") at full precision. The
//! positive state value is the one Origin picked in each run (its list follows the state column, so two
//! runs took 0). Origin lists the curve from (1, 1) to (0, 0); Sheaf from (0, 0) to (1, 1).

#![allow(clippy::excessive_precision)] // the digits as Origin printed them

use sheaf_calc::{RocOptions, roc};

struct Report {
    positives: usize,
    negatives: usize,
    area: f64,
    se: f64,
    p: f64,
    lcl: f64,
    ucl: f64,
    /// Origin's rows: cut-off, 1 - specificity, sensitivity.
    curve: &'static [(f64, f64, f64)],
}

fn check(name: &str, values: &[f64], states: &[u8], positive_state: u8, o: Report) {
    let positive: Vec<bool> = states.iter().map(|&s| s == positive_state).collect();
    let r = roc(values, &positive, &RocOptions::default()).unwrap();
    let close = |got: f64, want: f64, what: &str| {
        assert!((got - want).abs() <= 1e-12 * want.abs().max(1e-3), "{name} {what}: {got} against Origin's {want}");
    };
    assert_eq!((r.n_positive, r.n_negative), (o.positives, o.negatives), "{name}: case counts");
    close(r.auc, o.area, "area");
    close(r.se, o.se, "standard error");
    // The normal tail of `statrs` differs from Origin's (and scipy's) in the 11th digit.
    assert!((r.p - o.p).abs() <= 1e-9 * o.p, "{name} asymptotic prob: {} against Origin's {}", r.p, o.p);
    close(r.lcl, o.lcl, "95% LCL");
    close(r.ucl, o.ucl, "95% UCL");
    assert_eq!(r.points.len(), o.curve.len(), "{name}: curve points");
    for (pt, &(cut, x, y)) in r.points.iter().rev().zip(o.curve) {
        assert!((pt.cutoff - cut).abs() <= 1e-15, "{name}: cut-off {} against Origin's {cut}", pt.cutoff);
        close(1.0 - pt.specificity, x, "1 - specificity");
        close(pt.sensitivity, y, "sensitivity");
    }
}

#[test]
fn ties_inside_and_across_the_states() {
    check(
        "ties",
        &[0.1, 0.35, 0.4, 0.8, 0.2, 0.7, 0.55, 0.9, 0.6, 0.3, 0.4, 0.7, 0.35, 0.6, 0.8, 0.2],
        &[0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0],
        1,
        Report {
            positives: 8,
            negatives: 8,
            area: 0.8984375,
            se: 0.077493787947229317,
            p: 0.0074055333405002735,
            lcl: 0.74655246659784646,
            ucl: 1.0503225334021535,
            curve: &[
                (0.0, 1.0, 1.0),
                (0.15000000000000002, 0.875, 1.0),
                (0.25, 0.625, 1.0),
                (0.32499999999999996, 0.5, 1.0),
                (0.375, 0.375, 0.875),
                (0.47500000000000003, 0.25, 0.75),
                (0.575, 0.125, 0.75),
                (0.64999999999999991, 0.0, 0.625),
                (0.75, 0.0, 0.375),
                (0.85000000000000009, 0.0, 0.125),
                (1.0, 0.0, 0.0),
            ],
        },
    );
}

#[test]
fn overlapping_states_without_ties() {
    check(
        "overlap",
        &[2.1, 3.3, 1.7, 4.8, 2.9, 3.1, 4.2, 1.1, 2.5, 3.9, 0.7, 3.6, 2.2, 4.4, 1.9, 3.0, 2.8, 3.7],
        &[0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 1, 1],
        1,
        Report {
            positives: 9,
            negatives: 9,
            area: 0.83950617283950613,
            se: 0.10327314252246217,
            p: 0.015169399414045485,
            lcl: 0.63709453292520835,
            ucl: 1.041917812753804,
            curve: &[
                (-0.30000000000000004, 1.0, 1.0),
                (0.9, 0.88888888888888884, 1.0),
                (1.4, 0.77777777777777779, 1.0),
                (1.7999999999999998, 0.66666666666666663, 1.0),
                (2.0, 0.55555555555555558, 1.0),
                (2.1500000000000004, 0.44444444444444442, 1.0),
                (2.35, 0.33333333333333331, 1.0),
                (2.65, 0.33333333333333331, 0.88888888888888884),
                (2.8499999999999996, 0.33333333333333331, 0.77777777777777779),
                (2.95, 0.22222222222222221, 0.77777777777777779),
                (3.05, 0.22222222222222221, 0.66666666666666663),
                (3.2, 0.1111111111111111, 0.66666666666666663),
                (3.45, 0.1111111111111111, 0.55555555555555558),
                (3.6500000000000004, 0.1111111111111111, 0.44444444444444442),
                (3.8, 0.1111111111111111, 0.33333333333333331),
                (4.05, 0.1111111111111111, 0.22222222222222221),
                (4.3000000000000007, 0.1111111111111111, 0.1111111111111111),
                (4.6, 0.0, 0.1111111111111111),
                (5.8, 0.0, 0.0),
            ],
        },
    );
}

#[test]
fn a_reversed_separation_has_area_zero_and_no_standard_error() {
    // Origin took state 0 as positive: the three lowest values, so the area is 0.
    check(
        "separated",
        &[1.5, 2.5, 3.5, 0.5, 0.9, 1.1, 4.0],
        &[1, 1, 1, 0, 0, 0, 1],
        0,
        Report {
            positives: 3,
            negatives: 4,
            area: 0.0,
            se: 0.0,
            p: 0.033894853524689274,
            lcl: 0.0,
            ucl: 0.0,
            curve: &[
                (-0.5, 1.0, 1.0),
                (0.7, 1.0, 0.66666666666666663),
                (1.0, 1.0, 0.33333333333333331),
                (1.3, 1.0, 0.0),
                (2.0, 0.75, 0.0),
                (3.0, 0.5, 0.0),
                (3.75, 0.25, 0.0),
                (5.0, 0.0, 0.0),
            ],
        },
    );
}

#[test]
fn unequal_groups_with_an_area_below_one_half() {
    // Origin took state 0 as positive (4 cases against 5); its lower limit is below 0.
    check(
        "unequal",
        &[3.0, 5.0, 7.0, 9.0, 11.0, 1.0, 4.0, 6.0, 8.5],
        &[1, 1, 1, 1, 1, 0, 0, 0, 0],
        0,
        Report {
            positives: 4,
            negatives: 5,
            area: 0.3,
            se: 0.18708286933869706,
            p: 0.32718687779030559,
            lcl: -0.066675686028258907,
            ucl: 0.66667568602825888,
            curve: &[
                (0.0, 1.0, 1.0),
                (2.0, 1.0, 0.75),
                (3.5, 0.8, 0.75),
                (4.5, 0.8, 0.5),
                (5.5, 0.6, 0.5),
                (6.5, 0.6, 0.25),
                (7.75, 0.4, 0.25),
                (8.75, 0.4, 0.0),
                (10.0, 0.2, 0.0),
                (12.0, 0.0, 0.0),
            ],
        },
    );
}

/// Like `check`, with "Positive v.s. Low" and Origin's rows in Sheaf's order (both run from (0, 0)).
fn check_low(name: &str, values: &[f64], states: &[u8], positive_state: u8, o: Report) {
    let positive: Vec<bool> = states.iter().map(|&s| s == positive_state).collect();
    let r = roc(values, &positive, &RocOptions { higher_is_positive: false, ..Default::default() }).unwrap();
    let close = |got: f64, want: f64, what: &str| {
        assert!((got - want).abs() <= 1e-12 * want.abs().max(1e-3), "{name} {what}: {got} against Origin's {want}");
    };
    assert_eq!((r.n_positive, r.n_negative), (o.positives, o.negatives), "{name}: case counts");
    close(r.auc, o.area, "area");
    close(r.se, o.se, "standard error");
    assert!((r.p - o.p).abs() <= 1e-9 * o.p, "{name} asymptotic prob: {} against Origin's {}", r.p, o.p);
    close(r.lcl, o.lcl, "95% LCL");
    close(r.ucl, o.ucl, "95% UCL");
    assert_eq!(r.points.len(), o.curve.len(), "{name}: curve points");
    for (pt, &(cut, x, y)) in r.points.iter().zip(o.curve) {
        assert!((pt.cutoff - cut).abs() <= 1e-15, "{name}: cut-off {} against Origin's {cut}", pt.cutoff);
        close(1.0 - pt.specificity, x, "1 - specificity");
        close(pt.sensitivity, y, "sensitivity");
    }
}

#[test]
fn positive_for_low_values_mirrors_the_area_and_keeps_the_cutoffs() {
    // Origin 2016, "Positive v.s. Low", state 1 positive (re-workspace/scratch/t29roc/r-lowA).
    check_low(
        "unequal, low",
        &[3.0, 5.0, 7.0, 9.0, 11.0, 1.0, 4.0, 6.0, 8.5],
        &[1, 1, 1, 1, 1, 0, 0, 0, 0],
        1,
        Report {
            positives: 5,
            negatives: 4,
            area: 0.30000000000000004,
            se: 0.18708286933869711,
            p: 0.32718687779030581,
            lcl: -0.066675686028258963,
            ucl: 0.666675686028259,
            curve: &[
                (0.0, 0.0, 0.0),
                (2.0, 0.25, 0.0),
                (3.5, 0.25, 0.19999999999999996),
                (4.5, 0.5, 0.19999999999999996),
                (5.5, 0.5, 0.4),
                (6.5, 0.75, 0.4),
                (7.75, 0.75, 0.6),
                (8.75, 1.0, 0.6),
                (10.0, 1.0, 0.8),
                (12.0, 1.0, 1.0),
            ],
        },
    );
    // The overlap set, state 1 positive (r-lowB).
    check_low(
        "overlap, low",
        &[2.1, 3.3, 1.7, 4.8, 2.9, 3.1, 4.2, 1.1, 2.5, 3.9, 0.7, 3.6, 2.2, 4.4, 1.9, 3.0, 2.8, 3.7],
        &[0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 0, 1, 1, 1],
        1,
        Report {
            positives: 9,
            negatives: 9,
            area: 0.16049382716049387,
            se: 0.10327314252246217,
            p: 0.015169399414045485,
            lcl: -0.041917812753803935,
            ucl: 0.36290546707479165,
            curve: &[
                (-0.30000000000000004, 0.0, 0.0),
                (0.9, 0.11111111111111116, 0.0),
                (1.4, 0.22222222222222221, 0.0),
                (1.7999999999999998, 0.33333333333333337, 0.0),
                (2.0, 0.44444444444444442, 0.0),
                (2.1500000000000004, 0.55555555555555558, 0.0),
                (2.35, 0.66666666666666674, 0.0),
                (2.65, 0.66666666666666674, 0.11111111111111116),
                (2.8499999999999996, 0.66666666666666674, 0.22222222222222221),
                (2.95, 0.77777777777777779, 0.22222222222222221),
                (3.05, 0.77777777777777779, 0.33333333333333337),
                (3.2, 0.88888888888888884, 0.33333333333333337),
                (3.45, 0.88888888888888884, 0.44444444444444442),
                (3.6500000000000004, 0.88888888888888884, 0.55555555555555558),
                (3.8, 0.88888888888888884, 0.66666666666666674),
                (4.05, 0.88888888888888884, 0.77777777777777779),
                (4.3000000000000007, 0.88888888888888884, 0.88888888888888884),
                (4.6, 1.0, 0.88888888888888884),
                (5.8, 1.0, 1.0),
            ],
        },
    );
}

#[test]
fn a_text_state_column_with_yes_positive() {
    // Origin 2016 with states "no"/"yes" (its list offers them in order of appearance), "yes" positive
    // (r-txt). The calculation only sees which rows are positive, so the states are 1 for "yes".
    check(
        "text states",
        &[0.1, 0.35, 0.4, 0.8, 0.2, 0.7, 0.55, 0.9],
        &[0, 0, 1, 1, 0, 1, 0, 1],
        1,
        Report {
            positives: 4,
            negatives: 4,
            area: 0.9375,
            se: 0.0897587913521567,
            p: 0.04330814281079199,
            lcl: 0.76157600165392769,
            ucl: 1.1134239983460723,
            curve: &[
                (0.0, 1.0, 1.0),
                (0.15000000000000002, 0.75, 1.0),
                (0.275, 0.5, 1.0),
                (0.375, 0.25, 1.0),
                (0.47500000000000003, 0.25, 0.75),
                (0.625, 0.0, 0.75),
                (0.75, 0.0, 0.5),
                (0.85000000000000009, 0.0, 0.25),
                (1.0, 0.0, 0.0),
            ],
        },
    );
}
