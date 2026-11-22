//! ICH Q1E stability studies against numpy/scipy, made by `tools/stability-ref.py` (T368; none of these numbers
//! come from Origin). Tolerance 1e-9 relative (shelf lives 1e-8).

#[path = "fixtures/stability_reference.rs"]
mod fixtures;

use sheaf_calc::stability::{StabilityModel, StabilityOptions, stability};

fn close(a: f64, b: f64, rel: f64, what: &str) {
    if b.is_nan() {
        assert!(a.is_nan(), "{what}: {a} vs NaN");
    } else if b.is_infinite() {
        assert_eq!(a, b, "{what}");
    } else {
        assert!((a - b).abs() <= rel * b.abs().max(1.0), "{what}: {a} vs {b}");
    }
}

#[test]
fn stability_studies_match_numpy_and_scipy() {
    for c in fixtures::CASES {
        let s = stability(c.data, StabilityOptions { lower: c.lower, upper: c.upper, ..Default::default() }).unwrap();
        let model = match c.model {
            "separate" => StabilityModel::Separate,
            "common" => StabilityModel::CommonSlope,
            _ => StabilityModel::Pooled,
        };
        assert_eq!(s.model, model, "{}", c.name);
        close(s.p_slopes, c.p_slopes, 1e-9, &format!("{} p slopes", c.name));
        close(s.p_intercepts, c.p_intercepts, 1e-9, &format!("{} p intercepts", c.name));
        close(s.mse, c.mse, 1e-9, &format!("{} MSE", c.name));
        assert_eq!(s.df, c.df, "{}", c.name);
        close(s.t, c.t, 1e-9, &format!("{} t", c.name));
        assert_eq!(s.batches.len(), c.batches.len(), "{}", c.name);
        for (i, (got, want)) in s.batches.iter().zip(c.batches).enumerate() {
            close(got.intercept, want.0, 1e-9, &format!("{} batch {i} intercept", c.name));
            close(got.slope, want.1, 1e-9, &format!("{} batch {i} slope", c.name));
            close(got.shelf_life, want.2, 1e-8, &format!("{} batch {i} shelf life", c.name));
        }
    }
}
