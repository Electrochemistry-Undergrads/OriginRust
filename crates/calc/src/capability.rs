//! Process capability with normal and nonnormal models (Origin 2027's Quality improvement apps add nonnormal
//! capability; Sheaf has no Origin 2016 capability window to copy, so nothing here is measured in Origin;
//! `docs/spec/statistics.md` section 36).
//!
//! - **Normal**: the within standard deviation is `R bar / d2` from the ranges of complete subgroups of `n` (a
//!   short last subgroup is left out of it), or, for `n = 1`, the mean moving range of two over `d2(2) =
//!   1.128379`; the overall one is the sample standard deviation. `Cp = (USL - LSL) / 6 s_w`, `CPL = (mean - LSL)
//!   / 3 s_w`, `CPU = (USL - mean) / 3 s_w`, `Cpk = min(CPL, CPU)`, and `Pp`, `PPL`, `PPU`, `Ppk` the same with the
//!   overall deviation. The expected parts per million outside are the normal tails with either deviation.
//! - **Lognormal or Weibull** (maximum likelihood): the percentile (ISO 22514) method, with `x_lo`, `x_med` and
//!   `x_hi` the fitted quantiles at 0.00135, 0.5 and 0.99865: `Pp = (USL - LSL) / (x_hi - x_lo)`, `PPL = (x_med -
//!   LSL) / (x_med - x_lo)`, `PPU = (USL - x_med) / (x_hi - x_med)`, `Ppk = min(PPL, PPU)`; the expected parts
//!   per million are the fitted tails. The within indices are not defined (NaN).
//!
//! - **Box-Cox** (T419): the values (all above 0) and the limits are transformed by `y = (x^lambda - 1) / lambda`
//!   (`ln x` for `lambda = 0`), `lambda` given or the maximum-likelihood one, which maximizes the profile
//!   log-likelihood `(lambda - 1) sum ln x - n/2 ln(variance of y with divisor n)` over [-5, 5]; then the normal
//!   indices of the transformed values against the transformed limits. A limit at or below 0 that has no transform is
//!   left out.
//! - **Cpm** (T419) with a target `T` and both limits: `(USL - LSL) / 6 sqrt(s^2 + (mean - T)^2)` with the overall
//!   deviation, on the Box-Cox scale for that model; not defined for the lognormal and Weibull models.
//!
//! Either specification limit may be missing; the indices that need it are then NaN. Checked against scipy in
//! `crates/calc/tests/capability_reference.rs` (made by `tools/capability-ref.py`).

use crate::distfit::{Family, cdf, estimate, quantile};
use crate::qc;
use statrs::distribution::{ContinuousCDF, Normal};

/// The distribution the data are taken to follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CapModel {
    #[default]
    Normal,
    Lognormal,
    Weibull,
    /// Normal after a Box-Cox transformation (T419).
    BoxCox,
}

/// Choices beyond the model (T419).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CapOptions {
    /// Box-Cox: this lambda instead of the maximum-likelihood one.
    pub lambda: Option<f64>,
    /// The target for Cpm.
    pub target: Option<f64>,
}

impl CapModel {
    pub fn name(self) -> &'static str {
        match self {
            CapModel::Normal => "normal",
            CapModel::Lognormal => "lognormal",
            CapModel::Weibull => "Weibull",
            CapModel::BoxCox => "Box-Cox",
        }
    }
}

/// Parts per million below the lower and above the upper limit, and their sum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ppm {
    pub below: f64,
    pub above: f64,
    pub total: f64,
}

impl Ppm {
    fn of(below: f64, above: f64) -> Ppm {
        let (b, a) = (below * 1e6, above * 1e6);
        Ppm { below: b, above: a, total: b + a }
    }
}

/// The capability of a column.
#[derive(Debug, Clone, PartialEq)]
pub struct Capability {
    pub n: usize,
    pub mean: f64,
    pub sd_overall: f64,
    /// NaN for a nonnormal model.
    pub sd_within: f64,
    /// The fitted parameters: `[mean, sd]`, `[log mean, log sd]` or `[shape, scale]`.
    pub params: Vec<f64>,
    pub cp: f64,
    pub cpl: f64,
    pub cpu: f64,
    pub cpk: f64,
    pub pp: f64,
    pub ppl: f64,
    pub ppu: f64,
    pub ppk: f64,
    pub observed: Ppm,
    pub expected_overall: Ppm,
    /// NaN for a nonnormal model.
    pub expected_within: Ppm,
    /// Cpm with the target (NaN without a target or a limit, or for the lognormal and Weibull models).
    pub cpm: f64,
    /// The Box-Cox lambda (NaN for the other models).
    pub lambda: f64,
}

