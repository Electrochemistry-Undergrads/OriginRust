//! Kaplan-Meier (product-limit) estimate of a survival function, its quartiles and mean, and the rank tests
//! that compare groups (Statistics > Survival Analysis > Kaplan-Meier Estimator).
//!
//! What Origin's `kaplanmeier` X-Function does, measured on synthetic data (`docs/spec/statistics.md`,
//! section 20; fixtures in `crates/calc/tests/origin_survival.rs`):
//!
//! - Cases are sorted by time; at tied times the events come before the censored cases (so a case censored at
//!   time `t` is still at risk at `t`). The table has one row per case: the survival estimate and its
//!   standard error appear on the last of the tied event rows only (the others, and the censored rows, are
//!   blank); next to it the running number of events and the number of cases still at risk after the row.
//! - `S(t) = prod (1 - d_i / n_i)` over the event times up to `t`; the standard error is Greenwood's
//!   `S(t) sqrt(sum d_i / (n_i (n_i - d_i)))`, taken as 0 once `S` reaches 0.
//! - The p-th quartile (p = 25, 50, 75) is the smallest event time with `S(t) <= 1 - p/100`. Its limits are the
//!   smallest event times at which the **linear** limits `S -/+ z se` (z of the normal distribution for the
//!   chosen confidence level) are at or below `1 - p/100`: the lower limit of the quartile uses the lower
//!   limit of `S`, the upper limit of the quartile the upper limit of `S`. When the upper limit is never
//!   reached Origin reports the largest event time (measured once, for a quartile whose estimate is reached; the
//!   lower limit and a quartile whose estimate is never reached are `None`, which was not measured).
//! - The confidence limits of the curve itself are the linear limits `S -/+ z se`, **not cut off** at 0 and 1 (Origin's
//!   survival-function sheet holds `1.1163...` as an upper limit; measured with 90 and 95 percent, one and two
//!   groups, `tools/origin-ref/kaplan-plots2.lines.txt`). They stand on the event rows; the start is `(0, 1, 1)`
//!   and a curve that has reached 0 has the limits 0 and 0.
//! - The mean is the area under the curve up to the **largest time, censored or not**; its standard error is
//!   `sqrt(D / (D - 1) sum A_i^2 d_i / (n_i (n_i - d_i)))` with `A_i` the area after the i-th event time and `D` the
//!   number of events; the factor `D / (D - 1)` is applied only when the curve reaches 0 (the largest time is an
//!   event), not when the largest time is censored (measured in six runs; one event: not measured), and its limits
//!   are `mean -/+ z se`.
//! - Rank tests of groups use weights 1 (log-rank), `n_i` (Breslow, generalized Wilcoxon) and `sqrt(n_i)`
//!   (Tarone-Ware); the statistic is chi-square with `groups - 1` degrees of freedom. Two groups measured; for
//!   more groups the textbook quadratic form is used (not measured).
//!
//! Confidence limits of `S(t)` itself could not be read from Origin (the option only works together with its
//! plot, which the scripting interface refuses); the linear limits above are Sheaf's choice, inferred from
//! the quartile limits.

use nalgebra::{DMatrix, DVector};
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal};

/// Why a survival analysis could not be made.
#[derive(Debug, Clone, PartialEq)]
pub enum SurvivalError {
    /// No case with a time.
    NoCases,
    /// A time is infinite or negative.
    BadTime,
    /// The confidence level is not between 0 and 100 percent.
    BadLevel,
    /// A rank test needs at least two groups with cases.
    TooFewGroups,
}

impl std::fmt::Display for SurvivalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SurvivalError::NoCases => write!(f, "no case has a time"),
            SurvivalError::BadTime => write!(f, "a time is negative or not finite"),
            SurvivalError::BadLevel => write!(f, "the confidence level must lie between 0 and 100 percent"),
            SurvivalError::TooFewGroups => write!(f, "a comparison of groups needs at least two groups"),
        }
    }
}

impl std::error::Error for SurvivalError {}

/// One case of the survival table.
#[derive(Debug, Clone, PartialEq)]
pub struct SurvivalRow {
    pub time: f64,
    /// Censored cases (Origin marks them with `+`).
    pub censored: bool,
    /// Survival estimate and standard error, on the last event row of each time only.
    pub survival: Option<(f64, f64)>,
    /// Events up to and including this row.
    pub events: usize,
    /// Cases still at risk after this row.
    pub at_risk: usize,
}

