//! Binned scatter (Origin 2027 adds binned scatter plots; not in Origin 2016, so the rules are Sheaf's own,
//! `docs/spec/statistics.md` section 31): X is cut into bins, either of equal width or holding equal numbers of
//! points, and each bin gives the mean, standard deviation, standard error and count of its Y values at the mean
//! of its X values. Dense XY data then read as one point per bin.

/// How X is cut into bins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinRule {
    /// `n` bins of equal width from the smallest to the largest X (the last bin includes its right edge); empty
    /// bins are left out.
    EqualWidth(usize),
    /// `n` bins of (nearly) equal counts in X order: the first `count % n` bins hold one point more.
    EqualCount(usize),
}

impl Default for BinRule {
    fn default() -> Self {
        BinRule::EqualCount(10)
    }
}

/// One bin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bin {
    /// The bin's X interval: the edges for equal width, the smallest and largest X of its points for equal count.
    pub x_from: f64,
    pub x_to: f64,
    pub x_mean: f64,
    pub n: usize,
    pub y_mean: f64,
    /// Standard deviation (n - 1); NaN for one point.
    pub y_sd: f64,
    /// Standard error of the mean, `sd / sqrt n`; NaN for one point.
    pub y_se: f64,
}

/// Why the bins could not be made.
#[derive(Debug, Clone, PartialEq)]
pub enum BinScatterError {
    NoBins,
    TooFewPoints(usize),
}

impl std::fmt::Display for BinScatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinScatterError::NoBins => f.write_str("the number of bins must be at least 1"),
            BinScatterError::TooFewPoints(n) => write!(f, "{n} usable points are fewer than the bins asked for"),
        }
    }
}

impl std::error::Error for BinScatterError {}

fn bin_of(points: &[(f64, f64)], x_from: f64, x_to: f64) -> Bin {
    let n = points.len();
    let k = n as f64;
    let x_mean = points.iter().map(|p| p.0).sum::<f64>() / k;
    let y_mean = points.iter().map(|p| p.1).sum::<f64>() / k;
    let y_sd =
        if n > 1 { (points.iter().map(|p| (p.1 - y_mean).powi(2)).sum::<f64>() / (k - 1.0)).sqrt() } else { f64::NAN };
    Bin { x_from, x_to, x_mean, n, y_mean, y_sd, y_se: y_sd / k.sqrt() }
}

/// The bins of the points `(x[i], y[i])` (pairs with a missing value are left out) under `rule`.
pub fn binned_scatter(x: &[f64], y: &[f64], rule: BinRule) -> Result<Vec<Bin>, BinScatterError> {
    let mut points: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, b)| (*a, *b)).collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    match rule {
        BinRule::EqualWidth(0) | BinRule::EqualCount(0) => Err(BinScatterError::NoBins),
        BinRule::EqualCount(k) if points.len() < k => Err(BinScatterError::TooFewPoints(points.len())),
        BinRule::EqualWidth(_) if points.is_empty() => Err(BinScatterError::TooFewPoints(0)),
        BinRule::EqualCount(k) => {
            let (base, extra) = (points.len() / k, points.len() % k);
            let mut out = Vec::with_capacity(k);
            let mut at = 0;
            for b in 0..k {
                let size = base + usize::from(b < extra);
                let chunk = &points[at..at + size];
                out.push(bin_of(chunk, chunk[0].0, chunk[size - 1].0));
                at += size;
            }
            Ok(out)
        }
        BinRule::EqualWidth(k) => {
            let (lo, hi) = (points[0].0, points[points.len() - 1].0);
            let width = (hi - lo) / k as f64;
            let mut groups: Vec<Vec<(f64, f64)>> = vec![Vec::new(); k];
            for p in &points {
                let b = if width > 0.0 { (((p.0 - lo) / width) as usize).min(k - 1) } else { 0 };
                groups[b].push(*p);
            }
            Ok(groups
                .iter()
                .enumerate()
                .filter(|(_, g)| !g.is_empty())
                .map(|(b, g)| bin_of(g, lo + width * b as f64, lo + width * (b + 1) as f64))
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_counts_split_in_x_order_with_the_extra_points_first() {
        let x = [5.0, 1.0, 3.0, 2.0, 4.0, f64::NAN, 6.0, 7.0];
        let y = [50.0, 10.0, 30.0, 20.0, 40.0, 0.0, 60.0, 70.0];
        let b = binned_scatter(&x, &y, BinRule::EqualCount(3)).unwrap();
        assert_eq!(b.iter().map(|b| b.n).collect::<Vec<_>>(), [3, 2, 2]);
        assert_eq!((b[0].x_from, b[0].x_to, b[0].x_mean, b[0].y_mean), (1.0, 3.0, 2.0, 20.0));
        assert_eq!(b[0].y_sd, 10.0);
        assert!((b[0].y_se - 10.0 / 3f64.sqrt()).abs() < 1e-12);
        assert_eq!((b[2].x_from, b[2].x_to, b[2].y_mean), (6.0, 7.0, 65.0));
    }

    #[test]
    fn equal_widths_leave_out_empty_bins_and_keep_the_right_edge() {
        let x = [0.0, 0.5, 1.0, 9.0, 10.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = binned_scatter(&x, &y, BinRule::EqualWidth(5)).unwrap();
        assert_eq!(b.len(), 2, "{b:?}");
        assert_eq!((b[0].x_from, b[0].x_to, b[0].n, b[0].y_mean), (0.0, 2.0, 3, 2.0));
        assert_eq!((b[1].x_from, b[1].x_to, b[1].n), (8.0, 10.0, 2));
        assert!(b[1].y_sd.is_finite());
        let one = binned_scatter(&[3.0], &[4.0], BinRule::EqualWidth(4)).unwrap();
        assert_eq!((one.len(), one[0].n), (1, 1));
        assert!(one[0].y_sd.is_nan());
    }

    #[test]
    fn bad_requests_are_refused() {
        assert_eq!(binned_scatter(&[1.0], &[1.0], BinRule::EqualCount(0)), Err(BinScatterError::NoBins));
        assert_eq!(binned_scatter(&[1.0], &[1.0], BinRule::EqualCount(2)), Err(BinScatterError::TooFewPoints(1)));
        assert_eq!(binned_scatter(&[], &[], BinRule::EqualWidth(2)), Err(BinScatterError::TooFewPoints(0)));
    }
}
