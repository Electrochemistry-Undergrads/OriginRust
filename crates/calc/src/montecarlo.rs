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
//! - **Correlated inputs** (T496): a Gaussian copula. The uniforms of a row turn into normal scores `z = Phi^-1(u)`,
//!   are mixed by the Cholesky factor `L` of the correlation matrix given (`z' = L z`; pairs not given are 0) and turn
//!   back into uniforms `Phi(z')` before each input's inverse distribution function, so every input keeps its own
//!   distribution and the normal scores correlate as given (Spearman's rank correlation of the draws is about
//!   `6 / pi asin(r / 2)`). An input correlated with no other keeps exactly its independent draws; a matrix that is not
//!   positive definite is refused.

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

/// `n` draws of every input with the normal scores of input pairs `(i, j)` correlated by `r` (T496; see the module
/// notes). Without correlations it is [`draw`].
pub fn draw_correlated(
    inputs: &[InputDist],
    n: usize,
    seed: u64,
    correlations: &[(usize, usize, f64)],
) -> Result<Vec<Vec<f64>>, String> {
    if correlations.is_empty() {
        return draw(inputs, n, seed);
    }
    let k = inputs.len();
    let mut c = nalgebra::DMatrix::<f64>::identity(k, k);
    for &(i, j, r) in correlations {
        if i >= k || j >= k || i == j {
            return Err("a correlation joins two different inputs".into());
        }
        if !(r.is_finite() && r > -1.0 && r < 1.0) {
            return Err(format!("a correlation must lie strictly between -1 and 1 (got {r})"));
        }
        c[(i, j)] = r;
        c[(j, i)] = r;
    }
    let l = c
        .clone()
        .cholesky()
        .ok_or("the correlations cannot all hold together (the correlation matrix is not positive definite)")?
        .l();
    let normal = Normal::new(0.0, 1.0).expect("the standard normal");
    // The independent draws of a uniform input are its uniforms: recover them as `draw` makes them.
    let uniforms = draw(&vec![InputDist::Uniform { low: 0.0, high: 1.0 }; k], n, seed)?;
    let mut out = vec![Vec::with_capacity(n); k];
    let mut z = nalgebra::DVector::<f64>::zeros(k);
    for row in 0..n {
        for (zj, u) in z.iter_mut().zip(&uniforms) {
            *zj = normal.inverse_cdf(u[row]);
        }
        let mixed = &l * &z;
        for (j, d) in inputs.iter().enumerate() {
            // An input with no correlation keeps its own uniform exactly.
            let u = if (0..k).all(|i| i == j || c[(i, j)] == 0.0) { uniforms[j][row] } else { normal.cdf(mixed[j]) };
            out[j].push(d.at(u.clamp(f64::MIN_POSITIVE, 1.0 - f64::EPSILON)));
        }
    }
    Ok(out)
}

