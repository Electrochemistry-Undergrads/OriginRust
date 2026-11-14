//! The Simplex algorithm against Origin's `nlfit method:=s` on the 21 cases of `origin_nlsf.json`
//! (same data and starting values; `data/origin_simplex.json`, made with
//! `tools/origin-ref/nlfit-make-fixture.py` from the suite script with `nlfit method:=s;`).
//!
//! Origin's simplex stops near the minimum (parameters 1e-7 to 1e-5 from the Levenberg-Marquardt
//! result), reports the status "(-1)" and an iteration count of 0, and, unlike its
//! Levenberg-Marquardt, reaches the true minimum when a bound is active. So the comparison is:
//! where Origin's residual sum of squares equals the minimum, the parameters and statistics agree
//! with a loose tolerance; everywhere else our fit must be at least as good as Origin's.

use serde_json::Value;
use sheaf_fit::library;
use sheaf_fit::nlsf::{Algorithm, FitOptions, fit};
use sheaf_fit::report::{FitStatus, Weighting};

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(num).collect()
}

#[test]
fn simplex_reaches_origins_simplex_results() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_simplex.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    assert_eq!(cases.len(), 21);
    let mut at_minimum = 0;
    let mut problems = Vec::new();
    for c in &cases {
        let name = c["name"].as_str().unwrap();
        if c["max_iter"].is_u64() {
            continue; // Origin's iteration unit is not documented; see fitting.md
        }
        let mut f = library::get(c["func"].as_str().unwrap()).unwrap();
        for (key, set) in [("fixed", 0), ("lower", 1), ("upper", 2)] {
            if let Some(m) = c[key].as_object() {
                for (k, v) in m {
                    f = match set {
                        0 => f.with_fixed(k, num(v)),
                        1 => f.with_bounds(k, Some(num(v)), None),
                        _ => f.with_bounds(k, None, Some(num(v))),
                    };
                }
            }
        }
        for (k, v) in c["origin_initial"].as_object().unwrap() {
            if !f.params.iter().any(|p| &p.name == k && p.fixed) {
                f = f.with_initial(k, num(v));
            }
        }
        let mut opts = FitOptions { algorithm: Algorithm::Simplex, ..Default::default() };
        match c["weight"].as_str() {
            Some("ins") => opts.weighting = Weighting::Instrumental(vec_of(&c["err"])),
            Some("stat") => opts.weighting = Weighting::Statistical,
            _ => {}
        }
        let r = fit(&f, &vec_of(&c["x"]), &vec_of(&c["y"]), &opts).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(r.status, FitStatus::Converged, "{name}");
        let origin_rss = num(&c["stats"]["rss"]);
        if r.stats.rss > origin_rss * (1.0 + 1e-9) {
            problems.push(format!("{name}: rss {} worse than Origin's {origin_rss}", r.stats.rss));
            continue;
        }
        if (r.stats.rss - origin_rss).abs() > 1e-6 * origin_rss {
            continue; // Origin stopped short of the minimum (ill-conditioned or bounded)
        }
        at_minimum += 1;
        for p in c["params_origin"].as_array().unwrap() {
            let pn = p["name"].as_str().unwrap();
            let (mine, theirs) = (r.param(pn).unwrap().value, num(&p["value"]));
            if (mine - theirs).abs() > 2e-5 * theirs.abs().max(1e-9) {
                problems.push(format!("{name} {pn}: {mine} against Origin's {theirs}"));
            }
        }
        for (what, mine, theirs) in [
            ("R-square", r.stats.r_squared, num(&c["stats"]["r2"])),
            ("reduced chi-square", r.stats.reduced_chi_sqr, num(&c["stats"]["red_chi"])),
        ] {
            if (mine - theirs).abs() > 1e-6 * theirs.abs() {
                problems.push(format!("{name} {what}: {mine} against Origin's {theirs}"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(at_minimum >= 12, "only {at_minimum} cases were compared parameter by parameter");
}

/// Both algorithms end at the same minimum of the same fit; a bound that is active is kept.
#[test]
fn simplex_agrees_with_levenberg_marquardt_and_keeps_bounds() {
    let x: Vec<f64> = (1..=30).map(|i| i as f64 * 0.3).collect();
    let y: Vec<f64> = x.iter().map(|x| 1.5 + 4.0 * (-x / 2.0).exp() + 0.02 * (7.0 * x).sin()).collect();
    let f = library::get("ExpDec1").unwrap();
    let lm = fit(&f, &x, &y, &FitOptions::default()).unwrap();
    let sx = fit(&f, &x, &y, &FitOptions { algorithm: Algorithm::Simplex, ..Default::default() }).unwrap();
    for n in ["y0", "A1", "t1"] {
        let (a, b) = (lm.param(n).unwrap().value, sx.param(n).unwrap().value);
        assert!((a - b).abs() < 1e-5 * a.abs(), "{n}: {a} {b}");
    }
    let fb = library::get("ExpDec1").unwrap().with_bounds("t1", None, Some(1.5));
    let r = fit(&fb, &x, &y, &FitOptions { algorithm: Algorithm::Simplex, ..Default::default() }).unwrap();
    assert!((r.param("t1").unwrap().value - 1.5).abs() < 1e-12);
    assert!(r.stats.rss > sx.stats.rss);
}

/// One iteration of the simplex does not converge a fit; `Fitter` steps it the same way.
#[test]
fn simplex_iterations_can_be_stepped() {
    use sheaf_fit::nlsf::Fitter;
    let x: Vec<f64> = (1..=30).map(|i| i as f64 * 0.3).collect();
    let y: Vec<f64> = x.iter().map(|x| 1.5 + 4.0 * (-x / 2.0).exp()).collect();
    let f = library::get("ExpDec1").unwrap().with_initial("y0", 0.0).with_initial("A1", 1.0).with_initial("t1", 1.0);
    let mut fitter =
        Fitter::new(&f, &x, &y, &FitOptions { algorithm: Algorithm::Simplex, ..Default::default() }).unwrap();
    let first = fitter.step();
    assert!(!first.converged && first.iteration == 1);
    let second = fitter.step();
    assert!(second.chi2 <= first.chi2);
}
