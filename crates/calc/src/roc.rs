//! Receiver operating characteristic curve of a test variable against a two-state variable (Statistics >
//! ROC Curve).
//!
//! **Source of the numbers: textbook definitions checked against scipy, not Origin.** Origin's ROC tool is
//! a menu dialog that has no scriptable X-Function (`ROCCurve` returns false when called from LabTalk), so
//! nothing here was measured against it (`docs/spec/statistics.md`, section 19).
//!
//! - A case is *positive* when its state equals the chosen positive value, otherwise negative. By default
//!   larger test values point to the positive state; the option reverses that.
//! - The curve has one point per distinct cut-off: the case is called positive when its value is at or above
//!   the cut-off (at or below it when smaller values are positive). Sensitivity is the fraction of positives
//!   called positive, specificity the fraction of negatives called negative. The curve starts at
//!   (1 - specificity, sensitivity) = (0, 0) (a cut-off above every value) and ends at (1, 1).
//! - The area under the curve (AUC) is the trapezoid area, which equals the Mann-Whitney statistic
//!   `U / (n_pos n_neg)` with ties counting one half.
//! - The standard error is DeLong's (DeLong, DeLong and Clarke-Pearce 1988): the variance of the AUC is
//!   `S10 / n_pos + S01 / n_neg`, `S10` the sample variance of the placements of the positives (the fraction
//!   of negatives below each positive, ties one half) and `S01` the same for the negatives. The confidence
//!   interval is `AUC -/+ z SE` limited to [0, 1]; the p-value tests AUC = 0.5 with `z = (AUC - 0.5) / SE`
//!   (two-sided), 0 when the standard error is 0 and the AUC differs from 0.5.
//! - The best cut-off is the one with the largest Youden index `sensitivity + specificity - 1`, the highest
//!   cut-off among equal ones.

use statrs::distribution::{ContinuousCDF, Normal};

/// Options of [`roc`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RocOptions {
    /// Larger values of the test variable indicate the positive state.
    pub higher_is_positive: bool,
    /// Significance level of the confidence interval (`1 - alpha`).
    pub alpha: f64,
}

impl Default for RocOptions {
    fn default() -> Self {
        RocOptions { higher_is_positive: true, alpha: 0.05 }
    }
}

/// Why a ROC curve could not be made.
#[derive(Debug, Clone, PartialEq)]
pub enum RocError {
    /// The value and state columns hold no pair of numbers.
    NoCases,
    /// All cases are positive or all are negative.
    OneState,
    /// A value is infinite.
    NotFinite,
    /// The significance level is not between 0 and 1.
    BadAlpha,
}

impl std::fmt::Display for RocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RocError::NoCases => write!(f, "no row has a number in both the test variable and the state column"),
            RocError::OneState => write!(f, "all cases are in one state, so there is no curve to draw"),
            RocError::NotFinite => write!(f, "a test value is infinite"),
            RocError::BadAlpha => write!(f, "the significance level must lie between 0 and 1"),
        }
    }
}

impl std::error::Error for RocError {}

/// One point of the curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RocPoint {
    /// The cut-off (infinite for the starting point, which calls nobody positive).
    pub cutoff: f64,
    pub sensitivity: f64,
    pub specificity: f64,
}

/// The result of [`roc`].
#[derive(Debug, Clone, PartialEq)]
pub struct Roc {
    pub n_positive: usize,
    pub n_negative: usize,
    pub auc: f64,
    pub se: f64,
    pub lcl: f64,
    pub ucl: f64,
    /// `(AUC - 0.5) / SE` and its two-sided p-value.
    pub z: f64,
    pub p: f64,
    /// The curve, from the point that calls nobody positive to the one that calls everybody positive.
    pub points: Vec<RocPoint>,
    /// Index into `points` of the best cut-off (largest Youden index).
    pub best: usize,
}

/// Midranks (1-based, ties get the mean of their ranks) of `v`.
fn midranks(v: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut ranks = vec![0.0; v.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && v[order[j + 1]] == v[order[i]] {
            j += 1;
        }
        let r = (i + j) as f64 / 2.0 + 1.0;
        order[i..=j].iter().for_each(|&k| ranks[k] = r);
        i = j + 1;
    }
    ranks
}

