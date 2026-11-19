//! Distribution fit by maximum likelihood, goodness of fit, and the quantile pairs of probability plots.
//!
//! Five families are fitted to one sample: normal, lognormal, exponential, gamma and Weibull. The
//! lognormal, exponential, gamma and Weibull distributions have their threshold (location) fixed at 0, as
//! `scipy.stats.<dist>.fit(data, floc=0)` does, so the parameters are
//!
//! | family      | parameters                                                      |
//! |-------------|-----------------------------------------------------------------|
//! | normal      | mean `mu`, standard deviation `sigma` (divisor `n`: the maximum likelihood value) |
//! | lognormal   | `mu` and `sigma` of the logarithm of the data                    |
//! | exponential | scale `theta` (the mean; the rate is `1 / theta`)                |
//! | gamma       | shape `k`, scale `theta`                                          |
//! | Weibull     | shape `c`, scale `lambda`                                         |
//!
//! Normal, lognormal and exponential have closed-form estimates; the gamma shape and the Weibull shape
//! solve one equation in one unknown (Newton's method and bisection, to machine precision). The
//! quantile pairs of [`probplot`] are those of `scipy.stats.probplot` (Filliben's estimate of the
//! uniform order statistic medians). The rules are written down in `docs/spec/statistics.md`, section 16.

use crate::normality::{ad_normal_p, kolmogorov_cdf};
use statrs::distribution::{ContinuousCDF, Gamma, Normal};
use statrs::function::gamma::{checked_gamma_lr, digamma, ln_gamma};

/// A family of distributions that [`fit`] can estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Normal,
    Lognormal,
    Exponential,
    Gamma,
    Weibull,
}

impl Family {
    pub const ALL: [Family; 5] =
        [Family::Normal, Family::Lognormal, Family::Exponential, Family::Gamma, Family::Weibull];

    pub fn name(self) -> &'static str {
        match self {
            Family::Normal => "Normal",
            Family::Lognormal => "Lognormal",
            Family::Exponential => "Exponential",
            Family::Gamma => "Gamma",
            Family::Weibull => "Weibull",
        }
    }

    /// Names of the fitted parameters, in the order of [`DistFit::params`].
    pub fn param_names(self) -> &'static [&'static str] {
        match self {
            Family::Normal => &["Mean", "SD"],
            Family::Lognormal => &["Log mean (mu)", "Log SD (sigma)"],
            Family::Exponential => &["Scale (mean)"],
            Family::Gamma => &["Shape", "Scale"],
            Family::Weibull => &["Shape", "Scale"],
        }
    }

    /// True if the family needs every number to be positive (zero is allowed for the exponential).
    fn positive_only(self) -> bool {
        !matches!(self, Family::Normal)
    }
}

/// Why a fit or a probability plot is not possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitError {
    TooFew {
        needed: usize,
        have: usize,
    },
    /// The family needs positive numbers and the data contain a number that is not.
    NotPositive(Family),
    /// All numbers are equal, or so close together that the estimate would not be a finite number (no spread
    /// to fit).
    NoSpread,
    /// An estimate is not a finite number, or not above zero where it must be (numbers too large or too
    /// small for double precision).
    Degenerate(Family),
}

impl std::fmt::Display for FitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FitError::TooFew { needed, have } => {
                write!(f, "the fit needs at least {needed} numbers, there are {have}")
            }
            FitError::NotPositive(fam) => write!(
                f,
                "the {} distribution needs numbers above zero{}",
                fam.name().to_lowercase(),
                if *fam == Family::Exponential { " or equal to zero, with a positive mean" } else { "" }
            ),
            FitError::NoSpread => write!(f, "all numbers are equal, there is no distribution to fit"),
            FitError::Degenerate(fam) => write!(
                f,
                "the {} distribution cannot be fitted: an estimate is not a finite number (the numbers are too large, too small or too close together)",
                fam.name().to_lowercase()
            ),
        }
    }
}

