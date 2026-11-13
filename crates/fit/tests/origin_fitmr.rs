//! Multiple linear regression against Origin's `fitMR` (`data/origin_fitmr.json`, made with
//! `tools/origin-ref/fitmr-gen.py`): free intercept with one, two and three independent variables,
//! and an intercept held at 0, 1.5 and -2. Origin's tree gives the coefficients, standard errors,
//! t-values and probabilities, the sums of squares, mean squares, RMSE, degrees of freedom, R-square,
//! R, F and Prob>F. (Adjusted R-square and the confidence limits are not in the tree.)

use serde_json::Value;
use sheaf_fit::multi::{MultiOptions, fit_multiple};

fn get(o: &Value, k: &str) -> Option<f64> {
    o.get(k).and_then(Value::as_f64)
}

fn near(label: &str, got: f64, want: f64, rel: f64, abs: f64, bad: &mut Vec<String>) {
    if (got - want).abs() > rel * want.abs() + abs {
        bad.push(format!("{label}: got {got}, Origin {want}"));
    }
}

#[test]
fn multiple_regression_matches_origin() {
    let t = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_fitmr.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&t).unwrap();
    assert_eq!(cases.len(), 6);
    let mut bad = Vec::new();
    for c in &cases {
        let name = c["name"].as_str().unwrap();
        let xs: Vec<Vec<f64>> = c["x"]
            .as_array()
            .unwrap()
            .iter()
            .map(|col| col.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect())
            .collect();
        let y: Vec<f64> = c["y"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        let opts = MultiOptions { fix_intercept: c["fix"].as_f64(), ..Default::default() };
        let r = fit_multiple(&xs, &[], &y, &opts).unwrap();
        let o = &c["origin"];
        let free = c["fix"].is_null();
        // Origin numbers the coefficients from 1: the intercept, then the variables.
        for (i, p) in r.params.iter().enumerate() {
            let k = i + 1;
            near(&format!("{name} v{k}"), p.value, get(o, &format!("v{k}")).unwrap(), 1e-9, 1e-12, &mut bad);
            if i == 0 && !free {
                continue;
            }
            near(&format!("{name} e{k}"), p.std_error, get(o, &format!("e{k}")).unwrap(), 1e-8, 0.0, &mut bad);
            near(&format!("{name} t{k}"), p.t_value, get(o, &format!("t{k}")).unwrap(), 1e-8, 0.0, &mut bad);
            near(&format!("{name} p{k}"), p.prob_t, get(o, &format!("p{k}")).unwrap(), 1e-6, 1e-12, &mut bad);
        }
        let a = r.anova.as_ref().unwrap();
        near(&format!("{name} SSE"), r.stats.rss, get(o, "SSE").unwrap(), 1e-9, 0.0, &mut bad);
        near(&format!("{name} SST"), a.total_ss, get(o, "SST").unwrap(), 1e-9, 0.0, &mut bad);
        near(&format!("{name} ssr"), a.model_ss, get(o, "ssr").unwrap(), 1e-8, 0.0, &mut bad);
        near(&format!("{name} MSR"), a.model_ms, get(o, "MSR").unwrap(), 1e-8, 0.0, &mut bad);
        near(&format!("{name} MSE"), a.error_ms, get(o, "MSE").unwrap(), 1e-9, 0.0, &mut bad);
        near(&format!("{name} RMSE"), r.stats.reduced_chi_sqr.sqrt(), get(o, "RMSE").unwrap(), 1e-9, 0.0, &mut bad);
        near(&format!("{name} DOF"), r.stats.dof as f64, get(o, "DOF").unwrap(), 0.0, 0.0, &mut bad);
        near(&format!("{name} COD"), r.stats.r_squared, get(o, "COD").unwrap(), 1e-9, 0.0, &mut bad);
        near(&format!("{name} R"), r.stats.pearson_r.unwrap(), get(o, "Rvalue").unwrap(), 1e-9, 0.0, &mut bad);
        near(&format!("{name} Npts"), r.stats.n_points as f64, get(o, "Npts").unwrap(), 0.0, 0.0, &mut bad);
        near(&format!("{name} F"), a.f_value, get(o, "Fvalue").unwrap(), 1e-8, 0.0, &mut bad);
        near(&format!("{name} Prob>F"), a.prob_f, get(o, "PValue").unwrap(), 1e-6, 1e-12, &mut bad);
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
