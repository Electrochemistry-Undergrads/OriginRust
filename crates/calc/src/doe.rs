//! Two-level factorial designs and their effects (the base of Origin 2027's two-level split-plot designs; Origin
//! 2016 has no design of experiments, so nothing here is measured in Origin; `docs/spec/statistics.md` section 45).
//!
//! **Designs**: `k` factors at levels -1 and +1. A full design has `2^k` runs; a `2^(k-p)` fraction runs the first
//! `k - p` (base) factors as a full design and sets each added factor to the product of its generator's base
//! factors (Montgomery's standard minimum-aberration generators, below). Runs are in standard (Yates) order, factor A
//! alternating fastest; replicates repeat the whole design, centre points (all 0) follow, and a seeded shuffle gives the
//! run order. The resolution is the length of the shortest word in the defining relation.
//!
//! **Effects**: the factors are coded to -1..+1 from their smallest and largest values; the model has an intercept,
//! the main effects and the two-factor interactions, a term being dropped when its column is a multiple of an earlier
//! one (aliased). Least squares gives the coefficients; an effect is twice its coefficient. With residual degrees of
//! freedom the effects get standard errors and t tests; without, Lenth's pseudo standard error `PSE = 1.5 median{|c|
//! : |c| < 2.5 s0}` (`s0 = 1.5 median |c|`) of the effects and the margin of error `t(0.975, m/3) PSE` mark the active
//! ones. Checked against numpy in `crates/calc/tests/doe_reference.rs` (made by `tools/doe-ref.py`).
//!
//! **Split-plot designs** (T382): the first `w` factors are hard to change. Each whole plot holds one combination of
//! them (a full `2^w` design, replicated `r` times) and every combination of the `s` subplot factors. Randomizing
//! shuffles the whole plots, then the subplots within each, with the same generator. The effects are those of the
//! full model above; terms of whole-plot factors only belong to the whole-plot stratum, the others to the subplot
//! stratum. The whole-plot error is `b` times the residual sum of squares of the whole-plot means on the whole-plot
//! terms (`b` subplots per plot, `W - 1 - t_w` degrees of freedom); the subplot error is the rest of the full model's
//! residual. Each effect's standard error is `sqrt(4 MS / N)` with its stratum's mean square (balanced, orthogonal
//! designs), and each stratum without degrees of freedom falls back to Lenth's margin over its own effects.

use crate::stability::invert;
use statrs::distribution::{ContinuousCDF, StudentsT};

/// The generators of `2^(k-p)` fractions: for each added factor, the base factors it multiplies (0 = A).
pub fn generators(k: usize, p: usize) -> Option<Vec<Vec<usize>>> {
    let g: &[&[usize]] = match (k, p) {
        (_, 0) => &[],
        (3, 1) => &[&[0, 1]],
        (4, 1) => &[&[0, 1, 2]],
        (5, 1) => &[&[0, 1, 2, 3]],
        (5, 2) => &[&[0, 1], &[0, 2]],
        (6, 1) => &[&[0, 1, 2, 3, 4]],
        (6, 2) => &[&[0, 1, 2], &[1, 2, 3]],
        (6, 3) => &[&[0, 1], &[0, 2], &[1, 2]],
        (7, 1) => &[&[0, 1, 2, 3, 4, 5]],
        (7, 2) => &[&[0, 1, 2, 3], &[0, 1, 3, 4]],
        (7, 3) => &[&[0, 1, 2], &[1, 2, 3], &[0, 2, 3]],
        (7, 4) => &[&[0, 1], &[0, 2], &[1, 2], &[0, 1, 2]],
        (8, 4) => &[&[1, 2, 3], &[0, 2, 3], &[0, 1, 2], &[0, 1, 3]],
        _ => return None,
    };
    if p == 0 && !(2..=10).contains(&k) {
        return None;
    }
    Some(g.iter().map(|v| v.to_vec()).collect())
}

/// The letter of factor `i` (A, B, ...).
pub fn letter(i: usize) -> char {
    char::from(b'A' + i as u8)
}

