//! The noncentral t and F distributions, for power and sample size calculations.
//!
//! The noncentral t distribution function follows Lenth's algorithm AS 243 (a series in the
//! incomplete beta function, the same method R's `pt` uses with a non-centrality parameter), written
//! here from the published description. For `|ncp|` above [`NCT_SERIES_LIMIT`] the series would need
//! weights smaller than the smallest double, so a normal approximation takes over (power calculations
//! never ask for that range: the answer is 1 to within 1e-10 there). The noncentral F distribution
//! function is the Poisson mixture of incomplete beta functions, summed outward from the largest weight.
//!
//! The reference numbers of the tests are `scipy.stats.nct` and `scipy.stats.ncf` (scipy 1.18), written by
//! `tools/stats-dist-ref.py`.

use statrs::distribution::{ContinuousCDF, Normal, StudentsT};
use statrs::function::beta::beta_reg;
use statrs::function::gamma::ln_gamma;

/// Largest `|ncp|` for which [`nct_cdf`] sums the series.
pub const NCT_SERIES_LIMIT: f64 = 37.62;

fn std_normal_cdf(x: f64) -> f64 {
    Normal::new(0.0, 1.0).expect("standard normal").cdf(x)
}

/// Distribution function of the noncentral t distribution with `df` degrees of freedom and
/// non-centrality `ncp`, at `t`. NaN for a non-positive `df` or NaN input.
pub fn nct_cdf(t: f64, df: f64, ncp: f64) -> f64 {
    if t.is_nan() || df.is_nan() || ncp.is_nan() || df <= 0.0 {
        return f64::NAN;
    }
    if t == f64::INFINITY {
        return 1.0;
    }
    if t == f64::NEG_INFINITY {
        return 0.0;
    }
    if ncp == 0.0 {
        return StudentsT::new(0.0, 1.0, df).expect("t").cdf(t);
    }
    let (tt, del, negdel) = if t >= 0.0 { (t, ncp, false) } else { (-t, -ncp, true) };
    // R's own cut-off for the normal approximation (Abramowitz and Stegun 26.7.10).
    let mut value = if del.abs() > NCT_SERIES_LIMIT {
        let s = 1.0 / (4.0 * df);
        std_normal_cdf((tt * (1.0 - s) - del) / (1.0 + tt * tt * 2.0 * s).sqrt())
    } else {
        nct_series(tt, df, del)
    };
    if negdel {
        value = 1.0 - value;
    }
    value.clamp(0.0, 1.0)
}

/// Upper tail `P(T > t)` of the noncentral t distribution.
pub fn nct_sf(t: f64, df: f64, ncp: f64) -> f64 {
    // The upper tail at t is the lower tail of the mirrored variable at -t.
    nct_cdf(-t, df, -ncp)
}

/// AS 243 for `t >= 0`.
fn nct_series(t: f64, df: f64, del: f64) -> f64 {
    const ERRMAX: f64 = 1e-14;
    const ITRMAX: usize = 20_000;
    let x = t * t / (t * t + df);
    let mut tnc = 0.0;
    if x > 0.0 {
        let lambda = del * del;
        let mut p = 0.5 * (-0.5 * lambda).exp();
        let mut q = (2.0 / std::f64::consts::PI).sqrt() * p * del;
        let mut s = 0.5 - p;
        let mut a = 0.5;
        let b = 0.5 * df;
        let rxb = (1.0 - x).powf(b);
        let albeta = std::f64::consts::PI.sqrt().ln() + ln_gamma(b) - ln_gamma(a + b);
        let mut xodd = beta_reg(a, b, x);
        let mut godd = 2.0 * rxb * (a * x.ln() - albeta).exp();
        let mut xeven = 1.0 - rxb;
        let mut geven = b * x * rxb;
        tnc = p * xodd + q * xeven;
        for en in 1..=ITRMAX {
            let enf = en as f64;
            a += 1.0;
            xodd -= godd;
            xeven -= geven;
            godd *= x * (a + b - 1.0) / a;
            geven *= x * (a + b - 0.5) / (a + 0.5);
            p *= lambda / (2.0 * enf);
            q *= lambda / (2.0 * enf + 1.0);
            tnc += p * xodd + q * xeven;
            s -= p;
            if (2.0 * s * (xodd - godd)).abs() < ERRMAX {
                break;
            }
        }
    }
    tnc + std_normal_cdf(-del)
}

/// Weights of the Poisson mixture of the noncentral F distribution: calls `term(j, weight)` for the
/// indices `j` that matter, from the one of the largest weight outward in both directions.
fn poisson_mixture(ncp: f64, mut term: impl FnMut(f64, f64) -> f64) -> f64 {
    if ncp == 0.0 {
        return term(0.0, 1.0);
    }
    let half = ncp / 2.0;
    let ln_w = |j: f64| -half + j * half.ln() - ln_gamma(j + 1.0);
    let j0 = half.floor();
    let mut total = 0.0;
    // Upwards from j0 (included).
    let mut j = j0;
    loop {
        let w = ln_w(j).exp();
        let add = term(j, w);
        total += add;
        if (w < 1e-18 && j > half) || j > j0 + 100_000.0 {
            break;
        }
        j += 1.0;
    }
    // Downwards from j0 - 1.
    let mut j = j0 - 1.0;
    while j >= 0.0 {
        let w = ln_w(j).exp();
        let add = term(j, w);
        total += add;
        if w < 1e-18 {
            break;
        }
        j -= 1.0;
    }
    total
}

fn ncf_valid(df1: f64, df2: f64, ncp: f64) -> bool {
    df1 > 0.0 && df2 > 0.0 && ncp >= 0.0 && !df1.is_nan() && !df2.is_nan() && !ncp.is_nan()
}

/// Distribution function of the noncentral F distribution (`df1`, `df2` degrees of freedom,
/// non-centrality `ncp >= 0`) at `x`.
pub fn ncf_cdf(x: f64, df1: f64, df2: f64, ncp: f64) -> f64 {
    if x.is_nan() || !ncf_valid(df1, df2, ncp) {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 0.0;
    }
    if x == f64::INFINITY {
        return 1.0;
    }
    let y = df1 * x / (df1 * x + df2);
    let sum = poisson_mixture(ncp, |j, w| w * beta_reg(df1 / 2.0 + j, df2 / 2.0, y));
    sum.clamp(0.0, 1.0)
}

/// Upper tail `P(F > x)` of the noncentral F distribution, summed from the complementary incomplete beta
/// functions so that small tails keep their digits.
pub fn ncf_sf(x: f64, df1: f64, df2: f64, ncp: f64) -> f64 {
    if x.is_nan() || !ncf_valid(df1, df2, ncp) {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 1.0;
    }
    if x == f64::INFINITY {
        return 0.0;
    }
    let y = df1 * x / (df1 * x + df2);
    let sum = poisson_mixture(ncp, |j, w| w * beta_reg(df2 / 2.0, df1 / 2.0 + j, 1.0 - y));
    sum.clamp(0.0, 1.0)
}
