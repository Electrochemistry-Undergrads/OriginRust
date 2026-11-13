//! Dunnett's many-to-one comparison of means: every treatment group against one control group.
//!
//! The statistic of treatment `j` is `t_j = (mean_j - mean_control) / (s sqrt(1/n_j + 1/n_c))` with the pooled
//! standard deviation `s` of the ANOVA. The `t_j` follow a joint multivariate t distribution whose correlations
//! come from the shared control: `corr(t_i, t_j) = l_i l_j` with `l_j = sqrt(n_j / (n_j + n_c))`. The adjusted
//! (two-sided) p-value of `t_j` is the probability that the largest `|t|` of the whole family reaches `|t_j|`.
//!
//! The one-factor structure of the correlations reduces the multivariate t integral to a double integral
//! that is computed with Gauss-Legendre panels (not by Monte Carlo as scipy does), so the p-values are
//! reproducible and accurate to well below 1e-9:
//!
//! ```text
//! P(max |T_j| <= c) = Int_0^inf f_S(s) Int phi(x) Prod_j [ Phi((c s - l_j x) / r_j) - Phi((-c s - l_j x) / r_j) ] dx ds
//! ```
//!
//! with `r_j = sqrt(1 - l_j^2)` and `f_S` the density of `sqrt(chi-square(df) / df)`. Genz and Bretz describe
//! the same reduction for equi-correlated and one-factor problems.

use statrs::distribution::{ContinuousCDF, Normal, StudentsT};
use statrs::function::gamma::ln_gamma;

