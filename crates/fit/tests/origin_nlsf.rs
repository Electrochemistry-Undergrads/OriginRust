//! Nonlinear fits compared with the numbers Origin produced for the same data.
//!
//! `data/origin_nlsf.json` was made with `tools/origin-ref/` style runs of Origin (nlbegin, nlfit,
//! nlend with a report sheet) on data we generated ourselves: a smooth curve plus a noise term that
//! is a fixed sine formula, so the data are not random. The file stores the data exactly as Origin
//! held them and Origin's report numbers. See `docs/spec/fitting.md`, section "Verified against
//! Origin", for the tolerances and why they are what they are.

use serde_json::Value;
use sheaf_fit::library;
use sheaf_fit::nlsf::{FitOptions, fit};
use sheaf_fit::report::{FitStatus, Weighting};

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(num).collect()
}

fn close(label: &str, got: f64, want: f64, rel: f64) {
    let tol = rel * want.abs().max(1e-300);
    if (got - want).abs() > tol {
        let off = (got - want).abs() / want.abs().max(1e-300);
        FAILURES
            .with(|f| f.borrow_mut().push(format!("{label}: got {got}, Origin {want}, relative difference {off:e}")));
    }
}

fn close_abs(label: &str, got: f64, want: f64, tol: f64) {
    if (got - want).abs() > tol {
        FAILURES
            .with(|f| f.borrow_mut().push(format!("{label}: got {got}, Origin {want}, absolute tolerance {tol:e}")));
    }
}