/// Reads correlations written `a, b = r` (input names), separated by `;` or new lines (T496), as index pairs of
/// `names`.
pub fn parse_correlations(text: &str, names: &[&str]) -> Result<Vec<(usize, usize, f64)>, String> {
    let mut out: Vec<(usize, usize, f64)> = Vec::new();
    for part in text.split([';', '\n']).map(str::trim).filter(|p| !p.is_empty()) {
        let (pair, r) = part.split_once('=').ok_or_else(|| format!("write the correlation '{part}' as a, b = 0.5"))?;
        let (a, b) = pair.split_once(',').ok_or_else(|| format!("write the pair '{}' as a, b", pair.trim()))?;
        let index = |n: &str| {
            names.iter().position(|m| *m == n.trim()).ok_or_else(|| format!("{} is not one of the inputs", n.trim()))
        };
        let (i, j) = (index(a)?, index(b)?);
        if i == j {
            return Err(format!("{} cannot be correlated with itself", a.trim()));
        }
        let r: f64 = r.trim().parse().map_err(|_| format!("'{}' is not a number", r.trim()))?;
        if !(r > -1.0 && r < 1.0) {
            return Err(format!("the correlation of {} and {} must lie strictly between -1 and 1", a.trim(), b.trim()));
        }
        if out.iter().any(|&(x, y, _)| (x, y) == (i, j) || (x, y) == (j, i)) {
            return Err(format!("{} and {} are correlated twice", a.trim(), b.trim()));
        }
        out.push((i, j, r));
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

impl InputDist {
    /// Where the input sits (T490): the mean (normal), the middle (uniform), the mode (triangular), the median
    /// `e^mu` (lognormal) or the value (constant).
    pub fn centre(&self) -> f64 {
        match *self {
            InputDist::Normal { mean, .. } => mean,
            InputDist::Uniform { low, high } => (low + high) / 2.0,
            InputDist::Lognormal { mu, .. } => mu.exp(),
            InputDist::Triangular { mode, .. } => mode,
            InputDist::Constant(v) => v,
        }
    }

    /// The same distribution moved to centre `c` (T490): shifted as a whole, a lognormal scaled (`c` above 0).
    pub fn centred_at(&self, c: f64) -> InputDist {
        let d = c - self.centre();
        match *self {
            InputDist::Normal { sd, .. } => InputDist::Normal { mean: c, sd },
            InputDist::Uniform { low, high } => InputDist::Uniform { low: low + d, high: high + d },
            InputDist::Lognormal { sigma, .. } => InputDist::Lognormal { mu: c.ln(), sigma },
            InputDist::Triangular { low, mode, high } => {
                InputDist::Triangular { low: low + d, mode: mode + d, high: high + d }
            }
            InputDist::Constant(_) => InputDist::Constant(c),
        }
    }

    /// `draws` of this input as they fall when it is moved to centre `c` with the same random numbers (T490).
    pub fn move_draws(&self, draws: &[f64], c: f64) -> Vec<f64> {
        match *self {
            InputDist::Lognormal { mu, .. } => draws.iter().map(|x| x * c / mu.exp()).collect(),
            InputDist::Constant(_) => vec![c; draws.len()],
            _ => {
                let d = c - self.centre();
                draws.iter().map(|x| x + d).collect()
            }
        }
    }
}

/// What the optimisation of controllable inputs looks for (T490).
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum McGoal {
    /// The highest Cpk against the specification limits.
    #[default]
    Capability,
    /// The output's mean at this target with the least spread: the smallest mean squared deviation
    /// `(mean - target)^2 + s^2`.
    Target(f64),
}

/// One input whose centre the optimisation moves, between `low` and `high` (T490).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McControl {
    pub name: String,
    pub low: f64,
    pub high: f64,
}

impl McControl {
    /// How the control is written: `len = 9:11`.
    pub fn text(&self) -> String {
        format!("{} = {}:{}", self.name, self.low, self.high)
    }
}

/// Reads controls written `name = low:high`, separated by `,`, `;` or new lines (T490).
pub fn parse_controls(text: &str) -> Result<Vec<McControl>, String> {
    let mut out = Vec::new();
    for part in text.split([',', ';', '\n']).map(str::trim).filter(|p| !p.is_empty()) {
        let (name, range) =
            part.split_once('=').ok_or_else(|| format!("write the control '{part}' as name = low:high"))?;
        let (lo, hi) =
            range.split_once(':').ok_or_else(|| format!("write the range of '{}' as low:high", name.trim()))?;
        let num = |t: &str| t.trim().parse::<f64>().map_err(|_| format!("'{}' is not a number", t.trim()));
        let (low, high) = (num(lo)?, num(hi)?);
        if !(low.is_finite() && high.is_finite() && low < high) {
            return Err(format!("the range of {} must run from a lower to a higher number", name.trim()));
        }
        out.push(McControl { name: name.trim().to_string(), low, high });
    }
    Ok(out)
}

/// How far `output` is from the goal (T490): lower is better; `-Cpk` for [`McGoal::Capability`], the mean
/// squared deviation for [`McGoal::Target`]; infinite when it cannot be told (no spread, no numbers).
pub fn goal_score(goal: McGoal, output: &[f64], lsl: Option<f64>, usl: Option<f64>) -> f64 {
    let s = summarize(output, lsl, usl);
    let v = match goal {
        McGoal::Capability => -s.cpk,
        McGoal::Target(t) => (s.mean - t).powi(2) + if s.n < 2 { 0.0 } else { s.sd.powi(2) },
    };
    if v.is_finite() { v } else { f64::INFINITY }
}

/// The lowest `f` over the box `lo..hi` (T490): a grid (9 points a side for one or two variables, 5 for three,
/// 3 for more), then a compass search from its best point that halves its steps until they are 1e-5 of each
/// range. Deterministic; ties keep the earlier point.
pub fn search_box(f: &mut dyn FnMut(&[f64]) -> f64, lo: &[f64], hi: &[f64]) -> (Vec<f64>, f64) {
    let d = lo.len();
    let k: usize = match d {
        0 => return (Vec::new(), f(&[])),
        1 | 2 => 9,
        3 => 5,
        _ => 3,
    };
    let mut best = (lo.to_vec(), f64::INFINITY);
    let mut index = vec![0usize; d];
    loop {
        let x: Vec<f64> = (0..d).map(|j| lo[j] + (hi[j] - lo[j]) * index[j] as f64 / (k - 1) as f64).collect();
        let v = f(&x);
        if v < best.1 {
            best = (x, v);
        }
        let Some(j) = (0..d).find(|&j| index[j] + 1 < k) else { break };
        index[j] += 1;
        index[..j].fill(0);
    }
    let mut step: Vec<f64> = (0..d).map(|j| (hi[j] - lo[j]) / (2 * (k - 1)) as f64).collect();
    let mut rounds = 0;
    while rounds < 10_000 && (0..d).any(|j| step[j] > 1e-5 * (hi[j] - lo[j])) {
        rounds += 1;
        let mut moved = false;
        for j in 0..d {
            for sign in [1.0, -1.0] {
                let mut x = best.0.clone();
                x[j] = (x[j] + sign * step[j]).clamp(lo[j], hi[j]);
                if x[j] == best.0[j] {
                    continue;
                }
                let v = f(&x);
                if v < best.1 {
                    best = (x, v);
                    moved = true;
                }
            }
        }
        if !moved {
            step.iter_mut().for_each(|s| *s /= 2.0);
        }
    }
    best
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

    #[test]
    fn correlated_inputs_keep_their_distributions_and_correlate() {
        let ins = [
            InputDist::Normal { mean: 10.0, sd: 2.0 },
            InputDist::Uniform { low: 0.0, high: 4.0 },
            InputDist::Lognormal { mu: 0.0, sigma: 0.5 },
        ];
        let n = 20_000;
        let d = draw_correlated(&ins, n, 5, &[(0, 1, 0.8)]).unwrap();
        // Each input keeps its distribution.
        let s: Vec<McSummary> = d.iter().map(|v| summarize(v, None, None)).collect();
        assert!((s[0].mean - 10.0).abs() < 0.05 && (s[0].sd - 2.0).abs() < 0.05, "{:?}", s[0]);
        assert!(s[1].min >= 0.0 && s[1].max <= 4.0 && (s[1].mean - 2.0).abs() < 0.03, "{:?}", s[1]);
        // The normal scores correlate by 0.8 and the ranks by 6/pi asin(0.4) = 0.786, within sampling error (these
        // independent uniforms happen to correlate by 0.012 already); the third input is untouched.
        let nd = Normal::new(0.0, 1.0).unwrap();
        let z1: Vec<f64> = d[1].iter().map(|v| nd.inverse_cdf(v / 4.0)).collect();
        let r = correlation(&d[0], &z1, CorrKind::Pearson).r;
        assert!((r - 0.8).abs() < 0.015, "{r}");
        let rho = correlation(&d[0], &d[1], CorrKind::Spearman).r;
        assert!((rho - 6.0 / std::f64::consts::PI * (0.4f64).asin()).abs() < 0.02, "{rho}");
        assert_eq!(d[2], draw(&ins, n, 5).unwrap()[2], "an input correlated with no other keeps its draws");
        assert!(correlation(&d[0], &d[2], CorrKind::Spearman).r.abs() < 0.03);
        // Without correlations it is the plain draw; impossible matrices are refused.
        assert_eq!(draw_correlated(&ins, 50, 5, &[]).unwrap(), draw(&ins, 50, 5).unwrap());
        let bad = [(0, 1, 0.9), (1, 2, 0.9), (0, 2, -0.9)];
        assert!(draw_correlated(&ins, 50, 5, &bad).unwrap_err().contains("positive definite"));
        assert_eq!(parse_correlations("a, b = 0.8; c,a=-0.3", &["a", "b", "c"]).unwrap(), [(0, 1, 0.8), (2, 0, -0.3)]);
        assert!(parse_correlations("a, d = 0.5", &["a", "b"]).unwrap_err().contains("d is not one of the inputs"));
        assert!(parse_correlations("a, b = 1", &["a", "b"]).unwrap_err().contains("strictly between"));
        assert!(parse_correlations("a, b = 0.1; b, a = 0.2", &["a", "b"]).unwrap_err().contains("twice"));
        assert!(parse_correlations("a b = 0.1", &["a", "b"]).unwrap_err().contains("as a, b"));
    }

    #[test]
    fn inputs_move_to_a_centre_with_the_same_draws() {
        let ins = [
            InputDist::Normal { mean: 10.0, sd: 2.0 },
            InputDist::Uniform { low: 0.0, high: 4.0 },
            InputDist::Triangular { low: 0.0, mode: 1.0, high: 4.0 },
            InputDist::Lognormal { mu: 0.0, sigma: 0.5 },
            InputDist::Constant(3.0),
        ];
        let d = draw(&ins, 500, 9).unwrap();
        for (k, dist) in ins.iter().enumerate() {
            let c = dist.centre() * 1.5 + 0.5;
            let moved = dist.centred_at(c);
            assert!((moved.centre() - c).abs() < 1e-12, "{moved:?}");
            // Moving the draws is drawing the moved distribution with the same numbers.
            let again =
                draw(&[ins[0], ins[1], ins[2], ins[3], ins[4]].map(|x| if x == *dist { moved } else { x }), 500, 9)
                    .unwrap();
            let shifted = dist.move_draws(&d[k], c);
            assert!(shifted.iter().zip(&again[k]).all(|(a, b)| (a - b).abs() < 1e-9), "{dist:?}");
        }
    }

    #[test]
    fn controls_are_read_and_the_box_search_finds_the_lowest_point() {
        assert_eq!(
            parse_controls("len = 9:11; w=1.5 : 2.5").unwrap(),
            vec![
                McControl { name: "len".into(), low: 9.0, high: 11.0 },
                McControl { name: "w".into(), low: 1.5, high: 2.5 }
            ]
        );
        assert!(parse_controls("len 9:11").unwrap_err().contains("name = low:high"));
        assert!(parse_controls("len = 9").unwrap_err().contains("low:high"));
        assert!(parse_controls("len = 11:9").unwrap_err().contains("lower to a higher"));
        assert!(parse_controls("").unwrap().is_empty());
        // A bowl at (0.3, -1.2) inside the box, and a slope whose lowest point is a corner.
        let mut calls = 0;
        let (x, v) = search_box(
            &mut |x| {
                calls += 1;
                (x[0] - 0.3).powi(2) + 2.0 * (x[1] + 1.2).powi(2)
            },
            &[-2.0, -2.0],
            &[2.0, 2.0],
        );
        assert!((x[0] - 0.3).abs() < 1e-4 && (x[1] + 1.2).abs() < 1e-4 && v < 1e-7, "{x:?} {v}");
        assert!(calls < 1000, "{calls}");
        let (x, _) = search_box(&mut |x| x[0] - x[1] + x[2], &[0.0, 0.0, 0.0], &[1.0, 1.0, 1.0]);
        assert_eq!(x, [0.0, 1.0, 0.0]);
        // Goal scores: the target goal is the mean squared deviation; Cpk turns into its negative.
        let v = [9.0, 10.0, 11.0];
        assert!((goal_score(McGoal::Target(10.0), &v, None, None) - 1.0).abs() < 1e-12);
        assert!((goal_score(McGoal::Target(12.0), &v, None, None) - 5.0).abs() < 1e-12);
        assert!((goal_score(McGoal::Capability, &v, Some(7.0), Some(13.0)) + 1.0).abs() < 1e-12);
        assert_eq!(goal_score(McGoal::Capability, &[5.0, 5.0], Some(0.0), None), f64::INFINITY, "no spread");
    }
}
