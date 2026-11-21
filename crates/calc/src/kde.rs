//! Gaussian kernel density estimate with the bandwidth rules of Origin's Distribution tab (Kernel
//! Smooth curve of a violin plot).
//!
//! The estimator is `f(x) = 1 / (n w) * sum_i K((x - x_i) / w)` with the normal kernel
//! `K(u) = exp(-u^2 / 2) / sqrt(2 pi)` (Origin's LabTalk `ksdensity`). The bandwidth `w` is one of:
//!
//! - Scott: `0.9 * A * n^(-1/5)`, with `A = min(SD, IQR / 1.349)` (Origin's `kernelwidth`);
//! - Silverman: `1.059 * A * n^(-1/5)`;
//! - `k` times the standard deviation;
//! - a fixed number.
//!
//! Sheaf's choices, not documented by Origin: the IQR in `A` uses the quartiles of Origin's box
//! chart ([`crate::stats::quantile_origin`]); when the IQR is 0 (many equal values) `A` is the SD,
//! as R's `bw.nrd0` does; missing and non-finite values are left out; the curve is evaluated on
//! an even grid.

use crate::stats::quantile_origin;

/// How the bandwidth of the kernel is chosen.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Bandwidth {
    /// `0.9 * A * n^(-1/5)`, Origin's default for the Kernel Smooth curve.
    Scott,
    /// `1.059 * A * n^(-1/5)`.
    Silverman,
    /// `k` times the sample standard deviation.
    Sd(f64),
    /// A bandwidth given as a number.
    Fixed(f64),
}

/// The kernel density estimate of a data set on a grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Density {
    /// The bandwidth used.
    pub bandwidth: f64,
    /// The points where the density was evaluated, ascending.
    pub grid: Vec<f64>,
    /// The density at each grid point.
    pub density: Vec<f64>,
}

/// Number of even grid points of [`kernel_density`] (Sheaf's choice; Origin's is not documented).
/// Where these are further apart than [`MAX_STEP`] bandwidths, more points are added near the data.
pub const GRID_POINTS: usize = 128;
/// The widest step of the grid inside the stretches near the data, in bandwidths: a quarter keeps
/// the Gaussian kernel sampled at least 4 times per bandwidth, so the curve is smooth and its
/// highest sampled point is within 1 % of the true peak.
const MAX_STEP: f64 = 0.25;
/// How far from a value the grid is refined, in bandwidths (the kernel is under 1 % of its peak there).
const NEAR_REACH: f64 = 3.0;
/// Most points added by the refinement, however wide the range is.
const MAX_EXTRA_POINTS: usize = 8192;

fn finite_sorted(data: &[f64]) -> Vec<f64> {
    let mut v: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    v.sort_by(|a, b| a.total_cmp(b));
    v
}

/// The bandwidth of `rule` for `data`; `None` when there is no usable one: fewer than two finite
/// values or a zero standard deviation (except for [`Bandwidth::Fixed`] with a positive number).
pub fn bandwidth(data: &[f64], rule: Bandwidth) -> Option<f64> {
    if let Bandwidth::Fixed(w) = rule {
        return (w.is_finite() && w > 0.0).then_some(w);
    }
    let v = finite_sorted(data);
    let n = v.len();
    if n < 2 {
        return None;
    }
    let nf = n as f64;
    let mean = v.iter().sum::<f64>() / nf;
    let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (nf - 1.0)).sqrt();
    if sd.is_nan() || sd <= 0.0 {
        return None;
    }
    let w = match rule {
        Bandwidth::Sd(k) => k * sd,
        Bandwidth::Scott | Bandwidth::Silverman => {
            let iqr = quantile_origin(&v, 0.75) - quantile_origin(&v, 0.25);
            let spread = if iqr > 0.0 { sd.min(iqr / 1.349) } else { sd };
            let factor = if rule == Bandwidth::Scott { 0.9 } else { 1.059 };
            factor * spread * nf.powf(-0.2)
        }
        Bandwidth::Fixed(_) => unreachable!(),
    };
    (w.is_finite() && w > 0.0).then_some(w)
}

