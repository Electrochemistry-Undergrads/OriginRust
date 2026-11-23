//! qPCR relative quantification and standard curves against numpy and scipy, made by `tools/qpcr-ref.py` (T397; none
//! of these numbers come from Origin). Tolerance 1e-10 relative.

#[path = "fixtures/qpcr_reference.rs"]
mod fixtures;

use sheaf_calc::qpcr::{relative_quantification, standard_curve};

fn close(a: f64, b: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else {
        assert!((a - b).abs() <= 1e-10 * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn qpcr_matches_numpy_and_scipy() {
    let groups: Vec<String> = fixtures::GROUPS.iter().map(|s| s.to_string()).collect();
    for (name, et, er, rows) in fixtures::CASES {
        let r =
            relative_quantification(&groups, fixtures::TARGET, fixtures::REFERENCE, "control", *et, *er, 0.95).unwrap();
        assert_eq!(r.len(), rows.len(), "{name}");
        for ((g, got), (want_g, n, dm, ds, ddct, se, ratio, low, high, p)) in r.iter().zip(*rows) {
            let what = format!("{name} {g}");
            assert_eq!((g.as_str(), got.n), (*want_g, *n), "{what}");
            close(got.mean_dct, *dm, &format!("{what} mean dCt"));
            close(got.sd_dct, *ds, &format!("{what} sd dCt"));
            close(got.ddct, *ddct, &format!("{what} ddCt"));
            close(got.se_ddct, *se, &format!("{what} se"));
            close(got.ratio, *ratio, &format!("{what} ratio"));
            close(got.ratio_low, *low, &format!("{what} low"));
            close(got.ratio_high, *high, &format!("{what} high"));
            close(got.p, *p, &format!("{what} p"));
        }
    }
    let s = standard_curve(fixtures::QUANTITY, fixtures::CT).unwrap();
    let c = fixtures::CURVE;
    for (got, want, what) in [
        (s.slope, c[0], "slope"),
        (s.intercept, c[1], "intercept"),
        (s.r2, c[2], "R^2"),
        (s.factor, c[3], "factor"),
        (s.efficiency, c[4], "efficiency"),
    ] {
        close(got, want, what);
    }
}
