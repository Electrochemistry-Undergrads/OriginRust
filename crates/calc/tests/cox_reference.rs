//! Cox proportional hazards against an independent numpy/scipy fit, made by `tools/cox-ref.py` (T408; none of these
//! numbers come from Origin). Tolerance 1e-8 relative (the reference optimizer's tolerance). Breslow and (T445) Efron
//! ties.

#[path = "fixtures/cox_reference.rs"]
mod fixtures;

use sheaf_calc::cox::{Ties, cox_with};

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= 1e-8 * b.abs().max(1.0), "{what}: {a} vs {b}");
}

#[test]
fn cox_matches_numpy_and_scipy() {
    let n = fixtures::T.len();
    for c in fixtures::CASES {
        let x: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                if c.b.len() == 3 {
                    vec![fixtures::AGE[i], fixtures::TREATED[i], fixtures::DOSE[i]]
                } else {
                    vec![fixtures::TREATED[i]]
                }
            })
            .collect();
        let ties = if c.efron { Ties::Efron } else { Ties::Breslow };
        let f = cox_with(&x, fixtures::T, fixtures::FAILED, 0.95, ties).unwrap();
        for i in 0..c.b.len() {
            close(f.coefficients[i], c.b[i], &format!("{} b{i}", c.name));
            close(f.se[i], c.se[i], &format!("{} se{i}", c.name));
            close(f.z[i], c.z[i], &format!("{} z{i}", c.name));
            close(f.p[i], c.p[i], &format!("{} p{i}", c.name));
        }
        close(f.log_likelihood, c.ll, &format!("{} ll", c.name));
        close(f.log_likelihood_null, c.ll0, &format!("{} ll0", c.name));
        for (got, want, what) in
            [(f.lr, c.tests[0], "LR"), (f.wald, c.tests[1], "Wald"), (f.score, c.tests[2], "score")]
        {
            close(got.0, want.0, &format!("{} {what}", c.name));
            close(got.1, want.1, &format!("{} {what} p", c.name));
        }
        assert_eq!(f.times.len(), c.times.len(), "{}", c.name);
        for (i, (a, b)) in f.times.iter().zip(c.times).enumerate() {
            close(*a, *b, &format!("{} time {i}", c.name));
            close(f.survival[i], c.survival[i], &format!("{} survival {i}", c.name));
        }
    }
}
