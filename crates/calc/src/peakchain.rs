//! The peak analysis chain: baseline, find peaks, integrate every peak.
//!
//! This follows the goals of Origin's Peak Analyzer (create a baseline, subtract it, find peaks,
//! integrate peaks) as separate steps that [`analyze_peaks`] runs in order. The pieces that
//! Origin documents numerically are checked against it (`tests/origin_peakchain.rs`); the rules
//! are in `docs/spec/analysis-tools.md`.
//!
//! Steps:
//!
//! 1. The baseline is built from the chosen mode and subtracted from the data.
//! 2. Peaks are found on the corrected curve with one of the four methods of `pkFind`
//!    (local maximum, window search, first derivative, second derivative), optionally after
//!    smoothing, then filtered by height or number.
//! 3. Each peak is integrated between its limits: the places where the corrected curve meets
//!    the baseline (or the half height), but never beyond the valley that separates it from a
//!    neighbouring peak. Neighbouring peaks that overlap therefore share the valley point.
//!
//! Rows are 0-based indices here. The data must be ordered along X (rising or falling); rows
//! with a missing X or Y are skipped.

use crate::baseline::{BaselineMethod, subtract_baseline};
use crate::differentiate::{DiffOptions, differentiate};
use crate::interp::{InterpMethod, SplineBoundary};
use crate::peaks::{PeakDirection, PeakFilter, PeakOptions, find_peaks};
use crate::smooth::{SmoothOptions, smooth};

/// How peaks are located (the `method` of `pkFind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum FindMethod {
    /// A point larger than its `npts` neighbours on each side.
    #[default]
    LocalMax,
    /// The largest point of a window of `npts` points on each side.
    Window,
    /// A sign change of the first derivative from plus to minus.
    FirstDerivative,
    /// A minimum of the second derivative (also finds shoulders hidden in a bigger peak).
    SecondDerivative,
}

/// Settings of the peak search.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FindOptions {
    pub method: FindMethod,
    /// Neighbours compared (local maximum, window search) or half the window of the derivative.
    pub npts: usize,
    /// Savitzky-Golay window (order 2) applied before the search; 0 or 1 means no smoothing.
    pub smooth_points: usize,
    pub direction: PeakDirection,
    /// Height or number filter. `Height(0.0)` keeps peaks of at least 20 % of the largest
    /// magnitude, as `pkFind` does.
    pub filter: PeakFilter,
    /// Drop peaks narrower than this FWHM (in X units); 0 keeps all.
    pub min_fwhm: f64,
}

impl Default for FindOptions {
    fn default() -> Self {
        FindOptions {
            method: FindMethod::LocalMax,
            npts: 2,
            smooth_points: 0,
            direction: PeakDirection::Positive,
            filter: PeakFilter::Height(0.0),
            min_fwhm: 0.0,
        }
    }
}

/// Where a peak starts and ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LimitMode {
    /// Where the corrected curve reaches zero (the baseline) or the valley to the next peak.
    #[default]
    Baseline,
    /// Where the curve falls to half the peak height (area of the upper half only).
    HalfHeight,
}

/// How the baseline is made.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChainBaseline {
    /// Data are used as they are.
    None,
    Constant(f64),
    /// Straight line through the first and last point.
    EndPoints,
    /// Curve through given anchor points.
    Anchors(Vec<(f64, f64)>, InterpMethod),
    /// `n` anchors, each the lowest point of one of `n` equal stretches of the data (found on a
    /// lightly smoothed copy), joined by straight lines. This is Sheaf's own automatic mode.
    AutoMinima(usize),
    /// Origin's Baseline Mode "Constant": Y is the minimum (Origin's default), maximum, mean or
    /// median of the data.
    ConstantStat(BaselineStat),
}

/// Which value of the data a constant baseline takes (Origin's Constant choices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BaselineStat {
    #[default]
    Minimum,
    Maximum,
    Mean,
    Median,
}

