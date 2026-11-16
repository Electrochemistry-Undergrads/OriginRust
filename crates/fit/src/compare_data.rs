//! Compare Datasets: two fits of one model to two datasets, and whether one parameter set fits both.
//!
//! **Sheaf's own test, not Origin's** (Origin's report was not measured, `docs/spec/fitting.md` section 17). The
//! datasets are fitted separately and pooled into one fit with every parameter shared. With `p` parameters and
//! `n1 + n2` points the F-test of "one parameter set fits both datasets" is
//!
//! `F = ((RSS_shared - RSS_1 - RSS_2) / p) / ((RSS_1 + RSS_2) / (n_1 + n_2 - 2p))`, `Prob>F` from F(p, n_1 + n_2 - 2p).
//!
//! For each parameter the difference of the two fitted values is also tested, `t = (v_1 - v_2) / sqrt(se_1^2 + se_2^2)`
//! with `n_1 + n_2 - 2p` degrees of freedom. The module works on fit reports, so any model of the library, a
//! formula or a polynomial can be compared.

use crate::dist::{f_upper_p, t_two_sided_p};
use crate::report::{FitError, FitReport};

/// The difference of one parameter between the datasets.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamDifference {
    pub name: String,
    pub value: [f64; 2],
    pub std_error: [f64; 2],
    /// `value[0] - value[1]`.
    pub difference: f64,
    pub t: f64,
    pub prob: f64,
}

/// The comparison of two datasets.
#[derive(Debug, Clone, PartialEq)]
pub struct DatasetComparison {
    pub model: String,
    /// Free parameters of the model.
    pub params: usize,
    pub n: [usize; 2],
    pub rss: [f64; 2],
    /// Residual sum of squares of the pooled fit with all parameters shared.
    pub rss_shared: f64,
    pub n_shared: usize,
    pub f: f64,
    pub df1: usize,
    pub df2: usize,
    pub prob: f64,
    pub differences: Vec<ParamDifference>,
    pub warnings: Vec<String>,
}

/// Significance level at which the datasets are called different.
pub const ALPHA: f64 = 0.05;

impl DatasetComparison {
    /// True when the F-test rejects "one parameter set fits both datasets" at [`ALPHA`].
    pub fn different(&self) -> Option<bool> {
        self.prob.is_finite().then_some(self.prob < ALPHA)
    }
}

