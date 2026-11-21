//! Placing marked-up text: measuring it and turning it into one scene text
//! item per run.

use crate::fonts::TextMeasurer;
use crate::markup::{self, Accent, Run, Segment};
use crate::scene::{Anchor, Item, Stroke, TextItem, line_to, move_to, push_num};
use sheaf_core::Color;
use sheaf_core::graph::Font;

/// Ascent used for vertical placement, as a fraction of the font size.
pub const ASCENT: f64 = 0.72;
/// Descent used for vertical placement, as a fraction of the font size.
pub const DESCENT: f64 = 0.21;

/// Accent dimensions relative to the font size at the start of the span.
const ACCENT_GAP: f64 = 0.1;
const ACCENT_THICKNESS: f64 = 0.04;
const DOT_RADIUS: f64 = 0.06;
const TILDE_AMPLITUDE: f64 = 0.065;

fn accent_height(kind: Accent) -> f64 {
    match kind {
        Accent::Bar => ACCENT_THICKNESS,
        Accent::Dot => 2.0 * DOT_RADIUS,
        Accent::Tilde => 2.0 * TILDE_AMPLITUDE + ACCENT_THICKNESS,
    }
}

fn accent_item(kind: Accent, width: f64, em: f64, color: Color, at: impl Fn(f64, f64) -> (f64, f64)) -> Item {
    let mut d = String::new();
    let (fill, stroke) = match kind {
        Accent::Bar => {
            let y = -ACCENT_THICKNESS * em / 2.0;
            let (x, y0) = at(0.0, y);
            move_to(&mut d, x, y0);
            let (x, y1) = at(width, y);
            line_to(&mut d, x, y1);
            (None, Some(Stroke::solid(color, ACCENT_THICKNESS * em)))
        }
        Accent::Dot => {
            let radius = DOT_RADIUS * em;
            // A small closed polygon uses the same path commands in every exporter.
            for i in 0..16 {
                let (sin, cos) = (std::f64::consts::TAU * f64::from(i) / 16.0).sin_cos();
                let (x, y) = at(width / 2.0 + radius * cos, radius * (sin - 1.0));
                if i == 0 {
                    move_to(&mut d, x, y);
                } else {
                    line_to(&mut d, x, y);
                }
            }
            d.push('Z');
            (Some(color), None)
        }
        Accent::Tilde => {
            let wave_width = width.min(0.55 * em);
            let start = (width - wave_width) / 2.0;
            for i in 0..=16 {
                let fraction = f64::from(i) / 16.0;
                let wave = (std::f64::consts::TAU * fraction).sin();
                let y = -(TILDE_AMPLITUDE * (1.0 + wave) + ACCENT_THICKNESS / 2.0) * em;
                let (x, y) = at(start + wave_width * fraction, y);
                if i == 0 {
                    move_to(&mut d, x, y);
                } else {
                    line_to(&mut d, x, y);
                }
            }
            (None, Some(Stroke::solid(color, ACCENT_THICKNESS * em)))
        }
    };
    Item::Path { d, fill, stroke }
}

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

pub(crate) fn segments_width(m: &dyn TextMeasurer, segments: &[Segment], font: &Font, size: f64) -> f64 {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::Text(r) => m.width(&r.text, &run_font(font, r), size * r.scale),
            Segment::Pair { sub, sup } => segments_width(m, sub, font, size).max(segments_width(m, sup, font, size)),
            Segment::Accent { content, .. } => segments_width(m, content, font, size),
            Segment::LineGap(_) => 0.0,
        })
        .sum()
}

fn ink_top(segments: &[Segment]) -> f64 {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::Text(run) => run.rise + ASCENT * run.scale,
            Segment::Pair { sub, sup } => ink_top(sub).max(ink_top(sup)),
            Segment::Accent { kind, content, scale, .. } => {
                ink_top(content) + scale * (ACCENT_GAP + accent_height(*kind))
            }
            Segment::LineGap(_) => f64::NEG_INFINITY,
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

enum Ink<'a> {
    Glyphs(&'a Run),
    Accent { kind: Accent, rise: f64, width: f64, em: f64, color: Color },
}

/// Visits glyph runs and accents at local horizontal offsets. Both script branches begin at the same column;
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
            Segment::Accent { kind, content, scale, color } => {
                visit_ink(content, m, font, size, cursor, visit);
                let width = segments_width(m, content, font, size);
                if width > 0.0 {
                    let rise = (ink_top(content) + scale * ACCENT_GAP) * size;
                    visit(
                        Ink::Accent { kind: *kind, rise, width, em: scale * size, color: color.unwrap_or(font.color) },
                        cursor,
                    );
                }
                cursor += width;
            }
            Segment::LineGap(_) => {}
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
    place_segments(items, m, &runs, font, size, (x, y), anchor, rotate_ccw);
}

