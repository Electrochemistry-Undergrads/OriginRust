//! Rug strips beside an axis (T41, graphing spec 18.10): one short line per plotted point,
//! projected onto the axis, in a panel outside the frame between the ticks and the tick labels.

use crate::data::PlotData;
use crate::layout::outward_extent;
use crate::scene::{Item, Stroke, line_to, move_to};
use crate::ticks::ResolvedAxis;
use crate::{LayerGeometry, Rect};
use sheaf_core::Color;
use sheaf_core::graph::{Axis, AxisSide, Layer, Plot, PlotKind, Rug};

/// Size of the panel across the axis, in points: a percentage of the average of the frame's width and height.
pub(crate) fn extent(rug: &Rug, f: Rect) -> f64 {
    if rug.show && f.w.is_finite() && f.h.is_finite() { rug.size_frac() * (f.w + f.h) / 2.0 } else { 0.0 }
}

/// How far the rug of the axis' own edge pushes the tick labels out (0 without a rug).
pub(crate) fn main_extent(axis: &Axis, f: Rect) -> f64 {
    if axis.side == AxisSide::Hidden { 0.0 } else { extent(&axis.rug, f) }
}

/// How far the rug of the opposite edge reaches out of the frame (its ticks, when drawn, come first).
/// `ticks_across` is false where the layer draws no ticks on the opposite edge (a waterfall).
pub(crate) fn opposite_reach(axis: &Axis, f: Rect, ticks_across: bool) -> f64 {
    let s = extent(&axis.rug_opposite, f);
    if s == 0.0 || axis.side == AxisSide::Hidden {
        return 0.0;
    }
    s + if ticks_across && axis.opposite_ticks { outward_extent(axis) } else { 0.0 }
}

/// The colour of a plot: its line colour for line and area kinds, else its symbol colour.
pub(crate) fn plot_color(plot: &Plot) -> Color {
    match plot.kind {
        PlotKind::Line | PlotKind::LineSymbol | PlotKind::Area | PlotKind::StackedArea | PlotKind::Stacked100Area => {
            plot.line.color
        }
        _ => plot.symbol.color,
    }
}

/// The plots a rug shows: all with points, or the first plot of the layer whose Y column is `rug.plot`
/// (`<All>` when no plot has that column any more).
fn selected<'a, 'b>(layer: &Layer, rug: &Rug, pds: &'a [PlotData<'b>]) -> Vec<&'a PlotData<'b>> {
    let chosen = rug.plot.and_then(|id| layer.plots.iter().find(|p| p.y == id));
    pds.iter()
        .filter(|pd| {
            pd.pie.is_none()
                && pd.boxed.is_none()
                && pd.gboxes.is_empty()
                && pd.violin.is_none()
                && pd.ridge.is_none()
                && pd.hex.is_none()
                && pd.column_at.is_none()
        })
        .filter(|pd| chosen.is_none_or(|c| std::ptr::eq(c, pd.plot)))
        .collect()
}

/// Positions (0..1 along the axis, 0 at the axis' start) of the points of one plot that the rug
/// marks: finite in both directions, not masked, within the axis range.
///
/// Rugs mark the plotted positions (offsets, stacks and categorical X as drawn). A histogram is the
/// exception: its plot data is bins, so the rug marks the raw observations along the value axis (X) and
/// nothing on the count axis.
pub(crate) fn unit_positions(pd: &PlotData, axis: &ResolvedAxis, horizontal: bool) -> Vec<f64> {
    if let Some(raw) = &pd.hist_raw {
        if !horizontal {
            return Vec::new();
        }
        return raw
            .iter()
            .filter_map(|&v| {
                let u = axis.to_unit(v);
                (v.is_finite() && u.is_finite() && (-1e-9..=1.0 + 1e-9).contains(&u)).then_some(u.clamp(0.0, 1.0))
            })
            .collect();
    }
    pd.rows()
        .filter(|r| !pd.mask.is_some_and(|m| m.contains(r)))
        .filter_map(|r| {
            let (x, y) = (pd.x_at(r), pd.y_at(r));
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let u = axis.to_unit(if horizontal { x } else { y });
            (u.is_finite() && (-1e-9..=1.0 + 1e-9).contains(&u)).then_some(u.clamp(0.0, 1.0))
        })
        .collect()
}

