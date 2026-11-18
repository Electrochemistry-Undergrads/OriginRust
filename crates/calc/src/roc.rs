//! Receiver operating characteristic curve of a test variable against a two-state variable (Statistics >
//! ROC Curve).
//!
//! **Measured against Origin 2016** (`docs/spec/statistics.md`, section 19.5): Origin's ROC tool is a menu
//! dialog without a scriptable X-Function; it was driven through its window on four data sets and every
//! number below matches its report to the last digit.
//!
//! - A case is *positive* when its state equals the chosen positive value, otherwise negative. By default
//!   larger test values point to the positive state; the option reverses that.
//! - The curve has one point per distinct value: a case is called positive when its value is above the
//!   cut-off (below it when smaller values are positive). As Origin's "Interpolation of data points", the
//!   cut-off of a point is the midpoint between the last value called positive and the next one; the point
//!   that calls nobody positive and the one that calls everybody positive have cut-offs one beyond the data
//!   (the largest value + 1 and the smallest - 1), or 1 and 0 when every value lies in [0, 1]. Sensitivity
//!   is the fraction of positives called positive, specificity the fraction of negatives called negative.
//!   The curve starts at (1 - specificity, sensitivity) = (0, 0) and ends at (1, 1); Origin lists the
//!   same points in the opposite order.
//! - The area under the curve (AUC) is the trapezoid area, which equals the Mann-Whitney statistic
//!   `U / (n_pos n_neg)` with ties counting one half.
//! - The standard error is Origin's: Hanley and McNeil's form
//!   `(A (1 - A) + (n_neg - 1) (Q1 - A^2) + (n_pos - 1) (Q2 - A^2)) / (n_pos n_neg)` with `Q1` the integral of
//!   sensitivity squared over 1 - specificity and `Q2` that of specificity squared over sensitivity, both
//!   along the straight segments of the curve (a segment of tied values contributes
//!   `dx (y0^2 + y0 y1 + y1^2) / 3`). The confidence interval is `AUC -/+ z SE`, not limited to [0, 1] (as
//!   Origin). The p-value ("Asymptotic Prob") tests AUC = 0.5 against the standard error under that
//!   hypothesis, `sqrt((n_pos + n_neg + 1) / (12 n_pos n_neg))`, two-sided; `z` is that test statistic.
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
    /// `(AUC - 0.5) / SE0` (`SE0` the standard error when the area is 0.5) and its two-sided p-value.
    pub z: f64,
    pub p: f64,
    /// The curve, from the point that calls nobody positive to the one that calls everybody positive.
    pub points: Vec<RocPoint>,
    /// Index into `points` of the best cut-off (largest Youden index).
    pub best: usize,
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
    let (mf, nf) = (m as f64, n as f64);
    let normal = Normal::new(0.0, 1.0).expect("standard normal");
    // The curve: cut-offs from high to low (in the oriented values), one point per distinct value.
    let mut cases: Vec<(f64, bool)> = pos.iter().map(|&v| (v, true)).chain(neg.iter().map(|&v| (v, false))).collect();
    cases.sort_by(|a, b| b.0.total_cmp(&a.0));
    // Origin's end cut-offs, in the original direction: one beyond the data, or 0 and 1 for data in [0, 1].
    let (lo, hi) = cases.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), c| {
        let v = sign * c.0;
        (lo.min(v), hi.max(v))
    });
    let (lo_end, hi_end) = if lo >= 0.0 && hi <= 1.0 { (0.0, 1.0) } else { (lo - 1.0, hi + 1.0) };
    let (nobody, everybody) = if sign > 0.0 { (hi_end, lo_end) } else { (lo_end, hi_end) };
    let mut points = vec![RocPoint { cutoff: nobody, sensitivity: 0.0, specificity: 1.0 }];
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
        // The midpoint to the next value (Origin's interpolated cut-off), or the far end after the last.
        let cutoff = cases.get(i).map_or(everybody, |next| sign * (v + next.0) / 2.0);
        points.push(RocPoint { cutoff, sensitivity: tp as f64 / mf, specificity: 1.0 - fp as f64 / nf });
    }
    // Area, Q1 and Q2 along the straight segments of the curve.
    let (mut auc, mut q1, mut q2) = (0.0, 0.0, 0.0);
    for w in points.windows(2) {
        let (x0, y0, x1, y1) = (1.0 - w[0].specificity, w[0].sensitivity, 1.0 - w[1].specificity, w[1].sensitivity);
        let (dx, dy) = (x1 - x0, y1 - y0);
        auc += dx * (y0 + y1) / 2.0;
        q1 += dx * (y0 * y0 + y0 * y1 + y1 * y1) / 3.0;
        q2 += dy * ((1.0 - x0).powi(2) + (1.0 - x0) * (1.0 - x1) + (1.0 - x1).powi(2)) / 3.0;
    }
    let a2 = auc * auc;
    let var = (auc * (1.0 - auc) + (nf - 1.0) * (q1 - a2) + (mf - 1.0) * (q2 - a2)) / (mf * nf);
    let se = var.max(0.0).sqrt();
    let zc = normal.inverse_cdf(1.0 - options.alpha / 2.0);
    let (lcl, ucl) = (auc - zc * se, auc + zc * se);
    let se0 = ((mf + nf + 1.0) / (12.0 * mf * nf)).sqrt();
    let z = (auc - 0.5) / se0;
    let p = 2.0 * normal.cdf(-z.abs());
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
    /// 1 and 2 and ties 4 (2.5), 5 beats all (3): 7.5 of 9, AUC = 5/6. The test statistic uses the standard
    /// error of an area of 0.5: sqrt((3 + 3 + 1) / (12 * 9)).
    #[test]
    fn auc_test_statistic_and_cutoffs_by_hand() {
        let v = [3.0, 4.0, 5.0, 1.0, 2.0, 4.0];
        let s = [true, true, true, false, false, false];
        let r = roc(&v, &s, &RocOptions::default()).unwrap();
        near(r.auc, 5.0 / 6.0, 1e-15);
        near(r.z, (5.0 / 6.0 - 0.5) / (7.0f64 / 108.0).sqrt(), 1e-12);
        // Cut-offs as Origin's: one beyond the largest value, the midpoints, one below the smallest.
        let cut: Vec<f64> = r.points.iter().map(|p| p.cutoff).collect();
        assert_eq!(cut, [6.0, 4.5, 3.5, 2.5, 1.5, 0.0]);
        // Curve: values 5, 4, 3, 2, 1 called positive in turn.
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
        // Youden: sens + spec - 1 = 1/3, 1/3, 2/3 (3 and above positive): best is the cut-off 2.5.
        assert_eq!(r.points[r.best].cutoff, 2.5);
        // The interval is not limited to [0, 1], as Origin's.
        assert!(r.lcl < r.auc && r.ucl > 1.0, "{} {}", r.lcl, r.ucl);
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
        assert_eq!((r.auc, r.se), (1.0, 0.0));
        // The p-value tests against the standard error of an area of 0.5, so it is not 0.
        near(r.p, 2.0 * Normal::new(0.0, 1.0).unwrap().cdf(-0.5 / (5.0f64 / 48.0).sqrt()), 1e-15);
        let low = roc(&v, &s, &RocOptions { higher_is_positive: false, ..Default::default() }).unwrap();
        assert_eq!(low.auc, 0.0);
        // With "lower is positive" the cut-offs are reported in the original direction: nobody is called
        // positive below 0, 1 alone below 1.5.
        assert_eq!((low.points[0].cutoff, low.points[1].cutoff), (0.0, 1.5));
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
