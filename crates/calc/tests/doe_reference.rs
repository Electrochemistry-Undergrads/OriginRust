//! Two-level factorial effects against numpy and scipy, made by `tools/doe-ref.py` (T380; none of these numbers come
//! from Origin). Tolerance 1e-10 relative; terms and aliases exact.

#[path = "fixtures/doe_reference.rs"]
mod fixtures;

use sheaf_calc::doe::effects;

fn close(a: f64, b: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else {
        assert!((a - b).abs() <= 1e-10 * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn effects_match_numpy_and_scipy() {
    for c in fixtures::CASES {
        let x: Vec<Vec<f64>> = c.x.iter().map(|r| r.to_vec()).collect();
        let e = effects(&x, c.y).unwrap();
        close(e.intercept, c.intercept, &format!("{} intercept", c.name));
        assert_eq!(e.df, c.df, "{}", c.name);
        let got: Vec<(String, String)> = e.aliased.clone();
        let want: Vec<(String, String)> = c.aliased.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        assert_eq!(got, want, "{} aliases", c.name);
        assert_eq!(e.effects.len(), c.effects.len(), "{}", c.name);
        for (g, (term, coef, eff, se, t, p)) in e.effects.iter().zip(c.effects) {
            let what = format!("{} {term}", c.name);
            assert_eq!(g.term, *term, "{what}");
            close(g.coefficient, *coef, &format!("{what} coefficient"));
            close(g.effect, *eff, &format!("{what} effect"));
            close(g.se, *se, &format!("{what} se"));
            close(g.t, *t, &format!("{what} t"));
            close(g.p, *p, &format!("{what} p"));
        }
        close(e.pse, c.pse, &format!("{} PSE", c.name));
        close(e.margin, c.margin, &format!("{} margin", c.name));
    }
}
