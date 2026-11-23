//! Monte Carlo simulation (T393, Origin 2027's Monte Carlo simulation app; Sheaf's own, Origin's app
//! was not run; `docs/spec/statistics.md` section 51): the inputs of a model are drawn from their
//! distributions, the model's output is computed for every draw (by the caller, from a formula), and
//! the output is summarised, compared with specification limits and traced back to the inputs.
//!
//! - **Draws**: a seeded SplitMix64 stream gives uniform numbers in (0, 1); each input takes its own
//!   numbers in turn (input 1 row 1, input 2 row 1, ...), turned into values by the inverse of the
//!   distribution function (normal, lognormal: statrs; uniform and triangular: closed form).
//! - **Summary**: mean, sample standard deviation, minimum, maximum and the 5th, 50th and 95th
//!   percentiles (linear interpolation between order statistics, `(n - 1) p`).
//! - **Capability** against a lower and/or upper specification limit: the share of draws outside,
//!   `Cp = (USL - LSL) / 6 s` (both limits) and `Cpk = min(USL - mean, mean - LSL) / 3 s` (either).
//! - **Sensitivity**: Spearman's rank correlation of each input with the output.

use statrs::distribution::{ContinuousCDF, LogNormal, Normal};

use crate::correlation::{CorrKind, correlation};

/// The distribution of one input.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum InputDist {
    Normal {
        mean: f64,
        sd: f64,
    },
    Uniform {
        low: f64,
        high: f64,
    },
    /// `ln x` normal with mean `mu` and standard deviation `sigma`.
    Lognormal {
        mu: f64,
        sigma: f64,
    },
    Triangular {
        low: f64,
        mode: f64,
        high: f64,
    },
    Constant(f64),
}

impl InputDist {
    /// How the input is written: `normal(10, 1)`.
    pub fn text(&self) -> String {
        match *self {
            InputDist::Normal { mean, sd } => format!("normal({mean}, {sd})"),
            InputDist::Uniform { low, high } => format!("uniform({low}, {high})"),
            InputDist::Lognormal { mu, sigma } => format!("lognormal({mu}, {sigma})"),
            InputDist::Triangular { low, mode, high } => format!("triangular({low}, {mode}, {high})"),
            InputDist::Constant(v) => format!("constant({v})"),
        }
    }

    fn check(&self) -> Result<(), String> {
        let ok = match *self {
            InputDist::Normal { mean, sd } => mean.is_finite() && sd.is_finite() && sd > 0.0,
            InputDist::Uniform { low, high } => low.is_finite() && high.is_finite() && low < high,
            InputDist::Lognormal { mu, sigma } => mu.is_finite() && sigma.is_finite() && sigma > 0.0,
            InputDist::Triangular { low, mode, high } => {
                low.is_finite() && high.is_finite() && low < high && (low..=high).contains(&mode)
            }
            InputDist::Constant(v) => v.is_finite(),
        };
        if ok {
            Ok(())
        } else {
            Err(format!(
                "{}: the parameters do not make a distribution (a spread above 0, low below high, the mode between)",
                self.text()
            ))
        }
    }

    /// The value at cumulative probability `u` (0 < u < 1).
    fn at(&self, u: f64) -> f64 {
        match *self {
            InputDist::Normal { mean, sd } => Normal::new(mean, sd).map_or(f64::NAN, |d| d.inverse_cdf(u)),
            InputDist::Uniform { low, high } => low + (high - low) * u,
            InputDist::Lognormal { mu, sigma } => LogNormal::new(mu, sigma).map_or(f64::NAN, |d| d.inverse_cdf(u)),
            InputDist::Triangular { low, mode, high } => {
                let f = (mode - low) / (high - low);
                if u < f {
                    low + (u * (high - low) * (mode - low)).sqrt()
                } else {
                    high - ((1.0 - u) * (high - low) * (high - mode)).sqrt()
                }
            }
            InputDist::Constant(v) => v,
        }
    }
}

