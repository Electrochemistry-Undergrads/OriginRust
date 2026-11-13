//! One-way analysis of variance with the usual means comparisons.
//!
//! The summary table, the descriptive statistics of the groups and the means comparisons (Tukey,
//! Bonferroni, Sidak, Fisher's least significant difference, Scheffe) follow what Origin
//! reports for "One-Way ANOVA" (see `docs/spec/statistics.md`). Holm-Bonferroni (step-down) and Dunnett
//! (every group against a control) are added after scipy and the textbook definitions, not after Origin.

use crate::hypothesis::SampleStats;
use statrs::distribution::{ContinuousCDF, FisherSnedecor, Normal, StudentsT};
use statrs::function::gamma::ln_gamma;

/// The rows of the ANOVA summary table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnovaRow {
    pub df: f64,
    pub sum_sq: f64,
    pub mean_sq: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anova {
    /// Descriptive statistics of each group, in the order given.
    pub groups: Vec<SampleStats>,
    /// Between groups ("Model" in Origin's table).
    pub model: AnovaRow,
    /// Within groups ("Error").
    pub error: AnovaRow,
    /// Corrected total.
    pub total: AnovaRow,
    pub f: f64,
    pub p: f64,
    /// Share of the total variation that the groups explain: `SS model / SS total`.
    pub r_squared: f64,
    /// Square root of the error mean square.
    pub root_mse: f64,
    /// `root_mse` divided by the grand mean.
    pub cov: f64,
    pub grand_mean: f64,
}

/// The method of a means comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeansComparison {
    Tukey,
    Bonferroni,
    Sidak,
    /// Fisher's least significant difference: no adjustment for the number of comparisons.
    FisherLsd,
    Scheffe,
    /// Holm-Bonferroni: the step-down version of Bonferroni, all pairs. It has adjusted p-values but no
    /// confidence intervals (the limits are NaN).
    Holm,
    /// Dunnett: every group against one control group (the first group unless the caller says otherwise),
    /// with simultaneous confidence intervals. See [`crate::dunnett`].
    Dunnett,
}

impl MeansComparison {
    pub const ALL: [MeansComparison; 7] = [
        MeansComparison::Tukey,
        MeansComparison::Bonferroni,
        MeansComparison::Sidak,
        MeansComparison::FisherLsd,
        MeansComparison::Scheffe,
        MeansComparison::Holm,
        MeansComparison::Dunnett,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MeansComparison::Tukey => "Tukey",
            MeansComparison::Bonferroni => "Bonferroni",
            MeansComparison::Sidak => "Sidak",
            MeansComparison::FisherLsd => "Fisher LSD",
            MeansComparison::Scheffe => "Scheffe",
            MeansComparison::Holm => "Holm-Bonferroni",
            MeansComparison::Dunnett => "Dunnett",
        }
    }
}

/// One pairwise comparison of the means of group `i` and group `j` (`i < j`, counted from 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Comparison {
    pub i: usize,
    pub j: usize,
    /// `mean(j) - mean(i)`.
    pub diff: f64,
    /// Standard error of the difference, `sqrt(MSE (1/ni + 1/nj))`.
    pub se: f64,
    /// The statistic the p-value comes from: the studentized range q for Tukey, F for Scheffe, |t| for the others.
    pub stat: f64,
    pub p: f64,
    pub lcl: f64,
    pub ucl: f64,
    /// True where `p < alpha`.
    pub significant: bool,
}

/// Why an ANOVA could not be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnovaError {
    /// Fewer than two groups.
    TooFewGroups,
    /// A group has no numbers.
    EmptyGroup(usize),
    /// Every group has one number: there is no within-group variation to compare with.
    NoReplication,
}

impl std::fmt::Display for AnovaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnovaError::TooFewGroups => write!(f, "an ANOVA needs at least two groups"),
            AnovaError::EmptyGroup(i) => write!(f, "group {} has no numbers", i + 1),
            AnovaError::NoReplication => {
                write!(f, "every group has a single number; the ANOVA needs at least one group with two")
            }
        }
    }
}

