//! Placing marked-up text: measuring it and turning it into one scene text
//! item per run.

use crate::fonts::TextMeasurer;
use crate::markup::{self, Run};
use crate::scene::{Anchor, Item, Stroke, TextItem, push_num};
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
    let single = runs.len() == 1 && runs[0].rise == 0.0;
    if single {
        // One plain run: let the renderer align it with its own font metrics.
        items.push(item(&runs[0], x, y, anchor));
        if !runs[0].underline && !runs[0].strikeout {
            return;
        }
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
        if !single {
            items.push(item(run, px, py, Anchor::Start));
        }
        let font = run_font(font, run);
        let run_size = size * run.scale;
        let width = m.width(&run.text, &font, run_size);
        for metrics in
            [run.underline.then(|| m.underline_metrics(&font)), run.strikeout.then(|| m.strikeout_metrics(&font))]
                .into_iter()
                .flatten()
        {
            let offset = metrics.center * run_size;
            let mut d = String::from("M");
            push_num(&mut d, px + offset * sin);
            d.push(' ');
            push_num(&mut d, py + offset * cos);
            d.push('L');
            push_num(&mut d, px + width * cos + offset * sin);
            d.push(' ');
            push_num(&mut d, py - width * sin + offset * cos);
            items.push(Item::Path {
                d,
                fill: None,
                stroke: Some(Stroke::solid(font.color, metrics.thickness * run_size)),
            });
        }
        cursor += width;
    }
}
