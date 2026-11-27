//! Binned scatter (Origin 2027 adds binned scatter plots; not in Origin 2016, so the rules are Sheaf's own,
//! `docs/spec/statistics.md` section 31): X is cut into bins, either of equal width or holding equal numbers of
//! points, and each bin gives the mean, standard deviation, standard error and count of its Y values at the mean
//! of its X values. Dense XY data then read as one point per bin.

/// How X is cut into bins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinRule {
    /// Bins of this width with edges at its multiples (T501): `[k w, (k + 1) w)`; empty bins are left out.
    Width(f64),
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
    /// A width that cuts the X range into more than 10 000 bins (T501).
    TooManyBins,
    TooFewPoints(usize),
}

impl std::fmt::Display for BinScatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinScatterError::NoBins => f.write_str("the number of bins must be at least 1 (a bin width above 0)"),
            BinScatterError::TooManyBins => f.write_str("the bin width makes more than 10000 bins"),
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
        BinRule::Width(w) if !(w.is_finite() && w > 0.0) => Err(BinScatterError::NoBins),
        BinRule::Width(_) if points.is_empty() => Err(BinScatterError::TooFewPoints(0)),
        BinRule::Width(w) => {
            let first = (points[0].0 / w).floor();
            let last = (points[points.len() - 1].0 / w).floor();
            if last - first >= 10_000.0 {
                return Err(BinScatterError::TooManyBins);
            }
            let k = (last - first) as usize + 1;
            let mut groups: Vec<Vec<(f64, f64)>> = vec![Vec::new(); k];
            for p in &points {
                let b = ((p.0 / w).floor() - first).clamp(0.0, (k - 1) as f64) as usize;
                groups[b].push(*p);
            }
            Ok(groups
                .iter()
                .enumerate()
                .filter(|(_, g)| !g.is_empty())
                .map(|(b, g)| bin_of(g, (first + b as f64) * w, (first + b as f64 + 1.0) * w))
                .collect())
        }
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

/// The error bar of a bin mean (T501).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BinError {
    #[default]
    None,
    /// The standard deviation of the bin's Y values.
    Sd,
    /// The standard error of the mean.
    Se,
    /// The half-width of the 95 % confidence interval of the mean, `t(0.975, n - 1) SE`.
    Ci95,
}

impl BinError {
    /// The bar's half-length for `bin` (NaN for one point, or without bars).
    pub fn of(self, bin: &Bin) -> f64 {
        match self {
            BinError::None => f64::NAN,
            BinError::Sd => bin.y_sd,
            BinError::Se => bin.y_se,
            BinError::Ci95 => {
                use statrs::distribution::{ContinuousCDF, StudentsT};
                match StudentsT::new(0.0, 1.0, bin.n as f64 - 1.0) {
                    Ok(t) if bin.n > 1 => t.inverse_cdf(0.975) * bin.y_se,
                    _ => f64::NAN,
                }
            }
        }
    }
}

/// The least-squares line `y = a + b x` of the points (pairs with a missing value left out), `(a, b)`; NaN without two
/// different X values (T501).
pub fn linear_fit(x: &[f64], y: &[f64]) -> (f64, f64) {
    let pts: Vec<(f64, f64)> =
        x.iter().zip(y).filter(|(a, b)| a.is_finite() && b.is_finite()).map(|(a, b)| (*a, *b)).collect();
    let n = pts.len() as f64;
    let (mx, my) = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    if pts.len() < 2 || sxx <= 0.0 {
        return (f64::NAN, f64::NAN);
    }
    let b = sxy / sxx;
    (my - b * mx, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bins_of_a_width_error_bars_and_the_line() {
        // T501: edges at multiples of the width; empty bins left out.
        let x = [0.2, 0.9, 1.1, 3.5, 3.9, f64::NAN];
        let y = [1.0, 3.0, 5.0, 7.0, 9.0, 0.0];
        let b = binned_scatter(&x, &y, BinRule::Width(1.0)).unwrap();
        assert_eq!(
            b.iter().map(|b| (b.x_from, b.x_to, b.n)).collect::<Vec<_>>(),
            [(0.0, 1.0, 2), (1.0, 2.0, 1), (3.0, 4.0, 2)]
        );
        assert_eq!(binned_scatter(&x, &y, BinRule::Width(0.0)), Err(BinScatterError::NoBins));
        assert_eq!(binned_scatter(&x, &y, BinRule::Width(1e-6)), Err(BinScatterError::TooManyBins));
        // Error bars: SD 1.414..., SE 1, 95 % half-width t(0.975, 1) = 12.706...
        assert!((BinError::Sd.of(&b[0]) - 2f64.sqrt()).abs() < 1e-12);
        assert!((BinError::Se.of(&b[0]) - 1.0).abs() < 1e-12);
        assert!((BinError::Ci95.of(&b[0]) - 12.706204736174698).abs() < 1e-9, "{}", BinError::Ci95.of(&b[0]));
        assert!(BinError::Ci95.of(&b[1]).is_nan() && BinError::None.of(&b[0]).is_nan());
        // The line through (0, 1), (1, 3), (2, 5): y = 1 + 2 x.
        assert_eq!(linear_fit(&[0.0, 1.0, 2.0, f64::NAN], &[1.0, 3.0, 5.0, 9.0]), (1.0, 2.0));
        assert!(linear_fit(&[1.0, 1.0], &[1.0, 2.0]).0.is_nan());
    }

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