/// The ROC curve of `values` against `positive` (`positive[i]` tells whether row `i` is in the positive
/// state). Rows where the value is NaN are left out; the slices must have the same length.
pub fn roc(values: &[f64], positive: &[bool], options: &RocOptions) -> Result<Roc, RocError> {
    if !(options.alpha > 0.0 && options.alpha < 1.0) {
        return Err(RocError::BadAlpha);
    }
    assert_eq!(values.len(), positive.len(), "one state per value");
    if values.iter().any(|v| v.is_infinite()) {
        return Err(RocError::NotFinite);
    }
    // Orient the values so that larger always means positive.
    let sign = if options.higher_is_positive { 1.0 } else { -1.0 };
    let mut pos: Vec<f64> = Vec::new();
    let mut neg: Vec<f64> = Vec::new();
    for (v, &p) in values.iter().zip(positive) {
        if v.is_finite() {
            if p { pos.push(sign * v) } else { neg.push(sign * v) }
        }
    }
    let (m, n) = (pos.len(), neg.len());
    if m + n == 0 {
        return Err(RocError::NoCases);
    }
    if m == 0 || n == 0 {
        return Err(RocError::OneState);
    }
    // DeLong placements through midranks.
    let all: Vec<f64> = pos.iter().chain(&neg).copied().collect();
    let (r_all, r_pos, r_neg) = (midranks(&all), midranks(&pos), midranks(&neg));
    let (mf, nf) = (m as f64, n as f64);
    let v10: Vec<f64> = (0..m).map(|i| (r_all[i] - r_pos[i]) / nf).collect();
    let v01: Vec<f64> = (0..n).map(|j| 1.0 - (r_all[m + j] - r_neg[j]) / mf).collect();
    let auc = v10.iter().sum::<f64>() / mf;
    let var_of = |v: &[f64]| -> f64 {
        if v.len() < 2 {
            return 0.0;
        }
        v.iter().map(|x| (x - auc).powi(2)).sum::<f64>() / (v.len() as f64 - 1.0)
    };
    let se = (var_of(&v10) / mf + var_of(&v01) / nf).sqrt();
    let normal = Normal::new(0.0, 1.0).expect("standard normal");
    let zc = normal.inverse_cdf(1.0 - options.alpha / 2.0);
    let (lcl, ucl) = ((auc - zc * se).max(0.0), (auc + zc * se).min(1.0));
    let (z, p) = if se > 0.0 {
        let z = (auc - 0.5) / se;
        (z, 2.0 * normal.cdf(-z.abs()))
    } else {
        (if auc == 0.5 { 0.0 } else { f64::INFINITY * (auc - 0.5).signum() }, if auc == 0.5 { 1.0 } else { 0.0 })
    };

    // The curve: cut-offs from high to low (in the oriented values), one point per distinct value.
    let mut cases: Vec<(f64, bool)> = pos.iter().map(|&v| (v, true)).chain(neg.iter().map(|&v| (v, false))).collect();
    cases.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut points = vec![RocPoint { cutoff: f64::INFINITY, sensitivity: 0.0, specificity: 1.0 }];
    let (mut tp, mut fp) = (0usize, 0usize);
    let mut i = 0;
    while i < cases.len() {
        let v = cases[i].0;
        while i < cases.len() && cases[i].0 == v {
            if cases[i].1 {
                tp += 1
            } else {
                fp += 1
            }
            i += 1;
        }
        points.push(RocPoint { cutoff: sign * v, sensitivity: tp as f64 / mf, specificity: 1.0 - fp as f64 / nf });
    }
    // Real cut-offs only (the (0, 0) start has an infinite one); the highest wins among equal indexes.
    let mut best = 1;
    let mut best_j = f64::NEG_INFINITY;
    for (k, pt) in points.iter().enumerate().skip(1) {
        let j = pt.sensitivity + pt.specificity - 1.0;
        if j > best_j + 1e-12 {
            best_j = j;
            best = k;
        }
    }
    Ok(Roc { n_positive: m, n_negative: n, auc, se, lcl, ucl, z, p, points, best })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    /// Positives 3, 4, 5 and negatives 1, 2, 4. Pairs with the positive higher: 3 beats 1 and 2 (2), 4 beats
    /// 1 and 2 and ties 4 (2.5), 5 beats all (3): 7.5 of 9, AUC = 5/6. Placements of the positives
    /// (2/3, 5/6, 1), of the negatives (1, 1, 1/2); S10 = (1/36 + 0 + 1/36) / 2 = 1/36,
    /// S01 = (1/36 + 1/36 + 4/36) / 2 = 1/12, variance = (1/36) / 3 + (1/12) / 3 = 1/27.
    #[test]
    fn auc_and_delong_by_hand() {
        let v = [3.0, 4.0, 5.0, 1.0, 2.0, 4.0];
        let s = [true, true, true, false, false, false];
        let r = roc(&v, &s, &RocOptions::default()).unwrap();
        near(r.auc, 5.0 / 6.0, 1e-15);
        near(r.se, (1.0f64 / 27.0).sqrt(), 1e-15);
        near(r.z, (5.0 / 6.0 - 0.5) / (1.0f64 / 27.0).sqrt(), 1e-12);
        // Curve: cut-offs inf, 5, 4, 3, 2, 1.
        let pts: Vec<(f64, f64)> = r.points.iter().map(|p| (1.0 - p.specificity, p.sensitivity)).collect();
        let want =
            [(0.0, 0.0), (0.0, 1.0 / 3.0), (1.0 / 3.0, 2.0 / 3.0), (1.0 / 3.0, 1.0), (2.0 / 3.0, 1.0), (1.0, 1.0)];
        assert_eq!(pts.len(), want.len());
        for (g, w) in pts.iter().zip(want) {
            near(g.0, w.0, 1e-15);
            near(g.1, w.1, 1e-15);
        }
        // The trapezoid area of that curve is the AUC.
        let area: f64 = pts.windows(2).map(|w| (w[1].0 - w[0].0) * (w[0].1 + w[1].1) / 2.0).sum();
        near(area, r.auc, 1e-15);
        // Youden: sens + spec - 1 = 1/3 (cut 5), 1/3 (cut 4), 2/3 (cut 3): best is 3.
        assert_eq!(r.points[r.best].cutoff, 3.0);
        assert!(r.lcl < r.auc && r.ucl == 1.0);
    }

    #[test]
    fn best_cutoff_is_always_a_real_one() {
        let r = roc(&[1.0, 2.0, 3.0, 4.0], &[true, true, false, false], &RocOptions::default()).unwrap();
        assert!(r.points[r.best].cutoff.is_finite());
    }

    #[test]
    fn perfect_and_reversed() {
        let v = [1.0, 2.0, 3.0, 4.0];
        let s = [false, false, true, true];
        let r = roc(&v, &s, &RocOptions::default()).unwrap();
        assert_eq!((r.auc, r.se, r.p), (1.0, 0.0, 0.0));
        let low = roc(&v, &s, &RocOptions { higher_is_positive: false, ..Default::default() }).unwrap();
        assert_eq!(low.auc, 0.0);
        // With "lower is positive" the cut-offs are reported in the original direction.
        assert_eq!(low.points[1].cutoff, 1.0);
        let m = roc(&v, &[false, true, false, true], &RocOptions::default()).unwrap();
        near(m.auc, 0.75, 1e-15);
    }

    #[test]
    fn missing_and_errors() {
        let r = roc(&[1.0, f64::NAN, 3.0, 2.0], &[false, true, true, false], &RocOptions::default()).unwrap();
        assert_eq!((r.n_positive, r.n_negative), (1, 2));
        assert_eq!(roc(&[1.0, 2.0], &[true, true], &RocOptions::default()), Err(RocError::OneState));
        assert_eq!(roc(&[f64::NAN], &[true], &RocOptions::default()), Err(RocError::NoCases));
        assert_eq!(
            roc(&[1.0, 2.0], &[true, false], &RocOptions { alpha: 0.0, ..Default::default() }),
            Err(RocError::BadAlpha)
        );
    }
}
