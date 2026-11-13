//! Peak finding with the local-maximum method (Origin `pkfind`, `method = max`), with its smoothing
//! and foot options.
//!
//! A point is a positive peak when it is above zero, strictly larger than the `npts` points to its
//! left and not smaller than the `npts` points to its right (so a flat top is reported at its
//! first point); negative peaks are the mirror image (below zero). Points closer than `npts` to an
//! end of the data are never peaks. The height filter compares the magnitude of the value.
//!
//! With `smooth_points` above 1 the search (and the feet) work on a Savitzky-Golay curve of order 2
//! with the end points extrapolated; the height filter and the reported values use the data. The feet
//! of a peak are, on each side, the point of smallest magnitude between the peak and its neighbour
//! peak (or the end of the data), unless `foot_height` stops the walk earlier at the first point whose
//! magnitude is below it, and `max_half_width` limits how far from the peak a foot may lie.

/// Which peaks are wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PeakDirection {
    Positive,
    Negative,
    #[default]
    Both,
}

/// Peak filter (`filter` / `value`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeakFilter {
    /// Keep every candidate.
    None,
    /// Keep peaks whose magnitude is at least the value; 0 means 20 % of the largest magnitude of
    /// the data (Origin's default behaviour).
    Height(f64),
    /// Keep the given number of peaks with the largest magnitude.
    Number(usize),
}

/// Options of [`find_peaks`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakOptions {
    /// Neighbours compared on each side (Origin `npts`, default 2).
    pub npts: usize,
    pub direction: PeakDirection,
    pub filter: PeakFilter,
    /// Points of the smoothing window applied before the search (Origin `smooth`); 0 or 1 means none.
    /// An even number acts like the next odd one.
    pub smooth_points: usize,
    /// Threshold for the feet (Origin `fheight`): the walk away from a peak stops at the first point
    /// whose magnitude is below it. 0 means no threshold.
    pub foot_height: f64,
    /// Farthest distance in X of a foot from its peak (Origin `hwidth`); `None` means unlimited. A
    /// foot may lie at least two points away. The first point beyond the distance is still allowed.
    pub max_half_width: Option<f64>,
}

impl Default for PeakOptions {
    fn default() -> Self {
        PeakOptions {
            npts: 2,
            direction: PeakDirection::Both,
            filter: PeakFilter::Height(0.0),
            smooth_points: 0,
            foot_height: 0.0,
            max_half_width: None,
        }
    }
}

/// Peaks found, in ascending row order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PeakResult {
    /// 1-based row of each peak.
    pub rows: Vec<usize>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// 1-based row of the left foot of each peak.
    pub left: Vec<usize>,
    /// 1-based row of the right foot of each peak.
    pub right: Vec<usize>,
}

/// Finds peaks of `y(x)`.
pub fn find_peaks(x: &[f64], y: &[f64], opts: &PeakOptions) -> PeakResult {
    let n = x.len().min(y.len());
    let k = opts.npts.max(1);
    let smoothed: Option<Vec<f64>> = (opts.smooth_points >= 2).then(|| {
        let o = crate::SmoothOptions {
            method: crate::SmoothMethod::SavitzkyGolay { poly_order: 2 },
            npts: opts.smooth_points,
            boundary: crate::Boundary::Extrapolate,
        };
        crate::smooth(&y[..n], &o)
    });
    let s: &[f64] = smoothed.as_deref().unwrap_or(&y[..n]);
    let mut found: Vec<usize> = Vec::new();
    let want_pos = opts.direction != PeakDirection::Negative;
    let want_neg = opts.direction != PeakDirection::Positive;
    if n > 2 * k {
        for i in k..n - k {
            let v = s[i];
            if !v.is_finite() {
                continue;
            }
            let left = &s[i - k..i];
            let right = &s[i + 1..=i + k];
            let positive = want_pos && v > 0.0 && left.iter().all(|&a| v > a) && right.iter().all(|&a| v >= a);
            let negative = want_neg && v < 0.0 && left.iter().all(|&a| v < a) && right.iter().all(|&a| v <= a);
            if positive || negative {
                found.push(i);
            }
        }
    }
    match opts.filter {
        PeakFilter::None => {}
        PeakFilter::Height(h) => {
            let thr = if h == 0.0 {
                0.2 * y[..n].iter().filter(|a| a.is_finite()).fold(0.0_f64, |m, a| m.max(a.abs()))
            } else {
                h
            };
            found.retain(|&i| y[i].abs() >= thr);
        }
        PeakFilter::Number(m) => {
            found.sort_by(|&a, &b| y[b].abs().total_cmp(&y[a].abs()).then(a.cmp(&b)));
            found.truncate(m);
            found.sort_unstable();
        }
    }
    let mut left = Vec::with_capacity(found.len());
    let mut right = Vec::with_capacity(found.len());
    for (idx, &p) in found.iter().enumerate() {
        let before = if idx > 0 { found[idx - 1] } else { 0 };
        let after = found.get(idx + 1).copied().unwrap_or(n - 1);
        left.push(foot(x, s, p, before, false, opts) + 1);
        right.push(foot(x, s, p, after, true, opts) + 1);
    }
    PeakResult {
        rows: found.iter().map(|&i| i + 1).collect(),
        x: found.iter().map(|&i| x[i]).collect(),
        y: found.iter().map(|&i| y[i]).collect(),
        left,
        right,
    }
}

/// The foot of the peak `p` on one side. The search runs from the peak towards `limit` (the
/// neighbouring peak or the end of the data), stops early at `foot_height`, and is cut where the
/// distance in X from the peak reaches `max_half_width`. Without an early stop the foot is the point
/// of the searched stretch (the peak included) with the smallest magnitude; the one nearest the peak
/// wins ties.
fn foot(x: &[f64], s: &[f64], p: usize, limit: usize, rightwards: bool, opts: &PeakOptions) -> usize {
    let n = s.len();
    let mut candidates = vec![p];
    let mut j = p;
    loop {
        let next = if rightwards { j + 1 } else { j.wrapping_sub(1) };
        if next >= n || (rightwards && next > limit) || (!rightwards && next < limit) {
            break;
        }
        // The point may be used while the one before it is still inside the allowed distance; at
        // least two points on each side are always searched.
        if let Some(w) = opts.max_half_width
            && candidates.len() > 2
            && (x[j] - x[p]).abs() >= w * (1.0 - 1e-9)
        {
            break;
        }
        j = next;
        if opts.foot_height > 0.0 && s[j].abs() < opts.foot_height {
            return j;
        }
        candidates.push(j);
    }
    let mut best = p;
    for &c in &candidates {
        if s[c].abs() < s[best].abs() {
            best = c;
        }
    }
    best
}
