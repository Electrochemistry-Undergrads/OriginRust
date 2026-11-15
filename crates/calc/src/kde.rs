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
    let mut grid: Vec<f64> = (0..GRID_POINTS).map(|i| lo + (hi - lo) * i as f64 / (GRID_POINTS - 1) as f64).collect();
    if (hi - lo) / (GRID_POINTS - 1) as f64 > MAX_STEP * w {
        refine_near_data(&mut grid, &v, w, (lo, hi));
    }
    let density = grid.iter().map(|&x| density_at(&v, w, x)).collect();
    Some(Density { bandwidth: w, grid, density })
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
