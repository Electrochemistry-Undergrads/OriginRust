//! Outlier tests: Grubbs' test and Dixon's Q test of one suspect value.
//!
//! The rules follow what Origin reports for `grubbs` and `qtest` (see `docs/spec/statistics.md`, section 10).

use statrs::distribution::{ContinuousCDF, StudentsT};

/// Which end of the data the suspect value is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlierSide {
    Lowest,
    Highest,
}

impl OutlierSide {
    pub fn word(self) -> &'static str {
        match self {
            OutlierSide::Lowest => "lowest",
            OutlierSide::Highest => "highest",
        }
    }
}

/// Why an outlier test has no result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutlierError {
    /// Fewer numbers than the test needs (`needed` is the smallest count).
    TooFew {
        needed: usize,
        have: usize,
    },
    /// Dixon's table stops at ten numbers.
    TooMany {
        most: usize,
        have: usize,
    },
    /// All numbers are equal.
    AllEqual,
    BadAlpha,
}

impl std::fmt::Display for OutlierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutlierError::TooFew { needed, have } => {
                write!(f, "the test needs at least {needed} numbers, there are {have}")
            }
            OutlierError::TooMany { most, have } => {
                write!(f, "the test takes at most {most} numbers, there are {have}")
            }
            OutlierError::AllEqual => write!(f, "all valid input data are equal"),
            OutlierError::BadAlpha => write!(f, "the significance level must be between 0 and 1"),
        }
    }
}

/// Result of [`grubbs`].
#[derive(Debug, Clone, PartialEq)]
pub struct Grubbs {
    pub n: usize,
    /// The suspect value, the one farthest from the mean (the lowest if both ends are equally far).
    pub value: f64,
    /// Its 1-based position in the data that were given (missing values count as rows).
    pub index: usize,
    pub side: OutlierSide,
    /// `|value - mean| / SD`.
    pub g: f64,
    /// Critical value of `g` for the two-sided test at the level.
    pub critical: f64,
    /// Origin's approximate p-value `n * 2 * P(T > t)`; it is not limited to 1 and is NaN when `g` is the
    /// largest value it can take.
    pub p: f64,
    pub significant: bool,
}

/// Grubbs' test of the point farthest from the mean (NaN entries are ignored). `alpha` is the two-sided
/// significance level. Needs three numbers that are not all equal.
pub fn grubbs(data: &[f64], alpha: f64) -> Result<Grubbs, OutlierError> {
    if !(alpha > 0.0 && alpha < 1.0) {
        return Err(OutlierError::BadAlpha);
    }
    let valid: Vec<(usize, f64)> = data.iter().copied().enumerate().filter(|(_, v)| !v.is_nan()).collect();
    let n = valid.len();
    if n < 3 {
        return Err(OutlierError::TooFew { needed: 3, have: n });
    }
    let nf = n as f64;
    let mean = valid.iter().map(|(_, v)| v).sum::<f64>() / nf;
    let sd = (valid.iter().map(|(_, v)| (v - mean).powi(2)).sum::<f64>() / (nf - 1.0)).sqrt();
    if sd == 0.0 || !sd.is_finite() {
        return Err(OutlierError::AllEqual);
    }
    let mut best = valid[0];
    for &(i, v) in &valid {
        if (v - mean).abs() > (best.1 - mean).abs() {
            best = (i, v);
        }
    }
    let g = (best.1 - mean).abs() / sd;
    let t_crit = StudentsT::new(0.0, 1.0, nf - 2.0).expect("t").inverse_cdf(1.0 - alpha / (2.0 * nf));
    let critical = (nf - 1.0) / nf.sqrt() * (t_crit * t_crit / (nf - 2.0 + t_crit * t_crit)).sqrt();
    let denom = (nf - 1.0).powi(2) - nf * g * g;
    let p = if denom > 0.0 {
        let t = g * (nf * (nf - 2.0) / denom).sqrt();
        nf * 2.0 * (1.0 - StudentsT::new(0.0, 1.0, nf - 2.0).expect("t").cdf(t))
    } else {
        f64::NAN
    };
    Ok(Grubbs {
        n,
        value: best.1,
        index: best.0 + 1,
        side: if best.1 > mean { OutlierSide::Highest } else { OutlierSide::Lowest },
        g,
        critical,
        p,
        significant: g > critical,
    })
}

