//! Two-dimensional binning: how many (x, y) pairs fall into each cell of a grid of bins (the 2D
//! frequency count behind a heat map).
//!
//! Each axis is binned with the rules of [`crate::freq`]: the bin range and size can be left on automatic,
//! given as a width, or given as a number of equal bins. A cell holds the pairs whose x is at or above the
//! lower edge of its x bin and below the upper one, and likewise for y; as in the one-dimensional counts, the
//! last bin of an axis whose range is the data range cut into a number of bins also holds its upper edge.
//! Pairs outside the grid are not counted. Rows where either value is NaN are left out.

use crate::freq::{BinLayout, BinSize, bin_layout};

/// The most bins one axis may have.
pub const MAX_BINS_PER_AXIS: usize = 1000;

/// How one axis is binned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisBins {
    /// First bin edge; `None` takes the automatic or data minimum (see [`crate::freq::FreqOptions`]).
    pub min: Option<f64>,
    /// End of the last bin; `None` takes the automatic or data maximum.
    pub max: Option<f64>,
    pub size: BinSize,
}

impl Default for AxisBins {
    fn default() -> Self {
        AxisBins { min: None, max: None, size: BinSize::Auto }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bin2dOptions {
    pub x: AxisBins,
    pub y: AxisBins,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bin2d {
    /// Bin edges along x, one more than there are bins.
    pub x_edges: Vec<f64>,
    pub y_edges: Vec<f64>,
    /// `counts[ix][iy]`: the pairs in x bin `ix` and y bin `iy`.
    pub counts: Vec<Vec<usize>>,
    /// Rows where both values are numbers.
    pub pairs: usize,
    /// Pairs that fell inside the grid (the sum of the counts).
    pub counted: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bin2dError {
    /// No row has a number in both columns.
    NoPairs,
    /// The range or the bin size of the x axis is not usable.
    BadX,
    BadY,
    /// An axis would have more than [`MAX_BINS_PER_AXIS`] bins.
    TooManyBins,
}

impl std::fmt::Display for Bin2dError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bin2dError::NoPairs => write!(f, "no row has a number in both columns"),
            Bin2dError::BadX => write!(f, "the bin range or size of the X axis is not usable"),
            Bin2dError::BadY => write!(f, "the bin range or size of the Y axis is not usable"),
            Bin2dError::TooManyBins => write!(f, "an axis would have more than {MAX_BINS_PER_AXIS} bins"),
        }
    }
}

impl Bin2d {
    pub fn x_centers(&self) -> Vec<f64> {
        self.x_edges.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect()
    }

    pub fn y_centers(&self) -> Vec<f64> {
        self.y_edges.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect()
    }

    /// The number of pairs in each x bin (all y bins together).
    pub fn x_totals(&self) -> Vec<usize> {
        self.counts.iter().map(|row| row.iter().sum()).collect()
    }

