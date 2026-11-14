//! 2D frequency counts and binning (Origin's `twoDBinning`, Statistics > Descriptive Statistics >
//! 2D Frequency Counts/Binning): the points (x, y) are sorted into a grid of X bins and Y bins, and each
//! cell gets the count of its points, their percentage of all binned points, or the sum, mean,
//! minimum, maximum or median of their Y values. (Origin takes the statistic over Y even when the input
//! has a third column, also one designated Z, so there is no Z here.)
//!
//! A bin holds the values from its lower edge up to, but not including, its upper edge, so a point
//! on the last edge of an axis is not counted (as in Origin, and as in [`crate::freq_counts`]). The
//! rules, measured on Origin, are in `docs/spec/statistics.md` section 13.

use crate::freq::{BinSize, auto_bin_range, auto_bin_size};

/// What each cell of the grid holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bin2dStat {
    #[default]
    Count,
    /// The count as a percentage of all points that fell into a bin of the grid.
    PercentFrequency,
    Sum,
    Mean,
    Min,
    Max,
    Median,
}

impl Bin2dStat {
    pub const ALL: [Bin2dStat; 7] = [
        Bin2dStat::Count,
        Bin2dStat::PercentFrequency,
        Bin2dStat::Sum,
        Bin2dStat::Mean,
        Bin2dStat::Min,
        Bin2dStat::Max,
        Bin2dStat::Median,
    ];

    /// The long name Origin gives the result columns.
    pub fn label(self) -> &'static str {
        match self {
            Bin2dStat::Count => "Count",
            Bin2dStat::PercentFrequency => "Percent Frequency",
            Bin2dStat::Sum => "Sum",
            Bin2dStat::Mean => "Mean",
            Bin2dStat::Min => "Minimum",
            Bin2dStat::Max => "Maximum",
            Bin2dStat::Median => "Median",
        }
    }

    /// The name of Origin's `stats` option.
    pub fn code(self) -> &'static str {
        match self {
            Bin2dStat::Count => "count",
            Bin2dStat::PercentFrequency => "freq",
            Bin2dStat::Sum => "sum",
            Bin2dStat::Mean => "mean",
            Bin2dStat::Min => "min",
            Bin2dStat::Max => "max",
            Bin2dStat::Median => "median",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        Bin2dStat::ALL.into_iter().find(|k| k.code().eq_ignore_ascii_case(s))
    }

    /// Whether the statistic reads the values (rather than only counting the points).
    pub fn uses_values(self) -> bool {
        !matches!(self, Bin2dStat::Count | Bin2dStat::PercentFrequency)
    }
}

/// How the bins of one axis are laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisBins {
    /// First bin edge (or the centre of the first bin, with `centers`); `None` from the data.
    pub min: Option<f64>,
    /// End of the last bin (or its centre, with `centers`); `None` from the data.
    pub max: Option<f64>,
    /// A width, a number of bins over the range, or automatic.
    pub size: BinSize,
    /// `min` and `max` are the centres of the first and the last bin, and the bins are labelled by
    /// their centres (Origin's Bin Centers); otherwise they are edges and the bins are labelled by their
    /// upper ends (Bin Ends, the default).
    pub centers: bool,
    /// Add a bin for the values below the first edge.
    pub below: bool,
    /// Add a bin for the values at or above the last edge.
    pub above: bool,
}

impl Default for AxisBins {
    fn default() -> Self {
        AxisBins { min: None, max: None, size: BinSize::Auto, centers: false, below: false, above: false }
    }
}

/// One bin of an axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bin {
    /// Lower edge (`-inf` for the bin of values below the range).
    pub lo: f64,
    /// Upper edge (`+inf` for the bin of values above the range).
    pub hi: f64,
}

impl Bin {
    fn holds(&self, v: f64) -> bool {
        v >= self.lo && v < self.hi
    }
}

/// The bins of one axis, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisLayout {
    pub bins: Vec<Bin>,
    /// The bins are labelled by their centres.
    pub centers: bool,
    pub width: f64,
}

impl AxisLayout {
    /// The number a row or a column of the result is known by: the upper end of the bin, or its centre.
    /// The open bins below and above the range are labelled half a bin beyond their closed edge.
    pub fn label(&self, i: usize) -> f64 {
        let b = self.bins[i];
        let (lo, hi) = match (b.lo.is_finite(), b.hi.is_finite()) {
            (true, true) => (b.lo, b.hi),
            (false, _) => (b.hi - self.width, b.hi),
            (_, false) => (b.lo, b.lo + self.width),
        };
        if self.centers { (lo + hi) / 2.0 } else { hi }
    }

    /// "0 - 5": the range of a bin as Origin writes it in the comments of a result column (up to five
    /// decimals: "3.33333 - 6.66667", "20 - 36.66667"); the open bins read "< 0" and ">= 10".
    pub fn range_text(&self, i: usize) -> String {
        let b = self.bins[i];
        match (b.lo.is_finite(), b.hi.is_finite()) {
            (true, true) => format!("{} - {}", edge_text(b.lo), edge_text(b.hi)),
            (false, _) => format!("< {}", edge_text(b.hi)),
            (_, false) => format!(">= {}", edge_text(b.lo)),
        }
    }