/// The density of the Gaussian kernel estimate with bandwidth `w` at `x`, over the finite values
/// of `data`; NaN without values or with a bandwidth that is not positive.
pub fn density_at(data: &[f64], w: f64, x: f64) -> f64 {
    if !(w > 0.0 && w.is_finite()) {
        return f64::NAN;
    }
    let (mut sum, mut n) = (0.0, 0usize);
    for &xi in data.iter().filter(|v| v.is_finite()) {
        let u = (x - xi) / w;
        sum += (-0.5 * u * u).exp();
        n += 1;
    }
    if n == 0 {
        return f64::NAN;
    }
    sum / (n as f64 * w * (2.0 * std::f64::consts::PI).sqrt())
}

/// The density estimate of `data` with the bandwidth of `rule`, on [`GRID_POINTS`] points from
/// `min - extend_pct / 100 * w` to `max + extend_pct / 100 * w` (Origin's "Extend (% bandwidth)");
/// `None` where [`bandwidth`] is `None`.
pub fn kernel_density(data: &[f64], rule: Bandwidth, extend_pct: f64) -> Option<Density> {
    let w = bandwidth(data, rule)?;
    let v = finite_sorted(data);
    let reach = extend_pct.max(0.0) / 100.0 * w;
    let (lo, hi) = (v[0] - reach, v[v.len() - 1] + reach);
    let grid = refined_grid(&v, w, (lo, hi), GRID_POINTS);
    let density = grid.iter().map(|&x| density_at(&v, w, x)).collect();
    Some(Density { bandwidth: w, grid, density })
}

/// An even grid of `points` from `lo` to `hi`, refined near the ascending values `v` (see
/// `refine_near_data`) where its step is wider than `MAX_STEP` bandwidths `w`: the step is at most a
/// quarter bandwidth within 3 bandwidths of a value, with at most `MAX_EXTRA_POINTS` points added.
pub fn refined_grid(v: &[f64], w: f64, (lo, hi): (f64, f64), points: usize) -> Vec<f64> {
    let mut grid: Vec<f64> = (0..points).map(|i| lo + (hi - lo) * i as f64 / (points - 1) as f64).collect();
    if (hi - lo) / (points - 1) as f64 > MAX_STEP * w {
        refine_near_data(&mut grid, v, w, (lo, hi));
    }
    grid
}