/// The result of [`fit`].
#[derive(Debug, Clone, PartialEq)]
pub struct DistFit {
    pub family: Family,
    pub n: usize,
    /// The estimates, named by [`Family::param_names`].
    pub params: Vec<f64>,
    pub log_likelihood: f64,
    /// Akaike information criterion `2k - 2 ln L`, `k` the number of fitted parameters.
    pub aic: f64,
    /// Bayesian information criterion `k ln n - 2 ln L`.
    pub bic: f64,
    /// Kolmogorov-Smirnov distance between the sample and the fitted distribution function.
    pub ks_d: f64,
    /// P-value of that distance for a distribution known in advance (as `scipy.stats.kstest` gives it);
    /// parameters estimated from the same data make the true p-value smaller. Exact up to 1000 numbers
    /// (the matrix method); above that, and for a p-value below 1e-7 from 141 numbers on, the asymptotic
    /// series is used (see `normality::kolmogorov_cdf`).
    pub ks_p: f64,
    /// Anderson-Darling statistic `A2` against the fitted distribution. Infinite for an exponential fit to
    /// data that contain 0, because the fitted distribution function is 0 there.
    pub ad: f64,
    /// Approximate p-value of `A2`; only for the normal and the lognormal family (the formula of D'Agostino
    /// and Stephens for estimated mean and standard deviation, applied to A2 computed with the sample
    /// standard deviation, divisor `n - 1`, so that it equals the p-value of the Normality Test), NaN for
    /// the others.
    pub ad_p: f64,
}

/// The numbers without the missing ones.
fn clean(data: &[f64]) -> Vec<f64> {
    data.iter().copied().filter(|x| !x.is_nan()).collect()
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Trigamma function (derivative of digamma) by the recurrence up to 8 and the asymptotic series.
fn trigamma(mut x: f64) -> f64 {
    if x.is_nan() || x <= 0.0 {
        return f64::NAN;
    }
    let mut acc = 0.0;
    while x < 8.0 {
        acc += 1.0 / (x * x);
        x += 1.0;
    }
    let i = 1.0 / x;
    let i2 = i * i;
    acc + i
        + 0.5 * i2
        + i * i2 * (1.0 / 6.0 - i2 * (1.0 / 30.0 - i2 * (1.0 / 42.0 - i2 * (1.0 / 30.0 - i2 * 5.0 / 66.0))))
}

/// Maximum likelihood shape of the gamma distribution: solves `ln k - digamma(k) = ln(mean) - mean(ln x)`.
fn gamma_shape(s: f64) -> f64 {
    // Minka's starting value, then Newton on a function that is convex and decreasing in k.
    let mut k = (3.0 - s + ((s - 3.0).powi(2) + 24.0 * s).sqrt()) / (12.0 * s);
    for _ in 0..100 {
        if !(k.is_finite() && k > 0.0) {
            return f64::NAN;
        }
        let f = k.ln() - digamma(k) - s;
        let df = 1.0 / k - trigamma(k);
        let next = (k - f / df).max(k / 4.0);
        let done = (next - k).abs() <= 1e-15 * k;
        k = next;
        if done {
            break;
        }
    }
    k
}

/// Maximum likelihood shape of the Weibull distribution (the root of an increasing function, by
/// bisection on the logarithm of the shape) and the scale, for data scaled by their maximum.
fn weibull_estimates(x: &[f64]) -> (f64, f64) {
    let xmax = x.iter().copied().fold(f64::MIN, f64::max);
    let l: Vec<f64> = x.iter().map(|v| (v / xmax).ln()).collect();
    let mean_l = mean(&l);
    let g = |c: f64| {
        let w: Vec<f64> = l.iter().map(|li| (c * li).exp()).collect();
        let sw: f64 = w.iter().sum();
        let swl: f64 = w.iter().zip(&l).map(|(w, li)| w * li).sum();
        swl / sw - 1.0 / c - mean_l
    };
    let (mut lo, mut hi) = (1.0_f64, 1.0_f64);
    while g(lo) > 0.0 && lo > 1e-12 {
        lo /= 2.0;
    }
    while g(hi) < 0.0 && hi < 1e12 {
        hi *= 2.0;
    }
    let (mut a, mut b) = (lo.ln(), hi.ln());
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        if g(m.exp()) < 0.0 {
            a = m;
        } else {
            b = m;
        }
        if b - a < 1e-16 {
            break;
        }
    }
    let c = (0.5 * (a + b)).exp();
    let scale = xmax * (l.iter().map(|li| (c * li).exp()).sum::<f64>() / x.len() as f64).powf(1.0 / c);
    (c, scale)
}

