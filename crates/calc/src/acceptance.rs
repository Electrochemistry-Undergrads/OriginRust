//! Acceptance sampling by attributes (T369, Origin 2026b's Acceptance Sampling app): a single sampling
//! plan takes a sample of `n` items from a lot and accepts the lot when at most `c` of them are defective.
//! The probability of acceptance at a fraction defective `p` is the binomial `P(X <= c)` with `X ~ B(n, p)`
//! (a "type B" operating characteristic, which treats the lot as large).
//!
//! - [`design`] finds the plan with the smallest sample size whose producer's risk (rejecting a lot at the
//!   acceptable quality level AQL) is at most `alpha` and whose consumer's risk (accepting a lot at the lot
//!   tolerance percent defective LTPD) is at most `beta`.
//! - With a lot size `N` and rectifying inspection (rejected lots are sorted completely), the average
//!   outgoing quality is `AOQ(p) = p Pa(p) (N - n) / N`, its maximum over `p` is the AOQL, and the average
//!   total inspection is `ATI(p) = n + (1 - Pa(p)) (N - n)`.

use statrs::distribution::{Binomial, DiscreteCDF};

/// A single sampling plan: sample size and acceptance number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplingPlan {
    pub n: u64,
    pub c: u64,
}

/// Why a plan cannot be found or evaluated.
#[derive(Debug, Clone, PartialEq)]
pub enum AcceptanceError {
    /// An input outside its range, named.
    Bad(String),
    /// No plan with a sample of at most this size meets both risks.
    TooLarge(u64),
}

impl std::fmt::Display for AcceptanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcceptanceError::Bad(s) => f.write_str(s),
            AcceptanceError::TooLarge(n) => {
                write!(f, "no plan with a sample of at most {n} items meets both risks; widen AQL to LTPD or the risks")
            }
        }
    }
}

impl std::error::Error for AcceptanceError {}

/// The largest sample size [`design`] tries.
pub const MAX_SAMPLE: u64 = 1_000_000;

/// The probability of accepting a lot whose fraction defective is `p`: `P(X <= c)`, `X ~ B(n, p)`.
/// NaN for `p` outside 0 to 1.
pub fn accept_probability(plan: SamplingPlan, p: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    if plan.c >= plan.n {
        return 1.0;
    }
    Binomial::new(p, plan.n).map_or(f64::NAN, |b| b.cdf(plan.c))
}

fn check_fraction(name: &str, v: f64) -> Result<(), AcceptanceError> {
    if v > 0.0 && v < 1.0 {
        Ok(())
    } else {
        Err(AcceptanceError::Bad(format!("{name} must be between 0 and 1 (not included), not {v}")))
    }
}

/// The smallest `n > c` with `Pa(n, c, p) <= beta`, if one is at most `max_n` (`Pa` falls as `n` grows).
fn smallest_n(c: u64, p: f64, beta: f64, max_n: u64) -> Option<u64> {
    let pa = |n: u64| accept_probability(SamplingPlan { n, c }, p);
    let mut hi = c + 1;
    while pa(hi) > beta {
        if hi >= max_n {
            return None;
        }
        hi = (hi * 2).min(max_n);
    }
    let mut lo = c; // Pa(c) = 1 > beta
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if pa(mid) <= beta { hi = mid } else { lo = mid }
    }
    Some(hi)
}

/// The single sampling plan with the smallest sample size that accepts a lot at `aql` with probability at
/// least `1 - alpha` and one at `ltpd` with probability at most `beta`. For each acceptance number from 0
/// up, the smallest sample meeting the consumer's risk is taken; the first that also meets the producer's
/// risk is the plan (a larger acceptance number needs a larger sample).
pub fn design(aql: f64, ltpd: f64, alpha: f64, beta: f64) -> Result<SamplingPlan, AcceptanceError> {
    check_fraction("AQL", aql)?;
    check_fraction("LTPD", ltpd)?;
    check_fraction("the producer's risk", alpha)?;
    check_fraction("the consumer's risk", beta)?;
    if aql >= ltpd {
        return Err(AcceptanceError::Bad(format!("AQL ({aql}) must be below LTPD ({ltpd})")));
    }
    for c in 0..MAX_SAMPLE {
        let Some(n) = smallest_n(c, ltpd, beta, MAX_SAMPLE) else {
            return Err(AcceptanceError::TooLarge(MAX_SAMPLE));
        };
        if accept_probability(SamplingPlan { n, c }, aql) >= 1.0 - alpha {
            return Ok(SamplingPlan { n, c });
        }
    }
    Err(AcceptanceError::TooLarge(MAX_SAMPLE))
}