/// A two-level design.
#[derive(Debug, Clone, PartialEq)]
pub struct Design {
    pub k: usize,
    pub p: usize,
    /// Coded levels of each run (rows in run order).
    pub runs: Vec<Vec<f64>>,
    /// The standard-order number of each run (1-based; centre points after the factorial runs).
    pub standard: Vec<usize>,
    /// The words of the generators, e.g. "D = ABC".
    pub generator_text: Vec<String>,
    /// `None` for a full design.
    pub resolution: Option<usize>,
}

/// Why a design or an analysis could not be made.
#[derive(Debug, Clone, PartialEq)]
pub enum DoeError {
    NoDesign { k: usize, p: usize },
    BadReplicates,
    Ragged,
    Missing(usize),
    NotTwoLevel(usize),
    TooFewRuns,
    BadSplit,
    Unbalanced,
}

impl std::fmt::Display for DoeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoeError::NoDesign { k, p } => write!(
                f,
                "no 2^({k}-{p}) design: full designs take 2 to 10 factors; fractions are built in for 3 to 8 factors"
            ),
            DoeError::BadReplicates => f.write_str("give at least one replicate"),
            DoeError::Ragged => f.write_str("the factor columns and the response differ in length"),
            DoeError::Missing(r) => write!(f, "row {} has a missing value", r + 1),
            DoeError::NotTwoLevel(i) => write!(f, "factor {} does not vary", letter(*i)),
            DoeError::TooFewRuns => f.write_str("too few runs for the model"),
            DoeError::BadSplit => f.write_str("give at least one whole-plot and one subplot factor"),
            DoeError::Unbalanced => f.write_str(
                "every whole plot must hold the same number of runs and one level of each whole-plot factor",
            ),
        }
    }
}

impl std::error::Error for DoeError {}

/// Makes the `2^(k-p)` design with `replicates`, `center` centre points, shuffled by `seed` when `randomize`.
pub fn design(
    k: usize,
    p: usize,
    replicates: usize,
    center: usize,
    randomize: bool,
    seed: u64,
) -> Result<Design, DoeError> {
    let gens = generators(k, p).ok_or(DoeError::NoDesign { k, p })?;
    if replicates == 0 {
        return Err(DoeError::BadReplicates);
    }
    let base = k - p;
    let mut rows: Vec<(usize, Vec<f64>)> = Vec::new();
    for _ in 0..replicates {
        for i in 0..1usize << base {
            let mut r: Vec<f64> = (0..base).map(|j| if (i >> j) & 1 == 1 { 1.0 } else { -1.0 }).collect();
            for g in &gens {
                r.push(g.iter().map(|&j| r[j]).product());
            }
            rows.push((i + 1, r));
        }
    }
    let full = 1usize << base;
    for c in 0..center {
        rows.push((full + c + 1, vec![0.0; k]));
    }
    if randomize {
        let mut s = seed;
        for i in (1..rows.len()).rev() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u = (s >> 11) as f64 / (1u64 << 53) as f64;
            let j = ((u * (i + 1) as f64) as usize).min(i);
            rows.swap(i, j);
        }
    }
    // The defining relation: every product of the generator words (each with its added factor).
    let words: Vec<u32> =
        gens.iter().enumerate().map(|(i, g)| g.iter().fold(1u32 << (base + i), |w, &j| w ^ (1 << j))).collect();
    let resolution = (p > 0).then(|| {
        (1u32..1 << p)
            .map(|mask| (0..p).filter(|i| mask >> i & 1 == 1).fold(0u32, |w, i| w ^ words[i]).count_ones() as usize)
            .min()
            .unwrap_or(0)
    });
    let generator_text = gens
        .iter()
        .enumerate()
        .map(|(i, g)| format!("{} = {}", letter(base + i), g.iter().map(|&j| letter(j)).collect::<String>()))
        .collect();
    Ok(Design {
        k,
        p,
        standard: rows.iter().map(|r| r.0).collect(),
        runs: rows.into_iter().map(|r| r.1).collect(),
        generator_text,
        resolution,
    })
}

/// One model term.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    /// "A", "AB", ...
    pub term: String,
    pub coefficient: f64,
    pub effect: f64,
    /// NaN without residual degrees of freedom.
    pub se: f64,
    pub t: f64,
    pub p: f64,
}

