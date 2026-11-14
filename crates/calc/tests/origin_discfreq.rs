//! Discrete frequency against Origin's `discfreqs`.
//!
//! Reference script `tools/origin-ref/stats5/g1.py`; Origin's sheets are in `fixtures/discfreq_origin.rs` (made by
//! `make-fixture1.py`), read as shown (five decimals). The categories, counts, relative frequencies and the order of
//! the counts agree in every case, and the whole order for the value sorts. Origin's order among categories with the
//! same count follows no rule we found, so there only the cumulative frequency at the end of each run of equal counts
//! is compared.

#[path = "fixtures/discfreq_origin.rs"]
mod fixtures;

use fixtures::*;
use sheaf_calc::{Category, DiscSort, discrete_frequency};

fn label(c: &Category) -> String {
    match c {
        Category::Missing => "--".to_string(),
        Category::Number(x) => format!("{x}"),
        Category::Text(s) => s.clone(),
    }
}

/// As Origin shows a number in these sheets: up to five decimals.
fn shown(x: f64) -> String {
    let s = format!("{x:.5}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[test]
fn discrete_frequency_matches_origin() {
    for c in DISC_CASES {
        let values: Vec<Category> = c.data.iter().map(Cat::category).collect();
        let sort = DiscSort::from_code(c.sort).unwrap_or(DiscSort::CountDesc);
        let f = discrete_frequency(&values, c.case_sensitive, sort);
        assert_eq!(f.rows.len(), c.rows.len(), "{}: categories", c.name);
        let col = |name: &str| c.names.iter().position(|n| *n == name);
        let scale = if c.percent { 100.0 } else { 1.0 };
        let rel = f.relative();
        let cum = f.cumulative();
        // Each category with its count and relative frequency.
        for (i, row) in f.rows.iter().enumerate() {
            let at = c
                .rows
                .iter()
                .position(|r| r[0] == label(&row.value))
                .unwrap_or_else(|| panic!("{}: Origin has no category {:?}", c.name, row.value));
            if let Some(k) = col("Count") {
                assert_eq!(c.rows[at][k], row.count.to_string(), "{}: count of {:?}", c.name, row.value);
                assert_eq!(c.rows[i][k], row.count.to_string(), "{}: counts in Origin's order", c.name);
            }
            if let Some(k) = col("Relative Frequency") {
                assert_eq!(c.rows[at][k], shown(rel[i] * scale), "{}: relative frequency of {:?}", c.name, row.value);
            }
            let run_ends = i + 1 == f.rows.len() || f.rows[i + 1].count != row.count;
            let by_value = matches!(sort, DiscSort::ValueAsc | DiscSort::ValueDesc);
            if by_value {
                assert_eq!(c.rows[i][0], label(&row.value), "{}: row {i}", c.name);
            }
            if let Some(k) = col("Cumulative Frequency").filter(|_| by_value || run_ends) {
                assert_eq!(c.rows[i][k], shown(cum[i] * scale), "{}: cumulative frequency at row {i}", c.name);
            }
        }
    }
}
