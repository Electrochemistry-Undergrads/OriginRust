//! Placing marked-up text: measuring it and turning it into one scene text
//! item per run.

use crate::fonts::TextMeasurer;
use crate::markup::{self, Run};
use crate::scene::{Anchor, Item, TextItem};
use sheaf_core::graph::Font;

/// Ascent used for vertical placement, as a fraction of the font size.
pub const ASCENT: f64 = 0.72;
/// Descent used for vertical placement, as a fraction of the font size.
pub const DESCENT: f64 = 0.21;

fn run_font(font: &Font, run: &Run) -> Font {
    let mut f = font.clone();
    f.bold |= run.bold;
    f.italic |= run.italic;
    f
}

fn runs_width(m: &dyn TextMeasurer, runs: &[Run], font: &Font, size: f64) -> f64 {
    runs.iter().map(|r| m.width(&r.text, &run_font(font, r), size * r.scale)).sum()
}

/// Width in points of marked-up `text` at `size` (the font's own size is not used).
pub fn width(m: &dyn TextMeasurer, text: &str, font: &Font, size: f64) -> f64 {
    runs_width(m, &markup::parse(text), font, size)
}

/// Adds the runs of `text` to `items`. `(x, y)` is the anchor: the baseline
/// position of the start, middle or end of the text according to `anchor`.
/// The text is rotated counter-clockwise by `rotate_ccw` degrees about the anchor.
pub fn place(
    items: &mut Vec<Item>,
    m: &dyn TextMeasurer,
    text: &str,
    font: &Font,
    size: f64,
    (x, y): (f64, f64),
    anchor: Anchor,
    rotate_ccw: f64,
) {
    let runs = markup::parse(text);
    if runs.is_empty() {
        return;
    }
    let item = |run: &Run, x: f64, y: f64, anchor: Anchor| {
        Item::Text(TextItem {
            x,
            y,
            text: run.text.clone(),
            family: font.family.clone(),
            size: size * run.scale,
            bold: font.bold || run.bold,
            italic: font.italic || run.italic,
            color: font.color,
            anchor,
            rotate_ccw,
        })
    };
    if runs.len() == 1 && runs[0].rise == 0.0 {
        // One plain run: let the renderer align it with its own font metrics.
        items.push(item(&runs[0], x, y, anchor));
        return;
    }
    let total = runs_width(m, &runs, font, size);
    let mut cursor = match anchor {
        Anchor::Start => 0.0,
        Anchor::Middle => -total / 2.0,
        Anchor::End => -total,
    };
    let (sin, cos) = rotate_ccw.to_radians().sin_cos();
    for run in &runs {
        let dy = -run.rise * size;
        // Local frame: x along the text, y down. Rotate counter-clockwise on a y-down page.
        let px = x + cursor * cos + dy * sin;
        let py = y - cursor * sin + dy * cos;
        items.push(item(run, px, py, Anchor::Start));
        cursor += m.width(&run.text, &run_font(font, run), size * run.scale);
    }
}
