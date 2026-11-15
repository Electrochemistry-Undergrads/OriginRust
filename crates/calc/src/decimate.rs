//! Decimation as Origin's Analysis > Signal Processing > Decimation (`decimate`, measured;
//! `docs/spec/analysis-tools.md` 13.4): every `d`-th point, optionally after a triangular
//! smoothing.
//!
//! Measured rules:
//!
//! * The result has `ceil(n / d)` points; point `k` is row `k * d` of the input (X and Y).
//!   Factor 1 copies the data.
//! * Origin refuses (the X-Function returns false) fewer than 2 points, and for a factor of 3
//!   or more fewer than `10 * d` points. Factors 1 and 2 have no such limit.
//! * `ave` smooths with a triangle of half width `d`: the moving average of `d + 1` points
//!   applied twice (weights `1, 2, .., d + 1, .., 2, 1` over `(d + 1)^2`), centred on the kept
//!   row, missing rows before the start counted as 0.
//! * `fir` is the same triangle with a half width of `order` (default 30), whatever the factor.
//!   Origin takes an order from 8 to 60.
//! * The triangle is what Origin computes for the FIR filter only while `d` times the X step
//!   is above 0.5 (checked from 0.501 up to steps of 2, and `d` from 1 to 3). At 0.5 or below
//!   (60 points, `d = 1` at step 0.5, 0.25 and 0.1; `d = 2` at step 0.25) Origin's FIR result
//!   is another, oscillating filter that depends on that product and is not reproduced here:
//!   Sheaf refuses it instead of returning the triangle. The moving average is not affected.
//! * At the end the second pass reads the first pass only up to the last row, so the kept row
//!   whose window reaches past the end (the last one, when `n` is a multiple of `d`) loses the
//!   weight of the rows past the end: the filtered value of row `i` is the mean of
//!   `a[i..=i + h]` with `a[t]` the mean of rows `t - h..=t` (rows before the start 0) and
//!   `a[t] = 0` for `t >= n`.

/// The smoothing applied before taking every `d`-th point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecimateFilter {
    /// Origin's `none`.
    #[default]
    None,
    /// Origin's `ave`: the triangle of half width `d`.
    MovingAverage,
    /// Origin's `fir`: the triangle of half width `order`.
    Fir,
}

/// Origin's options of `decimate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimateOptions {
    /// Origin's `d`: keep every `factor`-th point.
    pub factor: usize,
    pub filter: DecimateFilter,
    /// Origin's `order`, used by the FIR filter only (8 to 60).
    pub order: usize,
}

impl Default for DecimateOptions {
    fn default() -> Self {
        DecimateOptions { factor: 2, filter: DecimateFilter::None, order: DEFAULT_ORDER }
    }
}

/// Origin's default `order`.
pub const DEFAULT_ORDER: usize = 30;
/// The range of `order` Origin takes.
pub const ORDER_RANGE: (usize, usize) = (8, 60);
/// From this factor on Origin wants `MIN_POINTS_PER_FACTOR * d` points.
pub const BIG_FACTOR: usize = 3;
pub const MIN_POINTS_PER_FACTOR: usize = 10;
/// Origin's FIR is the plain triangle only above this `d` times the X step.
pub const FIR_MIN_SPAN: f64 = 0.5;

/// What Origin's `decimate` refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecimateError {
    /// Fewer than 2 points.
    TooFewPoints,
    ZeroFactor,
    /// A factor of 3 or more needs `10 * d` points: `(factor, needed)`.
    FactorTooBig(usize, usize),
    /// The FIR order is outside 8 to 60.
    BadOrder(usize),
    /// FIR with `d` times the X step at 0.5 or below, which Origin filters differently.
    FirStepTooSmall,
}

impl std::fmt::Display for DecimateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecimateError::TooFewPoints => write!(f, "decimation needs at least 2 points"),
            DecimateError::ZeroFactor => write!(f, "the factor must be at least 1"),
            DecimateError::FactorTooBig(d, need) => {
                write!(f, "a factor of {d} needs at least {need} points")
            }
            DecimateError::FirStepTooSmall => {
                write!(f, "the FIR filter is not reproduced when the factor times the X step is {FIR_MIN_SPAN} or less")
            }
            DecimateError::BadOrder(o) => {
                write!(f, "the FIR order {o} is outside {} to {}", ORDER_RANGE.0, ORDER_RANGE.1)
            }
        }
    }
}

impl std::error::Error for DecimateError {}

/// The smoothed value at the `kept` rows of `y` for the triangle of half width `h`.
fn triangle(y: &[f64], h: usize, kept: impl Iterator<Item = usize>) -> Vec<f64> {
    let n = y.len();
    let b = h.saturating_add(1) as f64;
    // a[t]: mean of rows t-h..=t, rows before the start 0, defined for t < n only.
    let mut a = vec![0.0; n];
    let mut run = 0.0;
    for t in 0..n {
        run += y[t];
        if t > h {
            run -= y[t - h - 1];
        }
        a[t] = run / b;
    }
    kept.map(|i| {
        let end = i.saturating_add(h).min(n - 1);
        a[i..=end].iter().sum::<f64>() / b
    })
    .collect()
}

