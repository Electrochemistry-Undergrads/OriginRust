//! Best subset selection (T347, Origin 2025b's Best Subset Selection app; Sheaf's own definitions,
//! Origin's app was not run): every non-empty subset of the candidate predictors is fitted by
//! ordinary least squares with an intercept, on the rows where Y and every candidate are numbers (the
//! same rows for all subsets, so the criteria compare), and scored by
//!
//! - R-square `1 - RSS/TSS` and adjusted R-square `1 - (RSS/(n-k-1)) / (TSS/(n-1))`,
//! - Mallows' Cp `RSS / s^2 - n + 2 (k + 1)`, with `s^2` the mean square error of the model with every
//!   candidate (so that model has Cp = p + 1),
//! - AIC `n ln(RSS/n) + 2 (k + 1)` and BIC `n ln(RSS/n) + (k + 1) ln n`,
//!
//! with `n` the rows used, `k` the predictors of the subset and `p` the candidates.

use nalgebra::{DMatrix, DVector};

use crate::report::FitError;

/// Most candidate predictors (2^15 - 1 = 32767 subsets).
pub const MAX_CANDIDATES: usize = 15;

/// One fitted subset.
#[derive(Debug, Clone, PartialEq)]
pub struct SubsetModel {
    /// Indices of the predictors in the candidate list, ascending.
    pub vars: Vec<usize>,
    pub rss: f64,
    pub r_squared: f64,
    pub adj_r_squared: f64,
    pub cp: f64,
    pub aic: f64,
    pub bic: f64,
}

/// What [`best_subsets`] found.
#[derive(Debug, Clone, PartialEq)]
pub struct SubsetResult {
    /// Every subset, ordered by size, then by R-square (largest first).
    pub models: Vec<SubsetModel>,
    /// Rows used.
    pub n: usize,
    /// Indices into `models` of the best by adjusted R-square (largest), Cp (smallest |Cp - (k+1)|
    /// is not used: the smallest Cp), AIC and BIC (smallest).
    pub best_adj_r2: usize,
    pub best_cp: usize,
    pub best_aic: usize,
    pub best_bic: usize,
}