impl BaselineStat {
    /// The value of the finite numbers of `y`; `None` when there is none.
    pub fn of(self, y: &[f64]) -> Option<f64> {
        let mut v: Vec<f64> = y.iter().copied().filter(|a| a.is_finite()).collect();
        if v.is_empty() {
            return None;
        }
        Some(match self {
            BaselineStat::Minimum => v.iter().copied().fold(f64::INFINITY, f64::min),
            BaselineStat::Maximum => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            BaselineStat::Mean => v.iter().sum::<f64>() / v.len() as f64,
            BaselineStat::Median => {
                v.sort_by(f64::total_cmp);
                let n = v.len();
                if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
            }
        })
    }
}

/// Everything [`analyze_peaks`] needs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChainOptions {
    pub baseline: ChainBaseline,
    pub find: FindOptions,
    pub limits: LimitMode,
}

impl Default for ChainOptions {
    fn default() -> Self {
        ChainOptions { baseline: ChainBaseline::None, find: FindOptions::default(), limits: LimitMode::Baseline }
    }
}

/// One peak with its integration figures (the columns of the Peak Analyzer's result table).
#[derive(Debug, Clone, PartialEq)]
pub struct PeakInfo {
    /// 0-based index of the peak point.
    pub index: usize,
    /// X of the peak point (the centre).
    pub center: f64,
    /// Height above the baseline (negative for a negative peak).
    pub height: f64,
    /// Integral between the limits of the baseline-corrected curve.
    pub area: f64,
    /// Share of the sum of all areas, in percent.
    pub area_percent: f64,
    /// Full width at half height, interpolated, within the limits.
    pub fwhm: f64,
    pub left_x: f64,
    pub right_x: f64,
    /// 0-based index of the first and last data point inside the limits.
    pub left_index: usize,
    pub right_index: usize,
}

/// Result of [`analyze_peaks`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChainResult {
    pub baseline: Vec<f64>,
    pub corrected: Vec<f64>,
    pub peaks: Vec<PeakInfo>,
}

/// Anchors of [`ChainBaseline::AutoMinima`].
pub fn auto_minima_anchors(x: &[f64], y: &[f64], n: usize) -> Vec<(f64, f64)> {
    let len = x.len().min(y.len());
    let n = n.max(2).min(len.max(2));
    let ys = if len >= 7 { smooth(&y[..len], &SmoothOptions::default()) } else { y[..len].to_vec() };
    let mut out = Vec::new();
    for k in 0..n {
        let a = k * len / n;
        let b = ((k + 1) * len / n).max(a + 1).min(len);
        let mut best = a;
        for i in a..b {
            if ys[i].is_finite() && (!ys[best].is_finite() || ys[i] < ys[best]) {
                best = i;
            }
        }
        out.push((x[best], y[best]));
    }
    out
}

fn baseline_method(x: &[f64], y: &[f64], b: &ChainBaseline) -> Option<BaselineMethod> {
    let n = x.len().min(y.len());
    Some(match b {
        ChainBaseline::None => BaselineMethod::Constant(0.0),
        ChainBaseline::Constant(c) => BaselineMethod::Constant(*c),
        ChainBaseline::ConstantStat(s) => BaselineMethod::Constant(s.of(&y[..n])?),
        ChainBaseline::EndPoints => BaselineMethod::Line((x[0], y[0]), (x[n - 1], y[n - 1])),
        ChainBaseline::Anchors(points, method) => {
            if points.len() < 2 {
                return None;
            }
            let mut p = points.clone();
            p.sort_by(|a, b| a.0.total_cmp(&b.0));
            BaselineMethod::Anchors { points: p, method: *method, boundary: SplineBoundary::NotAKnot }
        }
        ChainBaseline::AutoMinima(k) => {
            let mut p = auto_minima_anchors(x, y, *k);
            p.dedup_by(|a, b| a.0 == b.0);
            if p.len() < 2 {
                return None;
            }
            BaselineMethod::Anchors { points: p, method: InterpMethod::Linear, boundary: SplineBoundary::NotAKnot }
        }
    })
}