/// Lays out free graph labels from the first line's baseline, including explicit extra gaps.
pub(crate) fn place_label(
    items: &mut Vec<Item>,
    m: &dyn TextMeasurer,
    raw: &str,
    font: &Font,
    (x, y): (f64, f64),
    rotate_ccw: f64,
    plot_color: &impl Fn(usize) -> Option<Color>,
) {
    let size = font.size_pt;
    let normalized;
    let text = if raw.contains('\r') {
        normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
        &normalized
    } else {
        raw
    };
    let segments = markup::label_segments(text, &|name| m.expand_inline_family(name), plot_color);
    if !text.contains('\n') {
        place_segments(items, m, &segments, font, size, (x, y), Anchor::Start, rotate_ccw);
        return;
    }
    let rows = text_rows(segments);
    let (sin, cos) = rotate_ccw.to_radians().sin_cos();
    let mut offset = 0.0;
    let mut next_advance = 0.0;
    for row in rows {
        if let [Segment::LineGap(percent)] = row.as_slice() {
            if next_advance > 0.0 {
                next_advance += f64::from(*percent) / 100.0 * size;
            }
            continue;
        }
        offset += next_advance;
        let position = (x + offset * sin, y + offset * cos);
        place_segments(items, m, &row, font, size, position, Anchor::Start, rotate_ccw);
        next_advance = 1.3 * size;
    }
}

/// Splits the parsed text so styles spanning a newline are retained on each row.
fn text_rows(segments: Vec<Segment>) -> Vec<Vec<Segment>> {
    let mut rows = vec![Vec::new()];
    for segment in segments {
        let chunks = match segment {
            Segment::Text(mut run) if run.text.contains('\n') => {
                let text = std::mem::take(&mut run.text);
                text.split('\n')
                    .map(|line| {
                        if line.is_empty() {
                            Vec::new()
                        } else {
                            vec![Segment::Text(Run { text: line.into(), ..run.clone() })]
                        }
                    })
                    .collect()
            }
            Segment::Pair { sub, sup } => {
                let mut sub = text_rows(sub).into_iter();
                let mut sup = text_rows(sup).into_iter();
                let count = sub.len().max(sup.len());
                (0..count)
                    .map(|_| {
                        let sub = sub.next().unwrap_or_default();
                        let sup = sup.next().unwrap_or_default();
                        if sub.is_empty() && sup.is_empty() { Vec::new() } else { vec![Segment::Pair { sub, sup }] }
                    })
                    .collect()
            }
            Segment::Accent { kind, content, scale, color } => text_rows(content)
                .into_iter()
                .map(|content| {
                    if content.is_empty() { Vec::new() } else { vec![Segment::Accent { kind, content, scale, color }] }
                })
                .collect(),
            other => vec![vec![other]],
        };
        let mut chunks = chunks.into_iter();
        rows.last_mut().expect("initial row").extend(chunks.next().unwrap_or_default());
        rows.extend(chunks);
    }
    rows
}

pub(crate) fn place_segments(
    items: &mut Vec<Item>,
    m: &dyn TextMeasurer,
    runs: &[Segment],
    font: &Font,
    size: f64,
    (x, y): (f64, f64),
    anchor: Anchor,
    rotate_ccw: f64,
) {
    if runs.is_empty() {
        return;
    }
    // T293: a font with Underline underlines every run.
    let underline_all = font.underline;
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
    let mut glyphs = runs;
    while let [Segment::Accent { content, .. }] = glyphs {
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
        if !(run.underline || underline_all) && !run.strikeout && matches!(runs, [Segment::Text(_)]) {
            return;
        }
    }
    let total = segments_width(m, runs, font, size);
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
    visit_ink(runs, m, font, size, cursor, &mut |mark, cursor| {
        let run = match mark {
            Ink::Glyphs(run) => run,
            Ink::Accent { kind, rise, width, em, color } => {
                items.push(accent_item(kind, width, em, color, |along, vertical| {
                    let dx = cursor + along;
                    let dy = vertical - rise;
                    (x + dx * cos + dy * sin, y - dx * sin + dy * cos)
                }));
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
        for metrics in [
            (run.underline || underline_all).then(|| m.underline_metrics(&font)),
            run.strikeout.then(|| m.strikeout_metrics(&font)),
        ]
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
