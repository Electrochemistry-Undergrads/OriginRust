//! Visible dash lengths in points (graphing spec 2.1).
//!
//! Origin's thin-line patterns do not scale in proportion to the pen width. Use the
//! measured 0.5/1 pt samples, then blend into the existing normal-width pattern at
//! 1.5 pt. Interpolation and proportional scaling below 0.5 pt are Sheaf decisions;
//! Origin's device-dependent pen rounding and round caps are not reproduced here.

use sheaf_core::graph::Dash;

pub(crate) fn lengths(style: Dash, width: f64) -> Vec<f64> {
    let normal = style.pattern();
    if width >= 1.5 || width <= 0.0 || !width.is_finite() {
        return normal.iter().map(|v| v * width).collect();
    }
    // On, off, ... in points, from verified 6400 px Origin 2016 exports.
    let (half, one): (&[f64], &[f64]) = match style {
        Dash::Solid => (&[], &[]),
        Dash::Dash => (&[3.24, 2.40], &[5.88, 4.32]),
        Dash::Dot => (&[1.32, 2.16], &[2.28, 3.84]),
        Dash::DashDot => (&[3.84, 0.96, 1.32, 1.08], &[6.84, 1.80, 2.28, 1.92]),
        Dash::DashDotDot => (&[3.00, 0.96, 1.32, 0.96, 1.32, 1.08], &[5.40, 1.80, 2.28, 1.80, 2.28, 1.92]),
        Dash::ShortDash => (&[1.80, 0.84], &[3.24, 1.44]),
        Dash::ShortDot => (&[0.96, 0.36], &[1.80, 0.48]),
        Dash::ShortDashDot => (&[2.64, 0.48, 0.96, 0.60], &[4.68, 0.84, 1.80, 0.96]),
    };
    if width <= 0.5 {
        half.iter().map(|v| v * (width / 0.5)).collect()
    } else if width <= 1.0 {
        half.iter().zip(one).map(|(a, b)| a + (b - a) * ((width - 0.5) * 2.0)).collect()
    } else {
        one.iter().zip(normal).map(|(a, b)| a + (b * 1.5 - a) * ((width - 1.0) * 2.0)).collect()
    }
}