/// Finds the peak points of an already baseline-corrected curve; returns 0-based indices in
/// ascending order. A negative direction mirrors the curve; `Both` returns both kinds.
pub fn find_peak_indices(x: &[f64], c: &[f64], opts: &FindOptions) -> Vec<usize> {
    let mut all = Vec::new();
    if opts.direction != PeakDirection::Negative {
        all.extend(positive_peaks(x, c, opts));
    }
    if opts.direction != PeakDirection::Positive {
        let m: Vec<f64> = c.iter().map(|v| -v).collect();
        all.extend(positive_peaks(x, &m, opts));
    }
    all.sort_unstable();
    all.dedup();
    all
}

fn positive_peaks(x: &[f64], c: &[f64], opts: &FindOptions) -> Vec<usize> {
    let n = x.len().min(c.len());
    let c = &c[..n];
    let ys = if opts.smooth_points >= 3 {
        smooth(c, &SmoothOptions { npts: opts.smooth_points, ..SmoothOptions::default() })
    } else {
        c.to_vec()
    };
    let k = opts.npts.max(1);
    let mut found: Vec<usize> = Vec::new();
    match opts.method {
        FindMethod::LocalMax => {
            let o = PeakOptions {
                npts: k,
                direction: PeakDirection::Positive,
                filter: PeakFilter::None,
                ..PeakOptions::default()
            };
            found = find_peaks(&x[..n], &ys, &o).rows.iter().map(|r| r - 1).collect();
        }
        FindMethod::Window => {
            if n > 2 * k {
                for i in k..n - k {
                    let v = ys[i];
                    if v > 0.0 && ys[i - k..i].iter().all(|&a| v > a) && ys[i + 1..=i + k].iter().all(|&a| v >= a) {
                        found.push(i);
                    }
                }
            }
        }
        FindMethod::FirstDerivative => {
            let d = differentiate(&x[..n], &ys, &DiffOptions { order: 1, smooth: true, poly: 2, npts: 2 * k + 1 });
            for i in 1..n {
                if d[i - 1] > 0.0 && d[i] <= 0.0 {
                    let j = if ys[i - 1] >= ys[i] { i - 1 } else { i };
                    if ys[j] > 0.0 {
                        found.push(j);
                    }
                }
            }
        }
        FindMethod::SecondDerivative => {
            let d = differentiate(&x[..n], &ys, &DiffOptions { order: 2, smooth: true, poly: 3, npts: 2 * k + 1 });
            for i in 1..n.saturating_sub(1) {
                if d[i].is_finite() && d[i] < 0.0 && d[i] < d[i - 1] && d[i] <= d[i + 1] && ys[i] > 0.0 {
                    found.push(i);
                }
            }
        }
    }
    found.dedup();
    match opts.filter {
        PeakFilter::None => {}
        PeakFilter::Height(h) => {
            let thr = if h == 0.0 {
                0.2 * ys.iter().filter(|a| a.is_finite()).fold(0.0_f64, |m, a| m.max(a.abs()))
            } else {
                h
            };
            found.retain(|&i| ys[i] >= thr);
        }
        PeakFilter::Number(m) => {
            found.sort_by(|&a, &b| ys[b].total_cmp(&ys[a]).then(a.cmp(&b)));
            found.truncate(m);
            found.sort_unstable();
        }
    }
    // The point reported is the largest raw point next to the smoothed maximum.
    if opts.smooth_points >= 3 {
        let r = opts.smooth_points / 2;
        for i in found.iter_mut() {
            let (a, b) = (i.saturating_sub(r), (*i + r).min(n - 1));
            let mut best = *i;
            for j in a..=b {
                if c[j] > c[best] {
                    best = j;
                }
            }
            *i = best;
        }
        found.dedup();
    }
    found
}