/// The effects of a two-level experiment.
#[derive(Debug, Clone, PartialEq)]
pub struct Effects {
    pub intercept: f64,
    pub effects: Vec<Effect>,
    /// Terms left out as aliases: (term, the earlier term it repeats).
    pub aliased: Vec<(String, String)>,
    pub df: usize,
    /// Residual mean square (NaN without degrees of freedom).
    pub mse: f64,
    /// Lenth's pseudo standard error and margin of error (always computed).
    pub pse: f64,
    pub margin: f64,
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// Estimates the effects of the factors `x` (rows of actual levels) on `y`.
pub fn effects(x: &[Vec<f64>], y: &[f64]) -> Result<Effects, DoeError> {
    if x.len() != y.len() || x.is_empty() || x.iter().any(|r| r.len() != x[0].len()) {
        return Err(DoeError::Ragged);
    }
    if let Some(r) = (0..y.len()).find(|&r| !y[r].is_finite() || x[r].iter().any(|v| !v.is_finite())) {
        return Err(DoeError::Missing(r));
    }
    let k = x[0].len();
    let n = x.len();
    // Coding to -1..+1.
    let mut coded = vec![vec![0.0; k]; n];
    for j in 0..k {
        let (lo, hi) = x.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), r| (a.min(r[j]), b.max(r[j])));
        if hi <= lo {
            return Err(DoeError::NotTwoLevel(j));
        }
        let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
        for r in 0..n {
            coded[r][j] = (x[r][j] - mid) / half;
        }
    }
    // Candidate terms: mains, then pairs.
    let mut candidates: Vec<(String, Vec<f64>)> =
        (0..k).map(|j| (letter(j).to_string(), coded.iter().map(|r| r[j]).collect())).collect();
    for a in 0..k {
        for b in a + 1..k {
            candidates.push((format!("{}{}", letter(a), letter(b)), coded.iter().map(|r| r[a] * r[b]).collect()));
        }
    }
    let mut terms: Vec<(String, Vec<f64>)> = Vec::new();
    let mut aliased = Vec::new();
    'next: for (name, col) in candidates {
        for (other, prev) in &terms {
            let dot: f64 = col.iter().zip(prev).map(|(a, b)| a * b).sum();
            let nn: f64 =
                col.iter().map(|a| a * a).sum::<f64>().sqrt() * prev.iter().map(|a| a * a).sum::<f64>().sqrt();
            if nn > 0.0 && (dot.abs() - nn).abs() <= 1e-9 * nn {
                aliased.push((name, other.clone()));
                continue 'next;
            }
        }
        // A zero column, or no run left to estimate it (the model is saturated), leaves the term out.
        if col.iter().all(|v| v.abs() < 1e-12) || terms.len() + 1 >= n {
            continue;
        }
        terms.push((name, col));
    }
    let m = terms.len() + 1;
    if n < m {
        return Err(DoeError::TooFewRuns);
    }
    let row = |r: usize| -> Vec<f64> { std::iter::once(1.0).chain(terms.iter().map(|t| t.1[r])).collect() };
    let mut xtx = vec![vec![0.0; m]; m];
    let mut xty = vec![0.0; m];
    for (r, yr) in y.iter().enumerate() {
        let xr = row(r);
        for i in 0..m {
            xty[i] += xr[i] * yr;
            for j in 0..m {
                xtx[i][j] += xr[i] * xr[j];
            }
        }
    }
    let inv = invert(&xtx).ok_or(DoeError::TooFewRuns)?;
    let beta: Vec<f64> = (0..m).map(|i| (0..m).map(|j| inv[i][j] * xty[j]).sum()).collect();
    let sse: f64 = (0..n).map(|r| (y[r] - row(r).iter().zip(&beta).map(|(a, b)| a * b).sum::<f64>()).powi(2)).sum();
    let df = n - m;
    let mse = if df > 0 { sse / df as f64 } else { f64::NAN };
    let tdist = (df > 0).then(|| StudentsT::new(0.0, 1.0, df as f64).ok()).flatten();
    let effects: Vec<Effect> = terms
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let c = beta[i + 1];
            let se = if df > 0 { (mse * inv[i + 1][i + 1]).sqrt() } else { f64::NAN };
            let t = c / se;
            // An exact fit has no error: t is infinite (p = 0) or 0/0 (no p-value).
            let p = match &tdist {
                Some(d) if t.is_finite() => 2.0 * d.sf(t.abs()),
                Some(_) if t.is_infinite() => 0.0,
                _ => f64::NAN,
            };
            Effect { term: name.clone(), coefficient: c, effect: 2.0 * c, se: 2.0 * se, t, p }
        })
        .collect();
    let mut abs: Vec<f64> = effects.iter().map(|e| e.effect.abs()).collect();
    let s0 = 1.5 * median(&mut abs.clone());
    abs.retain(|v| *v < 2.5 * s0);
    let pse = 1.5 * median(&mut abs);
    let dfl = effects.len() as f64 / 3.0;
    let margin = StudentsT::new(0.0, 1.0, dfl).map_or(f64::NAN, |d| d.inverse_cdf(0.975)) * pse;
    Ok(Effects { intercept: beta[0], effects, aliased, df, mse, pse, margin })
}

