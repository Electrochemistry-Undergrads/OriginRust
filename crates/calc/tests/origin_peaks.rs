//! Peak finding (local maximum method) against Origin 2016 `pkfind`.
//!
//! Reference scripts: `tools/origin-ref/pkfind-suite1.labtalk.txt` and `pkfind-suite2.labtalk.txt`
//! (run with `run-labtalk-data.ps1`; the cases of suite 2 that found no peak make Origin stop with an
//! error, so those were run one call at a time). Compared: the 1-based row of every peak (exact).

use sheaf_calc::{PeakDirection, PeakFilter, PeakOptions, find_peaks};
#[allow(dead_code)]
fn grid(n: usize, start: f64, step: f64) -> Vec<f64> {
    (0..n).map(|i| start + i as f64 * step).collect()
}

#[allow(dead_code)]
fn close(name: &str, got: f64, want: f64, tol: f64) {
    if want.is_nan() {
        assert!(got.is_nan(), "{name}: got {got:?}, Origin gave NaN");
        return;
    }
    let ok = (got - want).abs() <= tol * want.abs().max(1.0);
    assert!(ok, "{name}: got {got:?}, Origin gave {want:?}");
}

#[allow(dead_code)]
fn check(name: &str, got: &[f64], samples: &[(usize, f64)], tol: f64) {
    for &(i, w) in samples {
        close(&format!("{name}[{i}]"), got[i], w, tol);
    }
}

#[test]
fn ripple_plus_gaussians_matches_origin() {
    // 201 points, x = 0..20; three gaussians, one negative, and a ripple 0.12 sin(9x)
    let x = grid(201, 0.0, 0.1);
    let y: Vec<f64> = x
        .iter()
        .map(|&x| {
            (-(x - 4.0).powi(2)).exp() + 0.7 * (-(x - 9.0).powi(2) / 2.0).exp()
                - 0.6 * (-(x - 13.0).powi(2) / 0.5).exp()
                + 0.8 * (-(x - 17.0).powi(2)).exp()
                + 0.12 * (9.0 * x).sin()
                + 0.2
        })
        .collect();
    let cases: Vec<(&str, usize, PeakDirection, PeakFilter, &[usize])> = vec![
        (
            "d_none",
            2,
            PeakDirection::Both,
            PeakFilter::None,
            &[
                3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149, 157,
                164, 170, 177, 184, 191, 198,
            ],
        ),
        (
            "d_default",
            2,
            PeakDirection::Both,
            PeakFilter::Height(0.0),
            &[
                3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 132, 143, 149, 157, 164, 170, 177,
                184, 191, 198,
            ],
        ),
        ("n5", 5, PeakDirection::Both, PeakFilter::None, &[10, 17, 38, 44, 66, 87, 93, 132, 143, 149, 170, 191]),
        (
            "n3",
            3,
            PeakDirection::Both,
            PeakFilter::None,
            &[
                10, 17, 24, 38, 44, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 132, 143, 149, 157, 164, 170, 177, 184,
                191, 198,
            ],
        ),
        ("neg2", 2, PeakDirection::Negative, PeakFilter::None, &[126, 132, 138]),
        (
            "both2",
            2,
            PeakDirection::Both,
            PeakFilter::None,
            &[
                3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149, 157,
                164, 170, 177, 184, 191, 198,
            ],
        ),
        (
            "h05",
            2,
            PeakDirection::Both,
            PeakFilter::Height(0.5),
            &[38, 44, 51, 80, 87, 93, 100, 107, 132, 164, 170, 177],
        ),
        ("h05n", 2, PeakDirection::Negative, PeakFilter::Height(0.5), &[132]),
        ("num2", 4, PeakDirection::Both, PeakFilter::Number(2), &[38, 44]),
        (
            "h035",
            2,
            PeakDirection::Positive,
            PeakFilter::Height(0.35),
            &[24, 38, 44, 51, 58, 73, 80, 87, 93, 100, 107, 114, 157, 164, 170, 177, 184],
        ),
    ];
    for (name, npts, direction, filter, want) in cases {
        let r = find_peaks(&x, &y, &PeakOptions { npts, direction, filter });
        assert_eq!(r.rows, want, "{name}");
        for (i, &row) in r.rows.iter().enumerate() {
            assert_eq!(r.x[i], x[row - 1]);
            assert_eq!(r.y[i], y[row - 1]);
        }
    }
}

#[test]
fn triangles_flat_bottoms_and_sign_rules_match_origin() {
    // x = 0..24 step 0.1; eleven triangular peaks (half width 0.5) at 2, 4, .., 22 with heights
    // 1, .6, .3, .24, .22, .2, .19, .15, .12, .1, .05; shifted copies test the sign rules
    let x = grid(241, 0.0, 0.1);
    let hs = [1.0, 0.6, 0.3, 0.24, 0.22, 0.2, 0.19, 0.15, 0.12, 0.1, 0.05];
    let b: Vec<f64> = x
        .iter()
        .map(|&x| {
            hs.iter()
                .enumerate()
                .map(|(j, h)| {
                    let d = (x - 2.0 * (j as f64 + 1.0)).abs();
                    if d < 0.5 { h * (1.0 - d / 0.5) } else { 0.0 }
                })
                .sum()
        })
        .collect();
    let c: Vec<f64> = b.iter().map(|v| v + 0.5).collect();
    let d: Vec<f64> = b.iter().map(|v| v - 2.0).collect();
    let e: Vec<f64> = b.iter().map(|v| v - 0.5).collect();
    let f: Vec<f64> = b.iter().map(|v| -v).collect();
    let sets: [(&str, &Vec<f64>); 4] = [("C", &c), ("D", &d), ("E", &e), ("F", &f)];
    let cases: Vec<(&str, PeakDirection, PeakFilter, &[usize])> = vec![
        ("C_p", PeakDirection::Positive, PeakFilter::Height(0.0), &[21, 41, 61, 81, 101, 121, 141, 161, 181, 201, 221]),
        ("C_b", PeakDirection::Both, PeakFilter::Height(0.0), &[21, 41, 61, 81, 101, 121, 141, 161, 181, 201, 221]),
        ("D_n_none", PeakDirection::Negative, PeakFilter::None, &[26, 46, 66, 86, 106, 126, 146, 166, 186, 206, 226]),
        ("E_p_none", PeakDirection::Positive, PeakFilter::None, &[21, 41]),
        ("E_n_none", PeakDirection::Negative, PeakFilter::None, &[26, 46, 66, 86, 106, 126, 146, 166, 186, 206, 226]),
        ("F_n_none", PeakDirection::Negative, PeakFilter::None, &[21, 41, 61, 81, 101, 121, 141, 161, 181, 201, 221]),
    ];
    for (name, direction, filter, want) in cases {
        let series = sets.iter().find(|(k, _)| name.starts_with(k)).unwrap().1;
        let r = find_peaks(&x, series, &PeakOptions { npts: 2, direction, filter });
        assert_eq!(r.rows, want, "{name}");
    }
    // all-negative data have no positive peaks
    let r = find_peaks(&x, &d, &PeakOptions { npts: 2, direction: PeakDirection::Positive, filter: PeakFilter::None });
    assert!(r.rows.is_empty());
}
