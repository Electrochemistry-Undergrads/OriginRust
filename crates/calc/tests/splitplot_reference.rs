//! Two-stratum split-plot effects against numpy and scipy, made by `tools/splitplot-ref.py` (T382; none of these
//! numbers come from Origin). Tolerance 1e-10 relative; strata and degrees of freedom exact.

#[path = "fixtures/splitplot_reference.rs"]
mod fixtures;

use sheaf_calc::doe::split_plot_effects;

fn close(a: f64, b: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else {
        assert!((a - b).abs() <= 1e-10 * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn split_plot_strata_match_numpy_and_scipy() {
    for c in fixtures::CASES {
        let x: Vec<Vec<f64>> = c.x.iter().map(|r| r.to_vec()).collect();
        let s = split_plot_effects(&x, c.y, c.plot, c.wp).unwrap();
        assert_eq!(s.fit.effects.len(), c.effects.len(), "{}", c.name);
        for ((e, w), (term, whole, eff, se, t, p)) in s.fit.effects.iter().zip(&s.whole).zip(c.effects) {
            let what = format!("{} {term}", c.name);
            assert_eq!((e.term.as_str(), *w), (*term, *whole), "{what}");
            close(e.effect, *eff, &format!("{what} effect"));
            close(e.se, *se, &format!("{what} se"));
            close(e.t, *t, &format!("{what} t"));
            close(e.p, *p, &format!("{what} p"));
        }
        close(s.whole_plot.ss, c.whole_plot.0, &format!("{} whole-plot SS", c.name));
        assert_eq!(s.whole_plot.df, c.whole_plot.1, "{}", c.name);
        close(s.whole_plot.margin, c.whole_plot.2, &format!("{} whole-plot margin", c.name));
        close(s.subplot.ss, c.subplot.0, &format!("{} subplot SS", c.name));
        assert_eq!(s.subplot.df, c.subplot.1, "{}", c.name);
        close(s.subplot.margin, c.subplot.2, &format!("{} subplot margin", c.name));
    }
}
