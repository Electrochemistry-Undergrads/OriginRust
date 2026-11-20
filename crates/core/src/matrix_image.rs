//! The Image menu on a matrix sheet (T207, matrices spec 7i): picture adjustments, arithmetic,
//! conversion, geometric transforms and 3 x 3 spatial filters. Each works on the sheet's values as
//! gray levels and keeps the sheet's size, ranges and labels (Rotate by 90 or 270 degrees swaps the
//! size). Missing values stay missing. Origin's own formulas are not measured; these are Sheaf's,
//! written next to each operation.

use crate::matrix::{FlipDirection, MatrixError, MatrixSheet, Rotation};

/// One Image menu operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageOp {
    /// Adjustments > Brightness: adds `amount` to every value.
    Brightness(f64),
    /// Adjustments > Contrast: stretches the values about their mean by `factor` (1 leaves them).
    Contrast(f64),
    /// Adjustments > Gamma: `lo + (hi - lo) * t^(1/gamma)`, with `t` the value's place between the
    /// sheet's lowest `lo` and highest `hi` value.
    Gamma(f64),
    /// Arithmetic Transform > Math: `value op operand`.
    Math(MathOp, f64),
    /// Conversion > To Binary: 1 where the value is at least the threshold, else 0.
    ToBinary(f64),
    /// Geometric Transform > Rotate by a multiple of 90 degrees.
    Rotate(Rotation),
    /// Geometric Transform > Flip.
    Flip(FlipDirection),
    /// Spatial Filters > Smooth: the mean of each 3 x 3 neighbourhood.
    Smooth,
    /// Spatial Filters > Sharpen: `5 v - (the four neighbours)`.
    Sharpen,
    /// Spatial Filters > Edge Detection: the Sobel gradient magnitude.
    EdgeDetection,
}

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
            ImageOp::ToBinary(_) => "Image To Binary",
            ImageOp::Rotate(_) => "Image Rotate",
            ImageOp::Flip(_) => "Image Flip",
            ImageOp::Smooth => "Image Smooth",
            ImageOp::Sharpen => "Image Sharpen",
            ImageOp::EdgeDetection => "Image Edge Detection",
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
            ImageOp::Brightness(a) => map(&|x| x + a),
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
            ImageOp::ToBinary(t) => map(&|x| if x >= t { 1.0 } else { 0.0 }),
            ImageOp::Rotate(r) => return Ok(s.rotate90(r)),
            ImageOp::Flip(d) => return Ok(s.flip(d, false)),
            ImageOp::Smooth | ImageOp::Sharpen | ImageOp::EdgeDetection => {
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
                            _ => {
                                let gx = (n(-1, 1) + 2.0 * n(0, 1) + n(1, 1)) - (n(-1, -1) + 2.0 * n(0, -1) + n(1, -1));
                                let gy = (n(1, -1) + 2.0 * n(1, 0) + n(1, 1)) - (n(-1, -1) + 2.0 * n(-1, 0) + n(-1, 1));
                                gx.hypot(gy)
                            }
                        };
                    }
                }
                out
            }
        };
        Ok(s.like(rows, cols, data))
    }
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
        assert_eq!(ImageOp::Brightness(10.0).apply(&s).unwrap().values(), [10.0, 60.0, 110.0]);
        assert_eq!(ImageOp::Contrast(2.0).apply(&s).unwrap().values(), [-50.0, 50.0, 150.0]);
        let g = ImageOp::Gamma(2.0).apply(&s).unwrap();
        assert!((g.value(0, 1) - 100.0 * 0.5f64.sqrt()).abs() < 1e-12, "{:?}", g.values());
        assert_eq!(ImageOp::Math(MathOp::Divide, 50.0).apply(&s).unwrap().values(), [0.0, 1.0, 2.0]);
        assert!(ImageOp::Math(MathOp::Divide, 0.0).apply(&s).is_err());
        assert_eq!(ImageOp::ToBinary(50.0).apply(&s).unwrap().values(), [0.0, 1.0, 1.0]);
        assert!(ImageOp::Gamma(0.0).apply(&s).is_err());
        let missing = sheet(1, 2, &[f64::NAN, 4.0]);
        assert!(ImageOp::Brightness(1.0).apply(&missing).unwrap().value(0, 0).is_nan());
    }

    #[test]
    fn spatial_filters_use_3_by_3_neighbourhoods_with_repeated_edges() {
        // A single bright cell in the middle of a 3 x 3 sheet.
        let s = sheet(3, 3, &[0.0, 0.0, 0.0, 0.0, 9.0, 0.0, 0.0, 0.0, 0.0]);
        let smooth = ImageOp::Smooth.apply(&s).unwrap();
        assert_eq!(smooth.value(1, 1), 1.0);
        assert_eq!(smooth.value(0, 0), 1.0, "the corner sees the centre once");
        assert_eq!(ImageOp::Sharpen.apply(&s).unwrap().value(1, 1), 45.0);
        let edges = ImageOp::EdgeDetection.apply(&s).unwrap();
        assert_eq!(edges.value(1, 1), 0.0, "flat around the centre's own position");
        assert!(edges.value(1, 0) > 0.0, "an edge next to the bright cell");
        let flat = sheet(2, 2, &[3.0; 4]);
        assert!(ImageOp::EdgeDetection.apply(&flat).unwrap().values().iter().all(|v| *v == 0.0));
    }
}
