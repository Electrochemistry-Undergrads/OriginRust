//! Origin 2016, generated 101-row ramps; Y type 5, Normal rescale, 8% margin.
//! Raw scripts/results: scratch/reciprocal-autorange; graphing.md section 3.8.
use sheaf_core::graph::ScaleKind;
use sheaf_core::scales::auto_range;

const MEASURED: &[(f64, f64, f64, f64, f64)] = &[
    (1.0, 9.0, 1.0, 25.0, 2.0),
    (10.0, 100.0, 1.0, 360.0, 20.0),
    (2.0, 3.0, 1.75, 3.25, 0.5),
    (1.0, 2.0, 0.75, 2.25, 0.5),
    (1.0, 3.0, 0.5, 4.0, 1.0),
    (1.0, 5.0, 1.0, 8.0, 2.0),
    (1.0, 10.0, 1.0, 36.0, 2.0),
    (2.0, 4.0, 1.5, 4.5, 1.0),
    (5.0, 6.0, 4.75, 6.25, 0.5),
    (0.1, 0.9, 1.0, 2.5, 0.2),
    (100.0, 900.0, 1.0, 2500.0, 200.0),
    (10.0, 12.0, 9.5, 12.5, 1.0),
    (100.0, 110.0, 97.5, 112.5, 5.0),
    (0.01, 0.02, 0.0075, 0.0225, 0.005),
    (3.0, 4.0, 2.75, 4.25, 0.5),
    (1.0, 12.0, 1.0, 100.0, 20.0),
    (2.0, 2.1, 1.975, 2.125, 0.05),
    (2.0, 2.5, 1.9, 2.6, 0.2),
    (10.0, 11.0, 9.75, 11.25, 0.5),
    (1000.0, 1001.0, 999.75, 1001.25, 0.5),
    (80.0, 90.0, 77.5, 92.5, 5.0),
    (4.0, 12.0, 3.0, 15.0, 2.0),
    (2.0, 6.0, 1.0, 8.0, 2.0),
    (0.1, 0.2, 0.075, 0.225, 0.05),
];

#[test]
fn reciprocal_ranges_match_origin_across_narrow_wide_and_scaled_data() {
    for &(lo, hi, from, to, inc) in MEASURED {
        let got = auto_range(ScaleKind::Reciprocal, lo, hi, 8.0).unwrap();
        for (field, value, expected) in [("from", got.from, from), ("to", got.to, to), ("inc", got.inc, inc)] {
            assert!(
                (value - expected).abs() <= 1e-12 * expected.abs().max(1.0),
                "{lo}..{hi} {field}: got {value}, Origin {expected}"
            );
        }
    }
}

// Held out before the rule was selected. Origin reversed the limits of .04.. .48;
// this API returns ascending extents; axis direction remains the caller's choice.
#[test]
fn reciprocal_holdout_extents_and_steps_match_origin() {
    let cases: &[(f64, f64, f64, f64, f64)] = &[
        (0.5, 4.5, 1.0, 13.0, 2.0),
        (1.0, 6.0, 1.0, 10.0, 2.0),
        (1.0, 7.0, 1.0, 14.0, 2.0),
        (1.0, 8.0, 1.0, 19.0, 2.0),
        (1.0, 10.2, 1.0, 40.0, 5.0),
        (1.0, 10.5, 1.0, 45.0, 5.0),
        (1.0, 10.8, 1.0, 50.0, 5.0),
        (1.0, 11.0, 1.0, 55.0, 5.0),
        (1.0, 11.5, 1.0, 75.0, 10.0),
        (2.0, 8.0, 1.0, 11.0, 2.0),
        (2.0, 10.0, 1.0, 15.0, 2.0),
        (3.0, 12.0, 2.0, 16.0, 2.0),
        (0.2, 1.8, 1.0, 5.0, 0.5),
        (5.0, 45.0, 1.0, 130.0, 20.0),
        (20.0, 180.0, 1.0, 500.0, 50.0),
        (0.04, 0.48, 1.0, 4.0, 0.2),
        (6.0, 12.0, 5.0, 14.0, 2.0),
        (7.0, 8.0, 6.75, 8.25, 0.5),
    ];
    for &(lo, hi, from, to, inc) in cases {
        let got = auto_range(ScaleKind::Reciprocal, lo, hi, 8.0).unwrap();
        for (field, value, expected) in [("from", got.from, from), ("to", got.to, to), ("inc", got.inc, inc)] {
            assert!(
                (value - expected).abs() <= 1e-12 * expected.abs().max(1.0),
                "{lo}..{hi} {field}: got {value}, Origin {expected}"
            );
        }
    }
}

#[test]
fn projected_tick_limit_boundaries_follow_origin() {
    let cases: &[(f64, f64, f64, f64, f64)] = &[
        (1.0, 11.34, 1.0, 67.5, 5.0),
        (1.0, 11.35, 1.0, 70.0, 10.0),
        (1.0, 11.8, 1.0, 90.0, 10.0),
        (1.0, 12.5, 1.0, 160.0, 20.0),
        (1.0, 12.7, 1.0, 200.0, 20.0),
        (1.0, 12.8, 1.0, 250.0, 50.0),
        (1.0, 13.0, 1.0, 325.0, 50.0),
        (1.0, 13.2, 1.0, 550.0, 100.0),
        (2.0, 16.7, 1.0, 41.0, 2.0),
        (2.0, 16.8, 1.0, 42.0, 2.0),
        (2.0, 16.9, 1.0, 42.0, 2.0),
        (2.0, 17.0, 1.0, 42.5, 5.0),
        (2.0, 20.0, 1.0, 75.0, 10.0),
        (2.0, 25.0, 1.0, 325.0, 50.0),
        (3.0, 30.0, 1.0, 110.0, 20.0),
    ];
    for &(lo, hi, from, to, inc) in cases {
        let got = auto_range(ScaleKind::Reciprocal, lo, hi, 8.0).unwrap();
        for (field, value, expected) in [("from", got.from, from), ("to", got.to, to), ("inc", got.inc, inc)] {
            assert!(
                (value - expected).abs() <= 1e-12 * expected.abs().max(1.0),
                "{lo}..{hi} {field}: got {value}, Origin {expected}"
            );
        }
    }
}
