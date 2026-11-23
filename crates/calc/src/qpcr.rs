//! qPCR relative quantification and amplification efficiency (Origin 2027's assay apps, "analyze qPCR
//! experiments"; not in Origin 2016, so nothing here is measured in Origin; `docs/spec/statistics.md` section 50).
//!
//! **Relative quantification**: each well has a group, a target Ct and a reference Ct. Per group, `dCt = Ct_target -
//! Ct_reference` (mean and sample SD). Against the control group, `ddCt = mean dCt - mean dCt(control)` and the Livak
//! fold change `2^-ddCt`; with amplification factors `E_t`, `E_r` (2 = 100 % efficiency) the Pfaffl ratio `E_t^(mean
//! Ct_t(control) - mean Ct_t) / E_r^(mean Ct_r(control) - mean Ct_r)`, which is `2^-ddCt` for `E = 2`. The standard
//! error of `ddCt` is `sqrt(s^2/n + s_c^2/n_c)`; the confidence interval of the ratio comes from `ddCt -+ t SE` with
//! Welch's degrees of freedom (mapped through the ratio's base: `E_t` and `E_r` enter as `ln E` weights when they
//! differ, see `ratio_at`), and the two-sided Welch t test compares the group's dCt with the control's.
//!
//! **Efficiency**: a dilution series gives Ct against `log10` quantity; the least-squares slope gives the
//! amplification factor `E = 10^(-1/slope)` and the efficiency `(E - 1) 100 %`.
//!
//! Checked against numpy and scipy in `crates/calc/tests/qpcr_reference.rs` (made by `tools/qpcr-ref.py`).

use statrs::distribution::{ContinuousCDF, StudentsT};

/// One group's result.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupResult {
    pub n: usize,
    pub mean_target: f64,
    pub mean_reference: f64,
    pub mean_dct: f64,
    pub sd_dct: f64,
    pub ddct: f64,
    pub se_ddct: f64,
    /// Expression ratio against the control (Livak `2^-ddCt` when both factors are 2).
    pub ratio: f64,
    pub ratio_low: f64,
    pub ratio_high: f64,
    /// Welch's two-sided test of dCt against the control (NaN for the control itself).
    pub p: f64,
}

/// Why the quantification failed.
#[derive(Debug, Clone, PartialEq)]
pub enum QpcrError {
    Ragged,
    NoControl,
    TooFewWells(String),
    BadEfficiency(f64),
    BadLevel,
    NoSlope,
}

impl std::fmt::Display for QpcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QpcrError::Ragged => f.write_str("the groups, target and reference Ct differ in length"),
            QpcrError::NoControl => f.write_str("no well belongs to the control group"),
            QpcrError::TooFewWells(g) => write!(f, "group {g} needs at least two wells"),
            QpcrError::BadEfficiency(e) => write!(f, "an amplification factor must lie between 1 and 3 (got {e})"),
            QpcrError::BadLevel => f.write_str("the confidence level must lie between 0 and 1"),
            QpcrError::NoSlope => f.write_str("the standard curve needs at least three points at two quantities"),
        }
    }
}

impl std::error::Error for QpcrError {}

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let sd = if v.len() > 1 { (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() } else { f64::NAN };
    (m, sd)
}

/// The ratio at a given `ddCt` shift `d` from the group's point estimate: the target and reference differences are
/// shifted in proportion so that, for equal factors, the ratio is `E^-(ddCt + d)`.
fn ratio_at(e_t: f64, e_r: f64, dt: f64, dr: f64, d: f64) -> f64 {
    // ln ratio = dt ln E_t - dr ln E_r; a shift d of ddCt (= dr - dt) moves it by -d times the mean log factor.
    let lt = e_t.ln();
    let lr = e_r.ln();
    (dt * lt - dr * lr - d * 0.5 * (lt + lr)).exp()
}

