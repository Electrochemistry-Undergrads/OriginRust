//! Classical seasonal decomposition against numpy, made by `tools/decompose-ref.py` (T370; none of these numbers
//! come from Origin). Tolerance 1e-10 relative.

#[path = "fixtures/decompose_reference.rs"]
mod fixtures;

use sheaf_calc::decompose::{DecomposeModel, decompose};

fn same(got: &[f64], want: &[f64], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}");
    for (i, (a, b)) in got.iter().zip(want).enumerate() {
        if b.is_nan() {
            assert!(a.is_nan(), "{what} {i}: {a} vs NaN");
        } else {
            assert!((a - b).abs() <= 1e-10 * b.abs().max(1.0), "{what} {i}: {a} vs {b}");
        }
    }
}

#[test]
fn decompositions_match_numpy() {
    for (name, y, m, mult, trend, seasonal, residual, indices) in fixtures::CASES {
        let model = if *mult { DecomposeModel::Multiplicative } else { DecomposeModel::Additive };
        let d = decompose(y, *m, model).unwrap();
        same(&d.trend, trend, &format!("{name} trend"));
        same(&d.seasonal, seasonal, &format!("{name} seasonal"));
        same(&d.residual, residual, &format!("{name} residual"));
        same(&d.indices, indices, &format!("{name} indices"));
    }
}