/// Integrates each peak of the corrected curve `c` between its limits. `peaks` are 0-based
/// indices in ascending order; the sign of each peak follows `c`.
pub fn integrate_peaks(x: &[f64], c: &[f64], peaks: &[usize], mode: LimitMode) -> Vec<PeakInfo> {
    let n = x.len().min(c.len());
    let mut out: Vec<PeakInfo> = Vec::with_capacity(peaks.len());
    // Valley between neighbours of the same sign: the point closest to the baseline.
    let valley = |a: usize, b: usize| -> usize {
        let sign = if c[a] >= 0.0 { 1.0 } else { -1.0 };
        let mut best = a;
        for i in a..=b {
            if sign * c[i] < sign * c[best] {
                best = i;
            }
        }
        best
    };
    for (k, &p) in peaks.iter().enumerate() {
        if p >= n {
            continue;
        }
        let sign = if c[p] >= 0.0 { 1.0 } else { -1.0 };
        let level = match mode {
            LimitMode::Baseline => 0.0,
            LimitMode::HalfHeight => c[p] / 2.0,
        };
        let lo_bound = if k > 0 { valley(peaks[k - 1], p) } else { 0 };
        let hi_bound = if k + 1 < peaks.len() { valley(p, peaks[k + 1]) } else { n - 1 };
        // Left limit: walk out until the curve is at or below the level.
        let (mut li, mut lx, mut ly) = (p, x[p], c[p]);
        let mut j = p;
        while j > lo_bound {
            if sign * (c[j - 1] - level) <= 0.0 {
                let (ya, yb) = (c[j - 1], c[j]);
                lx = if yb == ya { x[j] } else { x[j - 1] + (level - ya) * (x[j] - x[j - 1]) / (yb - ya) };
                ly = level;
                li = j;
                break;
            }
            j -= 1;
            li = j;
            lx = x[j];
            ly = c[j];
        }
        let (mut ri, mut rx, mut ry) = (p, x[p], c[p]);
        let mut j = p;
        while j < hi_bound {
            if sign * (c[j + 1] - level) <= 0.0 {
                let (ya, yb) = (c[j], c[j + 1]);
                rx = if yb == ya { x[j] } else { x[j] + (level - ya) * (x[j + 1] - x[j]) / (yb - ya) };
                ry = level;
                ri = j;
                break;
            }
            j += 1;
            ri = j;
            rx = x[j];
            ry = c[j];
        }
        // Area: trapezoids over the limit points and the data points between them.
        let mut area = 0.0;
        let (mut px, mut py) = (lx, ly);
        for i in li..=ri {
            if x[i] == px && i == li && ly == c[i] {
                continue;
            }
            area += (x[i] - px) * (py + c[i]) / 2.0;
            px = x[i];
            py = c[i];
        }
        area += (rx - px) * (py + ry) / 2.0;
        if x[n - 1] < x[0] {
            area = -area;
        }
        // Width at half height inside the limits.
        let half = c[p] / 2.0;
        let mut wl = lx;
        let mut i = p;
        while i > li {
            if sign * (c[i - 1] - half) <= 0.0 {
                wl = x[i - 1] + (half - c[i - 1]) * (x[i] - x[i - 1]) / (c[i] - c[i - 1]);
                break;
            }
            i -= 1;
        }
        let mut wr = rx;
        let mut i = p;
        while i < ri {
            if sign * (c[i + 1] - half) <= 0.0 {
                wr = x[i] + (half - c[i]) * (x[i + 1] - x[i]) / (c[i + 1] - c[i]);
                break;
            }
            i += 1;
        }
        out.push(PeakInfo {
            index: p,
            center: x[p],
            height: c[p],
            area,
            area_percent: 0.0,
            fwhm: (wr - wl).abs(),
            left_x: lx,
            right_x: rx,
            left_index: li,
            right_index: ri,
        });
    }
    // Origin's "AreaIntgP(%)": the share of the area of the whole corrected curve (measured: two
    // peaks on a slope gave 35.29 % and 40.93 %, a lone Gaussian 99.98 %), not of the sum of the
    // peaks.
    let mut total = 0.0;
    for i in 1..n {
        if x[i].is_finite() && x[i - 1].is_finite() && c[i].is_finite() && c[i - 1].is_finite() {
            total += (x[i] - x[i - 1]) * (c[i] + c[i - 1]) / 2.0;
        }
    }
    for p in out.iter_mut() {
        p.area_percent = if total != 0.0 { 100.0 * p.area / total } else { f64::NAN };
    }
    out
}

/// Why [`analyze_peaks`] could not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    /// Fewer than two usable points, or fewer than two anchors with different X.
    NoBaseline,
    /// X is neither rising nor falling (the rows are not sorted along X).
    UnsortedX,
}