/// Why the capability could not be computed.
#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityError {
    NoLimits,
    LimitsReversed,
    TooFewValues,
    BadSubgroup(usize),
    NoSpread,
    NoFit(String),
    /// Box-Cox needs every value above 0.
    NotPositive,
    /// A given Box-Cox lambda that is not a finite number.
    BadLambda,
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityError::NoLimits => f.write_str("give a lower or an upper specification limit"),
            CapabilityError::LimitsReversed => f.write_str("the lower specification limit must be below the upper"),
            CapabilityError::TooFewValues => f.write_str("at least 2 values (and one complete subgroup) are needed"),
            CapabilityError::BadSubgroup(n) => {
                write!(f, "the subgroup size must be 1 to {} (got {n})", qc::MAX_GROUP)
            }
            CapabilityError::NoSpread => f.write_str("the values do not vary"),
            CapabilityError::NoFit(e) => write!(f, "the distribution could not be fitted: {e}"),
            CapabilityError::NotPositive => f.write_str("the Box-Cox transformation needs every value above 0"),
            CapabilityError::BadLambda => f.write_str("the Box-Cox lambda must be a number"),
        }
    }
}

impl std::error::Error for CapabilityError {}

/// The fitted distribution function at `x` (for drawing the fitted curve).
pub fn fitted_cdf(model: CapModel, params: &[f64], x: f64) -> f64 {
    match model {
        CapModel::Normal => Normal::new(params[0], params[1]).map_or(f64::NAN, |d| d.cdf(x)),
        CapModel::Lognormal => cdf(Family::Lognormal, params, x),
        CapModel::Weibull => cdf(Family::Weibull, params, x),
        CapModel::BoxCox if x > 0.0 => {
            Normal::new(params[0], params[1]).map_or(f64::NAN, |d| d.cdf(boxcox(x, params[2])))
        }
        CapModel::BoxCox => 0.0,
    }
}

/// The Box-Cox transform of `x` (above 0): `(x^lambda - 1) / lambda`, or `ln x` for `lambda = 0`.
pub fn boxcox(x: f64, lambda: f64) -> f64 {
    let l = x.ln();
    if lambda == 0.0 { l } else { (lambda * l).exp_m1() / lambda }
}

/// The profile log-likelihood of `lambda` for values above 0 (up to a constant).
fn boxcox_llf(x: &[f64], lambda: f64) -> f64 {
    let n = x.len() as f64;
    let y: Vec<f64> = x.iter().map(|v| boxcox(*v, lambda)).collect();
    let mean = y.iter().sum::<f64>() / n;
    let var = y.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (lambda - 1.0) * x.iter().map(|v| v.ln()).sum::<f64>() - n / 2.0 * var.ln()
}

/// The maximum-likelihood Box-Cox lambda of values above 0, searched in [-5, 5]: the best of a grid of 0.05, then a
/// golden-section search around it.
pub fn boxcox_lambda(x: &[f64]) -> f64 {
    let grid = (-100..=100).map(|i| f64::from(i) * 0.05);
    let best = grid.fold((0.0, f64::NEG_INFINITY), |(bl, bv), l| {
        let v = boxcox_llf(x, l);
        if v > bv { (l, v) } else { (bl, bv) }
    });
    let (mut a, mut b) = ((best.0 - 0.05).max(-5.0), (best.0 + 0.05).min(5.0));
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let (mut c, mut d) = (b - g * (b - a), a + g * (b - a));
    let (mut fc, mut fd) = (boxcox_llf(x, c), boxcox_llf(x, d));
    while b - a > 1e-12 {
        if fc > fd {
            b = d;
            (d, fd) = (c, fc);
            c = b - g * (b - a);
            fc = boxcox_llf(x, c);
        } else {
            a = c;
            (c, fc) = (d, fd);
            d = a + g * (b - a);
            fd = boxcox_llf(x, d);
        }
    }
    (a + b) / 2.0
}

