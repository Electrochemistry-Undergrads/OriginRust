//! Placing marked-up text: measuring it and turning it into one scene text
//! item per run.

use crate::fonts::TextMeasurer;
use crate::markup::{self, Run, Segment};
use crate::scene::{Anchor, Item, Stroke, TextItem, push_num};
use sheaf_core::graph::Font;

/// Ascent used for vertical placement, as a fraction of the font size.
pub const ASCENT: f64 = 0.72;
/// Descent used for vertical placement, as a fraction of the font size.
pub const DESCENT: f64 = 0.21;

/// Clearance and stroke thickness relative to the size at the start of an overbar span.
const OVERBAR_GAP: f64 = 0.1;
const OVERBAR_THICKNESS: f64 = 0.04;

fn run_font(font: &Font, run: &Run) -> Font {
    let mut f = font.clone();
    f.bold |= run.bold;
    f.italic |= run.italic;
    f.color = run.color.unwrap_or(font.color);
    if let Some(family) = &run.family {
        f.family.clone_from(family);
    }
    f
}

fn segments_width(m: &dyn TextMeasurer, segments: &[Segment], font: &Font, size: f64) -> f64 {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::Text(r) => m.width(&r.text, &run_font(font, r), size * r.scale),
            Segment::Pair { sub, sup } => segments_width(m, sub, font, size).max(segments_width(m, sup, font, size)),
            Segment::Overbar { content, .. } => segments_width(m, content, font, size),
        })
        .sum()
}

fn ink_top(segments: &[Segment]) -> f64 {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::Text(run) => run.rise + ASCENT * run.scale,
            Segment::Pair { sub, sup } => ink_top(sub).max(ink_top(sup)),
            Segment::Overbar { content, scale, .. } => ink_top(content) + scale * (OVERBAR_GAP + OVERBAR_THICKNESS),
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

enum Ink<'a> {
    Glyphs(&'a Run),
    Overbar { rise: f64, width: f64, stroke: Stroke },
}

/// Visits glyph runs and overbars at local horizontal offsets. Both script branches begin at the same column;
/// painting the upper branch first matches Origin's vector output.
fn visit_ink(
    segments: &[Segment],
    m: &dyn TextMeasurer,
    font: &Font,
    size: f64,
    start: f64,
    visit: &mut impl FnMut(Ink<'_>, f64),
) {
    let mut cursor = start;
    for segment in segments {
        match segment {
            Segment::Text(run) => {
                visit(Ink::Glyphs(run), cursor);
                cursor += m.width(&run.text, &run_font(font, run), size * run.scale);
            }
            Segment::Pair { sub, sup } => {
                visit_ink(sup, m, font, size, cursor, visit);
                visit_ink(sub, m, font, size, cursor, visit);
                cursor += segments_width(m, sub, font, size).max(segments_width(m, sup, font, size));
            }
            Segment::Overbar { content, scale, color } => {
                visit_ink(content, m, font, size, cursor, visit);
                let width = segments_width(m, content, font, size);
                if width > 0.0 {
                    let rise = (ink_top(content) + scale * (OVERBAR_GAP + OVERBAR_THICKNESS / 2.0)) * size;
                    let stroke = Stroke::solid(color.unwrap_or(font.color), scale * size * OVERBAR_THICKNESS);
                    visit(Ink::Overbar { rise, width, stroke }, cursor);
                }
                cursor += width;
            }
        }
    }
}

/// Width in points of marked-up `text` at `size` (the font's own size is not used).
pub fn width(m: &dyn TextMeasurer, text: &str, font: &Font, size: f64) -> f64 {
    let segments = markup::segments(text, &|name| m.expand_inline_family(name));
    segments_width(m, &segments, font, size)
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
    let runs = markup::segments(text, &|name| m.expand_inline_family(name));
    if runs.is_empty() {
        return;
    }
    let item = |run: &Run, x: f64, y: f64, anchor: Anchor| {
        Item::Text(TextItem {
            x,
            y,
            text: run.text.clone(),
            family: run.family.as_ref().unwrap_or(&font.family).clone(),
            size: size * run.scale,
            bold: font.bold || run.bold,
            italic: font.italic || run.italic,
            color: run.color.unwrap_or(font.color),
            anchor,
            rotate_ccw,
        })
    };
    let mut glyphs = runs.as_slice();
    while let [Segment::Overbar { content, .. }] = glyphs {
        glyphs = content;
    }
    let single_run = match glyphs {
        [Segment::Text(run)] if run.rise == 0.0 => Some(run),
        _ => None,
    };
    let single = single_run.is_some();
    if let Some(run) = single_run {
        // One plain run: let the renderer align it with its own font metrics.
        items.push(item(run, x, y, anchor));
        if !run.underline && !run.strikeout && matches!(runs.as_slice(), [Segment::Text(_)]) {
            return;
        }
    }
    let total = segments_width(m, &runs, font, size);
    let cursor = match anchor {
        Anchor::Start => 0.0,
        Anchor::Middle => -total / 2.0,
        Anchor::End => -total,
    };
    let (sin, cos) = rotate_ccw.to_radians().sin_cos();
    let line = |px: f64, py: f64, width, stroke| {
        let mut d = String::from("M");
        push_num(&mut d, px);
        d.push(' ');
        push_num(&mut d, py);
        d.push('L');
        push_num(&mut d, px + width * cos);
        d.push(' ');
        push_num(&mut d, py - width * sin);
        Item::Path { d, fill: None, stroke: Some(stroke) }
    };
    visit_ink(&runs, m, font, size, cursor, &mut |mark, cursor| {
        let run = match mark {
            Ink::Glyphs(run) => run,
            Ink::Overbar { rise, width, stroke } => {
                items.push(line(x + cursor * cos - rise * sin, y - cursor * sin - rise * cos, width, stroke));
                return;
            }
        };
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
            items.push(line(
                px + offset * sin,
                py + offset * cos,
                width,
                Stroke::solid(font.color, metrics.thickness * run_size),
            ));
        }
    });
}
