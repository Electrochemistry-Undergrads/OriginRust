//! Global fits (several datasets, some parameters shared) compared with the NLFit of Origin in
//! "Global Fit" mode. `data/origin_global.json` comes from `tools/origin-ref/nlfit-global-gen.py`;
//! the data are generated (a smooth curve plus fixed sine terms). Origin's report gives no
//! iteration count for these fits, so the comparison is of the results, started from Origin's own
//! starting values. See `docs/spec/fitting.md`, section 10.

use serde_json::Value;
use sheaf_fit::global::{GlobalDataset, GlobalOptions, GlobalWeighting, ParamOverride, dataset_param_name, fit_global};
use sheaf_fit::library;
use sheaf_fit::nlsf::{FitFunction, FitOptions, fit};
use sheaf_fit::report::FitStatus;

/// Both programs stop a few digits short of the exact minimum, so results agree to about 1e-9.
const TOL: f64 = 1e-6;

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(num).collect()
}

fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_global.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn datasets(c: &Value) -> Vec<GlobalDataset> {
    let x = vec_of(&c["x"]);
    c["ys"].as_array().unwrap().iter().map(|y| GlobalDataset { x: x.clone(), y: vec_of(y) }).collect()
}

fn options(c: &Value, f: &FitFunction) -> GlobalOptions {
    let share: Vec<&str> = c["share"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
    let mut o = GlobalOptions::new(f.params.iter().map(|p| share.contains(&p.name.as_str())).collect());
    let nd = c["nsets"].as_u64().unwrap() as usize;
    // Origin's starting values for every copy of every parameter, and fixed values by name.
    for (j, p) in f.params.iter().enumerate() {
        for d in 0..nd {
            if share.contains(&p.name.as_str()) && d > 0 {
                continue;
            }
            let name = dataset_param_name(&p.name, d);
            let fixed = c["fixed"].get(&name).map(num);
            o.overrides.push(ParamOverride {
                dataset: d,
                param: j,
                initial: Some(fixed.unwrap_or_else(|| num(&c["origin_initial"][&name]))),
                fixed: fixed.map(|_| true),
            });
        }
    }
    if c["weight"].as_str() == Some("ins") {
        o.weighting = GlobalWeighting::Instrumental(c["errs"].as_array().unwrap().iter().map(vec_of).collect());
    }
    o
}

fn close(label: &str, got: f64, want: f64, rel: f64, bad: &mut Vec<String>) {
    if (got - want).abs() > rel * want.abs().max(1e-300) {
        bad.push(format!(
            "{label}: got {got}, Origin {want}, relative difference {:e}",
            (got - want).abs() / want.abs()
        ));
    }
}

#[test]
fn global_fits_match_origin() {
    let mut bad = Vec::new();
    let all = cases();
    assert!(all.len() >= 6);
    for c in &all {
        let name = c["name"].as_str().unwrap();
        let f = library::get(c["func"].as_str().unwrap()).unwrap();
        let ds = datasets(c);
        let g = fit_global(&f, &ds, &options(c, &f)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(g.combined.status, FitStatus::Converged, "{name}");
        assert_eq!(g.datasets.len(), ds.len());
        for p in c["params_origin"].as_array().unwrap() {
            let pn = p["name"].as_str().unwrap();
            let mine = g.combined.param(pn).unwrap_or_else(|| panic!("{name}: no parameter {pn}"));
            let label = format!("{name} {pn}");
            close(&format!("{label} value"), mine.value, num(&p["value"]), TOL, &mut bad);
            if mine.fixed {
                assert_eq!(mine.std_error, 0.0, "{label}");
                continue;
            }
            close(&format!("{label} SE"), mine.std_error, num(&p["se"]), TOL, &mut bad);
            close(&format!("{label} t"), mine.t_value, num(&p["t"]), TOL, &mut bad);
            if (mine.dependency - num(&p["dep"])).abs() > 1e-5 {
                bad.push(format!("{label} dependency: got {}, Origin {}", mine.dependency, num(&p["dep"])));
            }
            // The same parameter seen from its dataset.
            let d = p["dataset"].as_u64().unwrap() as usize;
            let j = f.params.iter().position(|q| q.name == p["param"].as_str().unwrap()).unwrap();
            assert_eq!(g.datasets[d].params[j].value, mine.value, "{label} dataset view");
            assert_eq!(g.datasets[d].params[j].name, f.params[j].name);
        }
        let s = &c["stats"];
        let r = &g.combined.stats;
        assert_eq!(r.n_points, s["n"].as_u64().unwrap() as usize, "{name} points");
        assert_eq!(r.dof, s["dof"].as_u64().unwrap() as usize, "{name} dof");
        close(&format!("{name} reduced chi-square"), r.reduced_chi_sqr, num(&s["red_chi"]), TOL, &mut bad);
        close(&format!("{name} RSS"), r.rss, num(&s["rss"]), TOL, &mut bad);
        close(&format!("{name} R-square"), r.r_squared, num(&s["r2"]), TOL, &mut bad);
        close(&format!("{name} adjusted R-square"), r.adj_r_squared, num(&s["adj_r2"]), TOL, &mut bad);
        for (d, set) in ds.iter().enumerate() {
            let v = &g.datasets[d].stats;
            assert_eq!(v.n_points, set.x.len(), "{name} dataset {d} points");
            close(&format!("{name} dataset {d} RSS"), v.rss, num(&s["ds_rss"][d]), TOL, &mut bad);
            close(&format!("{name} dataset {d} R-square"), v.r_squared, num(&s["ds_r2"][d]), TOL, &mut bad);
        }
        let a = &c["anova"];
        let an = g.combined.anova.as_ref().unwrap();
        assert_eq!(an.model_df, a["model_df"].as_u64().unwrap() as usize, "{name} model df");
        assert_eq!(an.error_df, a["error_df"].as_u64().unwrap() as usize, "{name} error df");
        assert_eq!(an.total_df, a["total_df"].as_u64().unwrap() as usize, "{name} corrected total df");
        close(&format!("{name} model SS"), an.model_ss, num(&a["model_ss"]), TOL, &mut bad);
        close(&format!("{name} corrected total SS"), an.total_ss, num(&a["total_ss"]), TOL, &mut bad);
        close(&format!("{name} uncorrected total SS"), an.uncorrected_total_ss, num(&a["utotal_ss"]), TOL, &mut bad);
        close(&format!("{name} F"), an.f_value, num(&a["f"]), TOL, &mut bad);
    }
    assert!(bad.is_empty(), "differences from Origin:\n{}", bad.join("\n"));
}

/// With nothing shared, a global fit finds the parameter values of the fits of each dataset alone
/// (the standard errors differ: the reduced chi-square is that of all the points).
#[test]
fn global_fit_without_sharing_equals_independent_fits() {
    let all = cases();
    let c = all.iter().find(|c| c["name"] == "gauss_no_share").unwrap();
    let f = library::get("Gauss").unwrap();
    let ds = datasets(c);
    let g = fit_global(&f, &ds, &options(c, &f)).unwrap();
    for (d, set) in ds.iter().enumerate() {
        let mut start = f.clone();
        for p in ["y0", "xc", "w", "A"] {
            start = start.with_initial(p, num(&c["origin_initial"][dataset_param_name(p, d)]));
        }
        let alone = fit(&start, &set.x, &set.y, &FitOptions::default()).unwrap();
        for p in &alone.params {
            let mine = g.datasets[d].param(&p.name).unwrap();
            assert!(
                (mine.value - p.value).abs() <= 1e-6 * p.value.abs().max(1e-3),
                "dataset {d} {}: {} against {}",
                p.name,
                mine.value,
                p.value
            );
        }
    }
}

/// Direct weights `w` give the fit of instrumental errors `1/sqrt(w)` (Origin's instrumental weight
/// is 1/err^2 and a direct weight is used as given), also in a global fit.
#[test]
fn direct_weights_equal_instrumental_errors() {
    let all = cases();
    let c = all.iter().find(|c| c["name"] == "gauss_share_xc_instr").unwrap();
    let f = library::get("Gauss").unwrap();
    let ds = datasets(c);
    let ins = options(c, &f);
    let mut direct = ins.clone();
    let GlobalWeighting::Instrumental(errs) = &ins.weighting else { panic!("the case has errors") };
    direct.weighting =
        GlobalWeighting::Direct(errs.iter().map(|e| e.iter().map(|e| 1.0 / (e * e)).collect()).collect());
    let a = fit_global(&f, &ds, &ins).unwrap();
    let b = fit_global(&f, &ds, &direct).unwrap();
    for (p, q) in a.combined.params.iter().zip(&b.combined.params) {
        assert!((p.value - q.value).abs() <= 1e-9 * p.value.abs().max(1e-9));
        assert!((p.std_error - q.std_error).abs() <= 1e-9 * p.std_error.abs().max(1e-9));
    }
}

#[test]
fn bad_input_is_refused_with_a_message() {
    let all = cases();
    let c = &all[0];
    let f = library::get("Gauss").unwrap();
    let ds = datasets(c);
    let mut o = options(c, &f);
    o.shared.pop();
    assert!(fit_global(&f, &ds, &o).unwrap_err().to_string().contains("sharing flags"));
    assert!(fit_global(&f, &[], &options(c, &f)).unwrap_err().to_string().contains("at least one dataset"));
    let mut o = options(c, &f);
    o.weighting = GlobalWeighting::Direct(vec![vec![1.0; 20]]);
    assert!(fit_global(&f, &ds, &o).is_err());
}