/// Adds points to `grid` (which spans `lo..=hi`) so that, within `NEAR_REACH` bandwidths of any value
/// of the ascending `v`, the step is at most `MAX_STEP` bandwidths. Without this a few distant values
/// make the even grid much coarser than the kernel and the bulk of the data falls between points.
fn refine_near_data(grid: &mut Vec<f64>, v: &[f64], w: f64, (lo, hi): (f64, f64)) {
    // The stretches within reach of a value, merged where they overlap and clipped to the curve.
    let mut stretches: Vec<(f64, f64)> = Vec::new();
    for &x in v {
        let (a, b) = ((x - NEAR_REACH * w).max(lo), (x + NEAR_REACH * w).min(hi));
        match stretches.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => stretches.push((a, b)),
        }
    }
    let total: f64 = stretches.iter().map(|(a, b)| b - a).sum();
    // At most MAX_EXTRA_POINTS in all: a larger need coarsens the step (huge samples with a wide spread).
    let step = (MAX_STEP * w).max(total / MAX_EXTRA_POINTS as f64);
    for (a, b) in stretches {
        let n = ((b - a) / step).ceil().max(1.0) as usize;
        grid.extend((0..=n).map(|i| a + (b - a) * i as f64 / n as f64));
    }
    grid.sort_by(|a, b| a.total_cmp(b));
    // Stretch ends meeting the even grid or each other repeat a point.
    grid.dedup_by(|a, b| *a - *b < 1e-9 * w);
    // Keep the exact ends: dedup keeps the first of a pair, which could be one just above `lo`.
    grid[0] = lo;
    let last = grid.len() - 1;
    grid[last] = hi;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_few_values_or_no_spread_have_no_bandwidth() {
        for rule in [Bandwidth::Scott, Bandwidth::Silverman, Bandwidth::Sd(1.0)] {
            assert_eq!(bandwidth(&[], rule), None);
            assert_eq!(bandwidth(&[3.0], rule), None);
            assert_eq!(bandwidth(&[f64::NAN, 3.0, f64::INFINITY], rule), None);
            assert_eq!(bandwidth(&[2.0, 2.0, 2.0], rule), None);
            assert!(kernel_density(&[2.0, 2.0, 2.0], rule, 100.0).is_none());
        }
        assert_eq!(bandwidth(&[2.0], Bandwidth::Fixed(0.5)), Some(0.5));
        assert_eq!(bandwidth(&[2.0, 3.0], Bandwidth::Fixed(0.0)), None);
        assert_eq!(bandwidth(&[2.0, 3.0], Bandwidth::Fixed(-1.0)), None);
    }

    #[test]
    fn missing_values_are_left_out() {
        let a = [1.0, 2.0, 4.0, 8.0, 9.0];
        let b = [1.0, f64::NAN, 2.0, 4.0, f64::INFINITY, 8.0, 9.0, f64::NEG_INFINITY];
        assert_eq!(bandwidth(&a, Bandwidth::Scott), bandwidth(&b, Bandwidth::Scott));
        assert_eq!(kernel_density(&a, Bandwidth::Scott, 100.0), kernel_density(&b, Bandwidth::Scott, 100.0));
        assert!(density_at(&[f64::NAN], 1.0, 0.0).is_nan());
        assert!(density_at(&a, 0.0, 0.0).is_nan());
    }

    #[test]
    fn the_curve_covers_the_data_and_extends_by_the_bandwidth() {
        let d = kernel_density(&[1.0, 2.0, 4.0, 8.0, 9.0], Bandwidth::Scott, 300.0).unwrap();
        assert_eq!(d.grid.len(), GRID_POINTS, "a modest range keeps the even grid");
        assert!((d.grid[0] - (1.0 - 3.0 * d.bandwidth)).abs() < 1e-12);
        assert!((d.grid[GRID_POINTS - 1] - (9.0 + 3.0 * d.bandwidth)).abs() < 1e-12);
        let flat = kernel_density(&[1.0, 2.0, 4.0, 8.0, 9.0], Bandwidth::Scott, 0.0).unwrap();
        assert_eq!((flat.grid[0], flat.grid[GRID_POINTS - 1]), (1.0, 9.0));
    }

    #[test]
    fn a_ties_only_iqr_falls_back_to_the_sd() {
        // Quartiles equal, so IQR = 0: the SD stands in for A.
        let v = [5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 9.0];
        let w = bandwidth(&v, Bandwidth::Scott).unwrap();
        let n = 8.0f64;
        let mean = 44.0 / n;
        let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        assert!((w - 0.9 * sd * n.powf(-0.2)).abs() < 1e-12);
    }

    #[test]
    fn the_density_integrates_to_one() {
        let data: Vec<f64> = (0..40).map(|i| ((i * 37) % 41) as f64 / 4.0).collect();
        let d = kernel_density(&data, Bandwidth::Silverman, 400.0).unwrap();
        let area: f64 =
            d.grid.windows(2).zip(d.density.windows(2)).map(|(g, f)| (g[1] - g[0]) * (f[0] + f[1]) / 2.0).sum();
        assert!((area - 1.0).abs() < 5e-3, "area {area}");
    }

    /// 1000 deterministic draws from N(0, 1) (Box-Muller on a linear congruential generator).
    fn normal_sample(n: usize) -> Vec<f64> {
        let mut state = 12345u64;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((state >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        (0..n).map(|_| (-2.0 * next().ln()).sqrt() * (2.0 * std::f64::consts::PI * next()).cos()).collect()
    }

    #[test]
    fn far_outliers_do_not_leave_the_bulk_unsampled() {
        let mut data = normal_sample(1000);
        data.extend([1e6; 5]);
        let d = kernel_density(&data, Bandwidth::Scott, 200.0).unwrap();
        let w = d.bandwidth;
        assert!(w < 0.5, "the bandwidth follows the bulk: {w}");
        // The ends stay exact and the grid ascends.
        assert!((d.grid[0] - (data.iter().copied().fold(f64::MAX, f64::min) - 2.0 * w)).abs() < 1e-9);
        assert_eq!(d.grid[d.grid.len() - 1], 1e6 + 2.0 * w);
        assert!(d.grid.windows(2).all(|p| p[1] > p[0]));
        // Inside the bulk the step is at most a quarter of the bandwidth.
        let bulk: Vec<f64> = d.grid.iter().copied().filter(|x| (-3.0..3.0).contains(x)).collect();
        assert!(bulk.windows(2).all(|p| p[1] - p[0] <= 0.25 * w + 1e-9), "bulk steps");
        assert!(bulk.len() > 80, "{} points in the bulk", bulk.len());
        // The highest sampled point is at the mode of the bulk, not at an outlier.
        let (i, peak) = d.density.iter().enumerate().fold((0, 0.0), |m, (i, f)| if *f > m.1 { (i, *f) } else { m });
        assert!(d.grid[i].abs() < 1.0, "peak at {}", d.grid[i]);
        let true_peak = (-2000..=2000).map(|k| density_at(&data, w, k as f64 * 0.002)).fold(0.0, f64::max);
        assert!(peak > 0.99 * true_peak, "{peak} against {true_peak}");
        // Each outlier still has its bump.
        assert!(d.grid.iter().zip(&d.density).any(|(x, f)| (*x - 1e6).abs() < 0.1 * w && *f > 0.0));
    }

    #[test]
    fn the_refinement_is_capped() {
        // A wide spread of many values cannot add more than the cap.
        let data: Vec<f64> = (0..5000).map(|i| (i as f64).powi(3) * 1e-3).collect();
        let d = kernel_density(&data, Bandwidth::Fixed(0.01), 200.0).unwrap();
        assert!(d.grid.len() <= GRID_POINTS + MAX_EXTRA_POINTS + 2 * 5000, "{}", d.grid.len());
        assert!(d.grid.windows(2).all(|p| p[1] > p[0]));
    }
}

/// A bivariate kernel density estimate on a grid (Plot > Statistics > 2D Kernel Density, T116, measured
/// against Origin 2016 by T289).
#[derive(Clone, Debug, PartialEq)]
pub struct Density2d {
    /// The grid's X and Y positions, ascending.
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// The density at (x[i], y[j]), row by row: `z[j * x.len() + i]`.
    pub z: Vec<f64>,
    /// The bandwidths along X and Y.
    pub wx: f64,
    pub wy: f64,
}

/// How [`density_2d_with`] picks the bandwidths (the Method of Origin's dialog).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Kde2dMethod {
    /// "Bivariate Kernel Density Estimator", Origin's default: data-driven bandwidths. Sheaf uses the
    /// diffusion estimator of Botev, Grotowski and Kroese (2010) as published, which comes within a few
    /// percent of Origin's (Origin's exact rule is not documented); [`Kde2dMethod::RuleOfThumb`] stands
    /// in where it finds no solution.
    #[default]
    Bivariate,
    /// "Rule of Thumb": `sd * n^(-1/6) / 2` per axis (measured exact).
    RuleOfThumb,
}

