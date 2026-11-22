//! G and T control chart limits against scipy, made by `tools/rarechart-ref.py` (T364; none of these numbers come
//! from Origin). Tolerance 1e-9 relative (1e-7 for the Weibull fit, solved by a different method).

#[path = "fixtures/rarechart_reference.rs"]
mod fixtures;

use sheaf_calc::rarechart::{GLimits, TDistribution, g_chart, t_chart};

fn close(a: f64, b: f64, rel: f64, what: &str) {
    assert!((a - b).abs() <= rel * b.abs().max(1.0), "{what}: {a} vs {b}");
}

#[test]
fn g_chart_limits_match_scipy() {
    for (name, x, r) in fixtures::G {
        let s = g_chart(x, GLimits::Sigma, 3.0).unwrap();
        let p = g_chart(x, GLimits::Probability, 3.0).unwrap();
        close(s.params[0], r[0], 1e-12, name);
        for (got, want, what) in [
            (s.center, r[1], "sigma CL"),
            (s.ucl, r[2], "sigma UCL"),
            (s.lcl, r[3], "sigma LCL"),
            (p.center, r[4], "CL"),
            (p.ucl, r[5], "UCL"),
            (p.lcl, r[6], "LCL"),
        ] {
            close(got, want, 1e-9, &format!("{name} {what}"));
        }
    }
}

#[test]
fn t_chart_limits_match_scipy() {
    for (name, x, e, w) in fixtures::T {
        let c = t_chart(x, TDistribution::Exponential, 3.0).unwrap();
        for (got, want, what) in
            [(c.params[0], e[0], "mean"), (c.center, e[1], "CL"), (c.ucl, e[2], "UCL"), (c.lcl, e[3], "LCL")]
        {
            close(got, want, 1e-9, &format!("{name} exponential {what}"));
        }
        let c = t_chart(x, TDistribution::Weibull, 3.0).unwrap();
        for (got, want, what) in [
            (c.params[0], w[0], "shape"),
            (c.params[1], w[1], "scale"),
            (c.center, w[2], "CL"),
            (c.ucl, w[3], "UCL"),
            (c.lcl, w[4], "LCL"),
        ] {
            close(got, want, 1e-7, &format!("{name} Weibull {what}"));
        }
    }
}
