//! Prob>F: Origin's value is off the exact upper tail (scipy agrees with ours) by up to 4e-6 relative (1.6e-4 for
//! probabilities near 1e-13), so the tolerances there are 1e-5 and 1e-3.
//!
//! Model comparison against Origin's `fitcmpmodel` report (`data/origin_fitcmp.json`, 24 pairs of
//! fits made with `tools/origin-ref/fitcmp-suite*.lines.txt`, tables read by
//! `fitcmp-make-fixture.py`). The inputs are the residual sum of squares, points and parameters
//! Origin printed for each model; everything else is computed here and compared.

use serde_json::Value;
use sheaf_fit::compare::{ModelSummary, Verdict, compare};

fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn close(label: &str, got: f64, want: Option<f64>, rel: f64, bad: &mut Vec<String>) {
    let Some(want) = want else {
        if !got.is_nan() {
            bad.push(format!("{label}: got {got}, Origin leaves it empty"));
        }
        return;
    };
    if (got - want).abs() > rel * want.abs().max(1e-12) + 1e-300 {
        bad.push(format!("{label}: got {got}, Origin {want}"));
    }
}

fn text(v: Verdict, names: &[String]) -> String {
    match v {
        Verdict::Model(i) => names[i].clone(),
        Verdict::Inconclusive => "Inconclusive".into(),
        Verdict::NoConclusion => "No conclusion".into(),
    }
}

#[test]
fn comparison_matches_origin() {
    let t = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/origin_fitcmp.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&t).unwrap();
    assert!(cases.len() >= 24);
    let mut bad = Vec::new();
    for c in &cases {
        let src = c["source"].as_str().unwrap();
        let names: Vec<String> =
            c["names"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect();
        let ms: Vec<ModelSummary> = (0..2)
            .map(|i| ModelSummary {
                name: c["models"][i]["function"].as_str().unwrap().to_string(),
                rss: num(&c["rss"][i]).unwrap(),
                n: c["models"][i]["n"].as_u64().unwrap() as usize,
                params: c["models"][i]["k"].as_u64().unwrap() as usize,
            })
            .collect();
        let r = compare(&ms[0], &ms[1]);
        let mut conclusions = Vec::new();
        if c["use"]["aic"].as_bool().unwrap() {
            for i in 0..2 {
                close(&format!("{src} AIC{i}"), r.aic[i], num(&c["aic"]["aic"][i]), 1e-12, &mut bad);
                let w = num(&c["aic"]["weight"][i]);
                // tiny weights are compared in absolute terms
                if w.is_some_and(|w| w > 1e-10) {
                    close(&format!("{src} weight{i}"), r.aic_weight[i], w, 1e-9, &mut bad);
                }
            }
            conclusions.push(text(r.aic_verdict, &names));
        }
        if c["use"]["bic"].as_bool().unwrap() {
            for i in 0..2 {
                close(&format!("{src} BIC{i}"), r.bic[i], num(&c["bic"]["bic"][i]), 1e-12, &mut bad);
                close(&format!("{src} dBIC{i}"), r.bic_delta[i], num(&c["bic"]["delta"][i]), 1e-9, &mut bad);
            }
            conclusions.push(text(r.bic_verdict, &names));
        }
        if c["use"]["ftest"].as_bool().unwrap() {
            let f = &c["f"];
            close(&format!("{src} F"), r.ftest.f, num(&f["f"]), 1e-9, &mut bad);
            close(&format!("{src} df1"), r.ftest.df1 as f64, num(&f["df1"]), 0.0, &mut bad);
            close(&format!("{src} df2"), r.ftest.df2 as f64, num(&f["df2"]), 0.0, &mut bad);
            let tol = if num(&f["p"]).is_some_and(|p| p < 1e-9) { 1e-3 } else { 1e-5 };
            close(&format!("{src} Prob>F"), r.ftest.prob, num(&f["p"]), tol, &mut bad);
            conclusions.push(text(r.ftest.verdict, &names));
        }
        let want: Vec<String> =
            c["conclusions"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect();
        if conclusions != want {
            bad.push(format!("{src}: conclusions {conclusions:?}, Origin {want:?}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
