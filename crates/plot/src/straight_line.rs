//! Straight lines at an axis value (T67, graphing spec "Straight lines"): a vertical line at an X
//! value or a horizontal one at a Y value, drawn in data coordinates through the layer's axes, with
//! a label that shows the value.
//!
//! Measured in Origin 2016's Add Straight Line: the label's centre sits 5 % of the axis span away
//! from the line (vertical Top: above the frame, Bottom: below it, Left and Right: beside the line
//! half-way up; horizontal Top and Bottom: above and below the line at the middle of the X range,
//! Left and Right: outside the frame's left and right edges at the line's Y). Labels are not
//! clipped to the frame. A value outside the axis range draws nothing.

use crate::LayerGeometry;
use crate::fonts::TextMeasurer;
use crate::hit;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Item, ItemList, ObjectKind, SceneObject, line_to, move_to};
use crate::ticks::{ResolvedAxis, decimal, linear_labels, scientific};
use sheaf_calc::trend::sig_str;
use sheaf_core::Project;
use sheaf_core::graph::{Axis, Font, Layer, LineStyle, ScaleKind, TickFormat};
use sheaf_core::straight_line::{StraightLine, StraightLineLabel};

/// Distance from the line to the label's centre, as a fraction of the frame's width or height.
const LABEL_GAP: f64 = 0.05;

/// The label text: the value in the line's own fixed format, or in the axis' tick label format.
pub(crate) fn value_text(line: &StraightLine, axis: &Axis, r: &ResolvedAxis) -> String {
    let v = line.value;
    if let Some(d) = line.fixed_decimals() {
        return decimal(v, d);
    }
    match r.scale {
        ScaleKind::Log10 | ScaleKind::Ln | ScaleKind::Log2 => match axis.tick_format {
            TickFormat::Scientific(_) => scientific(v, None),
            TickFormat::Decimal(d) => decimal(v, usize::from(d.min(15))),
            _ => sig_str(v, 6),
        },
        _ => {
            let step = if r.step.is_finite() && r.step != 0.0 { r.step.abs() } else { 1.0 };
            linear_labels(axis, &[v], step).into_iter().next().unwrap_or_default()
        }
    }
}

/// Draws the straight lines of a layer into its overlay and records each as a pickable object.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw(
    out: &mut Vec<Item>,
    objects: &mut Vec<SceneObject>,
    _project: &Project,
    layer: &Layer,
    index: usize,
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
) {
    if geom.polar.is_some() || geom.ternary.is_some() {
        return;
    }
    let f = geom.frame;
    for (k, s) in layer.straight_lines.iter().enumerate() {
        let (along, across, axis, other) = if s.vertical {
            (&geom.x, &geom.y, &layer.x_axis, &layer.y_axis)
        } else {
            (&geom.y, &geom.x, &layer.y_axis, &layer.x_axis)
        };
        let _ = other;
        let u = along.to_unit(s.value);
        if !s.value.is_finite() || !u.is_finite() || !(-1e-9..=1.0 + 1e-9).contains(&u) {
            continue;
        }
        // Page position across the line's own axis, and the ends along the other one.
        let (lo, hi) = if s.vertical { (f.y, f.y + f.h) } else { (f.x, f.x + f.w) };
        let at_unit = |ua: f64| {
            if s.vertical { lo + (1.0 - ua) * (hi - lo) } else { lo + ua * (hi - lo) }
        };
        let (mut e0, mut e1) = (lo, hi);
        if let Some((a, b)) = s.span {
            let (ua, ub) = (across.to_unit(a), across.to_unit(b));
            if ua.is_finite() && ub.is_finite() {
                let (pa, pb) = (at_unit(ua).clamp(lo, hi), at_unit(ub).clamp(lo, hi));
                (e0, e1) = (pa.min(pb), pa.max(pb));
            }
        }
        let pos = if s.vertical { f.x + u * f.w } else { f.y + (1.0 - u) * f.h };
        let start = out.len();
        let width = s.width_pt.max(0.0);
        if width > 0.0 && e1 > e0 {
            let mut d = String::new();
            if s.vertical {
                move_to(&mut d, pos, e0);
                line_to(&mut d, pos, e1);
            } else {
                move_to(&mut d, e0, pos);
                line_to(&mut d, e1, pos);
            }
            let style = LineStyle { dash: s.dash, ..LineStyle::solid(s.color, width) };
            out.push(Item::Path { d, fill: None, stroke: Some(line_stroke(&style)) });
        }
        if s.show_label {
            let (gx, gy) = (LABEL_GAP * f.w, LABEL_GAP * f.h);
            let (cx, cy) = match (s.vertical, s.location) {
                (true, StraightLineLabel::Top) => (pos, f.y - gy),
                (true, StraightLineLabel::Bottom) => (pos, f.y + f.h + gy),
                (true, StraightLineLabel::Left) => (pos - gx, f.y + f.h / 2.0),
                (true, StraightLineLabel::Right) => (pos + gx, f.y + f.h / 2.0),
                (false, StraightLineLabel::Top) => (f.x + f.w / 2.0, pos - gy),
                (false, StraightLineLabel::Bottom) => (f.x + f.w / 2.0, pos + gy),
                (false, StraightLineLabel::Left) => (f.x - gx, pos),
                (false, StraightLineLabel::Right) => (f.x + f.w + gx, pos),
            };
            let font = s.font.clone().unwrap_or_else(|| Font { color: s.color, ..layer.new_label_font() });
            let size = font.size_pt;
            // The centre of the text is the anchor: drop the baseline by about a third of the size.
            crate::text::place(
                out,
                m,
                &value_text(s, axis, along),
                &font,
                size,
                (cx, cy + 0.35 * size),
                Anchor::Middle,
                0.0,
            );
        }
        hit::record(objects, out, start, ObjectKind::StraightLine(k), index, ItemList::Overlay, &[], m);
    }
}