/// Runs the chain on `y(x)`. Rows where X or Y is missing are left out of the calculation
/// (the baseline and corrected curve are NaN there, and an area is the integral over the valid
/// rows, as the multi-peak fit also skips them); the indices in the result are row indices of
/// the input. The areas are positive for a positive peak whichever way X runs.
pub fn analyze_peaks(x: &[f64], y: &[f64], opts: &ChainOptions) -> Result<ChainResult, ChainError> {
    chain(x, y, opts, None)
}

/// [`analyze_peaks`] with the peaks given by their X instead of searched (Origin's Multiple Peak
/// Fit, where the user picks the centres): each centre becomes the data point nearest to it (two
/// centres on one point give one peak), the search, its filter and the width filter are skipped,
/// and heights, widths and limits are measured above the lowest point of the corrected curve.
pub fn analyze_peaks_at(x: &[f64], y: &[f64], opts: &ChainOptions, centers: &[f64]) -> Result<ChainResult, ChainError> {
    chain(x, y, opts, Some(centers))
}

fn chain(x: &[f64], y: &[f64], opts: &ChainOptions, centers: Option<&[f64]>) -> Result<ChainResult, ChainError> {
    let len = x.len().min(y.len());
    let rows: Vec<usize> = (0..len).filter(|&i| x[i].is_finite() && y[i].is_finite()).collect();
    if rows.len() < 2 {
        return Err(ChainError::NoBaseline);
    }
    let xv: Vec<f64> = rows.iter().map(|&i| x[i]).collect();
    let yv: Vec<f64> = rows.iter().map(|&i| y[i]).collect();
    let rising = xv.windows(2).all(|w| w[1] >= w[0]);
    let falling = xv.windows(2).all(|w| w[1] <= w[0]);
    if !rising && !falling {
        return Err(ChainError::UnsortedX);
    }
    let method = baseline_method(&xv, &yv, &opts.baseline).ok_or(ChainError::NoBaseline)?;
    let (corrected, baseline) = subtract_baseline(&xv, &yv, &method);
    let mut peaks = match centers {
        Some(centers) => {
            let nearest = |c: f64| {
                (0..xv.len())
                    .min_by(|&a, &b| (xv[a] - c).abs().total_cmp(&(xv[b] - c).abs()))
                    .expect("two rows or more")
            };
            let mut idx: Vec<usize> = centers.iter().filter(|c| c.is_finite()).map(|&c| nearest(c)).collect();
            idx.sort_unstable();
            idx.dedup();
            let floor = corrected.iter().copied().fold(f64::INFINITY, f64::min);
            let lifted: Vec<f64> = corrected.iter().map(|v| v - floor).collect();
            integrate_peaks(&xv, &lifted, &idx, opts.limits)
        }
        None => {
            let idx = find_peak_indices(&xv, &corrected, &opts.find);
            let peaks = integrate_peaks(&xv, &corrected, &idx, opts.limits);
            if opts.find.min_fwhm > 0.0 && peaks.iter().any(|p| p.fwhm < opts.find.min_fwhm) {
                let idx: Vec<usize> = peaks.iter().filter(|p| p.fwhm >= opts.find.min_fwhm).map(|p| p.index).collect();
                integrate_peaks(&xv, &corrected, &idx, opts.limits)
            } else {
                peaks
            }
        }
    };
    for p in peaks.iter_mut() {
        p.index = rows[p.index];
        p.left_index = rows[p.left_index];
        p.right_index = rows[p.right_index];
    }
    let mut full_b = vec![f64::NAN; len];
    let mut full_c = vec![f64::NAN; len];
    for (k, &i) in rows.iter().enumerate() {
        full_b[i] = baseline[k];
        full_c[i] = corrected[k];
    }
    Ok(ChainResult { baseline: full_b, corrected: full_c, peaks })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gauss(x: f64, c: f64, s: f64, h: f64) -> f64 {
        h * (-(x - c).powi(2) / (2.0 * s * s)).exp()
    }

    fn grid() -> Vec<f64> {
        (0..401).map(|i| i as f64 * 0.05).collect()
    }

    #[test]
    fn separate_peaks_have_their_gaussian_area() {
        let x = grid();
        let y: Vec<f64> = x.iter().map(|&x| gauss(x, 5.0, 0.4, 1.0) + gauss(x, 14.0, 0.6, 0.5)).collect();
        let r = analyze_peaks(&x, &y, &ChainOptions::default()).unwrap();
        assert_eq!(r.peaks.len(), 2);
        let a1 = 1.0 * 0.4 * (2.0 * std::f64::consts::PI).sqrt();
        let a2 = 0.5 * 0.6 * (2.0 * std::f64::consts::PI).sqrt();
        assert!((r.peaks[0].area - a1).abs() < 2e-3 * a1, "{}", r.peaks[0].area);
        assert!((r.peaks[1].area - a2).abs() < 2e-3 * a2);
        assert!((r.peaks[0].fwhm - 0.4 * 2.354_820).abs() < 2e-3);
        // Area % is of the whole corrected curve (Origin's AreaIntgP); here the peaks hold almost all of it.
        assert!((r.peaks[0].area_percent + r.peaks[1].area_percent - 100.0).abs() < 0.5);
    }

    #[test]
    fn overlapping_peaks_split_at_the_valley() {
        let x = grid();
        let y: Vec<f64> = x.iter().map(|&x| gauss(x, 9.0, 0.5, 1.0) + gauss(x, 10.2, 0.5, 0.8)).collect();
        let r = analyze_peaks(&x, &y, &ChainOptions::default()).unwrap();
        assert_eq!(r.peaks.len(), 2);
        assert_eq!(r.peaks[0].right_index, r.peaks[1].left_index);
        assert!(r.peaks[0].right_x > 9.0 && r.peaks[0].right_x < 10.2);
        let total = r.peaks[0].area + r.peaks[1].area;
        let exact = (1.0 + 0.8) * 0.5 * (2.0 * std::f64::consts::PI).sqrt();
        assert!((total - exact).abs() < 5e-3 * exact);
    }

    #[test]
    fn sloped_baseline_by_end_points_and_by_minima() {
        let x = grid();
        let y: Vec<f64> = x.iter().map(|&x| 0.2 + 0.03 * x + gauss(x, 10.0, 0.5, 1.0)).collect();
        let exact = 0.5 * (2.0 * std::f64::consts::PI).sqrt();
        for b in [ChainBaseline::EndPoints, ChainBaseline::AutoMinima(4)] {
            let o = ChainOptions {
                baseline: b,
                find: FindOptions { filter: PeakFilter::Height(0.3), ..Default::default() },
                ..Default::default()
            };
            let r = analyze_peaks(&x, &y, &o).unwrap();
            assert_eq!(r.peaks.len(), 1);
            assert!((r.peaks[0].area - exact).abs() < 0.01 * exact, "{}", r.peaks[0].area);
        }
    }

    #[test]
    fn negative_peaks_and_methods() {
        let x = grid();
        let y: Vec<f64> = x.iter().map(|&x| gauss(x, 6.0, 0.5, 1.0) - gauss(x, 14.0, 0.5, 0.9)).collect();
        let o = ChainOptions {
            find: FindOptions { direction: PeakDirection::Both, ..Default::default() },
            ..Default::default()
        };
        let r = analyze_peaks(&x, &y, &o).unwrap();
        assert_eq!(r.peaks.len(), 2);
        assert!(r.peaks[1].area < 0.0 && r.peaks[1].height < 0.0);
        for m in [FindMethod::Window, FindMethod::FirstDerivative, FindMethod::SecondDerivative] {
            let o =
                ChainOptions { find: FindOptions { method: m, npts: 5, ..Default::default() }, ..Default::default() };
            let r = analyze_peaks(&x, &y, &o).unwrap();
            assert_eq!(r.peaks.len(), 1, "{m:?}");
            assert!((r.peaks[0].center - 6.0).abs() < 0.06, "{m:?}");
        }
    }

    #[test]
    fn second_derivative_finds_a_shoulder() {
        let x = grid();
        let y: Vec<f64> = x.iter().map(|&x| gauss(x, 10.0, 0.5, 1.0) + gauss(x, 11.3, 0.5, 0.45)).collect();
        let lm = analyze_peaks(&x, &y, &ChainOptions::default()).unwrap();
        assert_eq!(lm.peaks.len(), 1);
        let o = ChainOptions {
            find: FindOptions {
                method: FindMethod::SecondDerivative,
                npts: 5,
                filter: PeakFilter::None,
                ..Default::default()
            },
            ..Default::default()
        };
        let r = analyze_peaks(&x, &y, &o).unwrap();
        assert_eq!(r.peaks.len(), 2, "{:?}", r.peaks.iter().map(|p| p.center).collect::<Vec<_>>());
    }

    #[test]
    fn width_filter_drops_narrow_spikes() {
        let x = grid();
        let y: Vec<f64> = x.iter().map(|&x| gauss(x, 6.0, 0.5, 1.0) + gauss(x, 14.0, 0.03, 0.9)).collect();
        let mut o = ChainOptions::default();
        assert_eq!(analyze_peaks(&x, &y, &o).unwrap().peaks.len(), 2);
        o.find.min_fwhm = 0.5;
        assert_eq!(analyze_peaks(&x, &y, &o).unwrap().peaks.len(), 1);
    }

    #[test]
    fn half_height_limits_and_edges() {
        let x = grid();
        let y: Vec<f64> = x.iter().map(|&x| gauss(x, 10.0, 0.5, 1.0)).collect();
        let o = ChainOptions { limits: LimitMode::HalfHeight, ..Default::default() };
        let r = analyze_peaks(&x, &y, &o).unwrap();
        let p = &r.peaks[0];
        assert!((p.right_x - p.left_x - 0.5 * 2.354_820).abs() < 2e-3);
        assert_eq!(analyze_peaks(&x[..1], &y[..1], &o), Err(ChainError::NoBaseline));
        let bad = ChainOptions {
            baseline: ChainBaseline::Anchors(vec![(1.0, 0.0)], InterpMethod::Linear),
            ..Default::default()
        };
        assert_eq!(analyze_peaks(&x, &y, &bad), Err(ChainError::NoBaseline));
    }

    fn two_peaks() -> (Vec<f64>, Vec<f64>) {
        let x = grid();
        let y = x.iter().map(|&x| 0.1 + gauss(x, 5.0, 0.4, 1.0) + gauss(x, 14.0, 0.6, 0.5)).collect();
        (x, y)
    }

    #[test]
    fn missing_end_point_and_inner_point_are_skipped() {
        let (x, mut y) = two_peaks();
        let clean =
            analyze_peaks(&x, &y, &ChainOptions { baseline: ChainBaseline::EndPoints, ..Default::default() }).unwrap();
        y[0] = f64::NAN;
        y[90] = f64::NAN; // inside the first peak
        let o = ChainOptions { baseline: ChainBaseline::EndPoints, ..Default::default() };
        let r = analyze_peaks(&x, &y, &o).unwrap();
        assert_eq!(r.peaks.len(), 2);
        assert!(r.baseline[0].is_nan() && r.baseline[1].is_finite());
        for (a, b) in r.peaks.iter().zip(&clean.peaks) {
            assert!(a.area.is_finite() && a.area_percent.is_finite());
            assert!((a.area - b.area).abs() < 2e-3 * b.area.abs(), "{} {}", a.area, b.area);
            assert_eq!(a.index, b.index);
        }
    }

    #[test]
    fn reversed_x_gives_the_same_positive_areas() {
        // No offset: on a flat plateau the valley point between the peaks is a tie.
        let (x, y0) = two_peaks();
        let y: Vec<f64> = y0.iter().map(|v| v - 0.1).collect();
        let fwd = analyze_peaks(&x, &y, &ChainOptions::default()).unwrap();
        let xr: Vec<f64> = x.iter().rev().copied().collect();
        let yr: Vec<f64> = y.iter().rev().copied().collect();
        let rev = analyze_peaks(&xr, &yr, &ChainOptions::default()).unwrap();
        assert_eq!(rev.peaks.len(), 2);
        let mut a: Vec<f64> = rev.peaks.iter().map(|p| p.area).collect();
        a.reverse();
        for (r, f) in a.iter().zip(&fwd.peaks) {
            assert!(*r > 0.0 && (r - f.area).abs() < 1e-9 * f.area, "{r} {}", f.area);
        }
    }

    #[test]
    fn unsorted_x_is_refused() {
        let (mut x, y) = two_peaks();
        x.swap(10, 300);
        assert_eq!(analyze_peaks(&x, &y, &ChainOptions::default()), Err(ChainError::UnsortedX));
    }

    /// Origin's Peak Analyzer, Integrate Peaks with its defaults (Constant baseline at the minimum),
    /// on two Gaussians over a slope (`re-workspace/scratch/t04`, `[Book1]Integration_Result1`).
    #[test]
    #[allow(clippy::excessive_precision)] // the digits as Origin printed them
    fn the_peak_analyzer_defaults_match_origin_where_the_rules_are_known() {
        let x: Vec<f64> = (0..401).map(|k| k as f64 * 0.05).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&x| {
                0.2 + 0.01 * x + (-(x - 5.0) * (x - 5.0) / 0.5).exp() + 0.6 * (-(x - 12.0) * (x - 12.0) / 0.8).exp()
            })
            .collect();
        let opts = ChainOptions { baseline: ChainBaseline::ConstantStat(BaselineStat::Minimum), ..Default::default() };
        let r = analyze_peaks(&x, &y, &opts).unwrap();
        assert_eq!(r.peaks.len(), 2);
        let (a, b) = (&r.peaks[0], &r.peaks[1]);
        assert_eq!((a.index + 1, b.index + 1), (101, 241), "Origin's Row Index");
        assert!((a.center - 5.0).abs() < 1e-12 && (b.center - 12.0).abs() < 1e-12);
        assert!((a.height - 1.05).abs() < 1e-12 && (b.height - 0.72).abs() < 1e-9);
        assert!((a.fwhm - 1.2206410202285873).abs() < 1e-9, "{}", a.fwhm);
        assert!((b.fwhm - 1.7137688378654659).abs() < 1e-9, "{}", b.fwhm);
        // Area % is of the whole corrected curve (total 4.2045): Origin 35.292 % for its area
        // 1.48386 over X 0.3 to 6.8; Sheaf's window starts at 0, so its area is a little larger.
        let total = 1.4838624273683825 / 0.35292138167211256;
        assert!((a.area_percent / 100.0 * total - a.area).abs() < 1e-3, "{} {}", a.area_percent, a.area);
        assert!((a.right_x - 6.8).abs() < 1e-9 && (b.left_x - 6.8).abs() < 1e-9, "the valley between them");
    }

    #[test]
    fn given_centres_replace_the_search() {
        let x = grid();
        // Two peaks on an offset of 3: the search would find both; give only the second, and a
        // centre between points, which snaps to the nearest one.
        let y: Vec<f64> = x.iter().map(|&x| 3.0 + gauss(x, 5.0, 0.4, 1.0) + gauss(x, 14.0, 0.6, 0.5)).collect();
        let r = analyze_peaks_at(&x, &y, &ChainOptions::default(), &[14.01, 13.99, f64::NAN]).unwrap();
        assert_eq!(r.peaks.len(), 1, "two centres on one point are one peak");
        assert_eq!(r.peaks[0].center, 14.0);
        // Measured above the lowest point (about 3), not above zero.
        assert!((r.peaks[0].height - 0.5).abs() < 1e-3, "{}", r.peaks[0].height);
        assert!((r.peaks[0].fwhm - 0.6 * 2.354_820).abs() < 0.02, "{}", r.peaks[0].fwhm);
        // A small bump that the height filter would drop is kept when given.
        let r = analyze_peaks_at(&x, &y, &ChainOptions::default(), &[5.0, 14.0]).unwrap();
        assert_eq!(r.peaks.iter().map(|p| p.center).collect::<Vec<_>>(), vec![5.0, 14.0]);
        assert!(analyze_peaks_at(&x, &y, &ChainOptions::default(), &[]).unwrap().peaks.is_empty());
    }
}