/// The number of grid points along each axis of [`density_2d`] (Origin's 32 x 32 matrix).
pub const GRID_2D: usize = 32;
/// How far the grid reaches beyond the data on each side, as a fraction of the data's range (Origin: a quarter).
const EXTENT_2D: f64 = 0.25;
/// The number of cosine terms of the diffusion estimator along each axis.
const BOTEV_TERMS: usize = 256;

/// [`density_2d_with`] with Origin's default method, [`Kde2dMethod::Bivariate`].
pub fn density_2d(xs: &[f64], ys: &[f64]) -> Option<Density2d> {
    density_2d_with(xs, ys, Kde2dMethod::Bivariate)
}

/// The Gaussian product-kernel density of the finite `(x, y)` pairs on a [`GRID_2D`] square grid from
/// `min - range / 4` to `max + range / 4` along each axis (both ends included), with the bandwidths of
/// `method`; `None` with fewer than two pairs or no spread along an axis.
pub fn density_2d_with(xs: &[f64], ys: &[f64], method: Kde2dMethod) -> Option<Density2d> {
    let pts: Vec<(f64, f64)> =
        xs.iter().zip(ys).map(|(x, y)| (*x, *y)).filter(|(x, y)| x.is_finite() && y.is_finite()).collect();
    let (px, py): (Vec<f64>, Vec<f64>) = pts.iter().copied().unzip();
    let (wx, wy) = bandwidths_2d(&px, &py, method)?;
    let (gx, gy) = (grid_2d(&px), grid_2d(&py));
    let norm = 1.0 / (pts.len() as f64 * 2.0 * std::f64::consts::PI * wx * wy);
    let mut z = Vec::with_capacity(GRID_2D * GRID_2D);
    for &y in &gy {
        for &x in &gx {
            let s: f64 =
                pts.iter().map(|(px, py)| (-0.5 * (((x - px) / wx).powi(2) + ((y - py) / wy).powi(2))).exp()).sum();
            z.push(s * norm);
        }
    }
    Some(Density2d { x: gx, y: gy, z, wx, wy })
}