/// Validates the data for `family` and returns the numbers.
fn prepared(family: Family, data: &[f64]) -> Result<Vec<f64>, FitError> {
    let x = clean(data);
    if x.len() < 2 {
        return Err(FitError::TooFew { needed: 2, have: x.len() });
    }
    if family.positive_only() {
        let bad = if family == Family::Exponential { x.iter().any(|v| *v < 0.0) } else { x.iter().any(|v| *v <= 0.0) };
        if bad || x.iter().any(|v| !v.is_finite()) {
            return Err(FitError::NotPositive(family));
        }
    }
    if x.iter().any(|v| !v.is_finite()) {
        return Err(FitError::NoSpread);
    }
    Ok(x)
}

/// Largest gamma shape that [`estimate`] returns; above it the data are taken to have no spread.
const GAMMA_SHAPE_LIMIT: f64 = 1e10;

/// Standard deviation about `m` with divisor `n`, scaled by the largest deviation so that squares neither
/// underflow nor overflow.
fn spread(v: &[f64], m: f64) -> f64 {
    let big = v.iter().map(|a| (a - m).abs()).fold(0.0_f64, f64::max);
    if big == 0.0 || !big.is_finite() {
        return big;
    }
    big * (v.iter().map(|a| ((a - m) / big).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
}

/// The estimates if every one is finite and (except the mean of the normal and the log mean of the
/// lognormal) above zero, as the distribution needs them to be.
fn valid(family: Family, params: Vec<f64>) -> Result<Vec<f64>, FitError> {
    let free = match family {
        Family::Normal | Family::Lognormal => 1,
        _ => 0,
    };
    let ok = params.iter().enumerate().all(|(i, p)| p.is_finite() && (i < free || *p > 0.0));
    if ok { Ok(params) } else { Err(FitError::Degenerate(family)) }
}

/// The maximum likelihood estimates of `family` for the numbers of `data` (missing values are ignored).
pub fn estimate(family: Family, data: &[f64]) -> Result<Vec<f64>, FitError> {
    let x = prepared(family, data)?;
    let n = x.len() as f64;
    let flat = |v: &[f64]| v.iter().all(|a| *a == v[0]);
    match family {
        Family::Normal => {
            if flat(&x) {
                return Err(FitError::NoSpread);
            }
            let m = mean(&x);
            valid(family, vec![m, spread(&x, m)])
        }
        Family::Lognormal => {
            let l: Vec<f64> = x.iter().map(|v| v.ln()).collect();
            if flat(&l) {
                return Err(FitError::NoSpread);
            }
            let m = mean(&l);
            valid(family, vec![m, spread(&l, m)])
        }
        Family::Exponential => {
            let m = mean(&x);
            if m > 0.0 { valid(family, vec![m]) } else { Err(FitError::NotPositive(family)) }
        }
        Family::Gamma => {
            if flat(&x) {
                return Err(FitError::NoSpread);
            }
            let m = mean(&x);
            let s = m.ln() - x.iter().map(|v| v.ln()).sum::<f64>() / n;
            if s.is_nan() {
                return Err(FitError::Degenerate(family));
            }
            // `s` is zero for numbers that differ by rounding only (it can come out slightly negative), and
            // a shape above `GAMMA_SHAPE_LIMIT` (a coefficient of variation below about 1e-5) is no
            // estimate worth the name: both are data without spread.
            // (the shape is about 1 / (2 s) for a small `s`).
            if s <= 0.5 / GAMMA_SHAPE_LIMIT {
                return Err(FitError::NoSpread);
            }
            let k = gamma_shape(s);
            if k.is_nan() || k > GAMMA_SHAPE_LIMIT {
                return Err(if k.is_nan() { FitError::Degenerate(family) } else { FitError::NoSpread });
            }
            valid(family, vec![k, m / k])
        }
        Family::Weibull => {
            if flat(&x) {
                return Err(FitError::NoSpread);
            }
            let (c, scale) = weibull_estimates(&x);
            valid(family, vec![c, scale])
        }
    }
}

/// `(F(x), 1 - F(x))` of the gamma distribution; NaN for parameters outside the domain (never a panic).
fn gamma_cdf_sf(shape: f64, scale: f64, x: f64) -> (f64, f64) {
    let z = x / scale;
    if z.is_nan() || shape.is_nan() || shape <= 0.0 || !shape.is_finite() || scale.is_nan() || scale <= 0.0 {
        return (f64::NAN, f64::NAN);
    }
    if z <= 0.0 {
        return (0.0, 1.0);
    }
    if z == f64::INFINITY {
        return (1.0, 0.0);
    }
    match checked_gamma_lr(shape, z) {
        Ok(p) => (p, 1.0 - p),
        Err(_) => (f64::NAN, f64::NAN),
    }
}

fn std_normal() -> Normal {
    Normal::new(0.0, 1.0).expect("standard normal")
}

/// `(ln F(x), ln (1 - F(x)))` of the distribution, kept away from `ln 0`.
fn ln_cdf_sf(family: Family, p: &[f64], x: f64) -> (f64, f64) {
    const FLOOR: f64 = 1e-300;
    let ln = |v: f64| v.max(FLOOR).ln();
    match family {
        Family::Normal | Family::Lognormal => {
            let z = if family == Family::Normal { (x - p[0]) / p[1] } else { (x.ln() - p[0]) / p[1] };
            let n = std_normal();
            (ln(n.cdf(z)), ln(n.cdf(-z)))
        }
        Family::Exponential => {
            let u = x / p[0];
            (ln(-(-u).exp_m1()), -u)
        }
        Family::Weibull => {
            let u = (x / p[1]).powf(p[0]);
            (ln(-(-u).exp_m1()), -u)
        }
        Family::Gamma => {
            let (c, sf) = gamma_cdf_sf(p[0], p[1], x);
            (ln(c), ln(sf))
        }
    }
}

/// Distribution function of the fitted distribution.
pub fn cdf(family: Family, params: &[f64], x: f64) -> f64 {
    // The standard normal at the standardized value; NaN (not a panic) for a SD that is not above zero.
    let normal = |mu: f64, sigma: f64, v: f64| if sigma > 0.0 { std_normal().cdf((v - mu) / sigma) } else { f64::NAN };
    match family {
        Family::Normal => normal(params[0], params[1], x),
        Family::Lognormal => {
            if x <= 0.0 {
                0.0
            } else {
                normal(params[0], params[1], x.ln())
            }
        }
        Family::Exponential => {
            if x <= 0.0 {
                0.0
            } else {
                -(-x / params[0]).exp_m1()
            }
        }
        Family::Weibull => {
            if x <= 0.0 {
                0.0
            } else {
                -(-(x / params[1]).powf(params[0])).exp_m1()
            }
        }
        Family::Gamma => {
            if x <= 0.0 {
                0.0
            } else {
                gamma_cdf_sf(params[0], params[1], x).0
            }
        }
    }
}

/// Log-likelihood of the numbers of `data` under the distribution.
pub fn log_likelihood(family: Family, params: &[f64], data: &[f64]) -> f64 {
    let half_ln_2pi = 0.5 * (2.0 * std::f64::consts::PI).ln();
    data.iter()
        .filter(|x| !x.is_nan())
        .map(|&x| match family {
            Family::Normal => -params[1].ln() - half_ln_2pi - 0.5 * ((x - params[0]) / params[1]).powi(2),
            Family::Lognormal => {
                -x.ln() - params[1].ln() - half_ln_2pi - 0.5 * ((x.ln() - params[0]) / params[1]).powi(2)
            }
            Family::Exponential => -params[0].ln() - x / params[0],
            Family::Gamma => {
                (params[0] - 1.0) * x.ln() - x / params[1] - ln_gamma(params[0]) - params[0] * params[1].ln()
            }
            Family::Weibull => {
                let (c, s) = (params[0], params[1]);
                c.ln() - s.ln() + (c - 1.0) * (x.ln() - s.ln()) - (x / s).powf(c)
            }
        })
        .sum()
}

/// Fits `family` to `data` by maximum likelihood and tests the fit.
pub fn fit(family: Family, data: &[f64]) -> Result<DistFit, FitError> {
    let params = estimate(family, data)?;
    let mut x = clean(data);
    x.sort_by(|a, b| a.total_cmp(b));
    let n = x.len();
    let nf = n as f64;
    let ll = log_likelihood(family, &params, &x);
    let k = params.len() as f64;
    let mut ks_d: f64 = 0.0;
    for (i, v) in x.iter().enumerate() {
        let f = cdf(family, &params, *v);
        ks_d = ks_d.max((i as f64 + 1.0) / nf - f).max(f - i as f64 / nf);
    }
    // A2 of the sorted numbers against the distribution with the given parameters.
    let a_squared = |p: &[f64]| {
        let mut a_sum = 0.0;
        for i in 0..n {
            let (ln_f, _) = ln_cdf_sf(family, p, x[i]);
            let (_, ln_s) = ln_cdf_sf(family, p, x[n - 1 - i]);
            a_sum += (2.0 * i as f64 + 1.0) * (ln_f + ln_s);
        }
        -nf - a_sum / nf
    };
    // F(0) = 0 under the fitted exponential, so a 0 in the data makes A2 infinite (the log floor would turn
    // it into a large but meaningless finite number).
    let zero_in_exponential = family == Family::Exponential && x[0] == 0.0;
    let ad = if zero_in_exponential { f64::INFINITY } else { a_squared(&params) };
    if !(ll.is_finite() && (ad.is_finite() || zero_in_exponential) && ks_d.is_finite()) {
        return Err(FitError::Degenerate(family));
    }
    // The formula of D'Agostino and Stephens is for A2 computed with the sample standard deviation (divisor
    // n - 1), as the Normality Test does; the fit's own A2 uses the maximum likelihood value (divisor n).
    // The p-value is therefore taken from A2 recomputed with the n - 1 value, so that it is the p-value
    // the Normality Test gives for the same column.
    let ad_p = match family {
        Family::Normal | Family::Lognormal => {
            let ad_sample = a_squared(&[params[0], params[1] * (nf / (nf - 1.0)).sqrt()]);
            ad_normal_p(ad_sample * (1.0 + 0.75 / nf + 2.25 / (nf * nf)))
        }
        _ => f64::NAN,
    };
    Ok(DistFit {
        family,
        n,
        params,
        log_likelihood: ll,
        aic: 2.0 * k - 2.0 * ll,
        bic: k * nf.ln() - 2.0 * ll,
        ks_d,
        ks_p: (1.0 - kolmogorov_cdf(n, ks_d)).clamp(0.0, 1.0),
        ad,
        ad_p,
    })
}

// ----- probability plots -----------------------------------------------------------------------------------

/// Quantile of the standard form of the family (location 0, scale 1) at probability `p`; `shape` is the
/// sigma of the lognormal, the shape of the gamma and of the Weibull family, and is not used by the others.
pub fn standard_quantile(family: Family, shape: f64, p: f64) -> f64 {
    match family {
        Family::Normal => Normal::new(0.0, 1.0).expect("standard normal").inverse_cdf(p),
        Family::Lognormal => (shape * Normal::new(0.0, 1.0).expect("standard normal").inverse_cdf(p)).exp(),
        Family::Exponential => -(-p).ln_1p(),
        Family::Gamma => gamma_quantile(shape, p),
        Family::Weibull => (-(-p).ln_1p()).powf(1.0 / shape),
    }
}

/// Quantile of the standard gamma distribution (scale 1). statrs' `inverse_cdf` cannot go below about 9e-10, so
/// for a small shape and a small `p` the answer is found here by bisection on `ln x`, with the lower-tail
/// series `p ~ x^a / Gamma(a + 1)` as the starting bracket.
fn gamma_quantile(shape: f64, p: f64) -> f64 {
    let dist = Gamma::new(shape, 1.0).expect("positive shape");
    let x0 = dist.inverse_cdf(p);
    let accurate = |x: f64| {
        let f = gamma_cdf_sf(shape, 1.0, x).0;
        (f - p).abs() <= 1e-9 * p
    };
    if !(p > 0.0 && p < 1.0) || (x0.is_finite() && x0 > 1e-6 && accurate(x0)) {
        return x0;
    }
    // Bisection on t = ln x: F is increasing in t. The lower bound is the smallest normal number.
    let (mut lo, mut hi) = (-745.0_f64, 5.0 + (1.0 / shape).max(1.0) * 50.0);
    if gamma_cdf_sf(shape, 1.0, lo.exp()).0 >= p {
        // below the range of f64 numbers: the tail series, which underflows to 0
        return ((p.ln() + ln_gamma(shape + 1.0)) / shape).exp();
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if gamma_cdf_sf(shape, 1.0, mid.exp()).0 < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-14 {
            break;
        }
    }
    (0.5 * (lo + hi)).exp()
}

/// How the plotting positions (the cumulative probability of the i-th of n ordered values) are
/// estimated. Origin's `plot_prob` documents Blom, Benard, Hazen and Van der Waerden scores, Blom by
/// default (T151, not measured); `scipy.stats.probplot` uses Filliben's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scores {
    /// Filliben's medians of the uniform order statistics ([`plotting_positions`]).
    #[default]
    Filliben,
    /// `(i - 3/8) / (n + 1/4)`.
    Blom,
    /// Benard's median ranks, `(i - 0.3) / (n + 0.4)`.
    Benard,
    /// `(i - 1/2) / n`.
    Hazen,
    /// `i / (n + 1)`.
    VanDerWaerden,
}

impl Scores {
    pub const ALL: [Scores; 5] = [Scores::Filliben, Scores::Blom, Scores::Benard, Scores::Hazen, Scores::VanDerWaerden];

    pub fn name(self) -> &'static str {
        match self {
            Scores::Filliben => "Filliben",
            Scores::Blom => "Blom",
            Scores::Benard => "Benard",
            Scores::Hazen => "Hazen",
            Scores::VanDerWaerden => "Van der Waerden",
        }
    }

    /// The plotting positions of `n` ordered values, between 0 and 1.
    pub fn positions(self, n: usize) -> Vec<f64> {
        let nf = n as f64;
        let at = |a: f64, b: f64| (1..=n).map(|i| (i as f64 - a) / (nf + b)).collect();
        match self {
            Scores::Filliben => plotting_positions(n),
            Scores::Blom => at(0.375, 0.25),
            Scores::Benard => at(0.3, 0.4),
            Scores::Hazen => at(0.5, 0.0),
            Scores::VanDerWaerden => at(0.0, 1.0),
        }
    }
}

