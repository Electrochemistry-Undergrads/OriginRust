//! CART trees and seeded random forests against an independent Python implementation of the same rules, made by
//! `tools/forest-ref.py` (T377; none of these numbers come from Origin). Splits and class predictions agree exactly;
//! values and importances to 1e-12 relative.

#[path = "fixtures/forest_reference.rs"]
mod fixtures;

use sheaf_calc::forest::{ForestOptions, Target, forest, tree};

fn rows() -> Vec<Vec<f64>> {
    fixtures::X.iter().map(|r| r.to_vec()).collect()
}

fn same(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}");
    for (i, (u, v)) in a.iter().zip(b).enumerate() {
        if v.is_nan() {
            assert!(u.is_nan(), "{what} {i}: {u} vs NaN");
        } else {
            assert!((u - v).abs() <= 1e-12 * v.abs().max(1.0), "{what} {i}: {u} vs {v}");
        }
    }
}

#[test]
fn trees_match_the_python_cart() {
    let x = rows();
    let cls = Target::Class { y: fixtures::CLASSES.to_vec(), classes: 3 };
    let (t, imp) = tree(&x, &cls, 1, None).unwrap();
    assert_eq!(t.preorder(), fixtures::CLASS_TREE.0);
    same(&imp, &fixtures::CLASS_TREE.1, "class importance");
    let pred: Vec<f64> = x.iter().map(|r| t.predict(r)).collect();
    same(&pred, fixtures::CLASS_TREE.2, "class predictions");

    let val = Target::Value(fixtures::VALUES.to_vec());
    let (t, imp) = tree(&x, &val, 5, Some(4)).unwrap();
    assert_eq!(t.preorder(), fixtures::VALUE_TREE.0);
    same(&imp, &fixtures::VALUE_TREE.1, "value importance");
    let pred: Vec<f64> = x.iter().map(|r| t.predict(r)).collect();
    same(&pred, fixtures::VALUE_TREE.2, "value predictions");
}

#[test]
fn seeded_forests_match_the_python_forest() {
    let x = rows();
    let cls = Target::Class { y: fixtures::CLASSES.to_vec(), classes: 3 };
    let opts = ForestOptions { trees: 15, mtry: Some(2), min_leaf: 1, max_depth: None, seed: 42, bootstrap: true };
    let f = forest(&x, &cls, opts).unwrap();
    let (oob, err, imp, fitted, first) = fixtures::CLASS_FOREST;
    assert_eq!(f.trees[0].preorder(), first);
    same(&f.oob, oob, "class OOB");
    same(&[f.oob_error], &[err], "class OOB error");
    same(&f.importance, &imp, "class importance");
    same(&f.fitted, fitted, "class fitted");

    let val = Target::Value(fixtures::VALUES.to_vec());
    let f = forest(&x, &val, ForestOptions { mtry: Some(1), min_leaf: 5, ..opts }).unwrap();
    let (oob, err, imp, fitted, first) = fixtures::VALUE_FOREST;
    assert_eq!(f.trees[0].preorder(), first);
    same(&f.oob, oob, "value OOB");
    same(&[f.oob_error], &[err], "value OOB error");
    same(&f.importance, &imp, "value importance");
    same(&f.fitted, fitted, "value fitted");
}
