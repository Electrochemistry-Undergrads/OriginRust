//! The Image menu on a matrix sheet (T207, matrices spec 7i): picture adjustments, arithmetic,
//! conversion, geometric transforms and 3 x 3 spatial filters. Each works on the sheet's values as
//! gray levels and keeps the sheet's size, ranges and labels (Rotate by 90 or 270 degrees swaps the
//! size). Missing values stay missing. Origin's own formulas are not measured; these are Sheaf's,
//! written next to each operation.

use crate::matrix::{FlipDirection, MatrixError, MatrixSheet, Rotation};

/// One Image menu operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageOp {
    /// Adjustments > Brightness: adds `percent` (-100 to 100) of the sheet's range to every value and
    /// keeps the results inside the range, as Origin 2016's `imgBrightness a:=` does to the 0..255
    /// levels of an image (T278, measured: +10 adds 26 levels and saturates at 255).
    Brightness(f64),
    /// Adjustments > Contrast: stretches the values about their mean by `factor` (1 leaves them).
    Contrast(f64),
    /// Adjustments > Gamma: `lo + (hi - lo) * t^(1/gamma)`, with `t` the value's place between the
    /// sheet's lowest `lo` and highest `hi` value.
    Gamma(f64),
    /// Arithmetic Transform > Math: `value op operand`.
    Math(MathOp, f64),
    /// Conversion > To Binary: 1 where the value is between `low` and `high` (both included), else 0, as
    /// Origin 2016's `imgBinary t1:= t2:=` (T278, measured: levels outside the band turn black).
    ToBinary { low: f64, high: f64 },
    /// Geometric Transform > Rotate by a multiple of 90 degrees.
    Rotate(Rotation),
    /// Geometric Transform > Flip.
    Flip(FlipDirection),
    /// Spatial Filters > Smooth: the mean of each 3 x 3 neighbourhood.
    Smooth,
    /// Spatial Filters > Sharpen: `5 v - (the four neighbours)`.
    Sharpen,
    /// Spatial Filters > Edge Detection (T285): 1 where the filter's response, in levels of 0..255 over
    /// the sheet's range, is above `threshold`, else 0, as Origin 2016's `imgEdge t:= f:=` (measured:
    /// a step of k levels gives Sobel and Prewitt responses of exactly k).
    EdgeDetection { filter: EdgeFilter, threshold: f64 },
    /// Adjustments > Levels (T210): values from `input.0` to `input.1` are spread linearly over
    /// `output.0` to `output.1`; values outside the input range take the nearest end.
    Levels { input: (f64, f64), output: (f64, f64) },
    /// Geometric Transform > Crop (T210): the 0-based rows `rows.0..=rows.1` and columns
    /// `cols.0..=cols.1`, with the X and Y ranges of the cells kept.
    Crop { rows: (usize, usize), cols: (usize, usize) },
    /// Geometric Transform > Resize (T210): a new size, the ranges kept; nearest neighbour (Origin
    /// 2016's default `interpolate:=normal`, T282) or bilinear resampling end to end.
    Resize { rows: usize, cols: usize, bilinear: bool },
    /// Spatial Filters > Median (T284): the median of a `d` x `d` window, as Origin 2016's
    /// `imgMedian d:=` (measured: edge cells repeat outward; an even window reaches one cell further
    /// forward than back, and the lower of the two middle values is taken).
    Median(usize),
}

/// The edge filters of Edge Detection (Origin's `f:=` sh, sv, shv, ph, pv, phv). Horizontal finds
/// horizontal edges (a change from one row to the next), Vertical finds vertical ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeFilter {
    SobelHorizontal,
    SobelVertical,
    SobelBoth,
    PrewittHorizontal,
    PrewittVertical,
    PrewittBoth,
}

impl EdgeFilter {
    pub const ALL: [EdgeFilter; 6] = [
        EdgeFilter::SobelBoth,
        EdgeFilter::SobelHorizontal,
        EdgeFilter::SobelVertical,
        EdgeFilter::PrewittBoth,
        EdgeFilter::PrewittHorizontal,
        EdgeFilter::PrewittVertical,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EdgeFilter::SobelHorizontal => "Sobel Horizontal",
            EdgeFilter::SobelVertical => "Sobel Vertical",
            EdgeFilter::SobelBoth => "Sobel Horizontal/Vertical",
            EdgeFilter::PrewittHorizontal => "Prewitt Horizontal",
            EdgeFilter::PrewittVertical => "Prewitt Vertical",
            EdgeFilter::PrewittBoth => "Prewitt Horizontal/Vertical",
        }
    }
}

