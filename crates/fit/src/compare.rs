//! Comparing fitted models of one dataset: the corrected Akaike information criterion, the Bayesian
//! information criterion and the F-test, as Origin's Compare Models report gives them
//! (`docs/spec/fitting.md`, section 11).
//!
//! Each model is summarised by its residual sum of squares, the number of points and the number of
//! free parameters. With `k' = k + 1` (the error variance counts as a parameter):
//!
//! - AICc = `n ln(rss/n) + 2k' + 2k'(k'+1)/(n-k'-1)`, Akaike weight `exp(-d/2) / sum exp(-d_i/2)`
//!   with `d` the AICc minus the smaller one;
//! - BIC = `n ln(rss/n) + k' ln n`, and the BIC minus the smaller BIC;
//! - F-test: the model with more degrees of freedom is the null model;
//!   `F = |(rss0 - rss1) / df1| / (rss1 / df2)` with `df1` the difference of the degrees of freedom
//!   and `df2` those of the other model, probability from the upper tail of F(df1, df2).

use crate::dist::f_upper_p;
use crate::report::FitReport;

/// What the comparison needs of a fit.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSummary {
    pub name: String,
    pub rss: f64,
    pub n: usize,
    /// Free parameters.
    pub params: usize,
}

impl ModelSummary {
    /// The summary of a fit report (weighted residual sum of squares, free parameters = points
    /// minus degrees of freedom).
    pub fn from_report(r: &FitReport) -> Self {
        ModelSummary {
            name: r.model.clone(),
            rss: r.stats.rss,
            n: r.stats.n_points,
            params: r.stats.n_points.saturating_sub(r.stats.dof),
        }
    }

    /// Degrees of freedom `n - k`.
    pub fn dof(&self) -> usize {
        self.n.saturating_sub(self.params)
    }
}

/// The verdict of one comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The first (0) or second (1) model is preferred.
    Model(usize),
    /// BIC: the difference is 2 or less.
    Inconclusive,
    /// F-test with equal degrees of freedom (or no usable numbers).
    NoConclusion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FTest {
    pub f: f64,
    pub df1: usize,
    pub df2: usize,
    pub prob: f64,
    /// Index (0 or 1) of the model used as the null (simpler) model, if the test could be made.
    pub null_model: Option<usize>,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub models: [ModelSummary; 2],
    pub aic: [f64; 2],
    pub aic_weight: [f64; 2],
    pub aic_verdict: Verdict,
    pub bic: [f64; 2],
    /// BIC minus the smaller BIC.
    pub bic_delta: [f64; 2],
    pub bic_verdict: Verdict,
    pub ftest: FTest,
    pub warnings: Vec<String>,
}

/// F-test significance level (Origin picks the more complex model below it).
pub const F_TEST_ALPHA: f64 = 0.05;
/// BIC differences up to this size are called inconclusive.
pub const BIC_THRESHOLD: f64 = 2.0;

/// AICc of one model; NaN when `n - k' - 1` is not positive or the residual sum of squares is not.
pub fn aicc(m: &ModelSummary) -> f64 {
    let (n, k) = (m.n as f64, m.params as f64 + 1.0);
    if m.rss > 0.0 && n - k - 1.0 > 0.0 {
        n * (m.rss / n).ln() + 2.0 * k + 2.0 * k * (k + 1.0) / (n - k - 1.0)
    } else {
        f64::NAN
    }
}

/// BIC of one model.
pub fn bic(m: &ModelSummary) -> f64 {
    let (n, k) = (m.n as f64, m.params as f64 + 1.0);
    if m.rss > 0.0 && m.n > 0 { n * (m.rss / n).ln() + k * n.ln() } else { f64::NAN }
}