/// Filliben's estimate of the medians of the uniform order statistics, as `scipy.stats.probplot` uses
/// them as plotting positions.
pub fn plotting_positions(n: usize) -> Vec<f64> {
    let nf = n as f64;
    let mut v: Vec<f64> = (1..=n).map(|i| (i as f64 - 0.3175) / (nf + 0.365)).collect();
    if n >= 2 {
        let last = 0.5_f64.powf(1.0 / nf);
        v[0] = 1.0 - last;
        v[n - 1] = last;
    }
    v
}

/// The quantile pairs of a probability plot and the least-squares line through them.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbPlot {
    pub family: Family,
    /// The shape that fixed the theoretical quantiles (the fitted sigma, gamma or Weibull shape), if the
    /// family has one.
    pub shape: Option<f64>,
    /// Theoretical quantiles of the standard form at the plotting positions.
    pub theoretical: Vec<f64>,
    /// The numbers of the data in ascending order.
    pub ordered: Vec<f64>,
    /// The plotting positions (between 0 and 1).
    pub positions: Vec<f64>,
    /// Least-squares line `ordered = slope * theoretical + intercept` and its correlation coefficient:
    /// slope and intercept estimate the scale and the location.
    pub slope: f64,
    pub intercept: f64,
    pub r: f64,
}