    pub fn y_totals(&self) -> Vec<usize> {
        let ny = self.y_edges.len() - 1;
        (0..ny).map(|j| self.counts.iter().map(|row| row[j]).sum()).collect()
    }
}

fn edges(l: &BinLayout) -> Vec<f64> {
    (0..=l.bins).map(|i| if i == l.bins { l.end } else { l.start + l.inc * i as f64 }).collect()
}

/// The bin of `v` on the axis with edges `e` (`last_closed` as in [`BinLayout`]), if it lies inside.
fn locate(v: f64, l: &BinLayout, e: &[f64]) -> Option<usize> {
    let n = l.bins;
    let guess = ((v - l.start) / l.inc).floor();
    let mut i = if guess.is_nan() { return None } else { guess.clamp(0.0, (n - 1) as f64) as usize };
    // The guess can be one off at an edge because of rounding; the edges decide.
    while i > 0 && v < e[i] {
        i -= 1;
    }
    while i + 1 < n && v >= e[i + 1] {
        i += 1;
    }
    let inside = v >= e[i] && (v < e[i + 1] || (l.last_closed && i + 1 == n && v <= e[n]));
    inside.then_some(i)
}

/// Counts the pairs `(x[i], y[i])` in the cells of the grid. The slices should have the same length; the
/// shorter one decides.
pub fn bin_2d(x: &[f64], y: &[f64], opts: &Bin2dOptions) -> Result<Bin2d, Bin2dError> {
    let (xs, ys): (Vec<f64>, Vec<f64>) =
        x.iter().zip(y).filter(|(a, b)| !a.is_nan() && !b.is_nan()).map(|(a, b)| (*a, *b)).unzip();
    if xs.is_empty() {
        return Err(Bin2dError::NoPairs);
    }
    let lx = bin_layout(&xs, opts.x.min, opts.x.max, opts.x.size).ok_or(Bin2dError::BadX)?;
    let ly = bin_layout(&ys, opts.y.min, opts.y.max, opts.y.size).ok_or(Bin2dError::BadY)?;
    if lx.bins > MAX_BINS_PER_AXIS || ly.bins > MAX_BINS_PER_AXIS {
        return Err(Bin2dError::TooManyBins);
    }
    let (ex, ey) = (edges(&lx), edges(&ly));
    let mut counts = vec![vec![0usize; ly.bins]; lx.bins];
    let mut counted = 0;
    for (a, b) in xs.iter().zip(&ys) {
        if let (Some(i), Some(j)) = (locate(*a, &lx, &ex), locate(*b, &ly, &ey)) {
            counts[i][j] += 1;
            counted += 1;
        }
    }
    Ok(Bin2d { x_edges: ex, y_edges: ey, counts, pairs: xs.len(), counted })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_by_number_of_bins_include_the_maximum() {
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y = [0.0, 0.0, 5.0, 10.0, 10.0];
        let o = Bin2dOptions {
            x: AxisBins { size: BinSize::Count(2), ..Default::default() },
            y: AxisBins { size: BinSize::Count(2), ..Default::default() },
        };
        let r = bin_2d(&x, &y, &o).unwrap();
        assert_eq!(r.x_edges, vec![0.0, 2.0, 4.0]);
        assert_eq!(r.y_edges, vec![0.0, 5.0, 10.0]);
        // (0,0) (1,0) in cell (0,0); (2,5) in (1,1); (3,10) and (4,10) in (1,1).
        assert_eq!(r.counts, vec![vec![2, 0], vec![0, 3]]);
        assert_eq!((r.pairs, r.counted), (5, 5));
        assert_eq!(r.x_totals(), vec![2, 3]);
        assert_eq!(r.y_totals(), vec![2, 3]);
    }

    #[test]
    fn width_and_range_leave_pairs_outside() {
        let x = [0.5, 1.5, 2.5, 9.0, f64::NAN];
        let y = [0.5, 0.5, 0.5, 0.5, 0.5];
        let o = Bin2dOptions {
            x: AxisBins { min: Some(0.0), max: Some(3.0), size: BinSize::Width(1.0) },
            y: AxisBins { min: Some(0.0), max: Some(1.0), size: BinSize::Width(1.0) },
        };
        let r = bin_2d(&x, &y, &o).unwrap();
        assert_eq!(r.counts, vec![vec![1], vec![1], vec![1]]);
        assert_eq!((r.pairs, r.counted), (4, 3));
    }

    #[test]
    fn errors_are_named() {
        assert_eq!(bin_2d(&[f64::NAN], &[1.0], &Bin2dOptions::default()), Err(Bin2dError::NoPairs));
        let o = Bin2dOptions { y: AxisBins { size: BinSize::Width(-1.0), ..Default::default() }, ..Default::default() };
        assert_eq!(bin_2d(&[1.0, 2.0], &[1.0, 2.0], &o), Err(Bin2dError::BadY));
        let o = Bin2dOptions { x: AxisBins { size: BinSize::Width(1e-9), ..Default::default() }, ..Default::default() };
        assert_eq!(bin_2d(&[0.0, 1.0], &[1.0, 2.0], &o), Err(Bin2dError::TooManyBins));
    }
}