/// One-way ANOVA of the groups; NaN entries are ignored.
pub fn anova_one_way(groups: &[&[f64]]) -> Result<Anova, AnovaError> {
    if groups.len() < 2 {
        return Err(AnovaError::TooFewGroups);
    }
    let stats: Vec<SampleStats> = groups.iter().map(|g| SampleStats::of(g)).collect();
    if let Some(i) = stats.iter().position(|s| s.n == 0) {
        return Err(AnovaError::EmptyGroup(i));
    }
    let k = stats.len() as f64;
    let n: f64 = stats.iter().map(|s| s.n as f64).sum();
    if n <= k {
        return Err(AnovaError::NoReplication);
    }
    let grand_mean = stats.iter().map(|s| s.mean * s.n as f64).sum::<f64>() / n;
    let ss_model: f64 = stats.iter().map(|s| s.n as f64 * (s.mean - grand_mean).powi(2)).sum();
    let ss_error: f64 = stats.iter().filter(|s| s.n > 1).map(|s| (s.n as f64 - 1.0) * s.sd * s.sd).sum();
    let (df_model, df_error) = (k - 1.0, n - k);
    let (ms_model, ms_error) = (ss_model / df_model, ss_error / df_error);
    let f = ms_model / ms_error;
    let p = if f.is_nan() {
        f64::NAN
    } else if f.is_infinite() {
        0.0
    } else {
        FisherSnedecor::new(df_model, df_error).map(|d| 1.0 - d.cdf(f)).unwrap_or(f64::NAN)
    };
    let ss_total = ss_model + ss_error;
    let root_mse = ms_error.sqrt();
    Ok(Anova {
        groups: stats,
        model: AnovaRow { df: df_model, sum_sq: ss_model, mean_sq: ms_model },
        error: AnovaRow { df: df_error, sum_sq: ss_error, mean_sq: ms_error },
        total: AnovaRow { df: n - 1.0, sum_sq: ss_total, mean_sq: ss_total / (n - 1.0) },
        f,
        p,
        r_squared: ss_model / ss_total,
        root_mse,
        cov: root_mse / grand_mean,
        grand_mean,
    })
}

impl Anova {
    /// All pairwise comparisons of the group means with the given method, at significance level `alpha`
    /// (the confidence level of the intervals is `1 - alpha`). Pairs come in Origin's order: (1,2), (1,3), (2,3), (1,4), (2,4), (3,4), ...
    pub fn compare(&self, method: MeansComparison, alpha: f64) -> Vec<Comparison> {
        self.compare_with_control(method, 0, alpha)
    }

    /// Like [`Anova::compare`]; `control` (counted from 0) is the control group of Dunnett's test and
    /// ignored by the other methods.
    pub fn compare_with_control(&self, method: MeansComparison, control: usize, alpha: f64) -> Vec<Comparison> {
        let levels: Vec<(usize, f64)> = self.groups.iter().map(|g| (g.n, g.mean)).collect();
        let mut out = if method == MeansComparison::Dunnett {
            compare_to_control(&levels, control, self.error.mean_sq, self.error.df, alpha)
        } else {
            compare_means(&levels, self.error.mean_sq, self.error.df, method, alpha)
        };
        // The one-way table reports the size of the statistic; the two-way table keeps the sign.
        for c in &mut out {
            c.stat = c.stat.abs();
        }
        out
    }
}

