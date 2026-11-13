//! The automatic axis range against numbers measured in Origin (graphing spec section 4).
//! The data are in `data/origin_axis_range.json`: `[lo, hi, from, to, inc]` per case.

use sheaf_core::autorange::{LogTicks, linear_range, log_linear_step, log_range};

fn cases(key: &str) -> Vec<[f64; 5]> {
    let text = include_str!("data/origin_axis_range.json");
    let v: serde_json::Value = serde_json::from_str(text).unwrap();
    v[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let a = r.as_array().unwrap();
            [0, 1, 2, 3, 4].map(|i| a[i].as_f64().unwrap())
        })
        .collect()
}

fn near(a: f64, b: f64, scale: f64) -> bool {
    (a - b).abs() <= 1e-9 * scale.abs().max(1e-300)
}

#[test]
fn linear_cases_match_origin() {
    let all = cases("linear");
    assert!(all.len() >= 400, "{} cases", all.len());
    let mut bad = Vec::new();
    for [lo, hi, from, to, inc] in &all {
        let (f, t, s) = linear_range(*lo, *hi, 8.0);
        let scale = hi - lo;
        if !(near(f, *from, scale) && near(t, *to, scale) && near(s, *inc, *inc)) {
            bad.push(format!("{lo}..{hi}: got {f}..{t} step {s}, Origin {from}..{to} step {inc}"));
        }
    }
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), all.len(), bad.join("\n"));
}

#[test]
fn log_cases_match_origin() {
    let all = cases("log");
    assert!(all.len() >= 130, "{} cases", all.len());
    let mut bad = Vec::new();
    for [lo, hi, from, to, inc] in &all {
        let r = log_range(*lo, *hi, 8.0);
        let inc_ok = match r.ticks {
            LogTicks::Decades => *inc == 1.0,
            LogTicks::Linear(s) => near(s, *inc, *inc),
        };
        if !(near(r.from, *from, *from) && near(r.to, *to, *to) && inc_ok) {
            bad.push(format!("{lo}..{hi}: got {}..{} {:?}, Origin {from}..{to} inc {inc}", r.from, r.to, r.ticks));
        }
    }
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), all.len(), bad.join("\n"));
}

#[test]
fn margin_zero_gives_the_data_limits() {
    // Origin with Rescale Margin 0: 0..10 gives exactly 0..10 with step 5, 0..30 gives 0..30 with step 2.
    assert_eq!(linear_range(0.0, 10.0, 0.0), (0.0, 10.0, 5.0));
    assert_eq!(linear_range(0.0, 30.0, 0.0), (0.0, 30.0, 2.0));
    assert_eq!(linear_range(0.0, 60.0, 0.0), (0.0, 60.0, 20.0));
    // Margins from 2 to 16 % keep the limits of the default for 0..10 (measured).
    for m in [2.0, 4.0, 8.0, 12.0, 16.0, 20.0] {
        assert_eq!(linear_range(0.0, 10.0, m), (-2.5, 12.5, 5.0), "margin {m}");
    }
    assert_eq!(linear_range(0.0, 30.0, 4.0), (-2.0, 32.0, 2.0));
    assert_eq!(linear_range(0.0, 30.0, 16.0), (-5.0, 35.0, 2.0));
}

#[test]
fn log_linear_step_table() {
    assert_eq!(log_linear_step(0.116), 0.05);
    assert_eq!(log_linear_step(1.168), 0.5);
    assert_eq!(log_linear_step(5.8), 1.0);
    assert_eq!(log_linear_step(23.36), 5.0);
    assert_eq!(log_linear_step(47.19), 10.0);
    assert_eq!(log_linear_step(94.9), 20.0);
}