/// The largest Median window (cells each way).
pub const MAX_MEDIAN: usize = 31;

/// The operator of Arithmetic Transform > Math.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl ImageOp {
    /// The undo step's name.
    pub fn label(&self) -> &'static str {
        match self {
            ImageOp::Brightness(_) => "Image Brightness",
            ImageOp::Contrast(_) => "Image Contrast",
            ImageOp::Gamma(_) => "Image Gamma",
            ImageOp::Math(..) => "Image Math",
            ImageOp::ToBinary { .. } => "Image To Binary",
            ImageOp::Rotate(_) => "Image Rotate",
            ImageOp::Flip(_) => "Image Flip",
            ImageOp::Smooth => "Image Smooth",
            ImageOp::Sharpen => "Image Sharpen",
            ImageOp::EdgeDetection { .. } => "Image Edge Detection",
            ImageOp::Levels { .. } => "Image Levels",
            ImageOp::Crop { .. } => "Image Crop",
            ImageOp::Resize { .. } => "Image Resize",
            ImageOp::Median(_) => "Image Median",
        }
    }

    /// The operation applied to `s`; the result keeps the sheet's identity and labels.
    pub fn apply(&self, s: &MatrixSheet) -> Result<MatrixSheet, MatrixError> {
        let (rows, cols) = (s.rows, s.cols);
        let v = |r: usize, c: usize| s.value(r, c);
        let finite: Vec<f64> = s.values().iter().copied().filter(|x| x.is_finite()).collect();
        let map = |f: &dyn Fn(f64) -> f64| -> Vec<f64> {
            s.values().iter().map(|&x| if x.is_finite() { f(x) } else { f64::NAN }).collect()
        };
        let data = match *self {
            ImageOp::Brightness(p) => {
                if !(-100.0..=100.0).contains(&p) {
                    return Err(MatrixError::Invalid("the brightness is a percentage from -100 to 100".into()));
                }
                let lo = finite.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let delta = p / 100.0 * (hi - lo);
                map(&|x| (x + delta).clamp(lo, hi))
            }
            ImageOp::Contrast(k) => {
                let mean = finite.iter().sum::<f64>() / finite.len().max(1) as f64;
                map(&|x| mean + k * (x - mean))
            }
            ImageOp::Gamma(g) => {
                if !(g.is_finite() && g > 0.0) {
                    return Err(MatrixError::Invalid("gamma must be a positive number".into()));
                }
                let lo = finite.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if hi > lo { map(&|x| lo + (hi - lo) * ((x - lo) / (hi - lo)).powf(1.0 / g)) } else { map(&|x| x) }
            }
            ImageOp::Math(op, k) => {
                if op == MathOp::Divide && k == 0.0 {
                    return Err(MatrixError::Invalid("division by zero".into()));
                }
                map(&|x| match op {
                    MathOp::Add => x + k,
                    MathOp::Subtract => x - k,
                    MathOp::Multiply => x * k,
                    MathOp::Divide => x / k,
                })
            }
            ImageOp::ToBinary { low, high } => map(&|x| if (low..=high).contains(&x) { 1.0 } else { 0.0 }),
            ImageOp::Rotate(r) => return Ok(s.rotate90(r)),
            ImageOp::Levels { input: (a, b), output: (lo, hi) } => {
                if !(a.is_finite() && b.is_finite() && b > a) {
                    return Err(MatrixError::Invalid("the input high level must be above the low level".into()));
                }
                map(&|x| lo + (x.clamp(a, b) - a) / (b - a) * (hi - lo))
            }
            ImageOp::Crop { rows: (r0, r1), cols: (c0, c1) } => {
                if r0 > r1 || c0 > c1 || r1 >= rows || c1 >= cols {
                    return Err(MatrixError::Invalid(format!(
                        "the crop must lie inside the {rows} x {cols} cells, first before last"
                    )));
                }
                let (nr, nc) = (r1 - r0 + 1, c1 - c0 + 1);
                let data = (0..nr * nc).map(|k| v(r0 + k / nc, c0 + k % nc)).collect();
                let mut out = s.like(nr, nc, data);
                (out.x_from, out.x_to) = (s.x_at(c0), s.x_at(c1));
                (out.y_from, out.y_to) = (s.y_at(r0), s.y_at(r1));
                return Ok(out);
            }
            ImageOp::Resize { rows: nr, cols: nc, bilinear } => {
                if nr == 0 || nc == 0 {
                    return Err(MatrixError::Invalid("the new size needs at least one row and column".into()));
                }
                crate::matrix::check_cells(nr, nc)?;
                if !bilinear {
                    // Origin 2016's `imgResize interpolate:=normal` (measured, T282): cell k of the new
                    // size takes old cell floor(k * old / new), so 3 to 5 repeats cells 0 0 1 1 2.
                    let src = |k: usize, n_new: usize, n_old: usize| (k * n_old / n_new).min(n_old - 1);
                    let mut data = Vec::with_capacity(nr * nc);
                    for r in 0..nr {
                        for c in 0..nc {
                            data.push(v(src(r, nr, rows), src(c, nc, cols)));
                        }
                    }
                    return Ok(s.like(nr, nc, data));
                }
                // Cell centres map onto the old grid end to end; a missing neighbour makes a missing cell.
                let at = |i: usize, n_new: usize, n_old: usize| {
                    if n_new == 1 || n_old == 1 { 0.0 } else { i as f64 * (n_old - 1) as f64 / (n_new - 1) as f64 }
                };
                let mut data = Vec::with_capacity(nr * nc);
                for r in 0..nr {
                    let fr = at(r, nr, rows);
                    let (r0, tr) = (fr.floor() as usize, fr - fr.floor());
                    let r1 = (r0 + 1).min(rows - 1);
                    for c in 0..nc {
                        let fc = at(c, nc, cols);
                        let (c0, tc) = (fc.floor() as usize, fc - fc.floor());
                        let c1 = (c0 + 1).min(cols - 1);
                        let top = v(r0, c0) * (1.0 - tc) + v(r0, c1) * tc;
                        let bottom = v(r1, c0) * (1.0 - tc) + v(r1, c1) * tc;
                        data.push(top * (1.0 - tr) + bottom * tr);
                    }
                }
                return Ok(s.like(nr, nc, data));
            }
            ImageOp::Flip(d) => return Ok(s.flip(d, false)),
            ImageOp::Median(d) => {
                if !(1..=MAX_MEDIAN).contains(&d) {
                    return Err(MatrixError::Invalid(format!("the window size is 1 to {MAX_MEDIAN}")));
                }
                let back = (d as isize - 1) / 2;
                let mut out = Vec::with_capacity(rows * cols);
                let mut window = Vec::with_capacity(d * d);
                for r in 0..rows {
                    for c in 0..cols {
                        if !v(r, c).is_finite() {
                            out.push(f64::NAN);
                            continue;
                        }
                        window.clear();
                        for dr in -back..(d as isize - back) {
                            for dc in -back..(d as isize - back) {
                                let rr = (r as isize + dr).clamp(0, rows as isize - 1) as usize;
                                let cc = (c as isize + dc).clamp(0, cols as isize - 1) as usize;
                                let x = v(rr, cc);
                                if x.is_finite() {
                                    window.push(x);
                                }
                            }
                        }
                        window.sort_by(f64::total_cmp);
                        out.push(window[(window.len() - 1) / 2]);
                    }
                }
                return Ok(s.like(rows, cols, out));
            }
            ImageOp::Smooth | ImageOp::Sharpen | ImageOp::EdgeDetection { .. } => {
                // Edge Detection compares responses in levels of 0..255 over the sheet's range.
                let lo = finite.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let per_level = if hi > lo { (hi - lo) / 255.0 } else { f64::INFINITY };
                // The neighbourhood repeats the edge cells (a cell outside the sheet takes the
                // value of the nearest one); a missing centre stays missing.
                let at = |r: isize, c: isize| {
                    let rr = r.clamp(0, rows as isize - 1) as usize;
                    let cc = c.clamp(0, cols as isize - 1) as usize;
                    v(rr, cc)
                };
                let mut out = vec![f64::NAN; rows * cols];
                for r in 0..rows {
                    for c in 0..cols {
                        if !v(r, c).is_finite() {
                            continue;
                        }
                        let (ri, ci) = (r as isize, c as isize);
                        let n = |dr: isize, dc: isize| at(ri + dr, ci + dc);
                        out[r * cols + c] = match *self {
                            ImageOp::Smooth => {
                                let mut sum = 0.0;
                                for dr in -1..=1 {
                                    for dc in -1..=1 {
                                        sum += n(dr, dc);
                                    }
                                }
                                sum / 9.0
                            }
                            ImageOp::Sharpen => 5.0 * n(0, 0) - n(-1, 0) - n(1, 0) - n(0, -1) - n(0, 1),
                            ImageOp::EdgeDetection { filter, threshold } => {
                                use EdgeFilter as F;
                                let (side, sum) = match filter {
                                    F::SobelHorizontal | F::SobelVertical | F::SobelBoth => (2.0, 4.0),
                                    _ => (1.0, 3.0),
                                };
                                // A step of k gives k: the weighted difference over the kernel's weight.
                                let gx = ((n(-1, 1) + side * n(0, 1) + n(1, 1))
                                    - (n(-1, -1) + side * n(0, -1) + n(1, -1)))
                                    / sum;
                                let gy = ((n(1, -1) + side * n(1, 0) + n(1, 1))
                                    - (n(-1, -1) + side * n(-1, 0) + n(-1, 1)))
                                    / sum;
                                let response = match filter {
                                    F::SobelHorizontal | F::PrewittHorizontal => gy.abs(),
                                    F::SobelVertical | F::PrewittVertical => gx.abs(),
                                    _ => gx.abs().max(gy.abs()),
                                };
                                if response / per_level > threshold { 1.0 } else { 0.0 }
                            }
                            _ => unreachable!("the other operations are handled above"),
                        };
                    }
                }
                out
            }
        };
        Ok(s.like(rows, cols, data))
    }
}