/// The tail beyond three sigma, 0.00135.
fn alpha() -> f64 {
    normal().sf(3.0)
}

fn normal() -> Normal {
    Normal::new(0.0, 1.0).expect("the standard normal")
}

/// The within standard deviation: `R bar / d2` of complete subgroups of `n`, or the mean moving range of two
/// over `d2(2)` for `n = 1`. `None` without a complete subgroup.
pub fn within_sd(x: &[f64], n: usize) -> Option<f64> {
    let ranges: Vec<f64> = if n == 1 {
        x.windows(2).map(|w| (w[1] - w[0]).abs()).collect()
    } else {
        x.chunks_exact(n)
            .map(|g| {
                let (lo, hi) = g.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
                hi - lo
            })
            .collect()
    };
    if ranges.is_empty() {
        return None;
    }
    let rbar = ranges.iter().sum::<f64>() / ranges.len() as f64;
    Some(rbar / qc::d2(n.max(2))?)
}

/// The capability of the finite `values` (in row order) against the limits.
pub fn capability(
    values: &[f64],
    lsl: Option<f64>,
    usl: Option<f64>,
    subgroup: usize,
    model: CapModel,
) -> Result<Capability, CapabilityError> {
    capability_with(values, lsl, usl, subgroup, model, CapOptions::default())
}

