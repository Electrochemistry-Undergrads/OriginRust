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
    /// Prefix of `overlay` drawn before the clipped plots (Data on Top of Axes).
    /// The remaining overlay, including legends and annotations, is always drawn last.
    pub underlay_len: usize,
    /// What the items stand for, so that a click can be traced back to an object of the model
    /// (see [`crate::hit`]). Not written to any file.
    pub objects: Vec<SceneObject>,
}

impl LayerScene {
    /// Unclipped items drawn before and after the data, respectively.
    pub(crate) fn split_overlay(&self) -> (&[Item], &[Item]) {
        self.overlay.split_at(self.underlay_len.min(self.overlay.len()))
    }
}

/// An object of the model that a click can pick.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    /// The X axis: its line, ticks and tick labels.
    XAxis,
    /// The Y axis: its line, ticks and tick labels.
    YAxis,
    /// The title of the X axis.
    XTitle,
    /// The title of the Y axis.
    YTitle,
    /// The legend.
    Legend,
    /// A free text label: its index in `Layer::labels`.
    Label(usize),
    /// A drawn line, arrow, rectangle or ellipse: its index in `Layer::shapes`.
    Shape(usize),
    /// A slope annotation: its index in `Layer::slopes`.
    Slope(usize),
    /// A plot: its index in `Layer::plots`.
    Plot(usize),
}

/// Which list of a [`LayerScene`] an object's items are in.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ItemList {
    Plots,
    Overlay,
}

/// One pickable object: the model layer it belongs to, the items drawn for it and their bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneObject {
    pub kind: ObjectKind,
    /// The index of the model layer the object belongs to. A legend that lists several layers is
    /// drawn with the last layer's items but belongs to its own layer.
    pub layer: usize,
    pub list: ItemList,
    /// The items of `list` drawn for the object.
    pub items: std::ops::Range<usize>,
    /// The bounding box of those items in page points (text measured, strokes not widened).
    pub bounds: Rect,
    /// Boxes that count as the object besides its lines: one per text item, and the axis line.
    pub boxes: Vec<Rect>,
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
