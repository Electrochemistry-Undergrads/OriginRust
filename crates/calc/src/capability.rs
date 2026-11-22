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
}

impl CapModel {
    pub fn name(self) -> &'static str {
        match self {
            CapModel::Normal => "normal",
            CapModel::Lognormal => "lognormal",
            CapModel::Weibull => "Weibull",
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
    }
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
    let min_of = |a: f64, b: f64| match (a.is_nan(), b.is_nan()) {
        (true, _) => b,
        (_, true) => a,
        _ => a.min(b),
    };
    let nan_ppm = Ppm { below: f64::NAN, above: f64::NAN, total: f64::NAN };
    match model {
        CapModel::Normal => {
            let sd_within = within_sd(&x, subgroup).ok_or(CapabilityError::TooFewValues)?;
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
                Ppm::of(
                    lsl.map_or(0.0, |l| normal().cdf((l - mean) / s)),
                    usl.map_or(0.0, |u| normal().sf((u - mean) / s)),
                )
            };
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
            })
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
    fn bad_input_is_refused() {
        let x = [1.0, 2.0, 3.0];
        assert_eq!(capability(&x, None, None, 1, CapModel::Normal), Err(CapabilityError::NoLimits));
        assert_eq!(capability(&x, Some(3.0), Some(1.0), 1, CapModel::Normal), Err(CapabilityError::LimitsReversed));
        assert_eq!(capability(&x, Some(0.0), None, 0, CapModel::Normal), Err(CapabilityError::BadSubgroup(0)));
        assert_eq!(capability(&x, Some(0.0), None, 4, CapModel::Normal), Err(CapabilityError::TooFewValues));
        assert_eq!(capability(&[2.0; 4], Some(0.0), None, 1, CapModel::Normal), Err(CapabilityError::NoSpread));
        assert!(capability(&[-1.0, 2.0, 3.0], Some(0.0), None, 1, CapModel::Weibull).is_err());
    }
}
