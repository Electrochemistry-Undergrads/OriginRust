//! Tests of normality: Shapiro-Wilk, Kolmogorov-Smirnov, Lilliefors, Anderson-Darling and
//! D'Agostino's K squared (skewness, kurtosis and the omnibus test).
//!
//! Each test returns a statistic and a p-value, the probability under the null hypothesis (the data
//! come from a normal distribution) of a statistic as extreme as the observed one. Where Origin
//! reports a different number the differences are listed in `docs/spec/statistics.md`.
//! NaN entries are ignored.

use statrs::distribution::{ChiSquared, Continuous, ContinuousCDF, Normal};

fn clean(data: &[f64]) -> Vec<f64> {
    let mut v: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
    v.sort_by(|a, b| a.total_cmp(b));
    v
}

fn std_normal() -> Normal {
    Normal::new(0.0, 1.0).expect("standard normal")
}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    (mean, (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt())
}

/// Result of one test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestResult {
    pub statistic: f64,
    pub p: f64,
}

// ----- Shapiro-Wilk -----------------------------------------------------------------------------

fn poly(coef: &[f64], x: f64) -> f64 {
    coef.iter().rev().fold(0.0, |acc, c| acc * x + c)
}

/// Shapiro-Wilk W test (Royston's 1995 algorithm, valid for 3 to 5000 numbers). The statistic is W.
pub fn shapiro_wilk(data: &[f64]) -> Option<TestResult> {
    let x = clean(data);
    let n = x.len();
    if !(3..=5000).contains(&n) || x[n - 1] - x[0] < 1e-19 {
        return None;
    }
    let nf = n as f64;
    let half = n / 2;
    let norm = std_normal();
    // Coefficients a[0..half] for the ordered differences x[n-1-i] - x[i].
    let mut a = vec![0.0; half];
    if n == 3 {
        a[0] = std::f64::consts::FRAC_1_SQRT_2;
    } else {
        let m: Vec<f64> = (1..=half).map(|i| norm.inverse_cdf((i as f64 - 0.375) / (nf + 0.25))).collect();
        let summ2: f64 = 2.0 * m.iter().map(|v| v * v).sum::<f64>();
        let ssumm2 = summ2.sqrt();
        let rsn = 1.0 / nf.sqrt();
        let a1 = poly(&[0.0, 0.221157, -0.147981, -2.07119, 4.434685, -2.706056], rsn) - m[0] / ssumm2;
        let (first, fac) = if n > 5 {
            let a2 = -m[1] / ssumm2 + poly(&[0.0, 0.042981, -0.293762, -1.752461, 5.682633, -3.582633], rsn);
            a[1] = a2;
            (2, ((summ2 - 2.0 * m[0] * m[0] - 2.0 * m[1] * m[1]) / (1.0 - 2.0 * a1 * a1 - 2.0 * a2 * a2)).sqrt())
        } else {
            (1, ((summ2 - 2.0 * m[0] * m[0]) / (1.0 - 2.0 * a1 * a1)).sqrt())
        };
        a[0] = a1;
        for i in first..half {
            a[i] = -m[i] / fac;
        }
    }
    let mean = x.iter().sum::<f64>() / nf;
    let ssq: f64 = x.iter().map(|v| (v - mean).powi(2)).sum();
    let b: f64 = (0..half).map(|i| a[i] * (x[n - 1 - i] - x[i])).sum();
    let w = (b * b / ssq).min(1.0);
    let p = if n == 3 {
        let pi6 = 6.0 / std::f64::consts::PI;
        (pi6 * (w.sqrt().asin() - std::f64::consts::FRAC_PI_3)).max(0.0)
    } else {
        let w1 = 1.0 - w;
        let y = w1.ln();
        if n <= 11 {
            let gamma = poly(&[-2.273, 0.459], nf);
            if y >= gamma {
                1e-99
            } else {
                let y = -(gamma - y).ln();
                let mu = poly(&[0.544, -0.39978, 0.025054, -6.714e-4], nf);
                let sigma = poly(&[1.3822, -0.77857, 0.062767, -0.0020322], nf).exp();
                1.0 - Normal::new(mu, sigma).expect("positive spread").cdf(y)
            }
        } else {
            let xx = nf.ln();
            let mu = poly(&[-1.5861, -0.31082, -0.083751, 0.0038915], xx);
            let sigma = poly(&[-0.4803, -0.082676, 0.0030302], xx).exp();
            1.0 - Normal::new(mu, sigma).expect("positive spread").cdf(y)
        }
    };
    Some(TestResult { statistic: w, p })
}

// ----- Kolmogorov-Smirnov and Lilliefors ----------------------------------------------------------