/// A step of the survival curve: the values from `time` on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurvivalStep {
    pub time: f64,
    pub survival: f64,
    pub se: f64,
    /// Linear limits `S -/+ z se`, kept inside [0, 1].
    pub lower: f64,
    pub upper: f64,
    /// Cases at risk just before `time` and events at `time`.
    pub at_risk: usize,
    pub events: usize,
}

/// A quartile survival time with its limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quartile {
    /// 25, 50 or 75.
    pub percent: f64,
    pub estimate: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

/// The Kaplan-Meier estimate of one sample.
#[derive(Debug, Clone, PartialEq)]
pub struct KaplanMeier {
    pub n: usize,
    pub events: usize,
    pub censored: usize,
    /// The table, one row per case.
    pub rows: Vec<SurvivalRow>,
    /// The curve, one step per distinct event time (the start `(0, 1)` is not included).
    pub steps: Vec<SurvivalStep>,
    /// 25, 50 and 75 percent.
    pub quartiles: [Quartile; 3],
    pub mean: f64,
    pub mean_se: f64,
    pub mean_lower: f64,
    pub mean_upper: f64,
    /// The largest time, censored or not: where the mean is cut off.
    pub last_time: f64,
}

impl KaplanMeier {
    /// The cases that were censored, as plot points `(time, survival at that time)`.
    pub fn censor_marks(&self) -> Vec<(f64, f64)> {
        self.rows.iter().filter(|r| r.censored).map(|r| (r.time, self.survival_at(r.time))).collect()
    }

    /// The estimate at `t` (1 before the first event).
    pub fn survival_at(&self, t: f64) -> f64 {
        let k = self.steps.partition_point(|s| s.time <= t);
        k.checked_sub(1).map_or(1.0, |k| self.steps[k].survival)
    }
}

fn z_of(confidence: f64) -> Result<f64, SurvivalError> {
    if !(confidence > 0.0 && confidence < 100.0) {
        return Err(SurvivalError::BadLevel);
    }
    Ok(Normal::new(0.0, 1.0).expect("standard normal").inverse_cdf(0.5 + confidence / 200.0))
}

/// Kaplan-Meier estimate of `time` with `censored[i]` true for a censored case; `confidence` in percent.
/// Cases with a NaN time are left out.
pub fn kaplan_meier(time: &[f64], censored: &[bool], confidence: f64) -> Result<KaplanMeier, SurvivalError> {
    let z = z_of(confidence)?;
    let mut cases: Vec<(f64, bool)> =
        time.iter().zip(censored).filter(|(t, _)| !t.is_nan()).map(|(&t, &c)| (t, c)).collect();
    if cases.is_empty() {
        return Err(SurvivalError::NoCases);
    }
    if cases.iter().any(|(t, _)| !t.is_finite() || *t < 0.0) {
        return Err(SurvivalError::BadTime);
    }
    // Events first among ties.
    cases.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let n = cases.len();
    let mut rows = Vec::with_capacity(n);
    let mut steps: Vec<SurvivalStep> = Vec::new();
    let (mut s, mut green, mut events) = (1.0f64, 0.0f64, 0usize);
    let mut i = 0;
    while i < n {
        let t = cases[i].0;
        let j = cases[i..].iter().position(|c| c.0 != t).map_or(n, |p| i + p);
        let d = cases[i..j].iter().filter(|c| !c.1).count();
        let at_risk = n - i;
        if d > 0 {
            s *= 1.0 - d as f64 / at_risk as f64;
            if at_risk > d {
                green += d as f64 / (at_risk as f64 * (at_risk - d) as f64);
            }
            let se = if s > 0.0 { s * green.sqrt() } else { 0.0 };
            steps.push(SurvivalStep {
                time: t,
                survival: s,
                se,
                lower: s - z * se,
                upper: s + z * se,
                at_risk,
                events: d,
            });
        }
        for (k, c) in cases[i..j].iter().enumerate() {
            if !c.1 {
                events += 1;
            }
            let survival = (!c.1 && k + 1 == d).then(|| {
                let st = steps.last().expect("a step was pushed");
                (st.survival, st.se)
            });
            rows.push(SurvivalRow { time: t, censored: c.1, survival, events, at_risk: n - (i + k) - 1 });
        }
        i = j;
    }

    let quartiles = [25.0, 50.0, 75.0].map(|percent| {
        let level = 1.0 - percent / 100.0;
        let first = |f: &dyn Fn(&SurvivalStep) -> f64| steps.iter().find(|st| f(st) <= level).map(|st| st.time);
        let estimate = first(&|st| st.survival);
        // The largest-event-time fallback only applies to a quartile that is reached.
        let upper = first(&|st| st.upper).or_else(|| estimate.and(steps.last().map(|st| st.time)));
        Quartile { percent, estimate, lower: first(&|st| st.lower), upper }
    });

    // Mean: area under the curve up to the largest time; areas[k] is the area after step k.
    let last_time = cases[n - 1].0;
    let mut areas = vec![0.0; steps.len()];
    let mut area = 0.0;
    for k in (0..steps.len()).rev() {
        let end = steps.get(k + 1).map_or(last_time, |next| next.time);
        area += steps[k].survival * (end - steps[k].time);
        areas[k] = area;
    }
    let first_time = steps.first().map_or(last_time, |st| st.time);
    let mean = first_time + areas.first().copied().unwrap_or(0.0);
    let var: f64 = steps
        .iter()
        .zip(&areas)
        .filter(|(st, _)| st.at_risk > st.events)
        .map(|(st, a)| a * a * st.events as f64 / (st.at_risk as f64 * (st.at_risk - st.events) as f64))
        .sum();
    // Origin multiplies the variance by D / (D - 1), D the number of events, when the curve reaches 0 (the largest
    // time is an event); with a censored largest time (restricted mean) it leaves the factor out. Measured: 10/9,
    // 5/4 and 10/9 with the curve at 0, 1 in the two runs ending on a censored case.
    let reaches_zero = steps.last().is_some_and(|st| st.survival <= 0.0);
    let correction = if reaches_zero && events > 1 { events as f64 / (events - 1) as f64 } else { 1.0 };
    let mean_se = (var * correction).sqrt();
    Ok(KaplanMeier {
        n,
        events,
        censored: n - events,
        rows,
        steps,
        quartiles,
        mean,
        mean_se,
        mean_lower: mean - z * mean_se,
        mean_upper: mean + z * mean_se,
        last_time,
    })
}

