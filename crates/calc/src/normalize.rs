//! Normalisation (Origin `normalize`). All statistics are taken over the valid (non-missing)
//! points; missing points stay NaN.

/// Normalisation method (Origin option `method`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NormalizeMethod {
    /// Divide by a given value (`specify`).
    Divide(f64),
    /// Map to [0, 1] (`range`).
    ZeroToOne,
    /// Map to [0, 100] (`range100`).
    ZeroToHundred,
    /// Z scores with the sample standard deviation (`snd`).
    ZScore,
    /// Divide by the maximum (`max`).
    Max,
    /// Divide by the minimum (`min`).
    Min,
    /// Divide by the mean (`mean`).
    Mean,
    /// Divide by the median (`median`).
    Median,
    /// Divide by the sample standard deviation (`sd`).
    Sd,
    /// Divide by the Euclidean norm (`norm`).
    Norm,
    /// Divide by the mode (`mode`); all NaN when no value occurs twice.
    Mode,
    /// Divide by the sum (`sum`).
    Sum,
    /// Origin's `ref` (T260): scale the data so that its statistic `stat` equals `target` (the same
    /// statistic of a reference column): `y * target / stat(y)`. OriginLab does not print the formula;
    /// this reading is Sheaf's.
    MatchStat { stat: RefStat, target: f64 },
}

/// The statistics Origin's `ref` method can match (its `type` variable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefStat {
    Min,
    Max,
    Mean,
    Median,
    Sum,
}

impl RefStat {
    pub const ALL: [RefStat; 5] = [RefStat::Min, RefStat::Max, RefStat::Mean, RefStat::Median, RefStat::Sum];

    pub fn label(self) -> &'static str {
        match self {
            RefStat::Min => "minimum",
            RefStat::Max => "maximum",
            RefStat::Mean => "mean",
            RefStat::Median => "median",
            RefStat::Sum => "sum",
        }
    }
}

/// The statistic `stat` of the valid (finite) values of `y`; NaN when there is none.
pub fn ref_stat(y: &[f64], stat: RefStat) -> f64 {
    let mut v: Vec<f64> = y.iter().copied().filter(|a| a.is_finite()).collect();
    if v.is_empty() {
        return f64::NAN;
    }
    match stat {
        RefStat::Min => v.iter().copied().fold(f64::INFINITY, f64::min),
        RefStat::Max => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        RefStat::Mean => v.iter().sum::<f64>() / v.len() as f64,
        RefStat::Sum => v.iter().sum(),
        RefStat::Median => {
            v.sort_by(|a, b| a.total_cmp(b));
            let n = v.len();
            if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
        }
    }
}

/// Normalises `y`; the result has the length of the input.
pub fn normalize(y: &[f64], method: NormalizeMethod) -> Vec<f64> {
    let v: Vec<f64> = y.iter().copied().filter(|a| a.is_finite()).collect();
    let n = v.len();
    let nf = n as f64;
    let sum: f64 = v.iter().sum();
    let mean = sum / nf;
    let min = v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let sd = || (v.iter().map(|a| (a - mean) * (a - mean)).sum::<f64>() / (nf - 1.0)).sqrt();
    // (offset, divisor, multiplier): out = (y - offset) / divisor * mult
    let (off, div, mult) = match method {
        NormalizeMethod::Divide(d) => (0.0, d, 1.0),
        NormalizeMethod::ZeroToOne => (min, max - min, 1.0),
        NormalizeMethod::ZeroToHundred => (min, max - min, 100.0),
        NormalizeMethod::ZScore => (mean, sd(), 1.0),
        NormalizeMethod::Max => (0.0, max, 1.0),
        NormalizeMethod::Min => (0.0, min, 1.0),
        NormalizeMethod::Mean => (0.0, mean, 1.0),
        NormalizeMethod::Median => {
            let mut s = v.clone();
            s.sort_by(|a, b| a.total_cmp(b));
            let med = if n == 0 {
                f64::NAN
            } else if n % 2 == 1 {
                s[n / 2]
            } else {
                (s[n / 2 - 1] + s[n / 2]) / 2.0
            };
            (0.0, med, 1.0)
        }
        NormalizeMethod::Sd => (0.0, sd(), 1.0),
        NormalizeMethod::Norm => (0.0, v.iter().map(|a| a * a).sum::<f64>().sqrt(), 1.0),
        NormalizeMethod::Mode => {
            let mut s = v.clone();
            s.sort_by(|a, b| a.total_cmp(b));
            let (mut best, mut best_count) = (f64::NAN, 1);
            let mut i = 0;
            while i < n {
                let mut j = i;
                while j + 1 < n && s[j + 1] == s[i] {
                    j += 1;
                }
                if j - i + 1 > best_count {
                    best_count = j - i + 1;
                    best = s[i];
                }
                i = j + 1;
            }
            (0.0, best, 1.0)
        }
        NormalizeMethod::Sum => (0.0, sum, 1.0),
        NormalizeMethod::MatchStat { stat, target } => (0.0, ref_stat(y, stat), target),
    };
    y.iter().map(|&a| if a.is_finite() { (a - off) / div * mult } else { f64::NAN }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_a_reference_statistic_scales_the_data() {
        // T260: the mean of 1, 2, 3 (2) is made 10: everything times 5.
        let out =
            normalize(&[1.0, 2.0, f64::NAN, 3.0], NormalizeMethod::MatchStat { stat: RefStat::Mean, target: 10.0 });
        assert_eq!(out[..2], [5.0, 10.0]);
        assert!(out[2].is_nan());
        assert_eq!(out[3], 15.0);
        assert_eq!(ref_stat(&[4.0, 1.0, 3.0, 2.0], RefStat::Median), 2.5);
        assert_eq!(ref_stat(&[4.0, f64::NAN, 1.0], RefStat::Min), 1.0);
        assert_eq!(ref_stat(&[4.0, 1.0], RefStat::Sum), 5.0);
        assert!(ref_stat(&[f64::NAN], RefStat::Max).is_nan());
    }
}