/// Compares two models fitted to the same data.
pub fn compare(a: &ModelSummary, b: &ModelSummary) -> Comparison {
    let mut warnings = Vec::new();
    if a.n != b.n {
        warnings.push(format!(
            "the models were fitted to different numbers of points ({} and {}); the criteria are compared as they stand",
            a.n, b.n
        ));
    }
    let aic = [aicc(a), aicc(b)];
    let bic_v = [bic(a), bic(b)];
    if aic.iter().any(|v| v.is_nan()) {
        warnings.push("AIC is undefined for a model with too few points or a zero residual sum of squares".into());
    }
    let weights = {
        let m = aic[0].min(aic[1]);
        let e = [(-(aic[0] - m) / 2.0).exp(), (-(aic[1] - m) / 2.0).exp()];
        let s = e[0] + e[1];
        [e[0] / s, e[1] / s]
    };
    let aic_verdict = if aic.iter().any(|v| v.is_nan()) {
        Verdict::NoConclusion
    } else {
        Verdict::Model(if aic[0] <= aic[1] { 0 } else { 1 })
    };
    let bm = bic_v[0].min(bic_v[1]);
    let bic_delta = [bic_v[0] - bm, bic_v[1] - bm];
    let bic_verdict = if bic_v.iter().any(|v| v.is_nan()) {
        Verdict::NoConclusion
    } else if (bic_v[0] - bic_v[1]).abs() > BIC_THRESHOLD {
        Verdict::Model(if bic_v[0] < bic_v[1] { 0 } else { 1 })
    } else {
        Verdict::Inconclusive
    };

    let (d0, d1) = (a.dof(), b.dof());
    let ftest = if d0 == d1 {
        FTest { f: f64::NAN, df1: 0, df2: d0, prob: f64::NAN, null_model: None, verdict: Verdict::NoConclusion }
    } else {
        let (null, alt) = if d0 > d1 { (0, 1) } else { (1, 0) };
        let (mn, ma) = if null == 0 { (a, b) } else { (b, a) };
        let df1 = mn.dof() - ma.dof();
        let df2 = ma.dof();
        if df2 == 0 || ma.rss <= 0.0 {
            warnings.push("the F-test needs degrees of freedom left and a nonzero residual sum of squares".into());
            FTest { f: f64::NAN, df1, df2, prob: f64::NAN, null_model: Some(null), verdict: Verdict::NoConclusion }
        } else {
            let f = (((mn.rss - ma.rss) / df1 as f64) / (ma.rss / df2 as f64)).abs();
            let prob = f_upper_p(f, df1, df2);
            let verdict = if prob < F_TEST_ALPHA { Verdict::Model(alt) } else { Verdict::Model(null) };
            FTest { f, df1, df2, prob, null_model: Some(null), verdict }
        }
    };
    Comparison {
        models: [a.clone(), b.clone()],
        aic,
        aic_weight: weights,
        aic_verdict,
        bic: bic_v,
        bic_delta,
        bic_verdict,
        ftest,
        warnings,
    }
}

/// One row of a ranking of many models of one dataset.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub summary: ModelSummary,
    pub aic: f64,
    pub aic_delta: f64,
    pub aic_weight: f64,
    pub bic: f64,
    pub bic_delta: f64,
}

/// Ranks models by AICc (smallest first; models whose AICc is undefined go last, in input order)
/// and gives each the Akaike weight and the BIC difference to the best BIC.
pub fn rank(models: &[ModelSummary]) -> Vec<Ranked> {
    let aic: Vec<f64> = models.iter().map(aicc).collect();
    let bics: Vec<f64> = models.iter().map(bic).collect();
    let best_aic = aic.iter().copied().filter(|v| v.is_finite()).fold(f64::INFINITY, f64::min);
    let best_bic = bics.iter().copied().filter(|v| v.is_finite()).fold(f64::INFINITY, f64::min);
    let total: f64 = aic.iter().filter(|v| v.is_finite()).map(|v| (-(v - best_aic) / 2.0).exp()).sum();
    let mut rows: Vec<Ranked> = models
        .iter()
        .enumerate()
        .map(|(i, m)| Ranked {
            summary: m.clone(),
            aic: aic[i],
            aic_delta: aic[i] - best_aic,
            aic_weight: if aic[i].is_finite() { (-(aic[i] - best_aic) / 2.0).exp() / total } else { f64::NAN },
            bic: bics[i],
            bic_delta: bics[i] - best_bic,
        })
        .collect();
    rows.sort_by(|x, y| match (x.aic.is_finite(), y.aic.is_finite()) {
        (true, true) => x.aic.total_cmp(&y.aic),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => std::cmp::Ordering::Equal,
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(name: &str, rss: f64, n: usize, k: usize) -> ModelSummary {
        ModelSummary { name: name.into(), rss, n, params: k }
    }

    #[test]
    fn ranking_orders_by_aic_and_weights_sum_to_one() {
        let r = rank(&[m("a", 0.5, 20, 5), m("b", 0.4, 20, 3), m("c", 0.45, 20, 4)]);
        assert_eq!(r[0].summary.name, "b");
        assert_eq!(r[0].aic_delta, 0.0);
        let s: f64 = r.iter().map(|x| x.aic_weight).sum();
        assert!((s - 1.0).abs() < 1e-12);
    }

    #[test]
    fn too_few_points_leave_aic_undefined_and_rank_last() {
        let r = rank(&[m("tiny", 0.1, 4, 3), m("ok", 0.2, 20, 2)]);
        assert_eq!(r[0].summary.name, "ok");
        assert!(r[1].aic.is_nan() && r[1].aic_weight.is_nan());
    }
}