/// The normal indices of `x` (transformed values for Box-Cox) against the limits on the same scale.
fn normal_capability(
    x: &[f64],
    lsl: Option<f64>,
    usl: Option<f64>,
    subgroup: usize,
    observed: Ppm,
    target: Option<f64>,
) -> Result<Capability, CapabilityError> {
    let n = x.len();
    let mean = x.iter().sum::<f64>() / n as f64;
    let sd_overall = (x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt();
    if sd_overall <= 0.0 {
        return Err(CapabilityError::NoSpread);
    }
    let (lo, hi) = (lsl.unwrap_or(f64::NAN), usl.unwrap_or(f64::NAN));
    let sd_within = within_sd(x, subgroup).ok_or(CapabilityError::TooFewValues)?;
    if sd_within <= 0.0 {
        return Err(CapabilityError::NoSpread);
    }
    let indices = |s: f64| {
        let lower = (mean - lo) / (3.0 * s);
        let upper = (hi - mean) / (3.0 * s);
        ((hi - lo) / (6.0 * s), lower, upper, min_of(lower, upper))
    };
    let (cp, cpl, cpu, cpk) = indices(sd_within);
    let (pp, ppl, ppu, ppk) = indices(sd_overall);
    let tails = |s: f64| {
        Ppm::of(lsl.map_or(0.0, |l| normal().cdf((l - mean) / s)), usl.map_or(0.0, |u| normal().sf((u - mean) / s)))
    };
    let cpm = target.map_or(f64::NAN, |t| (hi - lo) / (6.0 * (sd_overall.powi(2) + (mean - t).powi(2)).sqrt()));
    Ok(Capability {
        n,
        mean,
        sd_overall,
        sd_within,
        params: vec![mean, sd_overall],
        cp,
        cpl,
        cpu,
        cpk,
        pp,
        ppl,
        ppu,
        ppk,
        observed,
        expected_overall: tails(sd_overall),
        expected_within: tails(sd_within),
        cpm,
        lambda: f64::NAN,
    })
}

/// The smaller of two indices, the other one when either is missing.
fn min_of(a: f64, b: f64) -> f64 {
    match (a.is_nan(), b.is_nan()) {
        (true, _) => b,
        (_, true) => a,
        _ => a.min(b),
    }
}

/// [`capability`] with a Box-Cox lambda and a Cpm target (T419).
pub fn capability_with(
    values: &[f64],
    lsl: Option<f64>,
    usl: Option<f64>,
    subgroup: usize,
    model: CapModel,
    options: CapOptions,
) -> Result<Capability, CapabilityError> {
    let target = options.target.filter(|t| t.is_finite());
    let lsl = lsl.filter(|v| v.is_finite());
    let usl = usl.filter(|v| v.is_finite());
    if lsl.is_none() && usl.is_none() {
        return Err(CapabilityError::NoLimits);
    }
    if let (Some(l), Some(u)) = (lsl, usl)
        && l >= u
    {
        return Err(CapabilityError::LimitsReversed);
    }
    if !(1..=qc::MAX_GROUP).contains(&subgroup) {
        return Err(CapabilityError::BadSubgroup(subgroup));
    }
    let x: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let n = x.len();
    if n < 2 {
        return Err(CapabilityError::TooFewValues);
    }
    let mean = x.iter().sum::<f64>() / n as f64;
    let sd_overall = (x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt();
    if sd_overall <= 0.0 {
        return Err(CapabilityError::NoSpread);
    }
    let lo = lsl.unwrap_or(f64::NAN);
    let hi = usl.unwrap_or(f64::NAN);
    let observed = Ppm::of(
        lsl.map_or(0.0, |l| x.iter().filter(|v| **v < l).count() as f64 / n as f64),
        usl.map_or(0.0, |u| x.iter().filter(|v| **v > u).count() as f64 / n as f64),
    );
    let nan_ppm = Ppm { below: f64::NAN, above: f64::NAN, total: f64::NAN };
    match model {
        CapModel::Normal => {
            let both = lsl.and(usl).and(target);
            normal_capability(&x, lsl, usl, subgroup, observed, both)
        }
        CapModel::BoxCox => {
            if x.iter().any(|v| *v <= 0.0) {
                return Err(CapabilityError::NotPositive);
            }
            let lambda = match options.lambda {
                Some(l) if l.is_finite() => l,
                Some(_) => return Err(CapabilityError::BadLambda),
                None => boxcox_lambda(&x),
            };
            let t = |v: f64| Some(boxcox(v, lambda)).filter(|y| y.is_finite());
            let y: Vec<f64> = x.iter().map(|v| boxcox(*v, lambda)).collect();
            let (tl, tu) = (lsl.and_then(t), usl.and_then(t));
            let tt = target.and_then(t).filter(|_| tl.is_some() && tu.is_some());
            let mut c = normal_capability(&y, tl, tu, subgroup, observed, tt)?;
            c.params.push(lambda);
            c.lambda = lambda;
            Ok(c)
        }
        CapModel::Lognormal | CapModel::Weibull => {
            let family = if model == CapModel::Lognormal { Family::Lognormal } else { Family::Weibull };
            let params = estimate(family, &x).map_err(|e| CapabilityError::NoFit(e.to_string()))?;
            let q = |p: f64| quantile(family, &params, p);
            let (x_lo, x_med, x_hi) = (q(alpha()), q(0.5), q(1.0 - alpha()));
            let ppl = (x_med - lo) / (x_med - x_lo);
            let ppu = (hi - x_med) / (x_hi - x_med);
            let expected_overall = Ppm::of(
                lsl.map_or(0.0, |l| cdf(family, &params, l)),
                usl.map_or(0.0, |u| 1.0 - cdf(family, &params, u)),
            );
            Ok(Capability {
                n,
                mean,
                sd_overall,
                sd_within: f64::NAN,
                params,
                cp: f64::NAN,
                cpl: f64::NAN,
                cpu: f64::NAN,
                cpk: f64::NAN,
                pp: (hi - lo) / (x_hi - x_lo),
                ppl,
                ppu,
                ppk: min_of(ppl, ppu),
                observed,
                expected_overall,
                expected_within: nan_ppm,
                cpm: f64::NAN,
                lambda: f64::NAN,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_indices_from_moving_ranges_and_subgroups() {
        let x = [10.0, 12.0, 11.0, 13.0, 12.0, 14.0];
        let c = capability(&x, Some(5.0), Some(20.0), 1, CapModel::Normal).unwrap();
        // Moving ranges 2, 1, 2, 1, 2: mean 1.6.
        let sw = 1.6 / std::f64::consts::FRAC_2_SQRT_PI;
        assert!((c.sd_within - sw).abs() < 1e-6, "{}", c.sd_within);
        assert!((c.cp - 15.0 / (6.0 * c.sd_within)).abs() < 1e-12);
        assert!((c.cpk - (12.0 - 5.0) / (3.0 * c.sd_within)).abs() < 1e-12, "the lower side is nearer");
        assert_eq!(c.observed.total, 0.0);
        // Subgroups of 4: one complete group (range 3), the last two values left out of the within estimate.
        let c = capability(&x, None, Some(13.5), 4, CapModel::Normal).unwrap();
        assert!((c.sd_within - 3.0 / 2.058751).abs() < 1e-12);
        assert!(c.cp.is_nan() && c.cpl.is_nan() && c.cpk == c.cpu);
        assert!((c.observed.above - 1e6 / 6.0).abs() < 1e-6);
    }

    #[test]
    fn box_cox_with_lambda_one_is_the_normal_capability_shifted_and_cpm_follows_the_target() {
        let x = [10.0, 12.0, 11.0, 13.0, 12.0, 14.0];
        let n = capability(&x, Some(5.0), Some(20.0), 1, CapModel::Normal).unwrap();
        let opts = CapOptions { lambda: Some(1.0), target: Some(12.0) };
        let b = capability_with(&x, Some(5.0), Some(20.0), 1, CapModel::BoxCox, opts).unwrap();
        // lambda = 1 shifts values and limits by 1: the same indices.
        for (u, v) in
            [(n.cp, b.cp), (n.cpk, b.cpk), (n.ppk, b.ppk), (n.expected_overall.total, b.expected_overall.total)]
        {
            assert!((u - v).abs() < 1e-9, "{u} {v}");
        }
        assert!((b.mean - (n.mean - 1.0)).abs() < 1e-12 && b.lambda == 1.0 && b.params[2] == 1.0);
        let m = 12.0;
        let cpm = 15.0 / (6.0 * (n.sd_overall.powi(2) + (n.mean - m).powi(2)).sqrt());
        assert!((b.cpm - cpm).abs() < 1e-12, "{} {cpm}", b.cpm);
        let n2 = capability_with(&x, Some(5.0), Some(20.0), 1, CapModel::Normal, opts).unwrap();
        assert!((n2.cpm - cpm).abs() < 1e-12 && n.cpm.is_nan());
        // One limit: no Cpm.
        assert!(capability_with(&x, Some(5.0), None, 1, CapModel::Normal, opts).unwrap().cpm.is_nan());
        // lambda 0 is the log; a lower limit of 0 has no log and is left out.
        let opts = CapOptions { lambda: Some(0.0), target: None };
        let b = capability_with(&x, Some(0.0), Some(20.0), 1, CapModel::BoxCox, opts).unwrap();
        assert!(b.cpl.is_nan() && b.cpk == b.cpu && b.expected_overall.below == 0.0);
        assert!((boxcox(2.0, 1e-300) - 2f64.ln()).abs() < 1e-15);
    }

    #[test]
    fn the_box_cox_lambda_of_lognormal_values_is_near_zero() {
        let x: Vec<f64> = (1..=40).map(|i| (f64::from(i) / 41.0 - 0.5).exp().powi(3)).collect();
        // exp of a uniform grid cubed: the logs are evenly spread, so lambda is near 0 (not exactly: not normal).
        let l = boxcox_lambda(&x);
        assert!(l.abs() < 0.5, "{l}");
        let best = boxcox_llf(&x, l);
        assert!(boxcox_llf(&x, l + 1e-4) < best && boxcox_llf(&x, l - 1e-4) < best);
    }

    #[test]
    fn bad_input_is_refused() {
        let x = [1.0, 2.0, 3.0];
        assert_eq!(capability(&x, None, None, 1, CapModel::Normal), Err(CapabilityError::NoLimits));
        assert_eq!(capability(&x, Some(3.0), Some(1.0), 1, CapModel::Normal), Err(CapabilityError::LimitsReversed));
        assert_eq!(capability(&x, Some(0.0), None, 0, CapModel::Normal), Err(CapabilityError::BadSubgroup(0)));
        assert_eq!(capability(&x, Some(0.0), None, 4, CapModel::Normal), Err(CapabilityError::TooFewValues));
        assert_eq!(capability(&[2.0; 4], Some(0.0), None, 1, CapModel::Normal), Err(CapabilityError::NoSpread));
        assert!(capability(&[-1.0, 2.0, 3.0], Some(0.0), None, 1, CapModel::Weibull).is_err());
        assert_eq!(
            capability(&[0.0, 2.0, 3.0], Some(1.0), None, 1, CapModel::BoxCox),
            Err(CapabilityError::NotPositive)
        );
        let nan = CapOptions { lambda: Some(f64::NAN), target: None };
        assert_eq!(capability_with(&x, Some(0.5), None, 1, CapModel::BoxCox, nan), Err(CapabilityError::BadLambda));
    }
}