    fn find(&self, v: f64) -> Option<usize> {
        self.bins.iter().position(|b| b.holds(v))
    }
}

/// A bin edge as Origin writes it in the column comments: up to five decimals without trailing zeros, and
/// numbers too large or too small for that as C's `%g`.
pub fn edge_text(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let a = v.abs();
    if !(1e-5..1e15).contains(&a) {
        let s = format!("{:.5e}", v);
        let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
        let m = m.trim_end_matches('0').trim_end_matches('.');
        let e: i32 = e.parse().unwrap_or(0);
        return format!("{m}e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs());
    }
    let s = format!("{v:.5}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".to_string() } else { s.to_string() }
}

/// `x` rounded up to one significant digit (16.1 gives 20, 8.05 gives 9).
fn ceil_one_digit(x: f64) -> f64 {
    let unit = 10f64.powf(x.abs().log10().floor());
    (x / unit - 1e-9).ceil() * unit
}

/// The first edge and the number of bins of a given width `w`: a given `min` is the first edge;
/// otherwise the smallest value rounded down to a multiple of `w`, one bin lower when the value is within
/// a tenth of a bin of that edge. A given `max` ends the range
/// in a whole number of bins (rounded); otherwise the bins cover the largest value plus half a bin. (At
/// the top this differs from the 1D rule of `freq::auto_bin_range`, which Origin uses when the size is
/// automatic too.)
fn edge_range(dmin: f64, dmax: f64, min: Option<f64>, max: Option<f64>, w: f64) -> Option<(f64, usize)> {
    let lo = match min {
        Some(m) => m,
        None => {
            let s = (dmin / w + 1e-9).floor() * w;
            if dmin - s < 0.1 * w - 1e-12 * w { s - w } else { s }
        }
    };
    let steps = match max {
        Some(m) if m > lo => (((m - lo) / w) + 0.5).floor().max(1.0),
        None if dmax >= lo => (((dmax - lo) / w) + 0.5).floor() + 1.0,
        _ => return None,
    };
    (lo.is_finite() && steps <= 1e6).then_some((lo, steps as usize))
}

/// With bin centres, given limits are the centres of the first and the last bin (a limit found from the
/// data is found as for the ends, then taken as a centre too); with both left automatic the bins are those
/// of the ends, labelled by their centres.
fn centred(opts: &AxisBins, lo: f64, w: f64, steps: usize) -> (f64, f64, usize) {
    if opts.centers && (opts.min.is_some() || opts.max.is_some()) {
        (lo - w / 2.0, w, steps + 1)
    } else {
        (lo, w, steps)
    }
}

/// Lays out the bins of one axis for the values `data` (NaN ignored). `None` when the range or the
/// size cannot be used. The rules were measured on Origin's `twoDBinning` (statistics.md 13).
pub fn axis_layout(data: &[f64], opts: &AxisBins) -> Option<AxisLayout> {
    let v: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    let dmin = v.iter().copied().fold(f64::INFINITY, f64::min);
    let dmax = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (start, width, n) = match opts.size {
        BinSize::Count(c) if c > 0 => match (opts.min, opts.max) {
            // Given limits: the centres run from the first to the last, or the edges do.
            (Some(lo), Some(hi)) if hi > lo => {
                if opts.centers {
                    let w = (hi - lo) / (c.max(2) - 1) as f64;
                    (lo - w / 2.0, w, c)
                } else {
                    (lo, (hi - lo) / c as f64, c)
                }
            }
            (Some(_), Some(_)) => return None,
            // One limit given: the other is the data's extreme rounded outwards to the automatic bin
            // size.
            (Some(lo), None) => {
                let a = auto_bin_size(v.len(), lo, dmax);
                let hi = (dmax / a - 1e-9).ceil() * a;
                if hi <= lo {
                    return None;
                }
                (lo, (hi - lo) / c as f64, c)
            }
            (None, Some(hi)) => {
                let a = auto_bin_size(v.len(), dmin, hi);
                let lo = (dmin / a + 1e-9).floor() * a;
                if hi <= lo {
                    return None;
                }
                (lo, (hi - lo) / c as f64, c)
            }
            // Automatic range: bins of the data range divided by their number, rounded up to one
            // significant digit, centred on the data and the first edge rounded down to the unit of
            // that digit. (Origin chose another first edge in 2 of the 8 measured axes.)
            (None, None) => {
                if v.is_empty() {
                    return None;
                }
                let range = dmax - dmin;
                let w = if range > 0.0 { ceil_one_digit(range / c as f64) } else { 1.0 };
                let unit = 10f64.powf(w.log10().floor());
                let centred = dmin - (w * c as f64 - range) / 2.0;
                ((centred / unit + 1e-9).floor() * unit, w, c)
            }
        },
        BinSize::Count(_) => return None,
        BinSize::Auto => {
            if v.is_empty() {
                return None;
            }
            // An automatic size comes with the 1D automatic range (`freq::auto_bin_range`), as for a
            // histogram; given limits replace either end.
            let w = auto_bin_size(v.len(), opts.min.unwrap_or(dmin), opts.max.unwrap_or(dmax));
            let (auto_lo, auto_hi) = auto_bin_range(dmin, dmax, w);
            let lo = opts.min.unwrap_or(auto_lo);
            let hi = opts.max.unwrap_or(auto_hi);
            if hi <= lo {
                return None;
            }
            let steps = (((hi - lo) / w) + 0.5).floor().max(1.0) as usize;
            centred(opts, lo, w, steps)
        }
        BinSize::Width(w) => {
            if !(w > 0.0 && w.is_finite()) {
                return None;
            }
            let (lo, steps) = edge_range(dmin, dmax, opts.min, opts.max, w)?;
            centred(opts, lo, w, steps)
        }
    };
    if !(start.is_finite() && width.is_finite() && width > 0.0) || n > 1_000_000 {
        return None;
    }
    let edge = |i: usize| start + width * i as f64;
    let mut bins = Vec::with_capacity(n + 2);
    if opts.below {
        bins.push(Bin { lo: f64::NEG_INFINITY, hi: start });
    }
    bins.extend((0..n).map(|i| Bin { lo: edge(i), hi: edge(i + 1) }));
    if opts.above {
        bins.push(Bin { lo: edge(n), hi: f64::INFINITY });
    }
    Some(AxisLayout { bins, centers: opts.centers, width })
}

/// Options of a 2D binning.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bin2dOptions {
    pub x: AxisBins,
    pub y: AxisBins,
    pub stat: Bin2dStat,
}