/// All pairwise comparisons of means, the way Origin's ANOVA tools do them: `levels` holds the size and
/// mean of each group (or factor level, or cell), `mse` and `dfe` the error mean square and its degrees of
/// freedom from the ANOVA table. The statistic of the t-based methods keeps the sign of the difference
/// (the studentized range q and Scheffe's F are never negative).
pub fn compare_means(
    levels: &[(usize, f64)],
    mse: f64,
    dfe: f64,
    method: MeansComparison,
    alpha: f64,
) -> Vec<Comparison> {
    if method == MeansComparison::Dunnett {
        return compare_to_control(levels, 0, mse, dfe, alpha);
    }
    let k = levels.len();
    let pairs = k * k.saturating_sub(1) / 2;
    let mut out = Vec::with_capacity(pairs);
    // A non-finite error mean square (infinite data) gives no usable comparisons; statrs would panic on NaN.
    if !mse.is_finite() || !dfe.is_finite() || levels.iter().any(|l| !l.1.is_finite()) {
        return out;
    }
    let Ok(t_dist) = StudentsT::new(0.0, 1.0, dfe) else {
        return out;
    };
    let t_crit = |a: f64| t_dist.inverse_cdf(1.0 - a / 2.0);
    let m = pairs as f64;
    // The critical values depend on the number of groups only, so they are found once.
    let q_crit = (method == MeansComparison::Tukey).then(|| qtukey(1.0 - alpha, k as f64, dfe));
    // Origin lists the pairs column by column: (1,2), (1,3), (2,3), (1,4), (2,4), (3,4), ...
    for j in 1..k {
        for i in 0..j {
            let ((ni, mi), (nj, mj)) = (levels[i], levels[j]);
            let diff = mj - mi;
            let se = (mse * (1.0 / ni as f64 + 1.0 / nj as f64)).sqrt();
            let t = if se == 0.0 { if diff == 0.0 { 0.0 } else { f64::INFINITY.copysign(diff) } } else { diff / se };
            let p_lsd = if t.is_infinite() { 0.0 } else { 2.0 * t_dist.cdf(-t.abs()) };
            let (stat, p, half) = match method {
                MeansComparison::FisherLsd => (t, p_lsd, t_crit(alpha) * se),
                MeansComparison::Bonferroni => (t, (m * p_lsd).min(1.0), t_crit(alpha / m) * se),
                // The adjusted p-value is found below, from all the p-values together.
                MeansComparison::Holm => (t, p_lsd, f64::NAN),
                MeansComparison::Dunnett => unreachable!("handled above"),
                MeansComparison::Sidak => {
                    (t, 1.0 - (1.0 - p_lsd).powf(m), t_crit(1.0 - (1.0 - alpha).powf(1.0 / m)) * se)
                }
                MeansComparison::Tukey => {
                    let q = std::f64::consts::SQRT_2 * t.abs();
                    let half = q_crit.expect("found above") / std::f64::consts::SQRT_2 * se;
                    (q, 1.0 - ptukey(q, k as f64, dfe), half)
                }
                MeansComparison::Scheffe => {
                    let f = t * t / (k as f64 - 1.0);
                    let dist = FisherSnedecor::new(k as f64 - 1.0, dfe).expect("positive degrees of freedom");
                    let crit = dist.inverse_cdf(1.0 - alpha);
                    (f, if f.is_infinite() { 0.0 } else { 1.0 - dist.cdf(f) }, ((k as f64 - 1.0) * crit).sqrt() * se)
                }
            };
            out.push(Comparison {
                i,
                j,
                diff,
                se,
                stat,
                p,
                lcl: diff - half,
                ucl: diff + half,
                significant: p < alpha,
            });
        }
    }
    if method == MeansComparison::Holm {
        let raw: Vec<f64> = out.iter().map(|c| c.p).collect();
        for (c, p) in out.iter_mut().zip(holm_adjust(&raw)) {
            c.p = p;
            c.significant = p < alpha;
        }
    }
    out
}

/// Holm's step-down adjustment of p-values: the smallest of `m` is multiplied by `m`, the next by `m - 1`, and
/// so on, and no adjusted value is smaller than one before it (in the order of the raw values); all are
/// capped at 1. The adjusted values come back in the order of the input; NaN stays NaN.
pub fn holm_adjust(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    let mut order: Vec<usize> = (0..m).filter(|&i| !p[i].is_nan()).collect();
    order.sort_by(|&a, &b| p[a].total_cmp(&p[b]));
    let count = order.len();
    let mut out = vec![f64::NAN; m];
    let mut running = 0.0_f64;
    for (rank, &i) in order.iter().enumerate() {
        running = running.max(((count - rank) as f64 * p[i]).min(1.0));
        out[i] = running;
    }
    out
}

/// Dunnett's comparison of every level with level `control` (counted from 0), the way [`compare_means`] takes its
/// arguments. Each comparison has `i = control` and `j` the other level; `diff` is `mean(j) - mean(control)`.
/// The p-values are adjusted for the whole family and the intervals are simultaneous.
pub fn compare_to_control(levels: &[(usize, f64)], control: usize, mse: f64, dfe: f64, alpha: f64) -> Vec<Comparison> {
    if control >= levels.len() || !mse.is_finite() || !dfe.is_finite() || levels.iter().any(|l| !l.1.is_finite()) {
        return Vec::new();
    }
    let (nc, mc) = levels[control];
    let others: Vec<usize> = (0..levels.len()).filter(|&j| j != control).collect();
    let sizes: Vec<usize> = others.iter().map(|&j| levels[j].0).collect();
    let loadings = crate::dunnett::dunnett_loadings(&sizes, nc);
    let d = crate::dunnett::dunnett_critical(alpha, &loadings, dfe);
    others
        .into_iter()
        .map(|j| {
            let (nj, mj) = levels[j];
            let diff = mj - mc;
            let se = (mse * (1.0 / nj as f64 + 1.0 / nc as f64)).sqrt();
            let t = if se == 0.0 { if diff == 0.0 { 0.0 } else { f64::INFINITY.copysign(diff) } } else { diff / se };
            let p = if t.is_infinite() { 0.0 } else { crate::dunnett::dunnett_p(t, &loadings, dfe) };
            Comparison {
                i: control,
                j,
                diff,
                se,
                stat: t,
                p,
                lcl: diff - d * se,
                ucl: diff + d * se,
                significant: p < alpha,
            }
        })
        .collect()
}