/// Weights of the rank tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankTest {
    /// Weight 1.
    LogRank,
    /// Weight `n` (generalized Wilcoxon).
    Breslow,
    /// Weight `sqrt(n)`.
    TaroneWare,
}

impl RankTest {
    pub fn name(self) -> &'static str {
        match self {
            RankTest::LogRank => "Log-rank",
            RankTest::Breslow => "Breslow",
            RankTest::TaroneWare => "Tarone-Ware",
        }
    }
}

/// Result of a rank test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankResult {
    pub chi_square: f64,
    pub df: usize,
    pub p: f64,
}

/// Compares the survival of groups, each `(times, censored)`; NaN times are left out.
pub fn rank_test(groups: &[(Vec<f64>, Vec<bool>)], test: RankTest) -> Result<RankResult, SurvivalError> {
    let groups: Vec<Vec<(f64, bool)>> = groups
        .iter()
        .map(|(t, c)| t.iter().zip(c).filter(|(t, _)| !t.is_nan()).map(|(&t, &c)| (t, c)).collect::<Vec<_>>())
        .filter(|g| !g.is_empty())
        .collect();
    let k = groups.len();
    if k < 2 {
        return Err(SurvivalError::TooFewGroups);
    }
    let mut times: Vec<f64> = groups.iter().flatten().filter(|c| !c.1).map(|c| c.0).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let mut groups = groups;
    for g in &mut groups {
        g.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    // Per group, the index of the first case with time >= t (the cases before it have left the risk set).
    let mut ptr = vec![0usize; k];
    let m = k - 1;
    let mut diff = DVector::<f64>::zeros(m);
    let mut cov = DMatrix::<f64>::zeros(m, m);
    for &t in &times {
        let mut at_risk = vec![0.0; k];
        let mut deaths = vec![0.0; k];
        for (gi, g) in groups.iter().enumerate() {
            while ptr[gi] < g.len() && g[ptr[gi]].0 < t {
                ptr[gi] += 1;
            }
            at_risk[gi] = (g.len() - ptr[gi]) as f64;
            deaths[gi] = g[ptr[gi]..].iter().take_while(|c| c.0 == t).filter(|c| !c.1).count() as f64;
        }
        let n: f64 = at_risk.iter().sum();
        let d: f64 = deaths.iter().sum();
        let w = match test {
            RankTest::LogRank => 1.0,
            RankTest::Breslow => n,
            RankTest::TaroneWare => n.sqrt(),
        };
        for a in 0..m {
            diff[a] += w * (deaths[a] - d * at_risk[a] / n);
            if n > 1.0 {
                let f = w * w * d * (n - d) / (n * n * (n - 1.0));
                for b in 0..m {
                    let delta = if a == b { n } else { 0.0 };
                    cov[(a, b)] += f * at_risk[a] * (delta - at_risk[b]);
                }
            }
        }
    }
    let chi_square = match cov.try_inverse() {
        Some(inv) => (diff.transpose() * inv * &diff)[(0, 0)],
        None => 0.0,
    };
    let p = ChiSquared::new(m as f64).expect("df").sf(chi_square);
    Ok(RankResult { chi_square, df: m, p })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "{a} vs {b}");
    }

    /// Hand worked: times 2, 3 (censored), 4, 4, 6 (censored): at 2: 1 - 1/5 = 0.8; at 4: n = 3, two events,
    /// S = 0.8 * (1 - 2/3) = 4/15.
    #[test]
    fn product_limit_with_a_censored_case_and_a_tie() {
        let km = kaplan_meier(&[2.0, 3.0, 4.0, 4.0, 6.0], &[false, true, false, false, true], 95.0).unwrap();
        assert_eq!((km.n, km.events, km.censored), (5, 3, 2));
        close(km.steps[0].survival, 0.8);
        close(km.steps[0].se, 0.8 * (1.0f64 / 20.0).sqrt());
        close(km.steps[1].survival, 4.0 / 15.0);
        close(km.steps[1].se, 4.0 / 15.0 * (1.0f64 / 20.0 + 2.0 / 3.0).sqrt());
        // Rows: the tied events come first, the estimate sits on the second.
        assert!(km.rows[2].survival.is_none() && km.rows[3].survival.is_some());
        assert_eq!(km.rows.iter().map(|r| r.at_risk).collect::<Vec<_>>(), vec![4, 3, 2, 1, 0]);
        // Mean: 2 + 0.8 * 2 + 4/15 * 2
        close(km.mean, 2.0 + 1.6 + 8.0 / 15.0);
        assert_eq!(km.quartiles[0].estimate, Some(4.0));
        assert_eq!(km.quartiles[2].estimate, None, "S never reaches 0.25");
        assert_eq!(km.quartiles[2].upper, None, "no limit for a quartile that is not reached");
        let marks = km.censor_marks();
        assert_eq!(marks.len(), 2);
        close(marks[0].1, 0.8);
        close(marks[1].1, 4.0 / 15.0);
    }

    #[test]
    fn errors() {
        assert_eq!(kaplan_meier(&[], &[], 95.0), Err(SurvivalError::NoCases));
        assert_eq!(kaplan_meier(&[1.0], &[false], 100.0), Err(SurvivalError::BadLevel));
        assert_eq!(kaplan_meier(&[-1.0], &[false], 95.0), Err(SurvivalError::BadTime));
        assert_eq!(
            rank_test(&[(vec![1.0], vec![false]), (vec![], vec![])], RankTest::LogRank),
            Err(SurvivalError::TooFewGroups)
        );
    }

    /// Group A dies at 1 and 3, group B at 2 and 4.
    /// Time 1: n = 4, nA = 2, d = 1: O-E = 1/2, var = 1/4.
    /// Time 2: n = 3, nA = 1: -1/3, var = (1/3)(2/3) = 2/9.
    /// Time 3: n = 2, nA = 1: 1/2, var = 1/4. Time 4: n = 1: variance 0.
    /// Sum O-E = 2/3; var = 13/18; chi = (4/9) / (13/18) = 8/13.
    #[test]
    fn log_rank_by_hand() {
        let g = vec![(vec![1.0, 3.0], vec![false, false]), (vec![2.0, 4.0], vec![false, false])];
        let r = rank_test(&g, RankTest::LogRank).unwrap();
        close(r.chi_square, 8.0 / 13.0);
        assert_eq!(r.df, 1);
        let b = rank_test(&g, RankTest::Breslow).unwrap();
        // weights 4, 3, 2: O-E = 4/2 - 3/3 + 2/2 = 2; var = 16/4 + 9*2/9 + 4/4 = 7; chi = 4/7
        close(b.chi_square, 4.0 / 7.0);
    }
}