thread_local! {
    static FAILURES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Relative tolerances. With Origin's starting values our engine takes the same steps as Origin
/// (same iteration counts); the largest relative difference seen in any parameter, standard error
/// or statistic was 4e-10. The tolerance leaves room for different floating-point libraries.
const PARAM_TOL: f64 = 1e-7;
const STAT_TOL: f64 = 1e-7;

#[test]
fn engine_matches_origin() {
    run(true);
}

/// With our own starting-value routines the path differs, but a fit that ends at the minimum gives
/// Origin's numbers to a looser tolerance. Fits with an active bound are left out: Origin stops
/// short of the constrained minimum there, so the result depends on the path.
#[test]
fn own_starting_values_reach_origin_results() {
    run(false);
}

fn run(origin_start: bool) {
    FAILURES.with(|f| f.borrow_mut().clear());
    let (ptol, stol) = if origin_start { (PARAM_TOL, STAT_TOL) } else { (1e-5, 1e-4) };
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_nlsf.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    assert!(cases.len() >= 21);
    for c in &cases {
        let name = c["name"].as_str().unwrap();
        let bounded = c["upper"].is_object() || c["lower"].is_object();
        if !origin_start && (bounded || c["max_iter"].is_u64()) {
            continue;
        }
        let mut f = library::get(c["func"].as_str().unwrap()).unwrap();
        if let Some(m) = c["fixed"].as_object() {
            for (k, v) in m {
                f = f.with_fixed(k, num(v));
            }
        }
        if let Some(m) = c["lower"].as_object() {
            for (k, v) in m {
                f = f.with_bounds(k, Some(num(v)), None);
            }
        }
        if let Some(m) = c["upper"].as_object() {
            for (k, v) in m {
                f = f.with_bounds(k, None, Some(num(v)));
            }
        }
        // Start where Origin started, so that both programs follow the same path.
        for (k, v) in c["origin_initial"].as_object().unwrap().iter().filter(|_| origin_start) {
            if !f.params.iter().any(|p| &p.name == k && p.fixed) {
                f = f.with_initial(k, num(v));
            }
        }
        let x = vec_of(&c["x"]);
        let y = vec_of(&c["y"]);
        let mut opts = FitOptions::default();
        if let Some(m) = c["max_iter"].as_u64() {
            opts.max_iterations = m as usize;
        }
        match c["weight"].as_str() {
            Some("ins") => opts.weighting = Weighting::Instrumental(vec_of(&c["err"])),
            Some("stat") => opts.weighting = Weighting::Statistical,
            _ => {}
        }
        let r = fit(&f, &x, &y, &opts).unwrap_or_else(|e| panic!("{name}: {e}"));
        if c["max_iter"].is_u64() {
            assert_eq!(c["stats"]["status"], "Failed(-200)");
            assert_eq!(r.status, FitStatus::MaxIterations, "{name}");
        } else {
            assert_eq!(c["stats"]["status"], "Succeeded(100)");
            assert_eq!(r.status, FitStatus::Converged, "{name}");
        }
        let iters = r.stats.iterations.unwrap();
        let origin_iters = c["origin_iterations"].as_u64().unwrap() as usize;
        if origin_start && iters != origin_iters {
            FAILURES.with(|f| f.borrow_mut().push(format!("{name}: {iters} iterations, Origin {origin_iters}")));
        }

        for p in c["params_origin"].as_array().unwrap() {
            let pn = p["name"].as_str().unwrap();
            let mine = r.param(pn).unwrap();
            let label = format!("{name} {pn}");
            close(&format!("{label} value"), mine.value, num(&p["value"]), ptol);
            if mine.fixed {
                continue;
            }
            close(&format!("{label} SE"), mine.std_error, num(&p["se"]), stol);
            close(&format!("{label} t"), mine.t_value, num(&p["t"]), stol);
            let pr = num(&p["prob"]);
            close_abs(&format!("{label} Prob>|t|"), mine.prob_t, pr, 1e-9 + 1e-6 * pr);
            close_abs(&format!("{label} dependency"), mine.dependency, num(&p["dep"]), 1e-6);
        }

        let s = &c["stats"];
        assert_eq!(r.stats.n_points, s["n"].as_u64().unwrap() as usize, "{name} points");
        assert_eq!(r.stats.dof, s["dof"].as_u64().unwrap() as usize, "{name} dof");
        close(&format!("{name} reduced chi-square"), r.stats.reduced_chi_sqr, num(&s["red_chi"]), stol);
        close(&format!("{name} RSS"), r.stats.rss, num(&s["rss"]), stol);
        close(&format!("{name} R-square"), r.stats.r_squared, num(&s["r2"]), stol);
        close(&format!("{name} adjusted R-square"), r.stats.adj_r_squared, num(&s["adj_r2"]), stol);

        let a = &c["anova"];
        let an = r.anova.as_ref().unwrap_or_else(|| panic!("{name}: no ANOVA"));
        assert_eq!(an.model_df, a["model_df"].as_u64().unwrap() as usize, "{name} model df");
        assert_eq!(an.total_df, a["total_df"].as_u64().unwrap() as usize, "{name} total df");
        close(&format!("{name} model SS"), an.model_ss, num(&a["model_ss"]), stol);
        close(&format!("{name} total SS"), an.total_ss, num(&a["total_ss"]), stol);
        close(&format!("{name} uncorrected total SS"), an.uncorrected_total_ss, num(&a["utotal_ss"]), stol);
        close(&format!("{name} F"), an.f_value, num(&a["f"]), stol);
    }
    let failures = FAILURES.with(|f| f.borrow().clone());
    assert!(
        failures.is_empty(),
        "differences from Origin:
{}",
        failures.join(
            "
"
        )
    );
}

/// A user-defined function typed as a formula gives the same fit as the built-in function of the
/// same equation: here Gauss, written out, against Origin's report for the built-in.
#[test]
fn user_defined_formula_matches_origin_builtin() {
    use sheaf_fit::nlsf::FitFunction;
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_nlsf.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    let c = cases.iter().find(|c| c["name"] == "gauss").unwrap();
    let mut f = FitFunction::from_formula(
        "User defined Gauss",
        "y = y0 + (A/(w*sqrt(pi/2)))*exp(-2*((x-xc)/w)^2)",
        &["y0", "xc", "w", "A"],
    )
    .unwrap();
    for (k, v) in c["origin_initial"].as_object().unwrap() {
        f = f.with_initial(k, num(v));
    }
    let r = fit(&f, &vec_of(&c["x"]), &vec_of(&c["y"]), &FitOptions::default()).unwrap();
    assert_eq!(r.stats.iterations.unwrap() as u64, c["origin_iterations"].as_u64().unwrap());
    for p in c["params_origin"].as_array().unwrap() {
        let mine = r.param(p["name"].as_str().unwrap()).unwrap();
        close("user Gauss value", mine.value, num(&p["value"]), PARAM_TOL);
        close("user Gauss SE", mine.std_error, num(&p["se"]), STAT_TOL);
    }
    close("user Gauss RSS", r.stats.rss, num(&c["stats"]["rss"]), STAT_TOL);
    let failures = FAILURES.with(|f| f.borrow().clone());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn user_defined_formula_with_fixed_and_bounded_parameters() {
    use sheaf_fit::nlsf::FitFunction;
    let x: Vec<f64> = (1..=30).map(|i| i as f64 * 0.3).collect();
    let y: Vec<f64> = x.iter().map(|x| 1.5 + 4.0 * (-x / 2.0).exp()).collect();
    let f = FitFunction::from_formula("U", "y0 + A*exp(-x/t)", &["y0", "A", "t"])
        .unwrap()
        .with_fixed("y0", 1.5)
        .with_bounds("t", Some(0.5), Some(10.0));
    let r = fit(&f, &x, &y, &FitOptions::default()).unwrap();
    assert!((r.param("A").unwrap().value - 4.0).abs() < 1e-6);
    assert!((r.param("t").unwrap().value - 2.0).abs() < 1e-6);
    assert!(r.param("y0").unwrap().fixed);
    assert_eq!(r.stats.dof, 28);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
}
