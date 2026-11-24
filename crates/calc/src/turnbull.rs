//! Turnbull's nonparametric estimate of the failure distribution from arbitrarily censored rows (for Origin 2026b's
//! reliability and survival analysis; not in Origin 2016, so nothing here is measured in Origin;
//! `docs/spec/statistics.md` section 53).
//!
//! Each row is a set of times in which the unit failed: exact `[t, t]`, right-censored `(t, inf)`, left-censored
//! `(-inf, t]` or interval-censored `(a, b]` (the conventions of [`Obs`]). The estimate puts all its mass on the
//! Turnbull (innermost) intervals: a left end of some row followed, in the sorted ends, by a right end of some row with
//! no other end between them (at equal times a closed left end sorts before a right end, and a right end before an
//! open left end). The probabilities maximize `sum ln P(row)` and are found by the self-consistency (EM) iteration
//! `s_j <- s_j * g_j`, `g_j` the mean over rows of `[row covers j] / P(row)`, stopped when every `g_j` is at most
//! `1 + 1e-12` (as `sum s_j g_j = 1`, this is the Kuhn-Tucker condition, and the log-likelihood is then within
//! `n * 1e-12` of its maximum). A row
//! covers a run of neighbouring intervals, so one iteration costs a pass over the rows and one over the intervals.
//! With exact and right-censored rows only the estimate is the Kaplan-Meier estimator.

use crate::lifedata::Obs;

/// The estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct Turnbull {
    /// Lower end of each Turnbull interval (`-inf` before every time).
    pub lower: Vec<f64>,
    /// Upper end of each Turnbull interval (`inf` after every time).
    pub upper: Vec<f64>,
    /// Whether the lower end belongs to the interval (exact times), else the interval is `(lower, upper]`.
    pub lower_closed: Vec<bool>,
    /// Probability of failing in each interval.
    pub probability: Vec<f64>,
    /// Survival after each interval: `1 - ` the running sum of the probabilities.
    pub survival: Vec<f64>,
    /// The maximized `sum ln P(row)`.
    pub log_likelihood: f64,
    /// Self-consistency iterations used.
    pub iterations: usize,
    /// The Kuhn-Tucker condition held to 1e-12 within the iteration limit.
    pub converged: bool,
    /// Number of rows.
    pub n: usize,
}

/// Why there is no estimate.
#[derive(Debug, Clone, PartialEq)]
pub enum TurnbullError {
    /// No rows.
    Empty,
    /// A row with a missing time, or an interval whose start is not before its end.
    BadRow(usize),
}

impl std::fmt::Display for TurnbullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnbullError::Empty => write!(f, "there are no rows to estimate from"),
            TurnbullError::BadRow(i) => write!(f, "row {} is not a failure window", i + 1),
        }
    }
}

impl std::error::Error for TurnbullError {}

/// A position on the time line: the time, then 0 for "at the time" or 1 for "just after it" (an open left end).
type Key = (f64, u8);

fn ends(o: &Obs) -> Option<(Key, Key)> {
    let ok = |t: f64| t.is_finite();
    match *o {
        Obs::Exact(t) if ok(t) => Some(((t, 0), (t, 0))),
        Obs::Right(t) if ok(t) => Some(((t, 1), (f64::INFINITY, 0))),
        Obs::Left(t) if ok(t) => Some(((f64::NEG_INFINITY, 0), (t, 0))),
        Obs::Interval(a, b) if ok(a) && ok(b) && a < b => Some(((a, 1), (b, 0))),
        _ => None,
    }
}

fn cmp(a: &Key, b: &Key) -> std::cmp::Ordering {
    a.0.total_cmp(&b.0).then(a.1.cmp(&b.1))
}

/// The iteration limit.
const MAX_ITER: usize = 200_000;
/// The Kuhn-Tucker tolerance.
const TOL: f64 = 1e-12;