/// Every `factor`-th point of `(x, y)`, smoothed first as `options.filter` says. Only the rows
/// both slices have are used.
pub fn decimate(x: &[f64], y: &[f64], options: &DecimateOptions) -> Result<(Vec<f64>, Vec<f64>), DecimateError> {
    let n = y.len().min(x.len());
    let d = options.factor;
    if d == 0 {
        return Err(DecimateError::ZeroFactor);
    }
    if n < 2 {
        return Err(DecimateError::TooFewPoints);
    }
    // `n < 10 * d` without the product, which overflows for a huge factor.
    if d >= BIG_FACTOR && (d > n || n / MIN_POINTS_PER_FACTOR < d) {
        return Err(DecimateError::FactorTooBig(d, d.saturating_mul(MIN_POINTS_PER_FACTOR)));
    }
    let rows = || (0..n).step_by(d);
    let ys = match options.filter {
        DecimateFilter::None => rows().map(|i| y[i]).collect(),
        DecimateFilter::MovingAverage => triangle(&y[..n], d, rows()),
        DecimateFilter::Fir => {
            if !(ORDER_RANGE.0..=ORDER_RANGE.1).contains(&options.order) {
                return Err(DecimateError::BadOrder(options.order));
            }
            let step = (x[n - 1] - x[0]).abs() / (n - 1) as f64;
            if step * (d as f64) <= FIR_MIN_SPAN * (1.0 + 1e-9) {
                return Err(DecimateError::FirStepTooSmall);
            }
            triangle(&y[..n], options.order, rows())
        }
    };
    Ok((rows().map(|i| x[i]).collect(), ys))
}

#[cfg(test)]
#[allow(clippy::excessive_precision)] // the digits as Origin printed them
mod tests {
    use super::*;

    // Probe p11 (re-workspace scratch): 31 points at X = 1, 1.5, .. 16.
    const Y: [f64; 31] = [
        7.0, 11.0, 12.0, 4.0, 6.0, 1.0, 2.0, 4.0, 7.0, 16.0, 6.0, 12.0, 20.0, 0.0, 14.0, 15.0, 14.0, 12.0, 15.0, 18.0,
        6.0, 12.0, 2.0, 15.0, 7.0, 0.0, 8.0, 16.0, 13.0, 15.0, 12.0,
    ];

    fn xs() -> Vec<f64> {
        (0..31).map(|i| 1.0 + 0.5 * i as f64).collect()
    }

