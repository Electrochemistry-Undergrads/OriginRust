//! Stem-and-leaf display (Origin 2026's "Stem and Leaf Plot" app, T275). The options are those of
//! OriginLab's app page (leaf unit, automatic split at 5, a cap on the leaves of a line, outliers
//! beyond 1.5 IQR on their own lines); the layout and the automatic leaf unit are Sheaf's own, as the
//! page does not describe them.

use crate::stats::quantile_origin;

/// What the display is built with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StemLeafOptions {
    /// The value of one leaf digit (Origin's Leaf Unit); `None` chooses it from the range.
    pub leaf_unit: Option<f64>,
    /// Origin's Auto Split Stem: give each stem two lines (leaves 0-4 and 5-9) when the display
    /// would otherwise have fewer than five lines (Sheaf's rule for "automatically").
    pub auto_split: bool,
    /// Origin's Max Number of Leaves per Stem (50): more leaves on a line are counted, not written.
    pub max_leaves: usize,
    /// Origin's Auto Trim Outliers: values more than 1.5 IQR outside the quartiles go to the
    /// Outliers lines.
    pub trim_outliers: bool,
}

impl Default for StemLeafOptions {
    fn default() -> Self {
        StemLeafOptions { leaf_unit: None, auto_split: false, max_leaves: 50, trim_outliers: false }
    }
}

/// One line of the display.
#[derive(Debug, Clone, PartialEq)]
pub struct StemLine {
    /// The stem as written: `12`, `-0`, `3` (a second line of a split stem repeats it), or
    /// `Low` / `High` for the outlier lines.
    pub stem: String,
    /// The leaves, smallest digit first; the outlier lines list the values.
    pub leaves: String,
    /// How many values the line holds.
    pub count: usize,
}

/// A whole display.
#[derive(Debug, Clone, PartialEq)]
pub struct StemLeaf {
    /// The leaf unit used.
    pub unit: f64,
    /// Whether each stem has two lines.
    pub split: bool,
    /// The lines from the smallest values to the largest, outlier lines first and last.
    pub lines: Vec<StemLine>,
}

/// The automatic leaf unit: a power of ten one decade below the range (Sheaf's rule), so that a
/// range of 12 to 87 has leaf unit 1 (stems are tens) and 0.5 to 4.9 has 0.1.
pub fn auto_leaf_unit(values: &[f64]) -> f64 {
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = if hi > lo { hi - lo } else { hi.abs().max(lo.abs()) };
    if span <= 0.0 || !span.is_finite() {
        return 1.0;
    }
    10f64.powi(span.log10().floor() as i32 - 1)
}