/// Critical values of Dixon's `r10` ratio for 3 to 10 numbers: significance levels 0.1, 0.05 and 0.01
/// (the table Origin uses; rows are n = 3, ..., 10).
const DIXON_CRITICAL: [[f64; 3]; 8] = [
    [0.941, 0.970, 0.994],
    [0.765, 0.829, 0.926],
    [0.642, 0.710, 0.821],
    [0.560, 0.625, 0.740],
    [0.507, 0.568, 0.680],
    [0.468, 0.526, 0.634],
    [0.437, 0.493, 0.598],
    [0.412, 0.466, 0.568],
];

/// Result of [`dixon_q`].
#[derive(Debug, Clone, PartialEq)]
pub struct DixonQ {
    pub n: usize,
    pub value: f64,
    /// 1-based position in the data that were given (missing values count as rows).
    pub index: usize,
    pub side: OutlierSide,
    /// Gap between the suspect value and its neighbour, divided by the range.
    pub q: f64,
    pub critical: f64,
    pub significant: bool,
}

/// Dixon's Q test of the lowest or the highest value, whichever has the larger ratio (the lowest on a tie).
/// `alpha` must be 0.1, 0.05 or 0.01 (Origin offers these three); needs 3 to 10 numbers, not all equal.
pub fn dixon_q(data: &[f64], alpha: f64) -> Result<DixonQ, OutlierError> {
    let col = [0.1, 0.05, 0.01].iter().position(|a| (alpha - a).abs() < 1e-12).ok_or(OutlierError::BadAlpha)?;
    let mut valid: Vec<(usize, f64)> = data.iter().copied().enumerate().filter(|(_, v)| !v.is_nan()).collect();
    let n = valid.len();
    if n < 3 {
        return Err(OutlierError::TooFew { needed: 3, have: n });
    }
    if n > 10 {
        return Err(OutlierError::TooMany { most: 10, have: n });
    }
    valid.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let (lo, hi) = (valid[0], valid[n - 1]);
    let range = hi.1 - lo.1;
    if range == 0.0 {
        return Err(OutlierError::AllEqual);
    }
    let q_low = (valid[1].1 - lo.1) / range;
    let q_high = (hi.1 - valid[n - 2].1) / range;
    let (q, pick, side) =
        if q_high > q_low { (q_high, hi, OutlierSide::Highest) } else { (q_low, lo, OutlierSide::Lowest) };
    let critical = DIXON_CRITICAL[n - 3][col];
    Ok(DixonQ { n, value: pick.1, index: pick.0 + 1, side, q, critical, significant: q > critical })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grubbs_finds_the_outlier_and_skips_missing_values_in_the_index() {
        let g = grubbs(&[2.1, f64::NAN, 2.2, 2.6, 2.3, 2.5, 9.8, 2.2], 0.05).unwrap();
        assert_eq!((g.value, g.index, g.side, g.n), (9.8, 7, OutlierSide::Highest, 7));
        assert!(g.significant && g.g > g.critical);
        assert_eq!(grubbs(&[1.0, 2.0], 0.05), Err(OutlierError::TooFew { needed: 3, have: 2 }));
        assert_eq!(grubbs(&[3.0; 5], 0.05), Err(OutlierError::AllEqual));
    }

    #[test]
    fn dixon_q_ties_go_to_the_lowest() {
        let q = dixon_q(&[1.0, 2.0, 3.0], 0.05).unwrap();
        assert_eq!((q.q, q.index, q.side, q.significant), (0.5, 1, OutlierSide::Lowest, false));
        let big = dixon_q(&[1.0, 1.1, 1.2, 1.15, 3.0], 0.05).unwrap();
        assert!(big.significant && big.side == OutlierSide::Highest);
        assert_eq!(dixon_q(&[0.0; 11], 0.05), Err(OutlierError::TooMany { most: 10, have: 11 }));
    }
}