/// Draws the rug of one edge. `horizontal` is true for the bottom and top edges; `edge` is the page
/// position of the frame edge, `outward` +1 where "out of the frame" is the positive page direction and
/// `reach` how far the ticks go out of the frame there. `border` colours the borders.
#[allow(clippy::too_many_arguments)]
fn draw_edge(
    out: &mut Vec<Item>,
    layer: &Layer,
    rug: &Rug,
    pds: &[PlotData],
    axis: &ResolvedAxis,
    f: Rect,
    horizontal: bool,
    (edge, outward, reach): (f64, f64, f64),
    border: Color,
) {
    let size = extent(rug, f);
    if size <= 0.0 {
        return;
    }
    let plots = selected(layer, rug, pds);
    let n = plots.len();
    if n == 0 {
        return;
    }
    let stacked = !rug.overlap && n > 1;
    let (sub, gap) = if stacked {
        let gap = (rug.gap_pct.clamp(0.0, 100.0) / 100.0 * size).min(0.8 * size / (n - 1) as f64);
        ((size - gap * (n - 1) as f64) / n as f64, gap)
    } else {
        (size, 0.0)
    };
    let margin = rug.margin_pct.clamp(0.0, 49.9) / 100.0 * sub;
    let alpha = 1.0 - rug.transparency_pct.clamp(0.0, 100.0) / 100.0;
    let (along0, along_len) = if horizontal { (f.x, f.w) } else { (f.y, f.h) };
    let pos = |u: f64| if horizontal { along0 + u * along_len } else { along0 + (1.0 - u) * along_len };
    let thickness = rug.thickness_pt.max(0.0);
    let mut borders = String::new();
    for (k, pd) in plots.iter().enumerate() {
        // Distances from the frame edge of this sub-strip's inner and outer sides.
        let near = reach + k as f64 * (if stacked { sub + gap } else { 0.0 });
        let far = near + sub;
        let page = |dist: f64| edge + outward * dist;
        let mut color = rug.color.unwrap_or_else(|| plot_color(pd.plot));
        color.a = (f64::from(color.a) * alpha).round() as u8;
        let mut d = String::new();
        for u in unit_positions(pd, axis, horizontal) {
            let p = pos(u);
            let (a, b) = (page(near + margin), page(far - margin));
            if horizontal {
                move_to(&mut d, p, a);
                line_to(&mut d, p, b);
            } else {
                move_to(&mut d, a, p);
                line_to(&mut d, b, p);
            }
        }
        if !d.is_empty() && thickness > 0.0 {
            out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, thickness)) });
        }
        let (a, b) = (page(near), page(far));
        if rug.parallel_border {
            for c in [a, b] {
                if horizontal {
                    move_to(&mut borders, along0, c);
                    line_to(&mut borders, along0 + along_len, c);
                } else {
                    move_to(&mut borders, c, along0);
                    line_to(&mut borders, c, along0 + along_len);
                }
            }
        }
        if rug.perpendicular_border {
            for p in [along0, along0 + along_len] {
                if horizontal {
                    move_to(&mut borders, p, a);
                    line_to(&mut borders, p, b);
                } else {
                    move_to(&mut borders, a, p);
                    line_to(&mut borders, b, p);
                }
            }
        }
    }
    if !borders.is_empty() {
        out.push(Item::Path { d: borders, fill: None, stroke: Some(Stroke::solid(border, thickness.max(0.25))) });
    }
}

/// Draws the rugs of both axes of a Cartesian layer beside their edges.
pub(crate) fn draw(
    out: &mut Vec<Item>,
    layer: &Layer,
    xa: &Axis,
    ya: &Axis,
    geom: &LayerGeometry,
    pds: &[PlotData],
    xc: Color,
    yc: Color,
) {
    let f = geom.frame;
    for (axis, resolved, horizontal, color) in [(xa, &geom.x, true, xc), (ya, &geom.y, false, yc)] {
        if axis.side == AxisSide::Hidden || (!axis.rug.show && !axis.rug_opposite.show) {
            continue;
        }
        let reach = outward_extent(axis);
        let normal = axis.side == AxisSide::Normal;
        // (page position of the main edge, outward direction) for the axis' own edge and the one across.
        let (main, other) = match (horizontal, normal) {
            (true, true) => ((f.y + f.h, 1.0), (f.y, -1.0)),
            (true, false) => ((f.y, -1.0), (f.y + f.h, 1.0)),
            (false, true) => ((f.x, -1.0), (f.x + f.w, 1.0)),
            (false, false) => ((f.x + f.w, 1.0), (f.x, -1.0)),
        };
        draw_edge(out, layer, &axis.rug, pds, resolved, f, horizontal, (main.0, main.1, reach), color);
        let across = if axis.opposite_ticks { reach } else { 0.0 };
        draw_edge(out, layer, &axis.rug_opposite, pds, resolved, f, horizontal, (other.0, other.1, across), color);
    }
}