/// Reads `name = dist(a, b[, c])` (one input; `#` starts a comment). Names are letters, digits and
/// `_`, starting with a letter.
pub fn parse_input(line: &str) -> Result<(String, InputDist), String> {
    let line = line.split('#').next().unwrap_or("").trim();
    let (name, rest) =
        line.split_once('=').ok_or_else(|| format!("write the input '{line}' as name = normal(10, 1)"))?;
    let name = name.trim();
    if name.is_empty()
        || !name.starts_with(|c: char| c.is_ascii_alphabetic())
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!("'{name}' is not a usable input name (letters, digits and _, starting with a letter)"));
    }
    let rest = rest.trim();
    let (kind, args) = rest
        .strip_suffix(')')
        .and_then(|r| r.split_once('('))
        .ok_or_else(|| format!("write the distribution of {name} as normal(10, 1), uniform(a, b), lognormal(mu, sigma), triangular(a, mode, b) or constant(v)"))?;
    let nums: Vec<f64> = args
        .split(',')
        .map(|a| {
            a.trim().parse::<f64>().map_err(|_| format!("'{}' in the distribution of {name} is not a number", a.trim()))
        })
        .collect::<Result<_, _>>()?;
    let need = |k: usize| -> Result<(), String> {
        if nums.len() == k { Ok(()) } else { Err(format!("{}() of {name} takes {k} number(s)", kind.trim())) }
    };
    let d = match kind.trim().to_ascii_lowercase().as_str() {
        "normal" => {
            need(2)?;
            InputDist::Normal { mean: nums[0], sd: nums[1] }
        }
        "uniform" => {
            need(2)?;
            InputDist::Uniform { low: nums[0], high: nums[1] }
        }
        "lognormal" => {
            need(2)?;
            InputDist::Lognormal { mu: nums[0], sigma: nums[1] }
        }
        "triangular" => {
            need(3)?;
            InputDist::Triangular { low: nums[0], mode: nums[1], high: nums[2] }
        }
        "constant" => {
            need(1)?;
            InputDist::Constant(nums[0])
        }
        other => return Err(format!("unknown distribution '{other}' for {name}")),
    };
    d.check()?;
    Ok((name.to_string(), d))
}

/// `n` draws of every input (one vector per input, in order).
pub fn draw(inputs: &[InputDist], n: usize, seed: u64) -> Result<Vec<Vec<f64>>, String> {
    for d in inputs {
        d.check()?;
    }
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    };
    let mut out = vec![Vec::with_capacity(n); inputs.len()];
    for _ in 0..n {
        for (j, d) in inputs.iter().enumerate() {
            out[j].push(d.at(next()));
        }
    }
    Ok(out)
}

/// The summary of a set of draws (numbers only; others are left out).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct McSummary {
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    pub min: f64,
    pub max: f64,
    pub p05: f64,
    pub median: f64,
    pub p95: f64,
    /// Share of draws below the lower or above the upper limit (NaN without limits).
    pub out_of_spec: f64,
    pub cp: f64,
    pub cpk: f64,
}

/// The `p` percentile of ascending `sorted` values, `(n - 1) p` interpolated.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    let h = (n - 1) as f64 * p.clamp(0.0, 1.0);
    let (i, f) = (h.floor() as usize, h - h.floor());
    if i + 1 < n { sorted[i] + f * (sorted[i + 1] - sorted[i]) } else { sorted[n - 1] }
}