/// The Kolmogorov-Smirnov distance between the sample and the normal distribution with the sample's
/// own mean and standard deviation.
pub fn ks_distance(data: &[f64]) -> Option<f64> {
    let x = clean(data);
    let n = x.len();
    if n < 2 {
        return None;
    }
    let (mean, sd) = mean_sd(&x);
    if sd.is_nan() || sd <= 0.0 {
        return None;
    }
    let norm = std_normal();
    let nf = n as f64;
    let mut d: f64 = 0.0;
    for (i, v) in x.iter().enumerate() {
        let f = norm.cdf((v - mean) / sd);
        d = d.max((i as f64 + 1.0) / nf - f).max(f - i as f64 / nf);
    }
    Some(d)
}

/// Largest sample size for which [`kolmogorov_cdf`] uses the exact matrix method; above it the asymptotic
/// series is used, which is off by up to 0.0034 (absolute, in the p-value) at n = 1000 and by 0.0011 at n = 10000.
pub const KS_EXACT_MAX_N: usize = 1000;

/// Above this `sqrt(n) d` the p-value is below 1e-7 and the asymptotic series (accurate to better than 1e-9
/// there) is used instead of a matrix that would grow with `n d`.
const KS_SERIES_LAMBDA: f64 = 3.0;

/// P(D_n < d) for the two-sided Kolmogorov statistic with a fully specified distribution
/// (Marsaglia, Tsang and Wang's matrix method, exact for `n` up to [`KS_EXACT_MAX_N`] unless the p-value is
/// below 1e-7; the asymptotic series with Stephens' correction beyond).
pub fn kolmogorov_cdf(n: usize, d: f64) -> f64 {
    if d <= 0.0 {
        return 0.0;
    }
    if d >= 1.0 {
        return 1.0;
    }
    let nf = n as f64;
    if n > KS_EXACT_MAX_N || (n > 140 && nf.sqrt() * d > KS_SERIES_LAMBDA) {
        let lambda = (nf.sqrt() + 0.12 + 0.11 / nf.sqrt()) * d;
        let mut q = 0.0;
        for k in 1..=100 {
            let kf = k as f64;
            let term = (-2.0 * kf * kf * lambda * lambda).exp();
            q += if k % 2 == 1 { term } else { -term };
            if term < 1e-18 {
                break;
            }
        }
        return 1.0 - 2.0 * q;
    }
    let k = (nf * d) as usize + 1;
    let m = 2 * k - 1;
    let h = k as f64 - nf * d;
    let mut hm = vec![0.0; m * m];
    for i in 0..m {
        for j in 0..m {
            hm[i * m + j] = if (i as isize) - (j as isize) + 1 < 0 { 0.0 } else { 1.0 };
        }
    }
    for i in 0..m {
        hm[i * m] -= h.powi(i as i32 + 1);
        hm[(m - 1) * m + i] -= h.powi((m - i) as i32);
    }
    hm[(m - 1) * m] += if 2.0 * h - 1.0 > 0.0 { (2.0 * h - 1.0).powi(m as i32) } else { 0.0 };
    for i in 0..m {
        for j in 0..m {
            if (i as isize) - (j as isize) + 1 > 0 {
                for g in 1..=(i - j + 1) {
                    hm[i * m + j] /= g as f64;
                }
            }
        }
    }
    // Matrix power with a running decimal exponent, to avoid overflow.
    fn mul(a: &[f64], b: &[f64], m: usize) -> Vec<f64> {
        let mut c = vec![0.0; m * m];
        for i in 0..m {
            for k in 0..m {
                let aik = a[i * m + k];
                if aik != 0.0 {
                    for j in 0..m {
                        c[i * m + j] += aik * b[k * m + j];
                    }
                }
            }
        }
        c
    }
    fn power(a: &[f64], ea: i32, m: usize, n: usize) -> (Vec<f64>, i32) {
        if n == 1 {
            return (a.to_vec(), ea);
        }
        let (half, eh) = power(a, ea, m, n / 2);
        let mut b = mul(&half, &half, m);
        let mut eb = 2 * eh;
        if n % 2 == 1 {
            b = mul(a, &b, m);
            eb += ea;
        }
        if b[(m / 2) * m + m / 2] > 1e140 {
            for v in b.iter_mut() {
                *v *= 1e-140;
            }
            eb += 140;
        }
        (b, eb)
    }
    let (q, mut e) = power(&hm, 0, m, n);
    let mut s = q[(k - 1) * m + k - 1];
    for i in 1..=n {
        s = s * i as f64 / nf;
        if s < 1e-140 {
            s *= 1e140;
            e -= 140;
        }
    }
    s * 10f64.powi(e)
}