/// Fits every non-empty subset of `xs` (one vector per candidate, as long as `y`).
pub fn best_subsets(xs: &[Vec<f64>], y: &[f64]) -> Result<SubsetResult, FitError> {
    let p = xs.len();
    if p == 0 {
        return Err(FitError::Failed("best subset selection needs at least one candidate predictor".into()));
    }
    if p > MAX_CANDIDATES {
        return Err(FitError::Failed(format!(
            "best subset selection takes at most {MAX_CANDIDATES} candidate predictors, not {p}"
        )));
    }
    for x in xs {
        if x.len() != y.len() {
            return Err(FitError::LengthMismatch { x: x.len(), y: y.len() });
        }
    }
    let used: Vec<usize> = (0..y.len()).filter(|&i| y[i].is_finite() && xs.iter().all(|x| x[i].is_finite())).collect();
    let n = used.len();
    if n < p + 2 {
        return Err(FitError::TooFewPoints { have: n, need: p + 2 });
    }
    let yv: Vec<f64> = used.iter().map(|&i| y[i]).collect();
    let mean = yv.iter().sum::<f64>() / n as f64;
    let tss: f64 = yv.iter().map(|v| (v - mean) * (v - mean)).sum();
    let rss_of = |vars: &[usize]| -> Result<f64, FitError> {
        let a = DMatrix::from_fn(n, vars.len() + 1, |r, c| if c == 0 { 1.0 } else { xs[vars[c - 1]][used[r]] });
        let b = DVector::from_vec(yv.clone());
        let svd = a.clone().svd(true, true);
        let coef = svd.solve(&b, 1e-12).map_err(|e| FitError::Failed(e.to_string()))?;
        Ok((b - a * coef).norm_squared())
    };
    let all: Vec<usize> = (0..p).collect();
    let full_rss = rss_of(&all)?;
    let s2 = full_rss / (n - p - 1) as f64;
    let mut models = Vec::with_capacity((1 << p) - 1);
    for mask in 1u32..(1u32 << p) {
        let vars: Vec<usize> = (0..p).filter(|&j| mask & (1 << j) != 0).collect();
        let k = vars.len();
        let rss = if k == p { full_rss } else { rss_of(&vars)? };
        let nf = n as f64;
        let ln = (rss / nf).max(f64::MIN_POSITIVE).ln();
        models.push(SubsetModel {
            rss,
            r_squared: 1.0 - rss / tss,
            adj_r_squared: 1.0 - (rss / (n - k - 1) as f64) / (tss / (n - 1) as f64),
            cp: if s2 > 0.0 { rss / s2 - nf + 2.0 * (k + 1) as f64 } else { f64::NAN },
            aic: nf * ln + 2.0 * (k + 1) as f64,
            bic: nf * ln + (k + 1) as f64 * nf.ln(),
            vars,
        });
    }
    models.sort_by(|a, b| a.vars.len().cmp(&b.vars.len()).then(b.r_squared.total_cmp(&a.r_squared)));
    let best = |better: &dyn Fn(&SubsetModel, &SubsetModel) -> bool| -> usize {
        let mut i = 0;
        for (j, m) in models.iter().enumerate() {
            if better(m, &models[i]) {
                i = j;
            }
        }
        i
    };
    let best_adj_r2 = best(&|a, b| a.adj_r_squared > b.adj_r_squared);
    let best_cp = best(&|a, b| a.cp < b.cp);
    let best_aic = best(&|a, b| a.aic < b.aic);
    let best_bic = best(&|a, b| a.bic < b.bic);
    Ok(SubsetResult { models, n, best_adj_r2, best_cp, best_aic, best_bic })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// y depends on x1 and x3 only; x2 and x4 are noise-like.
    fn data() -> (Vec<Vec<f64>>, Vec<f64>) {
        let n = 40;
        let f = |i: usize, a: f64| ((i as f64) * a).sin();
        let x1: Vec<f64> = (0..n).map(|i| i as f64 / 4.0).collect();
        let x2: Vec<f64> = (0..n).map(|i| f(i, 1.7)).collect();
        let x3: Vec<f64> = (0..n).map(|i| f(i, 0.31) * 3.0).collect();
        let x4: Vec<f64> = (0..n).map(|i| f(i, 2.9)).collect();
        let y: Vec<f64> = (0..n).map(|i| 1.0 + 2.0 * x1[i] - 1.5 * x3[i] + 0.05 * f(i, 7.3)).collect();
        (vec![x1, x2, x3, x4], y)
    }

    #[test]
    fn the_true_predictors_win_and_the_criteria_are_consistent() {
        let (xs, y) = data();
        let r = best_subsets(&xs, &y).unwrap();
        assert_eq!(r.models.len(), 15);
        assert_eq!(r.n, 40);
        assert_eq!(r.models[r.best_bic].vars, [0, 2]);
        assert_eq!(r.models[r.best_aic].vars.len().min(3), r.models[r.best_aic].vars.len());
        assert!(r.models[r.best_aic].vars.contains(&0) && r.models[r.best_aic].vars.contains(&2));
        // The full model's Cp is p + 1.
        let full = r.models.iter().find(|m| m.vars.len() == 4).unwrap();
        assert!((full.cp - 5.0).abs() < 1e-9, "{}", full.cp);
        // Within a size, models are ordered by R-square; R-square never falls when a variable is added.
        for w in r.models.windows(2) {
            if w[0].vars.len() == w[1].vars.len() {
                assert!(w[0].r_squared >= w[1].r_squared);
            }
        }
        // A single-predictor subset matches the straight line fit.
        let m = r.models.iter().find(|m| m.vars == [0]).unwrap();
        let (x1, n) = (&xs[0], 40.0);
        let (mx, my) = (x1.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
        let sxy: f64 = x1.iter().zip(&y).map(|(a, b)| (a - mx) * (b - my)).sum();
        let sxx: f64 = x1.iter().map(|a| (a - mx) * (a - mx)).sum();
        let syy: f64 = y.iter().map(|b| (b - my) * (b - my)).sum();
        assert!((m.r_squared - sxy * sxy / (sxx * syy)).abs() < 1e-12);
    }

    #[test]
    fn missing_rows_and_limits() {
        let (mut xs, mut y) = data();
        y[3] = f64::NAN;
        xs[1][7] = f64::NAN;
        assert_eq!(best_subsets(&xs, &y).unwrap().n, 38);
        assert!(best_subsets(&[], &y).is_err());
        let many: Vec<Vec<f64>> = (0..16).map(|k| (0..40).map(|i| ((i * k) as f64).cos()).collect()).collect();
        assert!(best_subsets(&many, &y).unwrap_err().to_string().contains("at most 15"));
        assert!(best_subsets(&xs, &y[..2]).is_err());
    }
}
