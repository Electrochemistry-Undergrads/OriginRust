//! Confidence intervals of Pp and Ppk against numpy and scipy, made by `tools/capability-ci-ref.py` (T425; none of
//! these numbers come from Origin). To 1e-9.

#[path = "fixtures/capability_ci_reference.rs"]
mod fixtures;

use sheaf_calc::capability::{CapModel, CapOptions, capability_with};

fn close(a: f64, b: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else {
        assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn pp_and_ppk_intervals_match_scipy() {
    for (name, x, lsl, usl, level, lambda, r) in fixtures::CASES {
        let model = if lambda.is_some() { CapModel::BoxCox } else { CapModel::Normal };
        let opts = CapOptions { lambda: *lambda, confidence: *level, ..Default::default() };
        let c = capability_with(x, *lsl, *usl, 1, model, opts).unwrap();
        for (got, want, what) in [
            (c.pp, r[0], "Pp"),
            (c.pp_ci.0, r[1], "Pp lower"),
            (c.pp_ci.1, r[2], "Pp upper"),
            (c.ppk, r[3], "Ppk"),
            (c.ppk_ci.0, r[4], "Ppk lower"),
            (c.ppk_ci.1, r[5], "Ppk upper"),
        ] {
            close(got, want, &format!("{name} {what}"));
        }
    }
}