/// Kolmogorov-Smirnov test against the normal distribution with the sample's mean and standard
/// deviation. The p-value treats those two numbers as given, so it is too large; the Lilliefors test
/// corrects for that.
pub fn kolmogorov_smirnov(data: &[f64]) -> Option<TestResult> {
    let d = ks_distance(data)?;
    let n = data.iter().filter(|x| !x.is_nan()).count();
    Some(TestResult { statistic: d, p: (1.0 - kolmogorov_cdf(n, d)).clamp(0.0, 1.0) })
}

/// Lilliefors test: the Kolmogorov-Smirnov distance with the mean and standard deviation estimated from
/// the data. The p-value uses the approximation of Dallal and Wilkinson, reported as at most 0.2.
pub fn lilliefors(data: &[f64]) -> Option<TestResult> {
    let d = ks_distance(data)?;
    let n = data.iter().filter(|x| !x.is_nan()).count();
    if n < 5 {
        return None;
    }
    let nf = n as f64;
    let (kd, nd) = if n > 100 { (d * (nf / 100.0).powf(0.49), 100.0) } else { (d, nf) };
    let mut p = (-7.01256 * kd * kd * (nd + 2.78019) + 2.99587 * kd * (nd + 2.78019).sqrt() - 0.122119
        + 0.974598 / nd.sqrt()
        + 1.67997 / nd)
        .exp();
    if p > 0.1 {
        let kk = (nf.sqrt() - 0.01 + 0.85 / nf.sqrt()) * d;
        p = if kk <= 0.302 {
            1.0
        } else if kk <= 0.5 {
            2.76773 - 19.828315 * kk + 80.709644 * kk.powi(2) - 138.55152 * kk.powi(3) + 81.218052 * kk.powi(4)
        } else if kk <= 0.9 {
            -4.901232 + 40.662806 * kk - 97.490286 * kk.powi(2) + 94.029866 * kk.powi(3) - 32.355711 * kk.powi(4)
        } else if kk <= 1.31 {
            6.198765 - 19.558097 * kk + 23.186922 * kk.powi(2) - 12.234627 * kk.powi(3) + 2.423045 * kk.powi(4)
        } else {
            0.0
        };
    }
    Some(TestResult { statistic: d, p: p.clamp(0.0, 1.0) })
}

// ----- Anderson-Darling ----------------------------------------------------------------------------

/// Approximate p-value of the Anderson-Darling statistic `a = A2 (1 + 0.75/n + 2.25/n^2)` of a normal distribution
/// whose mean and standard deviation were estimated (the formula of D'Agostino and Stephens).
pub(crate) fn ad_normal_p(a: f64) -> f64 {
    let p = if a >= 0.6 {
        (1.2937 - 5.709 * a + 0.0186 * a * a).exp()
    } else if a >= 0.34 {
        (0.9177 - 4.279 * a - 1.38 * a * a).exp()
    } else if a >= 0.2 {
        1.0 - (-8.318 + 42.796 * a - 59.938 * a * a).exp()
    } else {
        1.0 - (-13.436 + 101.14 * a - 223.73 * a * a).exp()
    };
    p.clamp(0.0, 1.0)
}

/// Anderson-Darling test with mean and standard deviation estimated from the data. The statistic is
/// A squared; the p-value comes from the adjusted statistic A*^2 = A^2 (1 + 0.75/n + 2.25/n^2) by the
/// formulas of D'Agostino and Stephens.
pub fn anderson_darling(data: &[f64]) -> Option<TestResult> {
    let x = clean(data);
    let n = x.len();
    if n < 8 {
        return None;
    }
    let (mean, sd) = mean_sd(&x);
    if sd.is_nan() || sd <= 0.0 {
        return None;
    }
    let norm = std_normal();
    let nf = n as f64;
    let cdf: Vec<f64> = x.iter().map(|v| norm.cdf((v - mean) / sd).clamp(1e-300, 1.0 - 1e-16)).collect();
    let s: f64 = (0..n).map(|i| (2.0 * i as f64 + 1.0) * (cdf[i].ln() + (1.0 - cdf[n - 1 - i]).ln())).sum();
    let a2 = -nf - s / nf;
    let a = a2 * (1.0 + 0.75 / nf + 2.25 / (nf * nf));
    let p = ad_normal_p(a);
    Some(TestResult { statistic: a2, p: p.clamp(0.0, 1.0) })
}

// ----- D'Agostino ------------------------------------------------------------------------------------

/// Results of D'Agostino's tests: the skewness test (needs 8 numbers), the kurtosis test (5 numbers)
/// and the omnibus K squared test (8 numbers). The statistics of the first two are z-scores; their
/// p-values are two-sided. K squared is the sum of the two squared z-scores with 2 degrees of freedom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DAgostino {
    pub skewness: Option<TestResult>,
    pub kurtosis: Option<TestResult>,
    pub omnibus: Option<TestResult>,
}

