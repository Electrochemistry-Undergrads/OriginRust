//! Worksheet > Conditional Formatting > Conditional Outliers and Conditional Pareto. The options
//! follow Origin's documentation (Outliers: IQR or SD with a Coef, default 1.5; Pareto: Most or
//! Least Frequent N Items or N%). Per-column outliers, the order of items with equal counts and
//! the colours are Sheaf's own choices and unmeasured (docs/spec/gui-look.md).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Origin's default Coef of Conditional Outliers.
pub const DEFAULT_COEF: f64 = 1.5;

/// How Conditional Outliers finds its fences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutlierMethod {
    /// Q1 - Coef x IQR and Q3 + Coef x IQR, quartiles by Origin's box-chart rule.
    #[default]
    Iqr,
    /// Mean -/+ Coef x the sample standard deviation.
    Sd,
}

/// The fences of a set of numbers: values strictly outside them are outliers. `None` when there
/// are too few numbers (none for IQR, fewer than two for SD) or the Coef is not a number >= 0.
pub fn fences(values: &[f64], method: OutlierMethod, coef: f64) -> Option<(f64, f64)> {
    if values.is_empty() || !coef.is_finite() || coef < 0.0 {
        return None;
    }
    match method {
        OutlierMethod::Iqr => {
            let mut v = values.to_vec();
            v.sort_by(f64::total_cmp);
            let q1 = sheaf_calc::stats::quantile_origin(&v, 0.25);
            let q3 = sheaf_calc::stats::quantile_origin(&v, 0.75);
            let iqr = q3 - q1;
            Some((q1 - coef * iqr, q3 + coef * iqr))
        }
        OutlierMethod::Sd => {
            let n = values.len();
            if n < 2 {
                return None;
            }
            let mean = values.iter().sum::<f64>() / n as f64;
            let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1) as f64;
            let sd = var.sqrt();
            Some((mean - coef * sd, mean + coef * sd))
        }
    }
}

/// One item of Conditional Pareto: a number (by its bits, so equal numbers are one item) or a text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Number(u64),
    Text(String),
}

impl Item {
    pub fn number(v: f64) -> Item {
        // -0 and 0 are one item.
        Item::Number(if v == 0.0 { 0.0f64.to_bits() } else { v.to_bits() })
    }
}