    fn close(got: &[f64], want: &[f64]) {
        assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-12 * w.abs().max(1.0), "{got:?} vs {want:?}");
        }
    }

    fn run(factor: usize, filter: DecimateFilter, order: usize) -> (Vec<f64>, Vec<f64>) {
        decimate(&xs(), &Y, &DecimateOptions { factor, filter, order }).unwrap()
    }

    #[test]
    fn without_a_filter_every_dth_row_is_kept() {
        let (x, y) = run(3, DecimateFilter::None, 30);
        close(&x, &[1.0, 2.5, 4.0, 5.5, 7.0, 8.5, 10.0, 11.5, 13.0, 14.5, 16.0]);
        close(&y, &[7.0, 4.0, 2.0, 16.0, 20.0, 15.0, 15.0, 12.0, 7.0, 16.0, 12.0]);
        let (x, y) = run(1, DecimateFilter::None, 30);
        assert_eq!((x.len(), y), (31, Y.to_vec()));
    }

    #[test]
    fn the_moving_average_is_the_triangle_with_the_end_rule() {
        let (_, y) = run(3, DecimateFilter::MovingAverage, 30);
        close(&y, &[5.5625, 6.4375, 4.3125, 9.8125, 11.6875, 12.6875, 13.5625, 10.0, 7.5625, 11.0, 3.5]);
        let (_, y) = run(2, DecimateFilter::MovingAverage, 30);
        close(
            &y,
            &[
                6.1111111111111107,
                8.7777777777777768,
                4.6666666666666661,
                3.2222222222222219,
                7.6666666666666661,
                11.222222222222221,
                11.555555555555554,
                11.777777777777775,
                13.888888888888888,
                13.888888888888889,
                10.555555555555554,
                8.11111111111111,
                6.7777777777777768,
                8.4444444444444429,
                13.444444444444443,
                4.4444444444444438,
            ],
        );
    }

    #[test]
    fn the_fir_filter_is_the_triangle_of_the_order() {
        let (x, y) = run(2, DecimateFilter::Fir, 8);
        assert_eq!(x.len(), 16);
        close(
            &y,
            &[
                3.8765432098765431,
                5.0617283950617278,
                5.9012345679012332,
                6.7407407407407405,
                8.0740740740740744,
                9.30864197530864,
                10.76543209876543,
                11.654320987654316,
                12.086419753086417,
                11.555555555555554,
                10.753086419753084,
                10.209876543209875,
                7.5308641975308621,
                5.2098765432098748,
                3.1481481481481479,
                1.0864197530864197,
            ],
        );
    }

    #[test]
    fn what_origin_refuses_is_refused() {
        let o = |factor, filter, order| DecimateOptions { factor, filter, order };
        let none = DecimateFilter::None;
        let ok = |n: usize, o: DecimateOptions| {
            let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
            decimate(&x, &vec![0.0; n], &o)
        };
        assert_eq!(ok(1, o(2, none, 30)), Err(DecimateError::TooFewPoints));
        assert!(ok(2, o(2, none, 30)).is_ok());
        assert_eq!(ok(12, o(0, none, 30)), Err(DecimateError::ZeroFactor));
        assert!(ok(30, o(3, none, 30)).is_ok());
        assert_eq!(ok(29, o(3, none, 30)), Err(DecimateError::FactorTooBig(3, 30)));
        assert_eq!(ok(59, o(6, DecimateFilter::MovingAverage, 30)), Err(DecimateError::FactorTooBig(6, 60)));
        assert!(ok(60, o(2, DecimateFilter::Fir, 60)).is_ok());
        assert_eq!(ok(60, o(2, DecimateFilter::Fir, 61)), Err(DecimateError::BadOrder(61)));
        assert_eq!(ok(60, o(2, DecimateFilter::Fir, 7)), Err(DecimateError::BadOrder(7)));
        // The order does not matter without the FIR filter.
        assert!(ok(60, o(2, none, 1)).is_ok());
    }

    #[test]
    fn a_huge_factor_is_refused_not_overflowed() {
        let o = DecimateOptions { factor: usize::MAX, filter: DecimateFilter::MovingAverage, order: 30 };
        let x: Vec<f64> = (0..100).map(f64::from).collect();
        assert_eq!(decimate(&x, &x, &o), Err(DecimateError::FactorTooBig(usize::MAX, usize::MAX)));
        let o = DecimateOptions { factor: usize::MAX / 5, ..o };
        assert!(matches!(decimate(&x, &x, &o), Err(DecimateError::FactorTooBig(..))));
    }

    // Probe p14 (re-workspace scratch): 60 points at X = 1, 2, .. 60, `d = 1`, default order.
    const P4: [f64; 60] = [
        4.0, 18.0, 2.0, 8.0, 3.0, 15.0, 14.0, 15.0, 20.0, 12.0, 6.0, 3.0, 15.0, 0.0, 12.0, 13.0, 19.0, 0.0, 14.0, 8.0,
        7.0, 18.0, 3.0, 10.0, 0.0, 0.0, 0.0, 20.0, 17.0, 0.0, 12.0, 6.0, 13.0, 0.0, 16.0, 7.0, 14.0, 15.0, 17.0, 7.0,
        11.0, 7.0, 7.0, 14.0, 9.0, 0.0, 13.0, 17.0, 20.0, 3.0, 5.0, 20.0, 9.0, 3.0, 10.0, 16.0, 13.0, 16.0, 6.0, 9.0,
    ];

    #[test]
    fn fir_with_factor_one_is_the_order_30_triangle() {
        let x: Vec<f64> = (1..=60).map(f64::from).collect();
        let o = DecimateOptions { factor: 1, filter: DecimateFilter::Fir, order: 30 };
        let (_, y) = decimate(&x, &P4, &o).unwrap();
        assert_eq!(y.len(), 60);
        close(&y[..3], &[4.9989594172736727, 5.2965660770031215, 5.5702393340270548]);
        close(&y[28..32], &[9.4578563995837648, 9.4984391259105063, 9.2112382934443264, 8.9115504682622237]);
        close(&y[57..], &[1.0145681581685744, 0.66389177939646193, 0.327783558792924]);
    }

    #[test]
    fn fir_is_refused_where_origin_filters_differently() {
        let fir = |factor| DecimateOptions { factor, filter: DecimateFilter::Fir, order: 30 };
        let at = |step: f64, factor| {
            let x: Vec<f64> = (0..60).map(|i| 1.0 + step * i as f64).collect();
            decimate(&x, &P4, &fir(factor))
        };
        // Origin's own result at step 0.5 starts 3.8274402148869662, 5.6599095268221635, not the triangle.
        assert_eq!(at(0.5, 1), Err(DecimateError::FirStepTooSmall));
        assert_eq!(at(0.25, 2), Err(DecimateError::FirStepTooSmall));
        assert!(at(0.501, 1).is_ok());
        assert!(at(0.2, 3).is_ok());
        assert!(at(2.0, 1).is_ok());
    }
}
