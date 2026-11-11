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
    };
    y.iter().map(|&a| if a.is_finite() { (a - off) / div * mult } else { f64::NAN }).collect()
}