// ----- studentized range distribution ----------------------------------------------------------

/// Nodes and weights of the 16-point Gauss-Legendre rule on [-1, 1].
fn gauss_legendre_16() -> &'static [(f64, f64); 16] {
    static RULE: std::sync::OnceLock<[(f64, f64); 16]> = std::sync::OnceLock::new();
    RULE.get_or_init(|| {
        let n = 16usize;
        let mut out = [(0.0, 0.0); 16];
        for i in 0..n / 2 {
            // Newton iteration for the i-th positive root of the Legendre polynomial P16.
            let mut x = (std::f64::consts::PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
            let mut dp = 1.0;
            for _ in 0..100 {
                let (mut p0, mut p1) = (1.0, x);
                for k in 2..=n {
                    let kf = k as f64;
                    let p2 = ((2.0 * kf - 1.0) * x * p1 - (kf - 1.0) * p0) / kf;
                    p0 = p1;
                    p1 = p2;
                }
                dp = n as f64 * (x * p1 - p0) / (x * x - 1.0);
                let dx = p1 / dp;
                x -= dx;
                if dx.abs() < 1e-15 {
                    break;
                }
            }
            let w = 2.0 / ((1.0 - x * x) * dp * dp);
            out[i] = (-x, w);
            out[n - 1 - i] = (x, w);
        }
        out
    })
}

/// Integral of `f` over [a, b] with `panels` panels of the 16-point rule.
fn integrate(f: impl Fn(f64) -> f64, a: f64, b: f64, panels: usize) -> f64 {
    let rule = gauss_legendre_16();
    let h = (b - a) / panels as f64;
    let mut sum = 0.0;
    for p in 0..panels {
        let (lo, mid) = (a + h * p as f64, a + h * (p as f64 + 0.5));
        sum += rule.iter().map(|(x, w)| w * f(mid + x * h / 2.0)).sum::<f64>() * h / 2.0;
        let _ = lo;
    }
    sum
}