/// Compares the fit of dataset 1 (`a`), of dataset 2 (`b`) and the pooled fit (`shared`), all of one model.
pub fn compare_datasets(a: &FitReport, b: &FitReport, shared: &FitReport) -> Result<DatasetComparison, FitError> {
    let p = a.stats.n_points.saturating_sub(a.stats.dof);
    let pb = b.stats.n_points.saturating_sub(b.stats.dof);
    let ps = shared.stats.n_points.saturating_sub(shared.stats.dof);
    if p != pb || p != ps || a.params.len() != b.params.len() {
        return Err(FitError::Failed(
            "the two fits must be of the same model with the same free parameters".to_string(),
        ));
    }
    let (n1, n2) = (a.stats.n_points, b.stats.n_points);
    let df2 = (n1 + n2).saturating_sub(2 * p);
    let mut warnings = Vec::new();
    if p == 0 {
        return Err(FitError::Failed("the model has no free parameter to compare".into()));
    }
    if df2 == 0 {
        return Err(FitError::Failed(format!(
            "{n1} and {n2} points leave no degrees of freedom for {p} parameters in each dataset"
        )));
    }
    let own = a.stats.rss + b.stats.rss;
    let numerator = (shared.stats.rss - own) / p as f64;
    let f = numerator / (own / df2 as f64);
    if numerator < 0.0 {
        warnings.push(
            "the pooled fit has a smaller residual sum of squares than the two separate fits (a poor start); the F value is not meaningful"
                .to_string(),
        );
    }
    let prob = if f >= 0.0 { f_upper_p(f, p, df2) } else { f64::NAN };
    let differences = a
        .params
        .iter()
        .zip(&b.params)
        .map(|(x, y)| {
            let se = x.std_error.hypot(y.std_error);
            let t = (x.value - y.value) / se;
            ParamDifference {
                name: x.name.clone(),
                value: [x.value, y.value],
                std_error: [x.std_error, y.std_error],
                difference: x.value - y.value,
                t,
                prob: t_two_sided_p(t, df2),
            }
        })
        .collect();
    Ok(DatasetComparison {
        model: a.model.clone(),
        params: p,
        n: [n1, n2],
        rss: [a.stats.rss, b.stats.rss],
        rss_shared: shared.stats.rss,
        n_shared: shared.stats.n_points,
        f,
        df1: p,
        df2,
        prob,
        differences,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear;
    use crate::linear::{LinearOptions, PolyOptions};

    fn line(x: &[f64], y: &[f64]) -> FitReport {
        linear::fit_linear(x, y, &LinearOptions::default()).unwrap()
    }

    #[test]
    fn two_lines_of_one_law_are_not_different_and_two_laws_are() {
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        let noise = [0.05, -0.04, 0.03, -0.02, 0.06, -0.05, 0.01, -0.03, 0.04, -0.06];
        let y1: Vec<f64> = x.iter().zip(&noise).map(|(x, e)| 1.0 + 2.0 * x + e).collect();
        let y2: Vec<f64> = x.iter().zip(noise.iter().rev()).map(|(x, e)| 1.0 + 2.0 * x + e).collect();
        let pool = |a: &[f64], b: &[f64]| {
            let xs: Vec<f64> = x.iter().chain(&x).copied().collect();
            let ys: Vec<f64> = a.iter().chain(b).copied().collect();
            line(&xs, &ys)
        };
        let same = compare_datasets(&line(&x, &y1), &line(&x, &y2), &pool(&y1, &y2)).unwrap();
        assert_eq!((same.params, same.df1, same.df2), (2, 2, 16));
        assert!(same.prob > 0.05, "{}", same.prob);
        assert_eq!(same.different(), Some(false));
        let y3: Vec<f64> = x.iter().zip(&noise).map(|(x, e)| 3.0 + 2.5 * x + e).collect();
        let other = compare_datasets(&line(&x, &y1), &line(&x, &y3), &pool(&y1, &y3)).unwrap();
        assert!(other.prob < 1e-6, "{}", other.prob);
        assert_eq!(other.different(), Some(true));
        assert!(other.differences[1].prob < 1e-6 && other.differences[1].difference < 0.0);
    }

    #[test]
    fn the_pooled_rss_is_not_below_the_sum_of_the_separate_ones_for_a_line() {
        let x: Vec<f64> = (0..6).map(f64::from).collect();
        let y1 = [0.0, 1.1, 1.9, 3.2, 3.9, 5.1];
        let y2 = [0.3, 1.0, 2.2, 2.9, 4.2, 4.8];
        let xs: Vec<f64> = x.iter().chain(&x).copied().collect();
        let ys: Vec<f64> = y1.iter().chain(&y2).copied().collect();
        let c = compare_datasets(&line(&x, &y1), &line(&x, &y2), &line(&xs, &ys)).unwrap();
        assert!(c.rss_shared >= c.rss[0] + c.rss[1] - 1e-12);
        assert!(c.warnings.is_empty());
    }

    #[test]
    fn different_models_are_refused() {
        let x: Vec<f64> = (0..6).map(f64::from).collect();
        let y = [0.0, 1.0, 4.0, 9.0, 16.0, 25.5];
        let a = line(&x, &y);
        let b = linear::fit_polynomial(&x, &y, 2, &PolyOptions::default()).unwrap();
        assert!(compare_datasets(&a, &b, &a).is_err());
    }
}