/// Relative quantification of every group against `control`; `groups[i]` is well i's group.
pub fn relative_quantification(
    groups: &[String],
    target: &[f64],
    reference: &[f64],
    control: &str,
    e_target: f64,
    e_reference: f64,
    confidence: f64,
) -> Result<Vec<(String, GroupResult)>, QpcrError> {
    if groups.len() != target.len() || groups.len() != reference.len() {
        return Err(QpcrError::Ragged);
    }
    for e in [e_target, e_reference] {
        if !(1.0..=3.0).contains(&e) {
            return Err(QpcrError::BadEfficiency(e));
        }
    }
    if !(confidence > 0.0 && confidence < 1.0) {
        return Err(QpcrError::BadLevel);
    }
    // Groups in first order, complete wells only.
    let mut names: Vec<String> = Vec::new();
    let mut wells: Vec<Vec<(f64, f64)>> = Vec::new();
    for ((g, t), r) in groups.iter().zip(target).zip(reference) {
        if !(t.is_finite() && r.is_finite()) {
            continue;
        }
        match names.iter().position(|n| n == g) {
            Some(k) => wells[k].push((*t, *r)),
            None => {
                names.push(g.clone());
                wells.push(vec![(*t, *r)]);
            }
        }
    }
    let c = names.iter().position(|n| n == control).ok_or(QpcrError::NoControl)?;
    if let Some(k) = wells.iter().position(|w| w.len() < 2) {
        return Err(QpcrError::TooFewWells(names[k].clone()));
    }
    let stats = |w: &[(f64, f64)]| {
        let t: Vec<f64> = w.iter().map(|p| p.0).collect();
        let r: Vec<f64> = w.iter().map(|p| p.1).collect();
        let d: Vec<f64> = w.iter().map(|p| p.0 - p.1).collect();
        (mean_sd(&t).0, mean_sd(&r).0, mean_sd(&d))
    };
    let (ct_c, cr_c, (dm_c, ds_c)) = stats(&wells[c]);
    let nc = wells[c].len() as f64;
    Ok(names
        .iter()
        .zip(&wells)
        .enumerate()
        .map(|(k, (name, w))| {
            let (mt, mr, (dm, ds)) = stats(w);
            let n = w.len() as f64;
            let ddct = dm - dm_c;
            let (va, vb) = (ds * ds / n, ds_c * ds_c / nc);
            let se = (va + vb).sqrt();
            let df = (va + vb).powi(2) / (va * va / (n - 1.0) + vb * vb / (nc - 1.0));
            let (dt, dr) = (ct_c - mt, cr_c - mr);
            let ratio = ratio_at(e_target, e_reference, dt, dr, 0.0);
            let (low, high, p) = if k == c {
                (f64::NAN, f64::NAN, f64::NAN)
            } else {
                match StudentsT::new(0.0, 1.0, df) {
                    Ok(d) if se > 0.0 => {
                        let tq = d.inverse_cdf(0.5 + confidence / 2.0);
                        let t = ddct / se;
                        (
                            ratio_at(e_target, e_reference, dt, dr, tq * se),
                            ratio_at(e_target, e_reference, dt, dr, -tq * se),
                            2.0 * d.sf(t.abs()),
                        )
                    }
                    _ => (f64::NAN, f64::NAN, f64::NAN),
                }
            };
            (
                name.clone(),
                GroupResult {
                    n: w.len(),
                    mean_target: mt,
                    mean_reference: mr,
                    mean_dct: dm,
                    sd_dct: ds,
                    ddct,
                    se_ddct: if k == c { f64::NAN } else { se },
                    ratio,
                    ratio_low: low,
                    ratio_high: high,
                    p,
                },
            )
        })
        .collect())
}

/// A standard curve: slope, intercept, R^2, amplification factor and efficiency in percent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandardCurve {
    pub slope: f64,
    pub intercept: f64,
    pub r2: f64,
    pub factor: f64,
    pub efficiency: f64,
}

/// The standard curve of Ct against `log10(quantity)`.
pub fn standard_curve(quantity: &[f64], ct: &[f64]) -> Result<StandardCurve, QpcrError> {
    let pts: Vec<(f64, f64)> = quantity
        .iter()
        .zip(ct)
        .filter(|(q, c)| q.is_finite() && **q > 0.0 && c.is_finite())
        .map(|(q, c)| (q.log10(), *c))
        .collect();
    let n = pts.len() as f64;
    let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let syy: f64 = pts.iter().map(|p| (p.1 - my).powi(2)).sum();
    if pts.len() < 3 || sxx <= 0.0 {
        return Err(QpcrError::NoSlope);
    }
    let slope = sxy / sxx;
    let factor = 10f64.powf(-1.0 / slope);
    Ok(StandardCurve {
        slope,
        intercept: my - slope * mx,
        r2: if syy > 0.0 { sxy * sxy / (sxx * syy) } else { f64::NAN },
        factor,
        efficiency: (factor - 1.0) * 100.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn livak_and_pfaffl_agree_at_full_efficiency() {
        let g: Vec<String> = ["c", "c", "c", "t", "t", "t"].iter().map(|s| s.to_string()).collect();
        let target = [25.0, 25.2, 24.8, 23.0, 23.1, 22.9];
        let reference = [18.0, 18.1, 17.9, 18.0, 18.0, 18.0];
        let r = relative_quantification(&g, &target, &reference, "c", 2.0, 2.0, 0.95).unwrap();
        let (_, t) = &r[1];
        assert!((t.ddct + 2.0).abs() < 1e-12, "{t:?}");
        assert!((t.ratio - 4.0).abs() < 1e-12, "two cycles earlier: four times as much");
        assert!(t.ratio_low < 4.0 && t.ratio_high > 4.0 && t.p < 0.01);
        assert!(r[0].1.p.is_nan() && (r[0].1.ratio - 1.0).abs() < 1e-15);
        // A perfect doubling series: slope -1/log10(2) gives E = 2, 100 %.
        let q = [1.0, 10.0, 100.0, 1000.0];
        let ct: Vec<f64> = q.iter().map(|v: &f64| 30.0 - v.log10() / 2f64.log10()).collect();
        let s = standard_curve(&q, &ct).unwrap();
        assert!((s.factor - 2.0).abs() < 1e-12 && (s.efficiency - 100.0).abs() < 1e-9 && (s.r2 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn bad_input_is_refused() {
        let g: Vec<String> = ["c", "t"].iter().map(|s| s.to_string()).collect();
        assert_eq!(relative_quantification(&g, &[1.0], &[1.0, 2.0], "c", 2.0, 2.0, 0.95), Err(QpcrError::Ragged));
        assert_eq!(
            relative_quantification(&g, &[1.0, 2.0], &[1.0, 2.0], "x", 2.0, 2.0, 0.95),
            Err(QpcrError::NoControl)
        );
        assert_eq!(
            relative_quantification(&g, &[1.0, 2.0], &[1.0, 2.0], "c", 2.0, 2.0, 0.95),
            Err(QpcrError::TooFewWells("c".into()))
        );
        assert_eq!(
            relative_quantification(&g, &[1.0, 2.0], &[1.0, 2.0], "c", 3.5, 2.0, 0.95),
            Err(QpcrError::BadEfficiency(3.5))
        );
        assert_eq!(standard_curve(&[1.0, 1.0, 1.0], &[20.0, 21.0, 22.0]), Err(QpcrError::NoSlope));
    }
}