/// Checks a plan given by hand.
pub fn check_plan(plan: SamplingPlan) -> Result<(), AcceptanceError> {
    if plan.n == 0 || plan.c >= plan.n {
        return Err(AcceptanceError::Bad(format!(
            "the sample size must be above 0 and above the acceptance number (n {}, c {})",
            plan.n, plan.c
        )));
    }
    Ok(())
}

/// Average outgoing quality at `p` with rectifying inspection of lots of `lot` items.
pub fn aoq(plan: SamplingPlan, lot: u64, p: f64) -> f64 {
    if lot < plan.n {
        return f64::NAN;
    }
    p * accept_probability(plan, p) * (lot - plan.n) as f64 / lot as f64
}

/// Average total inspection per lot at `p`.
pub fn ati(plan: SamplingPlan, lot: u64, p: f64) -> f64 {
    if lot < plan.n {
        return f64::NAN;
    }
    plan.n as f64 + (1.0 - accept_probability(plan, p)) * (lot - plan.n) as f64
}

/// The AOQL (the largest average outgoing quality) and the fraction defective where it is reached: a scan
/// of 2000 points narrowed by golden-section search to 1e-12.
pub fn aoql(plan: SamplingPlan, lot: u64) -> (f64, f64) {
    let f = |p: f64| aoq(plan, lot, p);
    let steps = 2000;
    let (mut best, mut at) = (f64::NEG_INFINITY, 0.0);
    for i in 0..=steps {
        let p = i as f64 / steps as f64;
        let v = f(p);
        if v > best {
            (best, at) = (v, p);
        }
    }
    let h = 1.0 / steps as f64;
    let (mut a, mut b) = ((at - h).max(0.0), (at + h).min(1.0));
    let g = (5f64.sqrt() - 1.0) / 2.0;
    while b - a > 1e-12 {
        let (x1, x2) = (b - g * (b - a), a + g * (b - a));
        if f(x1) < f(x2) { a = x1 } else { b = x2 }
    }
    let p = (a + b) / 2.0;
    (f(p), p)
}

/// The fractions defective an OC curve is drawn at: `points` evenly spaced values from 0 to the first
/// fraction where the probability of acceptance has fallen below 0.001 (at most 1), but at least to
/// `at_least` (for example the LTPD).
pub fn curve_fractions(plan: SamplingPlan, at_least: f64, points: usize) -> Vec<f64> {
    let mut end = at_least.clamp(0.0, 1.0);
    let step = 0.001;
    let mut p = end;
    while p < 1.0 && accept_probability(plan, p) >= 0.001 {
        p += step;
    }
    end = end.max(p.min(1.0));
    if end <= 0.0 {
        end = 1.0;
    }
    let k = points.max(2) - 1;
    (0..=k).map(|i| end * i as f64 / k as f64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plan_with_c_at_least_n_accepts_everything_and_bad_inputs_are_refused() {
        assert_eq!(accept_probability(SamplingPlan { n: 3, c: 3 }, 0.5), 1.0);
        assert_eq!(accept_probability(SamplingPlan { n: 10, c: 0 }, 0.0), 1.0);
        assert!(accept_probability(SamplingPlan { n: 10, c: 0 }, 1.5).is_nan());
        assert!(design(0.05, 0.01, 0.05, 0.1).is_err());
        assert!(design(0.0, 0.05, 0.05, 0.1).is_err());
        assert!(check_plan(SamplingPlan { n: 5, c: 5 }).is_err());
        assert!(aoq(SamplingPlan { n: 50, c: 1 }, 20, 0.1).is_nan());
    }

    #[test]
    fn the_designed_plan_meets_both_risks_and_one_less_item_does_not() {
        let plan = design(0.01, 0.05, 0.05, 0.10).unwrap();
        assert!(accept_probability(plan, 0.01) >= 0.95);
        assert!(accept_probability(plan, 0.05) <= 0.10);
        let smaller = SamplingPlan { n: plan.n - 1, c: plan.c };
        assert!(accept_probability(smaller, 0.05) > 0.10);
    }

    #[test]
    fn aoq_is_largest_at_the_aoql() {
        let plan = SamplingPlan { n: 50, c: 1 };
        let (q, p) = aoql(plan, 1000);
        for i in 1..100 {
            assert!(aoq(plan, 1000, i as f64 / 100.0) <= q + 1e-15);
        }
        assert!(p > 0.0 && p < 0.2);
        assert_eq!(ati(plan, 1000, 0.0), 50.0);
        assert!((ati(plan, 1000, 1.0) - 1000.0).abs() < 1e-9);
    }
}
