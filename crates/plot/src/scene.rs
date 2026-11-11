//! The scene: a flat list of drawing primitives in page coordinates
//! (points). Layout produces it, `svg` writes it out.

use crate::Rect;
use sheaf_core::Color;
use std::fmt::Write;

/// How the end of a stroked line looks.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cap {
    Butt,
    Round,
    Square,
}

/// How two stroked segments are joined.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Join {
    Miter,
    Round,
}

/// Outline style.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
    /// Absolute dash lengths (on, off, ...); empty for a solid line.
    pub dash: Vec<f64>,
    pub cap: Cap,
    pub join: Join,
}

impl Stroke {
    /// A solid stroke with butt caps and mitred joins.
    pub fn solid(color: Color, width: f64) -> Stroke {
        Stroke { color, width, dash: Vec::new(), cap: Cap::Butt, join: Join::Miter }
    }
}

/// Where the text position `x` sits relative to the text.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

/// One piece of text in a single style, positioned at its baseline.
#[derive(Clone, Debug, PartialEq)]
pub struct TextItem {
    pub x: f64,
    pub y: f64,
    pub text: String,
    /// The font family asked for by the model (fallbacks are added when written).
    pub family: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: Color,
    pub anchor: Anchor,
    /// Counter-clockwise rotation in degrees about `(x, y)`.
    pub rotate_ccw: f64,
}

/// A drawing primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Rect {
        rect: Rect,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    /// `d` is SVG path data, already formatted.
    Path {
        d: String,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    Text(TextItem),
}

/// Everything drawn for one layer, in drawing order within each list.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerScene {
    /// Data is clipped to this rectangle.
    pub clip: Rect,
    pub grid: Vec<Item>,
    pub plots: Vec<Item>,
    /// Axes, ticks, labels, titles, legend and free labels.
    pub overlay: Vec<Item>,
}

/// A whole figure.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    pub background: Color,
    pub layers: Vec<LayerScene>,
}

/// Appends a number with at most 3 decimals and no trailing zeros.
pub fn push_num(s: &mut String, v: f64) {
    if !v.is_finite() {
        s.push('0');
        return;
    }
    let scaled = (v.abs() * 1000.0).round();
    if scaled >= 9.0e15 {
        let _ = write!(s, "{}", v.round());
        return;
    }
    let n = scaled as u64;
    if n == 0 {
        s.push('0');
        return;
    }
    if v < 0.0 {
        s.push('-');
    }
    let _ = write!(s, "{}", n / 1000);
    let f = n % 1000;
    if f != 0 {
        s.push('.');
        let (d1, d2, d3) = (f / 100, f / 10 % 10, f % 10);
        s.push(char::from(b'0' + d1 as u8));
        if d2 != 0 || d3 != 0 {
            s.push(char::from(b'0' + d2 as u8));
        }
        if d3 != 0 {
            s.push(char::from(b'0' + d3 as u8));
        }
    }
}

/// Path-data builder: `M x y`, `L x y`, `C ...`, `Z`.
pub fn move_to(d: &mut String, x: f64, y: f64) {
    d.push('M');
    push_num(d, x);
    d.push(' ');
    push_num(d, y);
}

/// Appends `L x y`.
pub fn line_to(d: &mut String, x: f64, y: f64) {
    d.push('L');
    push_num(d, x);
    d.push(' ');
    push_num(d, y);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: f64) -> String {
        let mut s = String::new();
        push_num(&mut s, v);
        s
    }

    #[test]
    fn numbers_have_at_most_three_decimals_and_no_trailing_zeros() {
        assert_eq!(n(1.0), "1");
        assert_eq!(n(1.5), "1.5");
        assert_eq!(n(1.2345), "1.235");
        assert_eq!(n(-0.0004), "0");
        assert_eq!(n(-12.05), "-12.05");
        assert_eq!(n(0.001), "0.001");
        assert_eq!(n(100.0), "100");
        assert_eq!(n(f64::NAN), "0");
        assert_eq!(n(1e20), "100000000000000000000");
    }
}
