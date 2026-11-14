//! The noncentral t and F distributions, for power and sample size calculations.
//!
//! The noncentral t distribution function follows Lenth's algorithm AS 243 (a series in the
//! incomplete beta function, the same method R's `pt` uses with a non-centrality parameter), written
//! here from the published description. For `|ncp|` above [`NCT_SERIES_LIMIT`] the series would need
//! weights smaller than the smallest double, so the same Poisson mixture is summed from its largest weight
//! outward, with the weights in logarithms and the incomplete beta functions evaluated term by term, up to
//! [`NCT_MIXTURE_LIMIT`]. Above that a normal approximation (Abramowitz and Stegun 26.7.10) takes over; it is
//! not exact for small `df` (the error reaches a few hundredths at `df` of 1 to 3), see the spec, section
//! 12.3. The noncentral F distribution function is the Poisson mixture of incomplete beta functions, summed
//! outward from the largest weight.
//!
//! The reference numbers of the tests are `scipy.stats.nct` and `scipy.stats.ncf` (scipy 1.18), written by
//! `tools/stats-dist-ref.py`.

use statrs::distribution::{ContinuousCDF, Normal, StudentsT};
use statrs::function::beta::beta_reg;
use statrs::function::gamma::ln_gamma;

/// Largest `|ncp|` for which [`nct_cdf`] sums the series.
pub const NCT_SERIES_LIMIT: f64 = 37.62;

/// Largest `|ncp|` for which [`nct_cdf`] sums the Poisson mixture from its largest weight; above it the normal
/// approximation is used.
pub const NCT_MIXTURE_LIMIT: f64 = 1000.0;

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
    let mut value = if del.abs() <= NCT_SERIES_LIMIT {
        nct_series(tt, df, del)
    } else if del.abs() <= NCT_MIXTURE_LIMIT {
        nct_mixture(tt, df, del)
    } else {
        let s = 1.0 / (4.0 * df);
        std_normal_cdf((tt * (1.0 - s) - del) / (1.0 + tt * tt * 2.0 * s).sqrt())
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

/// The series of AS 243 for `t >= 0` and a large `|del|`: the weights `p_j` and `q_j` (Poisson weights with mean
/// `del^2 / 2`, halved, and the odd companions of them) are formed in logarithms and only the terms around the
/// largest are summed, each with its own incomplete beta functions `I_x(j + 1/2, df/2)` and `I_x(j + 1, df/2)`.
fn nct_mixture(t: f64, df: f64, del: f64) -> f64 {
    let x = t * t / (t * t + df);
    let mut tnc = 0.0;
    if x > 0.0 {
        let b = 0.5 * df;
        let half = 0.5 * del * del;
        let ln_p0 = 0.5_f64.ln() - half;
        // ln of sqrt(2/pi) |del| p0 / (2j + 1)!!, with (2j + 1)!! = 2^(j+1) Gamma(j + 3/2) / sqrt(pi)
        let ln_q0 = 0.5 * (2.0 / std::f64::consts::PI).ln() + del.abs().ln() + ln_p0;
        let span = (14.0 * half.sqrt() + 60.0).ceil();
        let j0 = half.floor();
        let mut j = (j0 - span).max(0.0);
        let last = j0 + span;
        while j <= last {
            let ln_p = ln_p0 + j * half.ln() - ln_gamma(j + 1.0);
            let ln_q = ln_q0 + j * (2.0 * half).ln()
                - ((j + 1.0) * std::f64::consts::LN_2 + ln_gamma(j + 1.5) - 0.5 * std::f64::consts::PI.ln());
            tnc += ln_p.exp() * beta_reg(j + 0.5, b, x) + del.signum() * ln_q.exp() * beta_reg(j + 1.0, b, x);
            j += 1.0;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixture_and_series_agree_where_both_can_be_summed() {
        for df in [1.0, 4.0, 25.0] {
            for del in [8.0, 20.0, 30.0] {
                for t in [0.5 * del, del, 1.4 * del] {
                    let (a, b) = (nct_series(t, df, del), nct_mixture(t, df, del));
                    assert!((a - b).abs() < 1e-10, "df {df} ncp {del} t {t}: series {a}, mixture {b}");
                }
            }
        }
    }

    #[test]
    fn the_distribution_function_is_continuous_across_the_series_limit() {
        for df in [1.0, 3.0, 20.0] {
            for sign in [1.0, -1.0] {
                for t in [20.0, 37.0, 50.0] {
                    let below = nct_cdf(t, df, sign * (NCT_SERIES_LIMIT - 1e-9));
                    let above = nct_cdf(t, df, sign * (NCT_SERIES_LIMIT + 1e-9));
                    assert!((below - above).abs() < 1e-7, "df {df} sign {sign} t {t}: {below} against {above}");
                }
            }
        }
    }

    #[test]
    fn the_upper_limit_of_the_mixture_is_cheap_and_stays_a_probability() {
        for df in [1.0, 10.0] {
            let v = nct_cdf(NCT_MIXTURE_LIMIT, df, NCT_MIXTURE_LIMIT);
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