/// A split-plot design: rows in run order.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPlotDesign {
    /// The whole plot of each run (1-based, in run order of the plots).
    pub plot: Vec<usize>,
    pub runs: Vec<Vec<f64>>,
}

/// Makes a split-plot design with `wp` whole-plot and `sp` subplot factors.
pub fn split_plot_design(
    wp: usize,
    sp: usize,
    replicates: usize,
    randomize: bool,
    seed: u64,
) -> Result<SplitPlotDesign, DoeError> {
    if wp == 0 || sp == 0 || wp + sp > 8 {
        return Err(DoeError::NoDesign { k: wp + sp, p: 0 });
    }
    if replicates == 0 {
        return Err(DoeError::BadReplicates);
    }
    let level = |i: usize, j: usize| if (i >> j) & 1 == 1 { 1.0 } else { -1.0 };
    let mut plots: Vec<Vec<f64>> = Vec::new();
    for _ in 0..replicates {
        for i in 0..1usize << wp {
            plots.push((0..wp).map(|j| level(i, j)).collect());
        }
    }
    let mut s = seed;
    let mut draw = |n: usize| -> usize {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u = (s >> 11) as f64 / (1u64 << 53) as f64;
        ((u * n as f64) as usize).min(n - 1)
    };
    if randomize {
        for i in (1..plots.len()).rev() {
            let j = draw(i + 1);
            plots.swap(i, j);
        }
    }
    let mut plot = Vec::new();
    let mut runs = Vec::new();
    for (w, levels) in plots.iter().enumerate() {
        let mut subs: Vec<Vec<f64>> = (0..1usize << sp).map(|i| (0..sp).map(|j| level(i, j)).collect()).collect();
        if randomize {
            for i in (1..subs.len()).rev() {
                let j = draw(i + 1);
                subs.swap(i, j);
            }
        }
        for sub in subs {
            plot.push(w + 1);
            runs.push(levels.iter().chain(&sub).copied().collect());
        }
    }
    Ok(SplitPlotDesign { plot, runs })
}

/// One stratum's error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stratum {
    pub ss: f64,
    pub df: usize,
    /// Mean square (NaN without degrees of freedom).
    pub ms: f64,
    /// Lenth's margin of error over this stratum's effects.
    pub margin: f64,
}

/// The effects of a split-plot experiment: `whole[i]` says whether `effects[i]` is a whole-plot term.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPlotEffects {
    pub fit: Effects,
    pub whole: Vec<bool>,
    pub whole_plot: Stratum,
    pub subplot: Stratum,
}

fn lenth_margin(effects: &[f64]) -> f64 {
    let mut abs: Vec<f64> = effects.iter().map(|e| e.abs()).collect();
    if abs.is_empty() {
        return f64::NAN;
    }
    let s0 = 1.5 * median(&mut abs.clone());
    abs.retain(|v| *v < 2.5 * s0);
    let pse = 1.5 * median(&mut abs);
    StudentsT::new(0.0, 1.0, effects.len() as f64 / 3.0).map_or(f64::NAN, |d| d.inverse_cdf(0.975)) * pse
}