/// The display of the finite values of `data`; `None` when there is none or the unit is not usable.
pub fn stem_and_leaf(data: &[f64], o: &StemLeafOptions) -> Option<StemLeaf> {
    let mut values: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let (mut low, mut high) = (Vec::new(), Vec::new());
    if o.trim_outliers && values.len() >= 4 {
        let (q1, q3) = (quantile_origin(&values, 0.25), quantile_origin(&values, 0.75));
        let fence = 1.5 * (q3 - q1);
        let (lo, hi) = (q1 - fence, q3 + fence);
        low = values.iter().copied().filter(|v| *v < lo).collect();
        high = values.iter().copied().filter(|v| *v > hi).collect();
        values.retain(|v| (lo..=hi).contains(v));
    }
    let unit = match o.leaf_unit {
        Some(u) if u > 0.0 && u.is_finite() => u,
        Some(_) => return None,
        None => auto_leaf_unit(&values),
    };
    // (negative, stem, leaf) of each value: the digits of |v| / unit, cut toward zero.
    let digits: Vec<(bool, i64, i64)> = values
        .iter()
        .map(|&v| {
            let n = (v.abs() / unit + 1e-9).floor() as i64;
            (v < 0.0, n / 10, n % 10)
        })
        .collect();
    let stems = |split: bool| -> std::collections::BTreeMap<i64, Vec<i64>> {
        let h = if split { 2 } else { 1 };
        let mut lines: std::collections::BTreeMap<i64, Vec<i64>> = std::collections::BTreeMap::new();
        for &(neg, stem, leaf) in &digits {
            let half = i64::from(split && leaf >= 5);
            let key = if neg { -(stem * h + half) - 1 } else { stem * h + half };
            lines.entry(key).or_default().push(leaf);
        }
        // Every line between the first and the last, empty ones too.
        if let (Some(&a), Some(&b)) = (lines.keys().next(), lines.keys().next_back()) {
            for k in a..=b {
                lines.entry(k).or_default();
            }
        }
        lines
    };
    let mut split = false;
    let mut lines = stems(false);
    if o.auto_split && lines.len() < 5 {
        split = true;
        lines = stems(true);
    }
    let h = if split { 2 } else { 1 };
    let mut out = Vec::new();
    let list = |vals: &[f64]| vals.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(", ");
    if !low.is_empty() {
        out.push(StemLine { stem: "Low".into(), leaves: list(&low), count: low.len() });
    }
    for (key, mut leaves) in lines {
        leaves.sort_unstable();
        let stem = if key < 0 { format!("-{}", (-key - 1) / h) } else { format!("{}", key / h) };
        let shown: String = leaves.iter().take(o.max_leaves).map(|d| char::from(b'0' + *d as u8)).collect();
        let more = leaves.len().saturating_sub(o.max_leaves);
        let leaves_text = if more > 0 { format!("{shown}+{more}") } else { shown };
        out.push(StemLine { stem, leaves: leaves_text, count: leaves.len() });
    }
    if !high.is_empty() {
        out.push(StemLine { stem: "High".into(), leaves: list(&high), count: high.len() });
    }
    Some(StemLeaf { unit, split, lines: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(s: &StemLeaf) -> Vec<(String, String, usize)> {
        s.lines.iter().map(|l| (l.stem.clone(), l.leaves.clone(), l.count)).collect()
    }

    #[test]
    fn a_classic_display_with_empty_stems() {
        let data = [12.0, 15.0, 23.0, 27.0, 27.0, 41.0, 46.0, 58.0];
        let s = stem_and_leaf(&data, &StemLeafOptions::default()).unwrap();
        assert_eq!(s.unit, 1.0);
        let want = [("1", "25", 2), ("2", "377", 3), ("3", "", 0), ("4", "16", 2), ("5", "8", 1)];
        let got = rows(&s);
        for (g, w) in got.iter().zip(want) {
            assert_eq!((g.0.as_str(), g.1.as_str(), g.2), w);
        }
        assert_eq!(got.len(), 5);
    }

    #[test]
    fn negatives_get_minus_zero_and_stems_split_at_five() {
        // Unit 0.1: -1.2 is stem -1 leaf 2, -0.3 stem -0 leaf 3, 0.4 stem 0 leaf 4.
        let s = stem_and_leaf(&[-1.2, -0.3, 0.4, 1.7], &StemLeafOptions { leaf_unit: Some(0.1), ..Default::default() })
            .unwrap();
        let stems: Vec<String> = s.lines.iter().map(|l| l.stem.clone()).collect();
        assert_eq!(stems, ["-1", "-0", "0", "1"]);
        assert_eq!(s.lines[0].leaves, "2");
        // Split: the 5-9 half after the 0-4 half (before it for negatives), from the first line with data
        // to the last.
        let o = StemLeafOptions { leaf_unit: Some(0.1), auto_split: true, ..Default::default() };
        let s = stem_and_leaf(&[-1.2, -0.3, 0.4, 1.7], &o).unwrap();
        assert!(s.split);
        let stems: Vec<String> = s.lines.iter().map(|l| l.stem.clone()).collect();
        assert_eq!(stems, ["-1", "-0", "-0", "0", "0", "1", "1"]);
        let leaves: Vec<String> = s.lines.iter().map(|l| l.leaves.clone()).collect();
        assert_eq!(leaves, ["2", "", "3", "4", "", "", "7"]);
    }

    #[test]
    fn outliers_go_to_their_own_lines_and_long_lines_are_capped() {
        let mut data: Vec<f64> = (10..20).map(f64::from).collect();
        data.push(95.0);
        let o = StemLeafOptions { trim_outliers: true, ..Default::default() };
        let s = stem_and_leaf(&data, &o).unwrap();
        let last = s.lines.last().unwrap();
        assert_eq!((last.stem.as_str(), last.leaves.as_str(), last.count), ("High", "95", 1));
        assert!(s.lines.iter().all(|l| l.stem != "Low"));
        let capped =
            stem_and_leaf(&data[..10], &StemLeafOptions { leaf_unit: Some(1.0), max_leaves: 4, ..Default::default() })
                .unwrap();
        assert_eq!((capped.lines[0].leaves.as_str(), capped.lines[0].count), ("0123+6", 10));
        assert!(stem_and_leaf(&[f64::NAN], &StemLeafOptions::default()).is_none());
        assert_eq!(auto_leaf_unit(&[0.5, 4.9]), 0.1);
    }
}
