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
//! uniform order statistic medians). The rules are written down in `docs/spec/statistics.md`, section 12.

use crate::normality::{ad_normal_p, kolmogorov_cdf};
use statrs::distribution::{ContinuousCDF, Gamma, Normal};
use statrs::function::gamma::{digamma, ln_gamma};

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
    /// All numbers are equal (no spread to fit).
    NoSpread,
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
    /// parameters estimated from the same data make the true p-value smaller. For samples above 140 the
    /// asymptotic series is used.
    pub ks_p: f64,
    /// Anderson-Darling statistic `A2` against the fitted distribution.
    pub ad: f64,
    /// Approximate p-value of `A2`; only for the normal and the lognormal family (the formula of D'Agostino
    /// and Stephens for estimated mean and standard deviation), NaN for the others.
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
            Ok(vec![m, (x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n).sqrt()])
        }
        Family::Lognormal => {
            let l: Vec<f64> = x.iter().map(|v| v.ln()).collect();
            if flat(&l) {
                return Err(FitError::NoSpread);
            }
            let m = mean(&l);
            Ok(vec![m, (l.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n).sqrt()])
        }
        Family::Exponential => {
            let m = mean(&x);
            if m > 0.0 { Ok(vec![m]) } else { Err(FitError::NotPositive(family)) }
        }
        Family::Gamma => {
            if flat(&x) {
                return Err(FitError::NoSpread);
            }
            let m = mean(&x);
            let s = m.ln() - x.iter().map(|v| v.ln()).sum::<f64>() / n;
            let k = gamma_shape(s);
            Ok(vec![k, m / k])
        }
        Family::Weibull => {
            if flat(&x) {
                return Err(FitError::NoSpread);
            }
            let (c, scale) = weibull_estimates(&x);
            Ok(vec![c, scale])
        }
    }
}

fn gamma_dist(shape: f64, scale: f64) -> Gamma {
    Gamma::new(shape, 1.0 / scale).expect("positive shape and scale")
}

/// `(ln F(x), ln (1 - F(x)))` of the distribution, kept away from `ln 0`.
fn ln_cdf_sf(family: Family, p: &[f64], x: f64) -> (f64, f64) {
    const FLOOR: f64 = 1e-300;
    let ln = |v: f64| v.max(FLOOR).ln();
    match family {
        Family::Normal | Family::Lognormal => {
            let z = if family == Family::Normal { (x - p[0]) / p[1] } else { (x.ln() - p[0]) / p[1] };
            let n = Normal::new(0.0, 1.0).expect("standard normal");
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
            let g = gamma_dist(p[0], p[1]);
            (ln(g.cdf(x)), ln(g.sf(x)))
        }
    }
}

/// Distribution function of the fitted distribution.
pub fn cdf(family: Family, params: &[f64], x: f64) -> f64 {
    match family {
        Family::Normal => Normal::new(params[0], params[1]).expect("positive SD").cdf(x),
        Family::Lognormal => {
            if x <= 0.0 {
                0.0
            } else {
                Normal::new(params[0], params[1]).expect("positive SD").cdf(x.ln())
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
                gamma_dist(params[0], params[1]).cdf(x)
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
            Family::Normal => -params[1].ln() - half_ln_2pi - (x - params[0]).powi(2) / (2.0 * params[1] * params[1]),
            Family::Lognormal => {
                -x.ln() - params[1].ln() - half_ln_2pi - (x.ln() - params[0]).powi(2) / (2.0 * params[1] * params[1])
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
    let mut a_sum = 0.0;
    for i in 0..n {
        let f = cdf(family, &params, x[i]);
        ks_d = ks_d.max((i as f64 + 1.0) / nf - f).max(f - i as f64 / nf);
        let (ln_f, _) = ln_cdf_sf(family, &params, x[i]);
        let (_, ln_s) = ln_cdf_sf(family, &params, x[n - 1 - i]);
        a_sum += (2.0 * i as f64 + 1.0) * (ln_f + ln_s);
    }
    let ad = -nf - a_sum / nf;
    let ad_p = match family {
        Family::Normal | Family::Lognormal => ad_normal_p(ad * (1.0 + 0.75 / nf + 2.25 / (nf * nf))),
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
        Family::Gamma => Gamma::new(shape, 1.0).expect("positive shape").inverse_cdf(p),
        Family::Weibull => (-(-p).ln_1p()).powf(1.0 / shape),
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
    let positions = plotting_positions(n);
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