/// The lowest and highest of `v` widened by [`EXTENT_2D`] of the range on each side.
fn extent_2d(v: &[f64]) -> (f64, f64) {
    let (lo, hi) = v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| (a.min(x), b.max(x)));
    let r = hi - lo;
    (lo - EXTENT_2D * r, hi + EXTENT_2D * r)
}

fn grid_2d(v: &[f64]) -> Vec<f64> {
    let (lo, hi) = extent_2d(v);
    (0..GRID_2D).map(|k| lo + (hi - lo) * k as f64 / (GRID_2D - 1) as f64).collect()
}

/// The X and Y bandwidths of `method` for the pairs `(xs[i], ys[i])` (all finite); `None` with fewer
/// than two pairs or no spread along an axis.
pub fn bandwidths_2d(xs: &[f64], ys: &[f64], method: Kde2dMethod) -> Option<(f64, f64)> {
    let n = xs.len().min(ys.len());
    if n < 2 {
        return None;
    }
    let (xs, ys) = (&xs[..n], &ys[..n]);
    let sd = |v: &[f64]| {
        let m = v.iter().sum::<f64>() / n as f64;
        (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
    };
    let factor = (n as f64).powf(-1.0 / 6.0) / 2.0;
    let thumb = (sd(xs) * factor, sd(ys) * factor);
    if !(thumb.0 > 0.0 && thumb.1 > 0.0 && thumb.0.is_finite() && thumb.1.is_finite()) {
        return None;
    }
    Some(match method {
        Kde2dMethod::RuleOfThumb => thumb,
        Kde2dMethod::Bivariate => botev_bandwidths(xs, ys).unwrap_or(thumb),
    })
}

/// The 2D diffusion estimator's state: the squared cosine transform of the scaled data.
struct Diffusion {
    n: f64,
    /// `a2[i * BOTEV_TERMS + j]`: the transform at X term `i` and Y term `j`, squared.
    a2: Vec<f64>,
    /// `k^2` for each term `k`.
    sq: Vec<f64>,
}

impl Diffusion {
    /// The functional of order `(s0, s1)` at time `t` (the published code's `psi`).
    fn psi(&self, s0: i32, s1: i32, t: f64) -> f64 {
        use std::f64::consts::PI;
        let m = BOTEV_TERMS;
        let w: Vec<f64> = (0..m).map(|k| (-self.sq[k] * PI * PI * t).exp() * if k == 0 { 1.0 } else { 0.5 }).collect();
        let v: Vec<f64> = (0..m).map(|j| w[j] * self.sq[j].powi(s0)).collect();
        let total: f64 = (0..m)
            .map(|i| {
                let inner: f64 = self.a2[i * m..(i + 1) * m].iter().zip(&v).map(|(a, v)| a * v).sum();
                w[i] * self.sq[i].powi(s1) * inner
            })
            .sum();
        let sign = if (s0 + s1) % 2 == 0 { 1.0 } else { -1.0 };
        sign * total * PI.powi(2 * (s0 + s1))
    }

    /// The functional of order `s`: orders up to 4 at the time their own plug-in gives from the next
    /// order, order 5 at `t` (the published code's `func`), memoised over one evaluation.
    fn func(&self, s: (i32, i32), t: f64, memo: &mut Vec<((i32, i32), f64)>) -> f64 {
        if let Some((_, v)) = memo.iter().find(|(k, _)| *k == s) {
            return *v;
        }
        let order = s.0 + s.1;
        let v = if order <= 4 {
            let sf = self.func((s.0 + 1, s.1), t, memo) + self.func((s.0, s.1 + 1), t, memo);
            let c = (1.0 + 0.5f64.powi(order + 1)) / 3.0;
            let time = (-2.0 * c * k_const(s.0) * k_const(s.1) / self.n / sf).powf(1.0 / (2.0 + f64::from(order)));
            self.psi(s.0, s.1, time)
        } else {
            self.psi(s.0, s.1, t)
        };
        memo.push((s, v));
        v
    }

    /// The second-order functionals `(psi_02, psi_20, psi_11)` at time `t`.
    fn second_order(&self, t: f64) -> (f64, f64, f64) {
        let mut memo = Vec::with_capacity(18);
        (self.func((0, 2), t, &mut memo), self.func((2, 0), t, &mut memo), self.func((1, 1), t, &mut memo))
    }

    /// `t - (t - t*) / t*` with `t* = (2 pi N S(t))^(-1/3)`, whose root is the diffusion time.
    fn fixed_point(&self, t: f64) -> f64 {
        let (p02, p20, p11) = self.second_order(t);
        let star = (2.0 * std::f64::consts::PI * self.n * (p02 + p20 + 2.0 * p11)).powf(-1.0 / 3.0);
        t - (t - star) / star
    }
}

/// `(-1)^s (2s - 1)!! / sqrt(2 pi)`.
fn k_const(s: i32) -> f64 {
    let prod: f64 = (0..s).map(|i| f64::from(2 * i + 1)).product();
    let sign = if s % 2 == 0 { 1.0 } else { -1.0 };
    sign * prod / (2.0 * std::f64::consts::PI).sqrt()
}

/// The diffusion bandwidths of Botev, Grotowski and Kroese (2010, "Kernel density estimation via
/// diffusion", the published `kde2d` algorithm) for finite pairs with a spread along both axes: the data
/// scaled to [0, 1] over the grid's extent, an exact cosine transform of [`BOTEV_TERMS`] terms per axis,
/// and the root of the plug-in equation for the diffusion time. `None` where no root is bracketed.
fn botev_bandwidths(xs: &[f64], ys: &[f64]) -> Option<(f64, f64)> {
    use std::f64::consts::PI;
    let n = xs.len() as f64;
    let m = BOTEV_TERMS;
    // cos[k * len + p] = w_k cos(pi k t_p) with t the scaled data, w_0 = 1 and w_k = 2 otherwise.
    let cosines = |v: &[f64]| -> (Vec<f64>, f64) {
        let (lo, hi) = extent_2d(v);
        let width = hi - lo;
        let mut c = Vec::with_capacity(m * v.len());
        for k in 0..m {
            let w = if k == 0 { 1.0 } else { 2.0 };
            c.extend(v.iter().map(|x| w * (PI * k as f64 * (x - lo) / width).cos()));
        }
        (c, width)
    };
    let ((cx, sx), (cy, sy)) = (cosines(xs), cosines(ys));
    let len = xs.len();
    let mut a2 = vec![0.0; m * m];
    for i in 0..m {
        let ci = &cx[i * len..(i + 1) * len];
        for j in 0..m {
            let a = ci.iter().zip(&cy[j * len..(j + 1) * len]).map(|(p, q)| p * q).sum::<f64>() / n;
            a2[i * m + j] = a * a;
        }
    }
    let d = Diffusion { n, a2, sq: (0..m).map(|k| (k * k) as f64).collect() };
    // The published search: from a small upper end, doubled up to 0.1 until it brackets the root.
    let mut hi = 1e-12 + 0.01 * (n.clamp(50.0, 1050.0) - 50.0) / 1000.0;
    let f0 = d.fixed_point(0.0);
    let mut fhi = d.fixed_point(hi);
    // Until the ends differ in sign (a NaN never brackets).
    while (f0 * fhi).partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        if hi >= 0.1 || !f0.is_finite() {
            return None;
        }
        hi = (hi * 2.0).min(0.1);
        fhi = d.fixed_point(hi);
    }
    let (mut lo, mut flo) = (0.0, f0);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        let fm = d.fixed_point(mid);
        if fm == 0.0 {
            (lo, hi) = (mid, mid);
            break;
        }
        if (fm < 0.0) == (flo < 0.0) {
            (lo, flo) = (mid, fm);
        } else {
            hi = mid;
        }
    }
    let (p02, p20, p11) = d.second_order(0.5 * (lo + hi));
    let mix = p11 + (p20 * p02).sqrt();
    let t_y = (p02.powf(0.75) / (4.0 * PI * n * p20.powf(0.75) * mix)).powf(1.0 / 3.0);
    let t_x = (p20.powf(0.75) / (4.0 * PI * n * p02.powf(0.75) * mix)).powf(1.0 / 3.0);
    let (wx, wy) = (t_x.sqrt() * sx, t_y.sqrt() * sy);
    (wx > 0.0 && wy > 0.0 && wx.is_finite() && wy.is_finite()).then_some((wx, wy))
}