/// The items Conditional Pareto colours among `items` (one per usable cell, in range order).
/// Items are ranked by count, most frequent first, equal counts in order of first appearance.
/// N Items takes the first (or last) `floor(n)` items of the ranking and every item with the same
/// count as the last one taken; Most Frequent N% takes the items whose cumulative frequency (this
/// item included, from the most frequent) is at most N %; Least Frequent N% the items whose
/// cumulative frequency is at least 100 - N % (Origin's wording), so it always takes the least
/// frequent item for N > 0. N of 0 or less (or not a number) takes nothing.
pub fn pareto_selection(items: &[Item], most: bool, n: f64, percent: bool) -> Vec<Item> {
    if !(n > 0.0) || items.is_empty() {
        return Vec::new();
    }
    let mut order: Vec<(Item, usize)> = Vec::new();
    let mut index: HashMap<&Item, usize> = HashMap::new();
    for it in items {
        match index.get(it) {
            Some(i) => order[*i].1 += 1,
            None => {
                index.insert(it, order.len());
                order.push((it.clone(), 1));
            }
        }
    }
    // Stable: equal counts keep their order of first appearance.
    order.sort_by(|a, b| b.1.cmp(&a.1));
    let total = items.len() as f64;
    if percent {
        let mut cum = 0.0;
        let mut out = Vec::new();
        for (item, count) in order {
            cum += count as f64;
            let share = cum / total * 100.0;
            let take = if most { share <= n + 1e-9 } else { share >= 100.0 - n - 1e-9 };
            if take {
                out.push(item);
            }
        }
        out
    } else {
        let k = (n.floor() as usize).min(order.len());
        if k == 0 {
            return Vec::new();
        }
        if !most {
            order.reverse();
        }
        let threshold = order[k - 1].1;
        order
            .into_iter()
            .enumerate()
            .filter(|(i, (_, c))| *i < k || *c == threshold)
            .map(|(_, (item, _))| item)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nums(v: &[f64]) -> Vec<Item> {
        v.iter().map(|x| Item::number(*x)).collect()
    }

    #[test]
    fn iqr_fences_use_origins_box_chart_quartiles() {
        // The box-chart probe of crates/plot: Q1 5, Q3 7.5 (Origin's rule), 30 is the one outlier.
        let v = [4.0, 4.0, 5.0, 5.0, 5.0, 6.0, 6.0, 7.0, 7.0, 8.0, 8.0, 30.0];
        let (lo, hi) = fences(&v, OutlierMethod::Iqr, DEFAULT_COEF).unwrap();
        let mut s = v.to_vec();
        s.sort_by(f64::total_cmp);
        let (q1, q3) = (sheaf_calc::stats::quantile_origin(&s, 0.25), sheaf_calc::stats::quantile_origin(&s, 0.75));
        assert_eq!((lo, hi), (q1 - 1.5 * (q3 - q1), q3 + 1.5 * (q3 - q1)));
        let outliers: Vec<f64> = v.iter().copied().filter(|x| *x < lo || *x > hi).collect();
        assert_eq!(outliers, vec![30.0]);
        // Coef 3 (outer fences) is wider.
        let (lo3, hi3) = fences(&v, OutlierMethod::Iqr, 3.0).unwrap();
        assert!(lo3 < lo && hi3 > hi);
    }

    #[test]
    fn sd_fences_use_the_sample_standard_deviation() {
        let v = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        // Mean 5, sample variance 32/7.
        let sd = (32.0f64 / 7.0).sqrt();
        assert_eq!(fences(&v, OutlierMethod::Sd, 2.0).unwrap(), (5.0 - 2.0 * sd, 5.0 + 2.0 * sd));
        assert!(fences(&[1.0], OutlierMethod::Sd, 2.0).is_none());
        assert!(fences(&[], OutlierMethod::Iqr, 1.5).is_none());
        assert!(fences(&v, OutlierMethod::Iqr, -1.0).is_none());
        assert!(fences(&v, OutlierMethod::Iqr, f64::NAN).is_none());
    }

    #[test]
    fn pareto_items_by_count_with_ties() {
        // Counts: 1 x3, 2 x2, 3 x2, 4 x1.
        let items = nums(&[1.0, 2.0, 1.0, 3.0, 2.0, 1.0, 3.0, 4.0]);
        assert_eq!(pareto_selection(&items, true, 1.0, false), nums(&[1.0]));
        // Two items: 2 is second, 3 has the same count and comes too.
        assert_eq!(pareto_selection(&items, true, 2.0, false), nums(&[1.0, 2.0, 3.0]));
        assert_eq!(pareto_selection(&items, false, 1.0, false), nums(&[4.0]));
        assert_eq!(pareto_selection(&items, false, 2.0, false), nums(&[4.0, 3.0, 2.0]));
        assert!(pareto_selection(&items, true, 0.0, false).is_empty());
        assert!(pareto_selection(&items, true, f64::NAN, true).is_empty());
        assert_eq!(pareto_selection(&items, true, 99.0, false).len(), 4);
    }

    #[test]
    fn pareto_percent_follows_the_cumulative_frequency() {
        // Cumulative: 1 -> 37.5 %, 2 -> 62.5 %, 3 -> 87.5 %, 4 -> 100 %.
        let items = nums(&[1.0, 2.0, 1.0, 3.0, 2.0, 1.0, 3.0, 4.0]);
        assert_eq!(pareto_selection(&items, true, 62.5, true), nums(&[1.0, 2.0]));
        assert!(pareto_selection(&items, true, 30.0, true).is_empty(), "the first item alone is over 30 %");
        // Least frequent 20 %: cumulative >= 80 %: 3 and 4.
        assert_eq!(pareto_selection(&items, false, 20.0, true), nums(&[3.0, 4.0]));
        assert_eq!(pareto_selection(&items, false, 1.0, true), nums(&[4.0]));
    }

    #[test]
    fn pareto_counts_text_and_numbers_as_items() {
        let items = vec![
            Item::Text("a".into()),
            Item::number(0.0),
            Item::number(-0.0),
            Item::Text("a".into()),
            Item::Text("b".into()),
        ];
        assert_eq!(pareto_selection(&items, true, 2.0, false), vec![Item::Text("a".into()), Item::number(0.0)]);
    }
}
