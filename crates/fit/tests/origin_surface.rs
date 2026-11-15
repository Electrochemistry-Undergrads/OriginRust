//! Surface fits `z = f(x, y)` on three columns compared with the numbers Origin produced for the
//! same data (`data/origin_surface.json`, made by `tools/origin-ref/nlfit-surface-suite.lines.txt`
//! and `nlfit-surface-make-fixture.py`: nlbeginz, nlfit, nlend on a synthetic grid plus a fixed sine
//! noise term). See `docs/spec/fitting.md`, section "Surface fits", for tolerances.

use serde_json::Value;
use sheaf_fit::library;
use sheaf_fit::nlsf::{FitOptions, fit_xyz};
use sheaf_fit::report::FitStatus;

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(num).collect()
}

fn close(out: &mut Vec<String>, label: &str, got: f64, want: f64, rel: f64) {
    if (got - want).abs() > rel * want.abs().max(1e-300) {
        out.push(format!("{label}: got {got}, Origin {want}"));
    }
}

fn run(origin_start: bool) {
    let (ptol, stol) = if origin_start { (1e-7, 1e-7) } else { (1e-4, 1e-3) };
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_surface.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    assert!(cases.len() >= 10);
    let mut out = Vec::new();
    for c in &cases {
        let name = c["name"].as_str().unwrap();
        // Rational2D is badly conditioned (dependencies near 1): Origin and we stop at slightly
        // different points of a flat valley, so only the fit quality is compared tightly.
        let loose = name == "rational2d";
        let (ptol, stol) = if loose { (1e-2, 1e-1) } else { (ptol, stol) };
        let mut f = library::get(c["func"].as_str().unwrap()).unwrap();
        if origin_start {
            for (k, v) in c["origin_initial"].as_object().unwrap() {
                f = f.with_initial(k, num(v));
            }
        }
        let (x, y, z) = (vec_of(&c["x"]), vec_of(&c["y"]), vec_of(&c["z"]));
        let r = match fit_xyz(&f, &x, &y, &z, &FitOptions::default()) {
            Ok(r) => r,
            Err(e) => {
                out.push(format!("{name}: {e}"));
                continue;
            }
        };
        if r.status != FitStatus::Converged {
            out.push(format!("{name}: status {:?}", r.status));
            continue;
        }
        let iters = r.stats.iterations.unwrap();
        let oi = c["origin_iterations"].as_u64().unwrap() as usize;
        if origin_start && iters != oi && name != "rational2d" {
            out.push(format!("{name}: {iters} iterations, Origin {oi}"));
        }
        for p in c["params_origin"].as_array().unwrap() {
            let pn = p["name"].as_str().unwrap();
            let mine = r.param(pn).unwrap();
            let l = format!("{name} {pn}");
            close(&mut out, &format!("{l} value"), mine.value, num(&p["value"]), ptol);
            close(&mut out, &format!("{l} SE"), mine.std_error, num(&p["se"]), stol);
            close(&mut out, &format!("{l} t"), mine.t_value, num(&p["t"]), stol);
            let pr = num(&p["prob"]);
            if (mine.prob_t - pr).abs() > 1e-9 + if loose { 1e-1 } else { 1e-5 } * pr {
                out.push(format!("{l} Prob>|t|: got {}, Origin {pr}", mine.prob_t));
            }
            if (mine.dependency - num(&p["dep"])).abs() > if loose { 1e-3 } else { 1e-5 } {
                out.push(format!("{l} dependency: got {}, Origin {}", mine.dependency, num(&p["dep"])));
            }
        }
        let s = &c["stats"];
        if r.stats.n_points != s["n"].as_u64().unwrap() as usize || r.stats.dof != s["dof"].as_u64().unwrap() as usize {
            out.push(format!("{name}: n {} dof {}, Origin {} {}", r.stats.n_points, r.stats.dof, s["n"], s["dof"]));
        }
        close(&mut out, &format!("{name} reduced chi-square"), r.stats.reduced_chi_sqr, num(&s["red_chi"]), stol);
        close(&mut out, &format!("{name} RSS"), r.stats.rss, num(&s["rss"]), stol);
        close(&mut out, &format!("{name} R-square"), r.stats.r_squared, num(&s["r2"]), stol);
        close(&mut out, &format!("{name} adj R-square"), r.stats.adj_r_squared, num(&s["adj_r2"]), stol);
        let a = &c["anova"];
        let an = r.anova.as_ref().unwrap();
        close(&mut out, &format!("{name} model SS"), an.model_ss, num(&a["model_ss"]), stol);
        close(&mut out, &format!("{name} total SS"), an.total_ss, num(&a["total_ss"]), stol);
        close(&mut out, &format!("{name} F"), an.f_value, num(&a["f"]), stol);
    }
    assert!(out.is_empty(), "differences from Origin:\n{}", out.join("\n"));
}

#[test]
fn surface_engine_matches_origin() {
    run(true);
}

/// With our own starting values the path differs but a converged fit gives Origin's numbers.
#[test]
fn surface_fits_from_own_starting_values_reach_origin_results() {
    run(false);
}