/// Nodes and weights of the 16-point Gauss-Legendre rule on [-1, 1].
fn gauss_legendre_16() -> &'static [(f64, f64); 16] {
    static RULE: std::sync::OnceLock<[(f64, f64); 16]> = std::sync::OnceLock::new();
    RULE.get_or_init(|| {
        let n = 16usize;
        let mut out = [(0.0, 0.0); 16];
        for i in 0..n / 2 {
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
        let mid = a + h * (p as f64 + 0.5);
        sum += rule.iter().map(|(x, w)| w * f(mid + x * h / 2.0)).sum::<f64>() * h / 2.0;
    }
    sum
}

/// The correlation loadings `l_j = sqrt(n_j / (n_j + n_c))` of the treatments on the control.
pub fn dunnett_loadings(treatment_sizes: &[usize], control_size: usize) -> Vec<f64> {
    treatment_sizes.iter().map(|&n| (n as f64 / (n as f64 + control_size as f64)).sqrt()).collect()
}

/// `P(|T_1| <= c, ..., |T_m| <= c)` for the multivariate t with `df` degrees of freedom and the
/// one-factor correlations given by `loadings`.
pub fn dunnett_cdf(c: f64, loadings: &[f64], df: f64) -> f64 {
    if c.is_nan() || df.is_nan() || df <= 0.0 || loadings.iter().any(|l| !(0.0..1.0).contains(l)) {
        return f64::NAN;
    }
    if c <= 0.0 {
        return 0.0;
    }
    if c.is_infinite() {
        return 1.0;
    }
    let norm = Normal::new(0.0, 1.0).expect("standard normal");
    let phi = |x: f64| (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let rs: Vec<f64> = loadings.iter().map(|l| (1.0 - l * l).sqrt()).collect();
    // The inner integral over the common factor x for a given scale s of the standard deviation. The
    // integrand is even in x, so only x >= 0 is integrated. It is smooth, with a transition of width
    // about r_j / l_j at |x| = c s / l_j, so panels of 1/4 resolve it.
    let given_s = |s: f64| -> f64 {
        let cs = c * s;
        let g = |x: f64| {
            let mut prod = phi(x);
            for (l, r) in loadings.iter().zip(&rs) {
                prod *= norm.cdf((cs - l * x) / r) - norm.cdf((-cs - l * x) / r);
                if prod == 0.0 {
                    break;
                }
            }
            prod
        };
        (2.0 * integrate(g, 0.0, 9.0, 36)).clamp(0.0, 1.0)
    };
    if df > 5000.0 {
        return given_s(1.0);
    }
    // s = sqrt(chi-square / df) has the density below.
    let half = df / 2.0;
    let ln_c = std::f64::consts::LN_2 + half * half.ln() - ln_gamma(half);
    let density = |s: f64| if s <= 0.0 { 0.0 } else { (ln_c + (df - 1.0) * s.ln() - df * s * s / 2.0).exp() };
    let spread = 12.0 / (2.0 * df).sqrt();
    let (lo, hi) = ((1.0 - spread).max(0.0), 1.0 + spread + if df < 4.0 { 6.0 } else { 0.0 });
    let panels = ((hi - lo) * (2.0 * df).sqrt() / 2.0).ceil().clamp(8.0, 400.0) as usize;
    integrate(|s| density(s) * given_s(s), lo, hi, panels).clamp(0.0, 1.0)
}

/// The two-sided adjusted p-value of the statistic `t`: `1 - P(max |T_j| <= |t|)`.
pub fn dunnett_p(t: f64, loadings: &[f64], df: f64) -> f64 {
    if t.is_nan() {
        return f64::NAN;
    }
    (1.0 - dunnett_cdf(t.abs(), loadings, df)).clamp(0.0, 1.0)
}

/// The critical value `d` with `P(max |T_j| <= d) = 1 - alpha` (the half-width of a simultaneous
/// confidence interval is `d` times the standard error).
pub fn dunnett_critical(alpha: f64, loadings: &[f64], df: f64) -> f64 {
    if !(alpha > 0.0 && alpha < 1.0) || loadings.is_empty() || !(df > 0.0) {
        return f64::NAN;
    }
    let m = loadings.len() as f64;
    let target = 1.0 - alpha;
    // d lies between the t quantile of a single test and the Bonferroni quantile.
    let Ok(t) = StudentsT::new(0.0, 1.0, df) else {
        return f64::NAN;
    };
    let mut lo = t.inverse_cdf(1.0 - alpha / 2.0) * (1.0 - 1e-9);
    let mut hi = t.inverse_cdf(1.0 - alpha / (2.0 * m)) * (1.0 + 1e-9);
    let (mut flo, mut fhi) = (dunnett_cdf(lo, loadings, df) - target, dunnett_cdf(hi, loadings, df) - target);
    // Illinois variant of regula falsi: safe like bisection, fast like the secant method.
    let mut side = 0i8;
    for _ in 0..100 {
        if flo > 0.0 || fhi < 0.0 {
            // The bracket does not hold (numerical noise at the ends): fall back on the nearer end.
            return if flo > 0.0 { lo } else { hi };
        }
        let x = (lo * fhi - hi * flo) / (fhi - flo);
        let fx = dunnett_cdf(x, loadings, df) - target;
        if fx.abs() < 1e-13 || (hi - lo).abs() < 1e-12 * hi.abs() {
            return x;
        }
        if fx < 0.0 {
            lo = x;
            flo = fx;
            if side == -1 {
                fhi *= 0.5;
            }
            side = -1;
        } else {
            hi = x;
            fhi = fx;
            if side == 1 {
                flo *= 0.5;
            }
            side = 1;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_treatment_is_the_two_sided_t_test() {
        let l = dunnett_loadings(&[6], 5);
        let t = StudentsT::new(0.0, 1.0, 9.0).unwrap();
        for x in [0.3, 1.5, 2.4, 4.0] {
            let want = 2.0 * (1.0 - t.cdf(x));
            assert!((dunnett_p(x, &l, 9.0) - want).abs() < 1e-10, "{x}");
        }
        let crit = dunnett_critical(0.05, &l, 9.0);
        assert!((crit - t.inverse_cdf(0.975)).abs() < 1e-8, "{crit}");
    }

    #[test]
    fn many_treatments_lie_between_one_test_and_bonferroni() {
        let l = dunnett_loadings(&[5, 5, 5], 5);
        let t = StudentsT::new(0.0, 1.0, 16.0).unwrap();
        let single = 2.0 * (1.0 - t.cdf(2.5));
        let p = dunnett_p(2.5, &l, 16.0);
        assert!(p > single && p < 3.0 * single, "{p} {single}");
        let d = dunnett_critical(0.05, &l, 16.0);
        // The published two-sided 5 % point for 3 treatments against a control with 16 degrees of freedom.
        assert!((d - 2.59).abs() < 0.01, "{d}");
        assert!((dunnett_p(d, &l, 16.0) - 0.05).abs() < 1e-9);
    }

    #[test]
    fn large_degrees_of_freedom_use_the_normal_limit() {
        let l = dunnett_loadings(&[10, 10], 10);
        let near = dunnett_p(2.2, &l, 5001.0);
        let far = dunnett_p(2.2, &l, 4999.0);
        assert!((near - far).abs() < 1e-4, "{near} {far}");
    }
}