/// Probability that the range of `k` independent standard normal numbers is at most `w`.
fn range_cdf(w: f64, k: f64) -> f64 {
    if w <= 0.0 {
        return 0.0;
    }
    let norm = Normal::new(0.0, 1.0).expect("standard normal");
    let phi = |z: f64| (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    // The integrand is k phi(z) [Phi(z) - Phi(z - w)]^(k - 1); it is negligible outside [-9, 9 + w].
    let panels = (((w + 18.0) / 3.0).ceil() as usize).max(6);
    let v = k * integrate(|z| phi(z) * (norm.cdf(z) - norm.cdf(z - w)).powf(k - 1.0), -9.0, 9.0 + w, panels);
    v.clamp(0.0, 1.0)
}

/// Distribution function of the studentized range with `k` means and `df` degrees of freedom.
pub fn ptukey(q: f64, k: f64, df: f64) -> f64 {
    if q.is_nan() || k < 2.0 || df < 1.0 {
        return f64::NAN;
    }
    if q <= 0.0 {
        return 0.0;
    }
    if q.is_infinite() {
        return 1.0;
    }
    if df > 5000.0 {
        return range_cdf(q, k);
    }
    // The range is divided by s = sqrt(chi-square / df); s has the density below.
    let half = df / 2.0;
    let ln_c = std::f64::consts::LN_2 + half * half.ln() - ln_gamma(half);
    let density = |s: f64| if s <= 0.0 { 0.0 } else { (ln_c + (df - 1.0) * s.ln() - df * s * s / 2.0).exp() };
    let spread = 12.0 / (2.0 * df).sqrt();
    let (lo, hi) = ((1.0 - spread).max(0.0), 1.0 + spread + if df < 4.0 { 6.0 } else { 0.0 });
    let panels = ((hi - lo) * (2.0 * df).sqrt() / 2.0).ceil().clamp(8.0, 400.0) as usize;
    integrate(|s| density(s) * range_cdf(q * s, k), lo, hi, panels).clamp(0.0, 1.0)
}

/// Quantile of the studentized range: the `q` with `ptukey(q, k, df) = p`.
pub fn qtukey(p: f64, k: f64, df: f64) -> f64 {
    if !(p > 0.0 && p < 1.0) || k < 2.0 || df < 1.0 {
        return f64::NAN;
    }
    let (mut lo, mut hi) = (0.0, 4.0);
    while ptukey(hi, k, df) < p {
        lo = hi;
        hi *= 2.0;
        if hi > 1e4 {
            return f64::INFINITY;
        }
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if ptukey(mid, k, df) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-9 * hi {
            break;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    #[test]
    fn infinite_data_do_not_panic_in_comparisons() {
        let (a, b, c) = ([1.0, 2.0, 3.0], [2.0, f64::INFINITY, 4.0], [5.0, 6.0, 8.0]);
        let an = anova_one_way(&[&a, &b, &c]).unwrap();
        for m in [MeansComparison::FisherLsd, MeansComparison::Tukey, MeansComparison::Scheffe, MeansComparison::Sidak]
        {
            let _ = compare_means(&[(3, 2.0), (3, f64::INFINITY), (3, 6.0)], an.error.mean_sq, an.error.df, m, 0.05);
        }
    }

    #[test]
    fn tukey_with_few_error_degrees_of_freedom_is_fast() {
        let t = std::time::Instant::now();
        let q = qtukey(0.95, 4.0, 3.0);
        assert!((q - 6.8200).abs() < 0.01, "q = {q}");
        assert!(t.elapsed().as_secs_f64() < 2.0);
    }

    #[test]
    fn constant_groups_do_not_hang_or_panic() {
        let (a, b, c) = ([3.0; 3], [4.0; 3], [5.0; 3]);
        let an = anova_one_way(&[&a, &b, &c]).unwrap();
        assert_eq!(an.p, 0.0);
        for m in [
            MeansComparison::Tukey,
            MeansComparison::Bonferroni,
            MeansComparison::Sidak,
            MeansComparison::FisherLsd,
            MeansComparison::Scheffe,
        ] {
            let r = an.compare(m, 0.05);
            assert_eq!(r.len(), 3);
            assert!(r.iter().all(|c| c.significant));
        }
        let same = anova_one_way(&[&[3.0; 6], &[3.0; 4]]).unwrap();
        assert!(same.p.is_nan());
        let r = same.compare(MeansComparison::Tukey, 0.05);
        assert!(!r[0].significant);
    }

    use super::*;

    #[test]
    fn studentized_range_matches_published_critical_values() {
        // Table values of the 5 % point: q(3 means, 20 df) = 3.578, q(4, 10) = 4.327, q(2, 5) = 3.635.
        assert!((qtukey(0.95, 3.0, 20.0) - 3.5779).abs() < 2e-3);
        assert!((qtukey(0.95, 4.0, 10.0) - 4.3266).abs() < 2e-3);
        assert!((qtukey(0.95, 2.0, 5.0) - 3.6354).abs() < 2e-3);
        // With two means the range is |t| times sqrt 2.
        let t = StudentsT::new(0.0, 1.0, 7.0).unwrap().inverse_cdf(0.975);
        assert!((qtukey(0.95, 2.0, 7.0) - t * std::f64::consts::SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn two_groups_agree_with_the_t_test() {
        let a = [1.0, 2.0, 3.0, 4.5];
        let b = [3.0, 4.0, 6.0, 7.0, 7.5];
        let an = anova_one_way(&[&a, &b]).unwrap();
        let t = crate::hypothesis::ttest_two(&b, &a, 0.0, crate::hypothesis::Tail::Two, 0.05, true).unwrap();
        assert!((an.f - t.t * t.t).abs() < 1e-10);
        assert!((an.p - t.p).abs() < 1e-10);
        let tukey = an.compare(MeansComparison::Tukey, 0.05);
        assert!((tukey[0].p - t.p).abs() < 1e-6);
    }

    #[test]
    fn errors_are_named() {
        assert_eq!(anova_one_way(&[&[1.0, 2.0]]), Err(AnovaError::TooFewGroups));
        assert_eq!(anova_one_way(&[&[1.0, 2.0], &[f64::NAN]]), Err(AnovaError::EmptyGroup(1)));
        assert_eq!(anova_one_way(&[&[1.0], &[2.0]]), Err(AnovaError::NoReplication));
    }
}