/// Summary and capability of `values` against the optional limits.
pub fn summarize(values: &[f64], lsl: Option<f64>, usl: Option<f64>) -> McSummary {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let mean = if n == 0 { f64::NAN } else { v.iter().sum::<f64>() / n as f64 };
    let sd = if n < 2 { f64::NAN } else { (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt() };
    let out_of_spec = if lsl.is_none() && usl.is_none() || n == 0 {
        f64::NAN
    } else {
        v.iter().filter(|&&x| lsl.is_some_and(|l| x < l) || usl.is_some_and(|u| x > u)).count() as f64 / n as f64
    };
    let cp = match (lsl, usl) {
        (Some(l), Some(u)) => (u - l) / (6.0 * sd),
        _ => f64::NAN,
    };
    let cpk = match (lsl, usl) {
        (Some(l), Some(u)) => ((u - mean).min(mean - l)) / (3.0 * sd),
        (Some(l), None) => (mean - l) / (3.0 * sd),
        (None, Some(u)) => (u - mean) / (3.0 * sd),
        (None, None) => f64::NAN,
    };
    McSummary {
        n,
        mean,
        sd,
        min: v.first().copied().unwrap_or(f64::NAN),
        max: v.last().copied().unwrap_or(f64::NAN),
        p05: percentile(&v, 0.05),
        median: percentile(&v, 0.5),
        p95: percentile(&v, 0.95),
        out_of_spec,
        cp,
        cpk,
    }
}

/// Spearman's rank correlation of each input's draws with the output (NaN for a constant input).
pub fn sensitivity(inputs: &[Vec<f64>], output: &[f64]) -> Vec<f64> {
    inputs
        .iter()
        .map(|x| {
            let first = x.first().copied().unwrap_or(f64::NAN);
            if x.iter().all(|v| *v == first) { f64::NAN } else { correlation(x, output, CorrKind::Spearman).r }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_are_read_and_checked() {
        assert_eq!(parse_input("a = normal(10, 1)").unwrap(), ("a".into(), InputDist::Normal { mean: 10.0, sd: 1.0 }));
        assert_eq!(
            parse_input(" w_2=Triangular(1, 2, 4) # width").unwrap(),
            ("w_2".into(), InputDist::Triangular { low: 1.0, mode: 2.0, high: 4.0 })
        );
        assert!(parse_input("a normal(1, 2)").unwrap_err().contains("name = normal"));
        assert!(parse_input("1a = normal(1, 2)").unwrap_err().contains("not a usable input name"));
        assert!(parse_input("a = normal(1)").unwrap_err().contains("takes 2"));
        assert!(parse_input("a = normal(1, -2)").unwrap_err().contains("do not make a distribution"));
        assert!(parse_input("a = cauchy(0, 1)").unwrap_err().contains("unknown distribution"));
        assert!(parse_input("a = uniform(1, x)").unwrap_err().contains("not a number"));
    }

    #[test]
    fn draws_follow_their_distributions_and_repeat_with_the_seed() {
        let ins = [
            InputDist::Normal { mean: 10.0, sd: 2.0 },
            InputDist::Uniform { low: 0.0, high: 4.0 },
            InputDist::Triangular { low: 0.0, mode: 1.0, high: 4.0 },
            InputDist::Lognormal { mu: 0.0, sigma: 0.5 },
            InputDist::Constant(3.0),
        ];
        let d = draw(&ins, 20_000, 7).unwrap();
        let s: Vec<McSummary> = d.iter().map(|v| summarize(v, None, None)).collect();
        assert!((s[0].mean - 10.0).abs() < 0.05 && (s[0].sd - 2.0).abs() < 0.05, "{:?}", s[0]);
        assert!((s[1].mean - 2.0).abs() < 0.03 && s[1].min >= 0.0 && s[1].max <= 4.0);
        assert!((s[2].mean - 5.0 / 3.0).abs() < 0.02, "{:?}", s[2]);
        assert!((s[3].median - 1.0).abs() < 0.03, "lognormal median e^mu: {:?}", s[3]);
        assert_eq!((s[4].min, s[4].max), (3.0, 3.0));
        assert_eq!(draw(&ins, 50, 7).unwrap(), draw(&ins, 50, 7).unwrap());
        assert_ne!(draw(&ins, 50, 7).unwrap()[0], draw(&ins, 50, 8).unwrap()[0]);
    }

    #[test]
    fn capability_percentiles_and_sensitivity() {
        let v: Vec<f64> = (0..=100).map(f64::from).collect();
        let s = summarize(&v, Some(10.0), Some(90.0));
        assert_eq!((s.p05, s.median, s.p95), (5.0, 50.0, 95.0));
        assert!((s.out_of_spec - 20.0 / 101.0).abs() < 1e-12, "{}", s.out_of_spec);
        let sd = s.sd;
        assert!((s.cp - 80.0 / (6.0 * sd)).abs() < 1e-12 && (s.cpk - 40.0 / (3.0 * sd)).abs() < 1e-12);
        assert!(summarize(&v, None, None).out_of_spec.is_nan());
        let ins = draw(&[InputDist::Normal { mean: 0.0, sd: 1.0 }, InputDist::Normal { mean: 0.0, sd: 1.0 }], 2000, 3)
            .unwrap();
        let out: Vec<f64> = (0..2000).map(|i| 5.0 * ins[0][i] + 0.2 * ins[1][i]).collect();
        let r = sensitivity(&ins, &out);
        assert!(r[0] > 0.95 && r[1].abs() < 0.2, "{r:?}");
    }
}