/// The result: `cells[i][j]` belongs to X bin `i` and Y bin `j`; empty cells are NaN for the
/// statistics of values and 0 for count and percentage.
#[derive(Debug, Clone, PartialEq)]
pub struct Bin2d {
    pub x: AxisLayout,
    pub y: AxisLayout,
    pub cells: Vec<Vec<f64>>,
    /// Number of points that fell into a cell of the grid.
    pub binned: usize,
    pub stat: Bin2dStat,
}

/// Bins the points `(x[i], y[i])`; the statistics read the Y values. A point with a missing x or y is
/// left out. `None` when no bin layout can be made.
pub fn bin2d(x: &[f64], y: &[f64], opts: &Bin2dOptions) -> Option<Bin2d> {
    let n = x.len().min(y.len());
    let rows: Vec<usize> = (0..n).filter(|&i| x[i].is_finite() && y[i].is_finite()).collect();
    let xs: Vec<f64> = rows.iter().map(|&i| x[i]).collect();
    let ys: Vec<f64> = rows.iter().map(|&i| y[i]).collect();
    let xl = axis_layout(&xs, &opts.x)?;
    let yl = axis_layout(&ys, &opts.y)?;
    let mut groups: Vec<Vec<Vec<f64>>> = vec![vec![Vec::new(); yl.bins.len()]; xl.bins.len()];
    let mut binned = 0;
    for &i in &rows {
        if let (Some(a), Some(b)) = (xl.find(x[i]), yl.find(y[i])) {
            groups[a][b].push(y[i]);
            binned += 1;
        }
    }
    let cells = groups
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|mut g| match opts.stat {
                    Bin2dStat::Count => g.len() as f64,
                    Bin2dStat::PercentFrequency => {
                        if binned == 0 {
                            0.0
                        } else {
                            g.len() as f64 / binned as f64 * 100.0
                        }
                    }
                    _ if g.is_empty() => f64::NAN,
                    Bin2dStat::Sum => g.iter().sum(),
                    Bin2dStat::Mean => g.iter().sum::<f64>() / g.len() as f64,
                    Bin2dStat::Min => g.iter().copied().fold(f64::INFINITY, f64::min),
                    Bin2dStat::Max => g.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    Bin2dStat::Median => {
                        g.sort_by(f64::total_cmp);
                        let m = g.len() / 2;
                        if g.len() % 2 == 1 { g[m] } else { (g[m - 1] + g[m]) / 2.0 }
                    }
                })
                .collect()
        })
        .collect();
    Some(Bin2d { x: xl, y: yl, cells, binned, stat: opts.stat })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_are_written_with_up_to_five_decimals() {
        assert_eq!(edge_text(10.0 / 3.0), "3.33333");
        assert_eq!(edge_text(110.0 / 3.0), "36.66667");
        assert_eq!(edge_text(-2.5), "-2.5");
        assert_eq!(edge_text(0.0), "0");
        assert_eq!(edge_text(-1e-9), "-1e-09");
        assert_eq!(edge_text(1234567.0), "1234567");
        assert_eq!(edge_text(2e15), "2e+15");
    }
}