/// Arithmetic Transform > Subtract (T210): `s - other`, cell by cell; the sheets must be the same size.
pub fn subtract(s: &MatrixSheet, other: &MatrixSheet) -> Result<MatrixSheet, MatrixError> {
    if (s.rows, s.cols) != (other.rows, other.cols) {
        return Err(MatrixError::Invalid(format!(
            "the matrices differ in size ({} x {} and {} x {})",
            s.rows, s.cols, other.rows, other.cols
        )));
    }
    let data = s.values().iter().zip(other.values()).map(|(a, b)| a - b).collect();
    Ok(s.like(s.rows, s.cols, data))
}

/// The Red, Green and Blue sheets of a matrix book of a colour picture (T218), found by name;
/// `None` when the book has not all three or they differ in size.
pub fn rgb_sheets(sheets: &[MatrixSheet]) -> Option<[&MatrixSheet; 3]> {
    let find = |n: &str| sheets.iter().find(|s| s.name.eq_ignore_ascii_case(n));
    let (r, g, b) = (find("Red")?, find("Green")?, find("Blue")?);
    ((r.rows, r.cols) == (g.rows, g.cols) && (r.rows, r.cols) == (b.rows, b.cols)).then_some([r, g, b])
}

/// Conversion > Color to Gray (T218): `(2 R + 5 G + B) / 8` as one sheet named Gray, the weights of
/// Origin 2016's `imgC2gray` (T280, measured: pure red, green and blue gave 64, 159 and 32 of 255, and
/// (200, 100, 50) gave 119; Origin rounds to whole levels, Sheaf keeps the fraction).
pub fn color_to_gray(rgb: [&MatrixSheet; 3]) -> MatrixSheet {
    let [r, g, b] = rgb;
    let data =
        (0..r.rows * r.cols).map(|k| (2.0 * r.values()[k] + 5.0 * g.values()[k] + b.values()[k]) / 8.0).collect();
    let mut out = r.like(r.rows, r.cols, data);
    out.name = "Gray".into();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(rows: usize, cols: usize, values: &[f64]) -> MatrixSheet {
        let mut s = MatrixSheet::new("M", 1, rows, cols);
        s.values_mut().copy_from_slice(values);
        s
    }

    #[test]
    fn adjustments_arithmetic_and_binary_work_cell_by_cell() {
        let s = sheet(1, 3, &[0.0, 50.0, 100.0]);
        assert_eq!(ImageOp::Brightness(10.0).apply(&s).unwrap().values(), [10.0, 60.0, 100.0], "saturates");
        assert_eq!(ImageOp::Brightness(-20.0).apply(&s).unwrap().values(), [0.0, 30.0, 80.0]);
        assert!(ImageOp::Brightness(150.0).apply(&s).is_err());
        assert_eq!(ImageOp::Contrast(2.0).apply(&s).unwrap().values(), [-50.0, 50.0, 150.0]);
        let g = ImageOp::Gamma(2.0).apply(&s).unwrap();
        assert!((g.value(0, 1) - 100.0 * 0.5f64.sqrt()).abs() < 1e-12, "{:?}", g.values());
        assert_eq!(ImageOp::Math(MathOp::Divide, 50.0).apply(&s).unwrap().values(), [0.0, 1.0, 2.0]);
        assert!(ImageOp::Math(MathOp::Divide, 0.0).apply(&s).is_err());
        assert_eq!(ImageOp::ToBinary { low: 50.0, high: f64::INFINITY }.apply(&s).unwrap().values(), [0.0, 1.0, 1.0]);
        assert_eq!(ImageOp::ToBinary { low: 40.0, high: 60.0 }.apply(&s).unwrap().values(), [0.0, 1.0, 0.0], "a band");
        assert!(ImageOp::Gamma(0.0).apply(&s).is_err());
        let missing = sheet(1, 2, &[f64::NAN, 4.0]);
        assert!(ImageOp::Brightness(1.0).apply(&missing).unwrap().value(0, 0).is_nan());
    }

    /// Origin 2016's `imgMedian` on a 5 x 5 image (levels), measured 2026-11-21 (T284).
    #[test]
    fn median_matches_origin_2016() {
        let levels = [
            125.0, 0.0, 5.0, 11.0, 16.0, 5.0, 11.0, 16.0, 22.0, 27.0, 16.0, 22.0, 255.0, 33.0, 38.0, 27.0, 33.0, 38.0,
            43.0, 49.0, 38.0, 43.0, 49.0, 54.0, 60.0,
        ];
        let s = sheet(5, 5, &levels);
        let d3 = ImageOp::Median(3).apply(&s).unwrap();
        assert_eq!(
            d3.values(),
            [
                11.0, 5.0, 11.0, 16.0, 16.0, 16.0, 16.0, 16.0, 22.0, 27.0, 16.0, 22.0, 33.0, 38.0, 38.0, 27.0, 38.0,
                43.0, 49.0, 49.0, 38.0, 38.0, 43.0, 49.0, 54.0
            ]
        );
        let d2 = ImageOp::Median(2).apply(&s).unwrap();
        assert_eq!(
            d2.values(),
            [
                5.0, 5.0, 11.0, 16.0, 16.0, 11.0, 16.0, 22.0, 27.0, 27.0, 22.0, 33.0, 38.0, 38.0, 38.0, 33.0, 38.0,
                43.0, 49.0, 49.0, 38.0, 43.0, 49.0, 54.0, 60.0
            ]
        );
        let d5 = ImageOp::Median(5).apply(&s).unwrap();
        assert_eq!(&d5.values()[..5], &[16.0, 16.0, 16.0, 16.0, 16.0]);
        assert_eq!(d5.value(2, 2), 33.0);
        assert!(ImageOp::Median(0).apply(&s).is_err());
    }

    #[test]
    fn spatial_filters_use_3_by_3_neighbourhoods_with_repeated_edges() {
        // A single bright cell in the middle of a 3 x 3 sheet.
        let s = sheet(3, 3, &[0.0, 0.0, 0.0, 0.0, 9.0, 0.0, 0.0, 0.0, 0.0]);
        let smooth = ImageOp::Smooth.apply(&s).unwrap();
        assert_eq!(smooth.value(1, 1), 1.0);
        assert_eq!(smooth.value(0, 0), 1.0, "the corner sees the centre once");
        assert_eq!(ImageOp::Sharpen.apply(&s).unwrap().value(1, 1), 45.0);
        let edge = |filter, threshold| ImageOp::EdgeDetection { filter, threshold };
        let edges = edge(EdgeFilter::SobelBoth, 0.0).apply(&s).unwrap();
        assert_eq!(edges.value(1, 1), 0.0, "flat around the centre's own position");
        assert_eq!(edges.value(1, 0), 1.0, "an edge next to the bright cell");
        let flat = sheet(2, 2, &[3.0; 4]);
        assert!(edge(EdgeFilter::SobelBoth, 0.0).apply(&flat).unwrap().values().iter().all(|v| *v == 0.0));
    }

    /// Origin 2016's `imgEdge` on steps of 10, 20, 40 and 80 levels (T285, measured 2026-11-21): the
    /// step cells are edges while the threshold is below the step, for Sobel and Prewitt alike.
    #[test]
    fn edge_detection_thresholds_match_origin_2016() {
        // Rows of 0 0 0 k k k on a 0..255 sheet.
        for (k, edge_below) in [(10.0, 5.0), (20.0, 10.0), (40.0, 30.0), (80.0, 60.0)] {
            let mut v = vec![];
            for _ in 0..3 {
                v.extend([0.0, 0.0, 0.0, k, k, k]);
            }
            v.extend([0.0, 0.0, 0.0, 0.0, 0.0, 255.0]);
            let s = sheet(4, 6, &v);
            for filter in [EdgeFilter::SobelVertical, EdgeFilter::PrewittBoth] {
                let below = ImageOp::EdgeDetection { filter, threshold: edge_below }.apply(&s).unwrap();
                assert_eq!((below.value(1, 2), below.value(1, 3)), (1.0, 1.0), "{filter:?} k {k}");
                let at = ImageOp::EdgeDetection { filter, threshold: k }.apply(&s).unwrap();
                assert_eq!((at.value(1, 2), at.value(1, 3)), (0.0, 0.0), "{filter:?} k {k}: not above k");
            }
            let h = ImageOp::EdgeDetection { filter: EdgeFilter::SobelHorizontal, threshold: 0.0 }.apply(&s).unwrap();
            assert_eq!(h.value(1, 3), 0.0, "a vertical step is no horizontal edge");
        }
    }

    #[test]
    fn levels_crop_resize_and_subtract() {
        let s = sheet(2, 3, &[0.0, 50.0, 100.0, 150.0, 200.0, 250.0]);
        let lv = ImageOp::Levels { input: (50.0, 150.0), output: (0.0, 255.0) }.apply(&s).unwrap();
        assert_eq!(lv.values(), [0.0, 0.0, 127.5, 255.0, 255.0, 255.0]);
        assert!(ImageOp::Levels { input: (5.0, 5.0), output: (0.0, 1.0) }.apply(&s).is_err());
        let mut ranged = s.clone();
        (ranged.x_from, ranged.x_to, ranged.y_from, ranged.y_to) = (0.0, 20.0, 0.0, 10.0);
        let c = ImageOp::Crop { rows: (1, 1), cols: (1, 2) }.apply(&ranged).unwrap();
        assert_eq!((c.rows, c.cols, c.values().to_vec()), (1, 2, vec![200.0, 250.0]));
        assert_eq!((c.x_from, c.x_to, c.y_from, c.y_to), (10.0, 20.0, 10.0, 10.0));
        assert!(ImageOp::Crop { rows: (0, 2), cols: (0, 0) }.apply(&s).is_err());
        let r = ImageOp::Resize { rows: 3, cols: 5, bilinear: true }.apply(&s).unwrap();
        assert_eq!((r.rows, r.cols), (3, 5));
        assert_eq!(r.value(0, 0), 0.0);
        assert_eq!(r.value(2, 4), 250.0);
        assert_eq!(r.value(1, 2), 125.0, "the middle of the four central cells");
        // Nearest, as Origin 2016's default: a 3 x 3 sheet to 5 x 5 repeats cells 0 0 1 1 2 each way.
        let three = sheet(3, 3, &[0.0, 32.0, 64.0, 96.0, 128.0, 159.0, 191.0, 223.0, 255.0]);
        let n = ImageOp::Resize { rows: 5, cols: 5, bilinear: false }.apply(&three).unwrap();
        assert_eq!(&n.values()[..5], &[0.0, 0.0, 32.0, 32.0, 64.0]);
        assert_eq!(&n.values()[20..], &[191.0, 191.0, 223.0, 223.0, 255.0]);
        let d = subtract(&s, &s).unwrap();
        assert!(d.values().iter().all(|v| *v == 0.0));
        assert!(subtract(&s, &r).is_err());
    }

    #[test]
    fn colour_sheets_are_found_by_name_and_turned_to_gray() {
        let mk = |name: &str, v: f64| {
            let mut s = sheet(1, 2, &[v, v]);
            s.name = name.into();
            s
        };
        let sheets = vec![mk("Red", 100.0), mk("Green", 100.0), mk("Blue", 100.0)];
        let gray = color_to_gray(rgb_sheets(&sheets).unwrap());
        assert_eq!(gray.name, "Gray");
        assert!((gray.value(0, 1) - 100.0).abs() < 1e-9);
        assert!(rgb_sheets(&sheets[..2]).is_none());
        // Origin 2016's imgC2gray on (200, 100, 50): 118.75, shown as 119.
        let colour = vec![mk("Red", 200.0), mk("Green", 100.0), mk("Blue", 50.0)];
        assert_eq!(color_to_gray(rgb_sheets(&colour).unwrap()).value(0, 0), 118.75);
    }
}