/// Turnbull's estimate from the rows (see the module notes).
pub fn turnbull(obs: &[Obs]) -> Result<Turnbull, TurnbullError> {
    if obs.is_empty() {
        return Err(TurnbullError::Empty);
    }
    let mut rows: Vec<(Key, Key)> = Vec::with_capacity(obs.len());
    for (i, o) in obs.iter().enumerate() {
        rows.push(ends(o).ok_or(TurnbullError::BadRow(i))?);
    }
    // Identical rows count once with a weight.
    rows.sort_by(|a, b| cmp(&a.0, &b.0).then(cmp(&a.1, &b.1)));
    let mut groups: Vec<((Key, Key), f64)> = Vec::new();
    for r in rows {
        match groups.last_mut() {
            Some((g, w)) if cmp(&g.0, &r.0).is_eq() && cmp(&g.1, &r.1).is_eq() => *w += 1.0,
            _ => groups.push((r, 1.0)),
        }
    }
    // The ends in order, left ends before right ends at the same position.
    let mut all: Vec<(Key, bool)> = groups.iter().flat_map(|((l, r), _)| [(*l, true), (*r, false)]).collect();
    all.sort_by(|a, b| cmp(&a.0, &b.0).then(b.1.cmp(&a.1)));
    all.dedup_by(|a, b| a.1 == b.1 && cmp(&a.0, &b.0).is_eq());
    let mut intervals: Vec<(Key, Key)> = Vec::new();
    for w in all.windows(2) {
        if w[0].1 && !w[1].1 {
            intervals.push((w[0].0, w[1].0));
        }
    }
    let m = intervals.len();
    // The run of intervals each row covers: those with the row's left end at or before theirs and right end at or after.
    let cover: Vec<(usize, usize)> = groups
        .iter()
        .map(|((l, r), _)| {
            let lo = intervals.partition_point(|iv| cmp(&iv.0, l).is_lt());
            let hi = intervals.partition_point(|iv| cmp(&iv.1, r).is_le());
            (lo, hi)
        })
        .collect();
    let weights: Vec<f64> = groups.iter().map(|(_, w)| *w).collect();
    let total = obs.len() as f64;
    let mut s = vec![1.0 / m as f64; m];
    let mut prefix = vec![0.0; m + 1];
    let mut grad = vec![0.0; m + 1];
    let mut iterations = 0;
    let mut converged = false;
    loop {
        for j in 0..m {
            prefix[j + 1] = prefix[j] + s[j];
        }
        grad.iter_mut().for_each(|g| *g = 0.0);
        for (&(lo, hi), w) in cover.iter().zip(&weights) {
            let p = prefix[hi] - prefix[lo];
            grad[lo] += w / p;
            grad[hi] -= w / p;
        }
        // grad[j] becomes the mean over rows of [row covers j] / P(row).
        let mut run = 0.0;
        let mut worst: f64 = 0.0;
        for g in grad.iter_mut().take(m) {
            run += *g;
            *g = run / total;
            worst = worst.max(*g - 1.0);
        }
        if worst <= TOL {
            converged = true;
            break;
        }
        if iterations == MAX_ITER {
            break;
        }
        iterations += 1;
        let mut sum = 0.0;
        for (v, g) in s.iter_mut().zip(&grad) {
            *v *= g;
            sum += *v;
        }
        s.iter_mut().for_each(|v| *v /= sum);
    }
    let log_likelihood = cover.iter().zip(&weights).map(|(&(lo, hi), w)| w * s[lo..hi].iter().sum::<f64>().ln()).sum();
    let mut left = 1.0;
    let survival = s
        .iter()
        .map(|p| {
            left -= p;
            left.max(0.0)
        })
        .collect();
    Ok(Turnbull {
        lower: intervals.iter().map(|iv| iv.0.0).collect(),
        upper: intervals.iter().map(|iv| iv.1.0).collect(),
        lower_closed: intervals.iter().map(|iv| iv.0.1 == 0 && iv.0.0.is_finite()).collect(),
        probability: s,
        survival,
        log_likelihood,
        iterations,
        converged,
        n: obs.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_right_censored_rows_give_kaplan_meier() {
        // Failures at 1, 2 (twice), 4; censored at 2 and 3; 6 rows.
        let obs =
            [Obs::Exact(1.0), Obs::Exact(2.0), Obs::Exact(2.0), Obs::Right(2.0), Obs::Right(3.0), Obs::Exact(4.0)];
        let t = turnbull(&obs).unwrap();
        assert!(t.converged);
        assert_eq!(t.lower, vec![1.0, 2.0, 4.0]);
        assert_eq!(t.upper, vec![1.0, 2.0, 4.0]);
        assert!(t.lower_closed.iter().all(|c| *c));
        // Kaplan-Meier: 5/6, then 5/6 * 3/5 = 1/2, then at 4 one of one at risk: 0.
        let km = [5.0 / 6.0, 0.5, 0.0];
        for (a, b) in t.survival.iter().zip(km) {
            assert!((a - b).abs() < 1e-9, "{:?}", t.survival);
        }
    }

    #[test]
    fn overlapping_windows_share_the_innermost_interval() {
        // (0, 2] and (1, 3] overlap in (1, 2]; (4, 5] stands alone.
        let obs = [Obs::Interval(0.0, 2.0), Obs::Interval(1.0, 3.0), Obs::Interval(4.0, 5.0)];
        let t = turnbull(&obs).unwrap();
        assert_eq!((t.lower.clone(), t.upper.clone()), (vec![1.0, 4.0], vec![2.0, 5.0]));
        assert!(!t.lower_closed[0]);
        assert!((t.probability[0] - 2.0 / 3.0).abs() < 1e-9 && (t.probability[1] - 1.0 / 3.0).abs() < 1e-9);
        assert!((t.log_likelihood - 3.0 * (1.0f64 / 3.0).ln() - 2.0 * 2.0f64.ln()).abs() < 1e-9);
    }

    #[test]
    fn censored_ends_reach_the_infinities() {
        let t = turnbull(&[Obs::Left(1.0), Obs::Right(3.0)]).unwrap();
        assert_eq!(t.lower, vec![f64::NEG_INFINITY, 3.0]);
        assert_eq!(t.upper, vec![1.0, f64::INFINITY]);
        assert!(!t.lower_closed[0] && !t.lower_closed[1]);
        assert!((t.probability[0] - 0.5).abs() < 1e-12);
        // A right end equal to an open left end does not meet it.
        let t = turnbull(&[Obs::Interval(1.0, 2.0), Obs::Interval(2.0, 3.0)]).unwrap();
        assert_eq!(t.lower, vec![1.0, 2.0]);
        // A closed (exact) left end meets a right end at the same time.
        let t = turnbull(&[Obs::Interval(1.0, 2.0), Obs::Exact(2.0)]).unwrap();
        assert_eq!((t.lower.clone(), t.upper.clone(), t.probability.clone()), (vec![2.0], vec![2.0], vec![1.0]));
    }

    #[test]
    fn bad_rows_are_refused() {
        assert_eq!(turnbull(&[]), Err(TurnbullError::Empty));
        assert_eq!(turnbull(&[Obs::Exact(1.0), Obs::Interval(3.0, 2.0)]), Err(TurnbullError::BadRow(1)));
        assert_eq!(turnbull(&[Obs::Right(f64::NAN)]), Err(TurnbullError::BadRow(0)));
    }
}
