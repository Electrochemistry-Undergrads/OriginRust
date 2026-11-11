//! Peak finding with the local-maximum method (Origin `pkfind`, `method = max`).
//!
//! A point is a positive peak when it is above zero, strictly larger than the `npts` points to its
//! left and not smaller than the `npts` points to its right (so a flat top is reported at its
//! first point); negative peaks are the mirror image (below zero). Points closer than `npts` to an
//! end of the data are never peaks. The height filter compares the magnitude of the value.

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
}

impl Default for PeakOptions {
    fn default() -> Self {
        PeakOptions { npts: 2, direction: PeakDirection::Both, filter: PeakFilter::Height(0.0) }
    }
}

/// Peaks found, in ascending row order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PeakResult {
    /// 1-based row of each peak.
    pub rows: Vec<usize>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// Finds peaks of `y(x)`.
pub fn find_peaks(x: &[f64], y: &[f64], opts: &PeakOptions) -> PeakResult {
    let n = x.len().min(y.len());
    let k = opts.npts.max(1);
    let mut found: Vec<usize> = Vec::new();
    let want_pos = opts.direction != PeakDirection::Negative;
    let want_neg = opts.direction != PeakDirection::Positive;
    if n > 2 * k {
        for i in k..n - k {
            let v = y[i];
            if !v.is_finite() {
                continue;
            }
            let left = &y[i - k..i];
            let right = &y[i + 1..=i + k];
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
    PeakResult {
        rows: found.iter().map(|&i| i + 1).collect(),
        x: found.iter().map(|&i| x[i]).collect(),
        y: found.iter().map(|&i| y[i]).collect(),
    }
}