#[cfg(test)]
mod density_2d_tests {
    use super::*;

    #[test]
    fn the_2d_density_peaks_at_the_cluster_on_origins_grid() {
        let xs = [0.0, 0.1, -0.1, 0.05, 0.02, -0.04, 0.08, -0.07, 5.0];
        let ys = [0.0, -0.1, 0.1, 0.0, 0.06, -0.03, -0.05, 0.04, 5.0];
        for method in [Kde2dMethod::Bivariate, Kde2dMethod::RuleOfThumb] {
            let d = density_2d_with(&xs, &ys, method).unwrap();
            assert_eq!((d.x.len(), d.y.len(), d.z.len()), (GRID_2D, GRID_2D, GRID_2D * GRID_2D));
            assert!((d.x[0] - (-0.1 - 5.1 / 4.0)).abs() < 1e-12, "{}", d.x[0]);
            assert!((d.x[GRID_2D - 1] - (5.0 + 5.1 / 4.0)).abs() < 1e-12, "{}", d.x[GRID_2D - 1]);
            let k = d.z.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
            let (px, py) = (d.x[k % GRID_2D], d.y[k / GRID_2D]);
            assert!(px.abs() < 1.0 && py.abs() < 1.0, "{method:?}: the peak is at the cluster: {px} {py}");
        }
    }

    #[test]
    fn the_2d_density_integrates_to_about_one_and_rejects_degenerate_data() {
        let xs: Vec<f64> = (0..30).map(|i| ((i * 7) % 30) as f64).collect();
        let ys: Vec<f64> = (0..30).map(|i| ((i * 11) % 30) as f64).collect();
        let d = density_2d_with(&xs, &ys, Kde2dMethod::RuleOfThumb).unwrap();
        let (dx, dy) = (d.x[1] - d.x[0], d.y[1] - d.y[0]);
        let total: f64 = d.z.iter().sum::<f64>() * dx * dy;
        assert!((total - 1.0).abs() < 0.05, "{total}");
        assert!(density_2d(&[1.0], &[1.0]).is_none());
        assert!(density_2d(&[1.0, 1.0], &[1.0, 2.0]).is_none(), "no spread along X");
        assert!(density_2d(&[1.0, 2.0, f64::NAN], &[1.0, 2.0, 3.0]).is_some(), "the NaN pair is left out");
    }
}