/// Two-stratum effects of a split-plot experiment; `plot` is the whole plot of each row and the first `wp` columns
/// of `x` are the whole-plot factors.
pub fn split_plot_effects(x: &[Vec<f64>], y: &[f64], plot: &[usize], wp: usize) -> Result<SplitPlotEffects, DoeError> {
    if plot.len() != y.len() {
        return Err(DoeError::Ragged);
    }
    let mut fit = effects(x, y)?;
    let k = x[0].len();
    if wp == 0 || wp >= k {
        return Err(DoeError::BadSplit);
    }
    // Balance: every plot the same size, whole-plot factors constant within a plot.
    let mut ids: Vec<usize> = plot.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let rows_of = |id: usize| (0..y.len()).filter(move |&r| plot[r] == id);
    let b = rows_of(ids[0]).count();
    for &id in &ids {
        let rows: Vec<usize> = rows_of(id).collect();
        if rows.len() != b {
            return Err(DoeError::Unbalanced);
        }
        if rows.iter().any(|&r| (0..wp).any(|j| x[r][j] != x[rows[0]][j])) {
            return Err(DoeError::Unbalanced);
        }
    }
    let wcount = ids.len();
    let whole: Vec<bool> = fit.effects.iter().map(|e| e.term.bytes().all(|c| ((c - b'A') as usize) < wp)).collect();
    // Whole-plot means regressed on the whole-plot terms (coded as in the full model).
    let coded = |r: usize, j: usize| {
        let (lo, hi) = x.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, c), row| (a.min(row[j]), c.max(row[j])));
        (x[r][j] - 0.5 * (lo + hi)) / (0.5 * (hi - lo))
    };
    let wp_terms: Vec<&str> =
        fit.effects.iter().zip(&whole).filter(|(_, w)| **w).map(|(e, _)| e.term.as_str()).collect();
    let mut means = Vec::with_capacity(wcount);
    let mut design = Vec::with_capacity(wcount);
    for &id in &ids {
        let rows: Vec<usize> = rows_of(id).collect();
        means.push(rows.iter().map(|&r| y[r]).sum::<f64>() / b as f64);
        let r0 = rows[0];
        let mut row = vec![1.0];
        for t in &wp_terms {
            row.push(t.bytes().map(|c| coded(r0, (c - b'A') as usize)).product());
        }
        design.push(row);
    }
    let m = wp_terms.len() + 1;
    let mut xtx = vec![vec![0.0; m]; m];
    let mut xty = vec![0.0; m];
    for (row, mean) in design.iter().zip(&means) {
        for i in 0..m {
            xty[i] += row[i] * mean;
            for j in 0..m {
                xtx[i][j] += row[i] * row[j];
            }
        }
    }
    let inv = invert(&xtx).ok_or(DoeError::TooFewRuns)?;
    let beta: Vec<f64> = (0..m).map(|i| (0..m).map(|j| inv[i][j] * xty[j]).sum()).collect();
    let sse_means: f64 = design
        .iter()
        .zip(&means)
        .map(|(row, v)| (v - row.iter().zip(&beta).map(|(a, c)| a * c).sum::<f64>()).powi(2))
        .sum();
    let wp_df = wcount.saturating_sub(m);
    let wp_ss = b as f64 * sse_means;
    let full_ss = if fit.df > 0 { fit.mse * fit.df as f64 } else { 0.0 };
    let sp_df = fit.df.saturating_sub(wp_df);
    let sp_ss = (full_ss - wp_ss).max(0.0);
    let ms = |ss: f64, df: usize| if df > 0 { ss / df as f64 } else { f64::NAN };
    let n = y.len() as f64;
    let wp_eff: Vec<f64> = fit.effects.iter().zip(&whole).filter(|(_, w)| **w).map(|(e, _)| e.effect).collect();
    let sp_eff: Vec<f64> = fit.effects.iter().zip(&whole).filter(|(_, w)| !**w).map(|(e, _)| e.effect).collect();
    let whole_plot = Stratum { ss: wp_ss, df: wp_df, ms: ms(wp_ss, wp_df), margin: lenth_margin(&wp_eff) };
    let subplot = Stratum { ss: sp_ss, df: sp_df, ms: ms(sp_ss, sp_df), margin: lenth_margin(&sp_eff) };
    for (e, w) in fit.effects.iter_mut().zip(&whole) {
        let st = if *w { whole_plot } else { subplot };
        if st.df > 0 {
            e.se = (4.0 * st.ms / n).sqrt();
            e.t = e.effect / e.se;
            e.p = match StudentsT::new(0.0, 1.0, st.df as f64) {
                Ok(d) if e.t.is_finite() => 2.0 * d.sf(e.t.abs()),
                Ok(_) if e.t.is_infinite() => 0.0,
                _ => f64::NAN,
            };
        } else {
            e.se = f64::NAN;
            e.t = f64::NAN;
            e.p = f64::NAN;
        }
    }
    Ok(SplitPlotEffects { fit, whole, whole_plot, subplot })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_fraction_has_its_generator_and_resolution() {
        let d = design(4, 1, 1, 0, false, 1).unwrap();
        assert_eq!(d.runs.len(), 8);
        assert_eq!(d.generator_text, ["D = ABC"]);
        assert_eq!(d.resolution, Some(4));
        assert_eq!(d.runs[0], [-1.0, -1.0, -1.0, -1.0]);
        assert_eq!(d.runs[1], [1.0, -1.0, -1.0, 1.0]);
        assert_eq!(design(7, 4, 1, 0, false, 1).unwrap().resolution, Some(3));
        let r = design(3, 0, 2, 3, true, 7).unwrap();
        assert_eq!(r.runs.len(), 19);
        let mut s = r.standard.clone();
        s.sort_unstable();
        assert_eq!(s[..2], [1, 1], "two replicates of run 1");
        assert!(design(9, 3, 1, 0, false, 1).is_err());
    }

    #[test]
    fn a_split_plot_design_keeps_whole_plot_factors_within_plots() {
        let d = split_plot_design(1, 2, 2, true, 3).unwrap();
        assert_eq!(d.runs.len(), 16);
        for w in 1..=4 {
            let rows: Vec<&Vec<f64>> = d.runs.iter().zip(&d.plot).filter(|(_, p)| **p == w).map(|(r, _)| r).collect();
            assert_eq!(rows.len(), 4);
            assert!(rows.iter().all(|r| r[0] == rows[0][0]), "plot {w} changes A");
        }
        // Whole-plot error from replicated plots: an exact model plus a plot effect puts all error in that stratum.
        let bump = [0.3, -0.2, 0.1, -0.2];
        let y: Vec<f64> = d.runs.iter().zip(&d.plot).map(|(r, p)| 5.0 + 2.0 * r[0] + r[1] + bump[p - 1]).collect();
        let e = split_plot_effects(&d.runs, &y, &d.plot, 1).unwrap();
        assert_eq!(e.whole, [true, false, false, false, false, false]);
        assert!(e.whole_plot.ss > 0.1 && e.subplot.ss < 1e-20, "{:?} {:?}", e.whole_plot, e.subplot);
        assert_eq!((e.whole_plot.df, e.subplot.df), (2, 7));
        assert!(split_plot_effects(&d.runs, &y, &d.plot, 2).is_err(), "B changes within plots");
    }

    #[test]
    fn effects_of_an_exact_model_are_twice_the_coefficients() {
        let d = design(3, 0, 2, 0, false, 1).unwrap();
        // y = 10 + 2A - 1.5B + 0.5AC (coded), no noise: exact effects, zero error.
        let y: Vec<f64> = d.runs.iter().map(|r| 10.0 + 2.0 * r[0] - 1.5 * r[1] + 0.5 * r[0] * r[2]).collect();
        // Actual levels 20/40, 1/3, 100/200.
        let x: Vec<Vec<f64>> =
            d.runs.iter().map(|r| vec![30.0 + 10.0 * r[0], 2.0 + r[1], 150.0 + 50.0 * r[2]]).collect();
        let e = effects(&x, &y).unwrap();
        assert!((e.intercept - 10.0).abs() < 1e-12);
        let get = |t: &str| e.effects.iter().find(|f| f.term == t).unwrap().effect;
        assert!((get("A") - 4.0).abs() < 1e-12 && (get("B") + 3.0).abs() < 1e-12 && (get("AC") - 1.0).abs() < 1e-12);
        assert_eq!(e.df, 16 - 7);
        // A half fraction aliases two-factor interactions with main effects.
        let h = design(3, 1, 1, 0, false, 1).unwrap();
        let y: Vec<f64> = h.runs.iter().map(|r| r[0] + r[1]).collect();
        let e = effects(&h.runs, &y).unwrap();
        assert!(e.aliased.iter().any(|(a, b)| a == "AB" && b == "C"), "{:?}", e.aliased);
    }
}
