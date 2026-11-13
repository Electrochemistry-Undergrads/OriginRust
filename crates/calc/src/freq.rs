//! Frequency counts: how many values fall into each of a row of bins (Origin's `freqcounts`), the
//! numbers behind a histogram.
//!
//! A bin holds the values from its lower edge up to, but not including, its upper edge, so a value that
//! sits exactly on the very last edge is not counted (as in Origin). When the bin range and
//! size are left on automatic, the first bin starts below the smallest value and the last bin ends above
//! the largest, with a margin of one tenth of a bin; the bin size is a round number. Origin's own rule for
//! the automatic size is not published; the rule used here is a fit to Origin's answers and agrees with
//! them in about three of five cases (see `docs/spec/statistics.md`).

/// How the bin size is chosen when the caller does not give the range as well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinSize {
    /// From the number of points and the range of the data.
    Auto,
    /// A fixed width.
    Width(f64),
    /// A number of bins of equal width over the range.
    Count(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreqOptions {
    /// First bin edge; `None` takes the smallest value (rounded down to a multiple of the bin size).
    pub min: Option<f64>,
    /// End of the last bin; `None` takes the largest value (rounded up to a multiple of the bin size).
    pub max: Option<f64>,
    pub size: BinSize,
    /// Add a row for the values below the first edge.
    pub below: bool,
    /// Add a row for the values at or above the last edge. (Origin shows these two rows with one
    /// oddity that is not copied: the largest value of the data goes into the row below when it lies
    /// beyond the range and both rows are on.)
    pub above: bool,
}

impl Default for FreqOptions {
    fn default() -> Self {
        FreqOptions { min: None, max: None, size: BinSize::Auto, below: false, above: false }
    }
}

/// One bin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreqRow {
    pub center: f64,
    /// Upper edge; NaN for the open row of values above the range.
    pub end: f64,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Freq {
    pub rows: Vec<FreqRow>,
    pub bin_start: f64,
    pub bin_size: f64,
    /// Number of values that were counted in the rows (the base of the relative frequencies).
    pub total: usize,
}

impl Freq {
    pub fn cumulative(&self) -> Vec<usize> {
        self.rows
            .iter()
            .scan(0, |acc, r| {
                *acc += r.count;
                Some(*acc)
            })
            .collect()
    }

    /// Count divided by `total`.
    pub fn relative(&self) -> Vec<f64> {
        self.rows.iter().map(|r| r.count as f64 / self.total as f64).collect()
    }

    pub fn cumulative_relative(&self) -> Vec<f64> {
        self.cumulative().iter().map(|c| *c as f64 / self.total as f64).collect()
    }
}

/// A number rounded half up to one significant digit.
fn round_one_digit(x: f64) -> f64 {
    let e = x.abs().log10().floor();
    let unit = 10f64.powf(e);
    ((x / unit) + 0.5 + 1e-9).floor() * unit
}

/// The automatic bin size for `n` values between `min` and `max`.
pub fn auto_bin_size(n: usize, min: f64, max: f64) -> f64 {
    let range = max - min;
    if range.is_nan() || range <= 0.0 {
        let scale = if min == 0.0 { 1.0 } else { 10f64.powf(min.abs().log10().floor()) };
        return scale;
    }
    let k = (1.75 * (n.max(2) as f64).ln() - 0.25).max(1.0);
    round_one_digit(range / k)
}

/// First and last edge of the automatic bin range for bins of width `inc`: the data range widened to
/// multiples of `inc`, and by one more bin where the data come within about a tenth of a bin of an edge
/// (0.1 at the start, 0.08 at the end: the thresholds that fit Origin's answers best).
pub fn auto_bin_range(min: f64, max: f64, inc: f64) -> (f64, f64) {
    let mut start = (min / inc + 1e-9).floor() * inc;
    if min - start < 0.1 * inc - 1e-12 * inc.abs() {
        start -= inc;
    }
    let mut end = (max / inc - 1e-9).ceil() * inc;
    if end - max < 0.08 * inc - 1e-12 * inc.abs() {
        end += inc;
    }
    (start, end)
}

/// Frequency counts of `data` (NaN entries are ignored). `None` if there are no numbers, or the range or
/// the bin size is not usable.
pub fn freq_counts(data: &[f64], opts: &FreqOptions) -> Option<Freq> {
    let v: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
    if v.is_empty() {
        return None;
    }
    let dmin = v.iter().copied().fold(f64::INFINITY, f64::min);
    let dmax = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut last_closed = false;
    let (start, inc, bins) = match (opts.min, opts.max, opts.size) {
        (Some(lo), Some(hi), BinSize::Count(c)) if c > 0 && hi > lo => (lo, (hi - lo) / c as f64, c),
        (lo, hi, BinSize::Count(c)) if c > 0 => {
            let (lo, hi) = (lo.unwrap_or(dmin), hi.unwrap_or(dmax));
            if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) {
                return None;
            }
            last_closed = opts.max.is_none();
            (lo, (hi - lo) / c as f64, c)
        }
        (_, _, BinSize::Count(_)) => return None,
        (lo, hi, size) => {
            let inc = match size {
                BinSize::Width(w) if w > 0.0 && w.is_finite() => w,
                BinSize::Width(_) => return None,
                _ => auto_bin_size(v.len(), lo.unwrap_or(dmin), hi.unwrap_or(dmax)),
            };
            let (auto_start, auto_end) = auto_bin_range(dmin, dmax, inc);
            let start = lo.unwrap_or(auto_start);
            let end = hi.unwrap_or(auto_end);
            if end.partial_cmp(&start) != Some(std::cmp::Ordering::Greater) {
                return None;
            }
            (start, inc, (((end - start) / inc) + 0.5).floor().max(1.0) as usize)
        }
    };
    let edge = |i: usize| start + inc * i as f64;
    let mut counts = vec![0usize; bins];
    for x in &v {
        for (i, c) in counts.iter_mut().enumerate() {
            let (lo, hi) = (edge(i), edge(i + 1));
            if *x >= lo && (*x < hi || (last_closed && i + 1 == bins && *x <= hi)) {
                *c += 1;
                break;
            }
        }
    }
    let mut rows = Vec::with_capacity(bins + 2);
    if opts.below {
        rows.push(FreqRow { center: start - inc / 2.0, end: start, count: v.iter().filter(|x| **x < start).count() });
    }
    for (i, c) in counts.iter().enumerate() {
        rows.push(FreqRow { center: start + inc * (i as f64 + 0.5), end: edge(i + 1), count: *c });
    }
    if opts.above {
        let last = edge(bins);
        rows.push(FreqRow { center: last + inc / 2.0, end: f64::NAN, count: v.iter().filter(|x| **x >= last).count() });
    }
    let total = rows.iter().map(|r| r.count).sum();
    Some(Freq { rows, bin_start: start, bin_size: inc, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(f: &Freq) -> Vec<usize> {
        f.rows.iter().map(|r| r.count).collect()
    }

    #[test]
    fn a_value_on_the_last_edge_is_not_counted() {
        let x: Vec<f64> = (0..=10).map(f64::from).collect();
        let f = freq_counts(
            &x,
            &FreqOptions { min: Some(0.0), max: Some(10.0), size: BinSize::Width(2.0), ..Default::default() },
        )
        .unwrap();
        assert_eq!(counts(&f), vec![2, 2, 2, 2, 2]);
        assert_eq!(f.cumulative().last(), Some(&10));
        assert!((f.relative()[0] - 0.2).abs() < 1e-15);
    }

    #[test]
    fn automatic_range_leaves_a_margin() {
        // Smallest value 9.927 with bins of 0.9: the edge 9.9 is too close, so the range starts at 9.0.
        // The largest value 13.423 is 0.077 below the edge 13.5, more than 0.08 of a bin, so the end stays.
        let (s, e) = auto_bin_range(9.926854, 13.423311, 0.9);
        assert!((s - 9.0).abs() < 1e-9 && (e - 13.5).abs() < 1e-9, "{s} {e}");
        // A largest value 0.05 below the edge pushes the end out by one bin.
        let (_, e) = auto_bin_range(9.926854, 13.45, 0.9);
        assert!((e - 14.4).abs() < 1e-9, "{e}");
    }

    #[test]
    fn number_of_bins_gives_equal_widths() {
        let x: Vec<f64> = (0..=10).map(f64::from).collect();
        let f = freq_counts(
            &x,
            &FreqOptions { min: Some(0.0), max: Some(10.0), size: BinSize::Count(4), ..Default::default() },
        )
        .unwrap();
        assert!((f.bin_size - 2.5).abs() < 1e-15);
        assert_eq!(counts(&f), vec![3, 2, 3, 2]);
    }

    #[test]
    fn no_numbers_no_result() {
        assert!(freq_counts(&[f64::NAN], &FreqOptions::default()).is_none());
        assert!(freq_counts(&[1.0, 2.0], &FreqOptions { size: BinSize::Width(-1.0), ..Default::default() }).is_none());
    }
}