pub fn dagostino(data: &[f64]) -> DAgostino {
    let x = clean(data);
    let n = x.len();
    let none = DAgostino { skewness: None, kurtosis: None, omnibus: None };
    if n < 5 {
        return none;
    }
    let nf = n as f64;
    let mean = x.iter().sum::<f64>() / nf;
    let m = |k: i32| x.iter().map(|v| (v - mean).powi(k)).sum::<f64>() / nf;
    let (m2, m3, m4) = (m(2), m(3), m(4));
    if m2.is_nan() || m2 <= 0.0 {
        return none;
    }
    let norm = std_normal();
    let two_sided = |z: f64| 2.0 * norm.cdf(-z.abs());
    let kurtosis = {
        let b2 = m4 / (m2 * m2);
        let e = 3.0 * (nf - 1.0) / (nf + 1.0);
        let var = 24.0 * nf * (nf - 2.0) * (nf - 3.0) / ((nf + 1.0).powi(2) * (nf + 3.0) * (nf + 5.0));
        let xx = (b2 - e) / var.sqrt();
        let sqrt_b1 = 6.0 * (nf * nf - 5.0 * nf + 2.0) / ((nf + 7.0) * (nf + 9.0))
            * (6.0 * (nf + 3.0) * (nf + 5.0) / (nf * (nf - 2.0) * (nf - 3.0))).sqrt();
        let a = 6.0 + 8.0 / sqrt_b1 * (2.0 / sqrt_b1 + (1.0 + 4.0 / (sqrt_b1 * sqrt_b1)).sqrt());
        let term = (1.0 - 2.0 / a) / (1.0 + xx * (2.0 / (a - 4.0)).sqrt());
        let z = ((1.0 - 2.0 / (9.0 * a)) - term.cbrt()) / (2.0 / (9.0 * a)).sqrt();
        TestResult { statistic: z, p: two_sided(z) }
    };
    if n < 8 {
        return DAgostino { skewness: None, kurtosis: Some(kurtosis), omnibus: None };
    }
    let skewness = {
        let b1 = m3 / m2.powf(1.5);
        let y = b1 * ((nf + 1.0) * (nf + 3.0) / (6.0 * (nf - 2.0))).sqrt();
        let beta2 = 3.0 * (nf * nf + 27.0 * nf - 70.0) * (nf + 1.0) * (nf + 3.0)
            / ((nf - 2.0) * (nf + 5.0) * (nf + 7.0) * (nf + 9.0));
        let w2 = -1.0 + (2.0 * (beta2 - 1.0)).sqrt();
        let delta = 1.0 / (0.5 * w2.ln()).sqrt();
        let alpha = (2.0 / (w2 - 1.0)).sqrt();
        let z = delta * (y / alpha + ((y / alpha).powi(2) + 1.0).sqrt()).ln();
        TestResult { statistic: z, p: two_sided(z) }
    };
    let k2 = skewness.statistic.powi(2) + kurtosis.statistic.powi(2);
    let omnibus = ChiSquared::new(2.0).ok().map(|d| TestResult { statistic: k2, p: 1.0 - d.cdf(k2) });
    let _ = norm.pdf(0.0);
    DAgostino { skewness: Some(skewness), kurtosis: Some(kurtosis), omnibus }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapiro_wilk_of_a_straight_line_is_close_to_one() {
        let x: Vec<f64> = (1..=20).map(f64::from).collect();
        let r = shapiro_wilk(&x).unwrap();
        assert!(r.statistic > 0.95 && r.p > 0.5, "{r:?}");
    }

    #[test]
    fn shapiro_wilk_refuses_tiny_or_constant_samples() {
        assert!(shapiro_wilk(&[1.0, 2.0]).is_none());
        assert!(shapiro_wilk(&[2.0, 2.0, 2.0, 2.0]).is_none());
    }

    #[test]
    fn kolmogorov_distribution_known_values() {
        // P(D_1 < d) = 2 d - 1 for d between 1/2 and 1.
        assert!((kolmogorov_cdf(1, 0.75) - 0.5).abs() < 1e-12);
        // Large-sample limit: Q(1.36) is about 0.05.
        let p = 1.0 - kolmogorov_cdf(200, 1.36 / 200f64.sqrt());
        assert!((p - 0.05).abs() < 0.005, "{p}");
    }

    #[test]
    fn missing_values_are_ignored() {
        let a = [1.0, 2.0, 2.5, 3.0, 4.0, 4.2, 5.0, 7.0];
        let mut b = a.to_vec();
        b.insert(3, f64::NAN);
        assert_eq!(shapiro_wilk(&a), shapiro_wilk(&b));
    }
}