/// The quantile-quantile pairs of `data` against `family`. A family with a shape takes the shape fitted by
/// maximum likelihood unless `shape` is given. Missing values are ignored; two numbers are the least.
pub fn probplot(family: Family, data: &[f64], shape: Option<f64>) -> Result<ProbPlot, FitError> {
    probplot_with(family, data, shape, Scores::Filliben)
}

/// [`probplot`] with the plotting positions of `scores`.
pub fn probplot_with(family: Family, data: &[f64], shape: Option<f64>, scores: Scores) -> Result<ProbPlot, FitError> {
    let shape = match family {
        Family::Normal | Family::Exponential => None,
        _ => Some(match shape {
            Some(s) => s,
            None => {
                let p = estimate(family, data)?;
                if family == Family::Lognormal { p[1] } else { p[0] }
            }
        }),
    };
    let mut ordered = prepared(family, data)?;
    ordered.sort_by(|a, b| a.total_cmp(b));
    let n = ordered.len();
    let positions = scores.positions(n);
    let theoretical: Vec<f64> = positions.iter().map(|&p| standard_quantile(family, shape.unwrap_or(1.0), p)).collect();
    let (mx, my) = (mean(&theoretical), mean(&ordered));
    let sxx: f64 = theoretical.iter().map(|a| (a - mx).powi(2)).sum();
    let syy: f64 = ordered.iter().map(|b| (b - my).powi(2)).sum();
    let sxy: f64 = theoretical.iter().zip(&ordered).map(|(a, b)| (a - mx) * (b - my)).sum();
    let slope = sxy / sxx;
    Ok(ProbPlot {
        family,
        shape,
        theoretical,
        ordered,
        positions,
        slope,
        intercept: my - slope * mx,
        r: if syy > 0.0 { sxy / (sxx * syy).sqrt() } else { f64::NAN },
    })
}

#[cfg(test)]
mod score_tests {
    use super::*;

    #[test]
    fn plotting_position_scores() {
        let close = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12);
        assert!(close(&Scores::Blom.positions(4), &[0.625 / 4.25, 1.625 / 4.25, 2.625 / 4.25, 3.625 / 4.25]));
        assert!(close(&Scores::Benard.positions(3), &[0.7 / 3.4, 1.7 / 3.4, 2.7 / 3.4]));
        assert!(close(&Scores::Hazen.positions(2), &[0.25, 0.75]));
        assert!(close(&Scores::VanDerWaerden.positions(3), &[0.25, 0.5, 0.75]));
        assert_eq!(Scores::Filliben.positions(5), plotting_positions(5));
        let data = [3.0, 1.0, 2.0, 5.0, 4.0];
        let blom = probplot_with(Family::Normal, &data, None, Scores::Blom).unwrap();
        assert_eq!(blom.positions, Scores::Blom.positions(5));
        assert_eq!(probplot(Family::Normal, &data, None).unwrap().positions, plotting_positions(5));
    }
}
