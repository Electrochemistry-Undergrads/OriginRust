//! Significance brackets of a multiple comparisons plot (T64, graphing spec 17.7): a horizontal line over two groups
//! with a short vertical end down to each group, and the label centred above it.
//!
//! The groups stand at X = 1, 2, 3, ... The brackets are stacked: the pairs are sorted by span (the shortest first,
//! then in report order) and each bracket gets the lowest level that clears every element of the groups it spans (bar
//! with error bar, whisker, outlier, violin tip or point) and every earlier bracket whose span touches its own.
//!
//! Two ways to place the stack (documented in `docs/spec/statistics.md`, section 23):
//!
//! - **On the axis** (a linear or logarithmic Y axis whose upper limit is automatic): the levels are found in the
//!   axis-transformed space, so that on a log axis they stack multiplicatively, and the Y axis grows to hold them and
//!   one step of room for the topmost label. One level step is 6 % of the data range (in the transformed space) at
//!   least, and never less than the height of a label plus its gaps in points converted at the final axis scale:
//!   the layout resolves the axes once, converts the label step with the real frame and resolves again until the
//!   step no longer grows (as hexbin does), so that a label never touches the bracket above it.
//! - **In a band above the frame** (the Y axis' upper limit is fixed by the user, or the scale is neither linear nor
//!   logarithmic): the axis is not changed. The stack is made of the spans alone, one level step is the label step in
//!   points, and the lowest line stands just over the frame. The layout reserves the band's height above the frame (or
//!   above the labels and title of an X axis on the top), so nothing overlaps titles or legend; whatever would still
//!   leave the page is not drawn.

use std::collections::BTreeMap;

use crate::LayerGeometry;
use crate::data::{Dim, PlotData};
use crate::fonts::TextMeasurer;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Item};
use crate::text::{ASCENT, DESCENT};
use sheaf_core::Color;
use sheaf_core::compplot::SignificanceBracket;
use sheaf_core::graph::{Layer, LineStyle, ScaleKind};

/// One level step as a fraction of the data range: the minimum, when labels need no more.
pub(crate) const STEP_FRACTION: f64 = 0.06;
/// Length of the vertical ends and gap under the label, in points.
const END_PT: f64 = 5.0;
const LABEL_GAP_PT: f64 = 2.5;
/// Clearance kept between a label and the vertical end of the bracket above it, in points.
const STEP_PAD_PT: f64 = 1.0;

/// A bracket with its place: the X positions of its ends and the Y of its horizontal line, in the axis-transformed
/// space (the data units on a linear axis).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    /// Index into the layer's bracket list.
    pub index: usize,
    pub x0: f64,
    pub x1: f64,
    pub level: f64,
}

/// What stacking found: every bracket's place, the level step and the highest line (axis-transformed).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Plan {
    pub placed: Vec<Placed>,
    pub step: f64,
    /// The highest line plus one step for its label.
    pub top: f64,
}

/// Whether the brackets of `layer` go into a band above the frame instead of on the axis.
pub(crate) fn in_band(layer: &Layer) -> bool {
    !layer.brackets.is_empty()
        && (layer.y_axis.max.is_some()
            || !matches!(layer.y_axis.scale, ScaleKind::Linear | ScaleKind::Log10 | ScaleKind::Ln | ScaleKind::Log2))
}

/// The distance from a bracket's line up to the baseline of its label, in points: the gap, or more for a font whose
/// descenders (the "p" of "p = 0.01") would reach the line.
fn gap_pt(layer: &Layer) -> f64 {
    LABEL_GAP_PT.max(layer.legend.font.size_pt * DESCENT + 1.0)
}

/// The height of a label above its baseline, in points.
fn ascent_pt(layer: &Layer) -> f64 {
    layer.legend.font.size_pt * ASCENT
}

/// The least distance between two levels in points: the label over a line with its gap, and the end of the bracket
/// above it, which hangs down by [`END_PT`].
pub(crate) fn label_step_pt(layer: &Layer) -> f64 {
    gap_pt(layer) + ascent_pt(layer) + END_PT + STEP_PAD_PT
}

/// The height of the band above the frame (from the frame's top, or the base `band_base` the layout gives, up to the
/// top of the highest label), in points; `None` unless the brackets are in the band.
pub(crate) fn band_height_pt(layer: &Layer) -> Option<f64> {
    if !in_band(layer) {
        return None;
    }
    let plan = stack(&layer.brackets, &BTreeMap::new(), 0.0, 1.0);
    let top_level = plan.top - 1.0;
    Some((top_level - 1.0) * label_step_pt(layer) + END_PT + 1.0 + gap_pt(layer) + ascent_pt(layer))
}

/// The top of each group (key: its X position rounded to a whole number) in the transformed space and the data range
/// `(lo, hi)` there.
fn tops(sources: &[&PlotData], scale: ScaleKind) -> (BTreeMap<i64, f64>, Option<(f64, f64)>) {
    let mut tops: BTreeMap<i64, f64> = BTreeMap::new();
    let mut add = |pos: f64, y: f64| {
        let y = scale.forward(y);
        if pos.is_finite() && y.is_finite() {
            let e = tops.entry(pos.round() as i64).or_insert(f64::NEG_INFINITY);
            *e = e.max(y);
        }
    };
    for pd in sources {
        if let Some(b) = &pd.boxed {
            add(b.pos, pd.display_y(b.whisker_high));
            add(b.pos, pd.display_y(b.q3));
            b.outliers.iter().for_each(|v| add(b.pos, pd.display_y(*v)));
            if let Some((_, hi)) = pd.violin.as_ref().and_then(|v| v.y_extent()) {
                add(b.pos, pd.display_y(hi));
            }
            continue;
        }
        if pd.pie.is_some() || pd.hex.is_some() || pd.ridge.is_some() || pd.treemap.is_some() || pd.sunburst.is_some() {
            continue;
        }
        let dir = pd.plot.error.direction;
        for i in pd.rows() {
            let (x, y) = (pd.x_at(i), pd.y_at(i));
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            let above = pd.reach(pd.err_at(i), dir).map_or(0.0, |(_, a)| a);
            let mut top = y + above;
            if pd.bar.is_some() {
                top = top.max(pd.oy);
            }
            add(x, top);
        }
    }
    let range = crate::data::axis_range(sources.iter().copied(), Dim::Y, scale);
    let range = range.map(|(lo, hi)| {
        let lo = if scale == ScaleKind::Linear && sources.iter().any(|pd| pd.bar.is_some()) { lo.min(0.0) } else { lo };
        (scale.forward(lo), scale.forward(hi))
    });
    (tops, range.filter(|(lo, hi)| lo.is_finite() && hi.is_finite()))
}

/// Stacks `brackets` over the groups of `sources` on a Y axis of `scale`; `min_step` is the least level step in the
/// transformed space (0 for none). `None` without brackets or without data.
pub(crate) fn plan(
    brackets: &[SignificanceBracket],
    sources: &[&PlotData],
    scale: ScaleKind,
    min_step: f64,
) -> Option<Plan> {
    if brackets.is_empty() {
        return None;
    }
    let (tops, range) = tops(sources, scale);
    let (lo, hi) = range?;
    let span = hi - lo;
    let step = (STEP_FRACTION * if span > 0.0 { span } else { hi.abs().max(1.0) }).max(min_step);
    Some(stack(brackets, &tops, lo, step))
}

/// The stacking rule, on the group tops alone.
pub(crate) fn stack(brackets: &[SignificanceBracket], tops: &BTreeMap<i64, f64>, floor: f64, step: f64) -> Plan {
    let mut order: Vec<usize> = (0..brackets.len()).collect();
    let span = |b: &SignificanceBracket| b.group_b.abs_diff(b.group_a);
    order.sort_by_key(|&i| (span(&brackets[i]), i));
    let mut placed: Vec<Placed> = Vec::new();
    for i in order {
        let b = &brackets[i];
        let (a, z) = (b.group_a.min(b.group_b), b.group_a.max(b.group_b));
        let (x0, x1) = ((a + 1) as f64, (z + 1) as f64);
        let mut base = (a..=z).map(|g| tops.get(&((g + 1) as i64)).copied().unwrap_or(floor)).fold(floor, f64::max);
        for q in &placed {
            if q.x0 <= x1 && x0 <= q.x1 {
                base = base.max(q.level);
            }
        }
        placed.push(Placed { index: i, x0, x1, level: base + step });
    }
    let top = placed.iter().map(|p| p.level).fold(f64::NEG_INFINITY, f64::max) + step;
    placed.sort_by_key(|p| p.index);
    Plan { placed, step, top }
}

/// The upper end the Y axis must reach for the brackets of `layer`, in data units (`None` without brackets, and in a
/// band, which leaves the axis alone). `min_step` is the layer's label step in the transformed space.
pub(crate) fn needed_top(layer: &Layer, sources: &[&PlotData], min_step: f64) -> Option<f64> {
    if in_band(layer) {
        return None;
    }
    let scale = layer.y_axis.scale;
    plan(&layer.brackets, sources, scale, min_step).map(|p| scale.inverse(p.top))
}

/// The least level step the label height asks for at the final scale of `geom`, in the transformed space (0 when the
/// layer's brackets are not on the axis).
pub(crate) fn required_step(layer: &Layer, geom: &LayerGeometry) -> f64 {
    let span = geom.y.scale_span();
    if in_band(layer) || layer.brackets.is_empty() || !(span > 0.0 && geom.frame.h > 0.0) {
        return 0.0;
    }
    let need = label_step_pt(layer) * span / geom.frame.h;
    // A stack that cannot fit (more levels than the frame has room for) would make the step run away: the stack is
    // kept to 60 % of the axis, and its labels then crowd each other.
    let levels = (stack(&layer.brackets, &BTreeMap::new(), 0.0, 1.0).top - 1.0).max(1.0);
    need.min(0.6 * span / levels)
}

/// Draws the brackets of a layer into its overlay. `min_step` is the layer's label step in the transformed space as
/// the layout settled it; `band_base` the page Y the band above the frame starts at (the frame's top unless an X axis
/// stands on the top).
pub(crate) fn draw(
    out: &mut Vec<Item>,
    layer: &Layer,
    pds: &[PlotData],
    geom: &LayerGeometry,
    (min_step, band_base): (f64, f64),
    m: &dyn TextMeasurer,
) {
    if geom.polar.is_some() || geom.ternary.is_some() || layer.brackets.is_empty() {
        return;
    }
    let sources: Vec<&PlotData> = pds.iter().collect();
    let style = LineStyle::solid(Color::BLACK, 1.0);
    let font = layer.legend.font.clone();
    let size = font.size_pt;
    // (page X of both ends, page Y of the line) of each bracket, in drawing order.
    let mut lines: Vec<(usize, (f64, f64), (f64, f64))> = Vec::new();
    if in_band(layer) {
        let plan = stack(&layer.brackets, &BTreeMap::new(), 0.0, 1.0);
        let step = label_step_pt(layer);
        for p in &plan.placed {
            let y = band_base - END_PT - 1.0 - (p.level - 1.0) * step;
            let (x0, x1) = (geom.x.to_unit(p.x0), geom.x.to_unit(p.x1));
            if !(x0.is_finite() && x1.is_finite()) {
                continue;
            }
            let (px0, px1) = (geom.frame.x + x0 * geom.frame.w, geom.frame.x + x1 * geom.frame.w);
            // Over the page's top edge: not drawn.
            if y - gap_pt(layer) - ascent_pt(layer) < 0.0 {
                continue;
            }
            lines.push((p.index, (px0, y), (px1, y)));
        }
    } else {
        let scale = geom.y.scale;
        let Some(plan) = plan(&layer.brackets, &sources, scale, min_step) else { return };
        for p in &plan.placed {
            let level = scale.inverse(p.level);
            let Some(a) = geom.checked_data_to_page(p.x0, level) else { continue };
            let Some(b) = geom.checked_data_to_page(p.x1, level) else { continue };
            lines.push((p.index, a, b));
        }
    }
    for (index, a, b) in lines {
        let mut d = String::new();
        crate::scene::move_to(&mut d, a.0, a.1 + END_PT);
        crate::scene::line_to(&mut d, a.0, a.1);
        crate::scene::line_to(&mut d, b.0, b.1);
        crate::scene::line_to(&mut d, b.0, b.1 + END_PT);
        out.push(Item::Path { d, fill: None, stroke: Some(line_stroke(&style)) });
        let label = &layer.brackets[index].label;
        // An asterisk sits high in its line of text: lower it so that stars hug the bracket as words do.
        let drop = if label.starts_with('*') { size * 0.35 } else { 0.0 };
        crate::text::place(
            out,
            m,
            label,
            &font,
            size,
            ((a.0 + b.0) / 2.0, a.1 - gap_pt(layer) + drop),
            Anchor::Middle,
            0.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FixedMetrics;
    use crate::layout;
    use sheaf_core::analysis::{AnovaSpec, OutputTarget, Request, RowRange, Tool};
    use sheaf_core::compplot::{ComparisonPlotOptions, ComparisonPlotType};
    use sheaf_core::{ColumnData, Designation, Project};

    fn br(a: usize, b: usize) -> SignificanceBracket {
        SignificanceBracket { group_a: a, group_b: b, p: 0.001, label: "**".into() }
    }

    fn tops(v: &[f64]) -> BTreeMap<i64, f64> {
        v.iter().enumerate().map(|(i, t)| ((i + 1) as i64, *t)).collect()
    }

    #[test]
    fn shortest_spans_are_lowest_and_overlapping_brackets_never_share_a_level() {
        let b = [br(0, 3), br(0, 1), br(1, 2), br(2, 3), br(0, 2)];
        let plan = stack(&b, &tops(&[5.0, 8.0, 6.0, 9.0]), 0.0, 1.0);
        let level = |i: usize| plan.placed[i].level;
        // Over the tallest element beneath: Control-Low over 8, Low-Medium over 8 but touching, ...
        assert_eq!(level(1), 9.0, "0-1 clears the 8 bar by one step");
        assert_eq!(level(3), 11.0, "2-3 clears the 9 bar and touches 1-2 at group 2");
        assert_eq!(level(2), 10.0, "1-2 touches 0-1 at group 1, so it is above it");
        for (i, p) in plan.placed.iter().enumerate() {
            for (j, q) in plan.placed.iter().enumerate() {
                if i < j && p.x0 <= q.x1 && q.x0 <= p.x1 {
                    assert!((p.level - q.level).abs() >= 1.0 - 1e-12, "{p:?} and {q:?} overlap");
                }
            }
        }
        // A longer span is never lower than a shorter one it contains.
        assert!(level(0) > level(4) && level(4) > level(1) && level(4) > level(2));
        // Every bracket is above every group it spans, and the axis needs one more step.
        assert!(level(0) > 9.0);
        assert_eq!(plan.top, plan.placed.iter().map(|p| p.level).fold(0.0, f64::max) + 1.0);
    }

    #[test]
    fn disjoint_brackets_share_the_lowest_level() {
        let b = [br(0, 1), br(2, 3)];
        // 1 and 2 are not touching: groups 0-1 and 2-3.
        let plan = stack(&b, &tops(&[1.0, 1.0, 1.0, 1.0]), 0.0, 0.5);
        assert_eq!(plan.placed[0].level, plan.placed[1].level);
    }

    fn report_graph(kind: ComparisonPlotType) -> (Project, sheaf_core::WindowId) {
        let mut p = Project::new();
        let b = p.add_workbook("data");
        let g: [&[f64]; 3] = [&[1.0, 2.0, 1.5, 1.2], &[2.1, 2.4, 2.9, 2.2], &[5.0, 5.5, 6.1, 5.2]];
        let ids: Vec<_> =
            g.iter().map(|v| p.add_column(b, 0, Designation::Y, ColumnData::numeric(v.to_vec())).unwrap()).collect();
        let r = p
            .analyze_to(
                &Request { columns: ids, rows: RowRange::All, tool: Tool::Anova(AnovaSpec::default()) },
                &OutputTarget::NewSheet,
            )
            .unwrap();
        let o = ComparisonPlotOptions { plot_type: kind, significant_only: false, ..Default::default() };
        let graph = p.comparison_plot(b, r.new_sheet.unwrap(), &o).unwrap();
        (p, graph)
    }

    #[test]
    fn brackets_stand_above_the_data_for_every_plot_type_and_the_axis_holds_them() {
        for kind in ComparisonPlotType::ALL {
            let (p, g) = report_graph(kind);
            let graph = p.graph(g).unwrap();
            let data = crate::data::collect_graph(&p, graph);
            let geoms = layout::graph_geometry(&p, graph, &data, &FixedMetrics);
            let top = geoms[0].y.max;
            let l = &p.graph(g).unwrap().layers[0];
            let sources: Vec<&crate::data::PlotData> = data[0].plots.iter().collect();
            let plan = plan(&l.brackets, &sources, ScaleKind::Linear, data[0].bracket_step.get()).unwrap();
            assert!(top >= plan.top - 1e-9, "{kind:?}: axis top {top} holds the labels up to {}", plan.top);
            let (tops, _) = super::tops(&sources, ScaleKind::Linear);
            let tallest = tops.values().copied().fold(f64::NEG_INFINITY, f64::max);
            assert!(plan.placed.iter().all(|q| q.level > tallest - 1e-12 || q.x1 - q.x0 < 2.0), "{kind:?}");
            let lowest = plan.placed.iter().map(|q| q.level).fold(f64::INFINITY, f64::min);
            assert!(lowest > tops[&1].min(tallest), "{kind:?}");
        }
    }

    /// Bracket lines of a drawn layer as `(label, line points)`: each path is followed by its label's text item.
    fn drawn(items: &[Item], labels: &[&str]) -> Vec<(String, Vec<(f64, f64)>, (f64, f64))> {
        let mut out = Vec::new();
        for pair in items.windows(2) {
            if let [Item::Path { d, .. }, Item::Text(t)] = pair
                && labels.contains(&t.text.as_str())
                && d.starts_with('M')
            {
                let nums: Vec<f64> =
                    d.replace(['M', 'L'], " ").split_whitespace().filter_map(|v| v.parse().ok()).collect();
                let pts: Vec<(f64, f64)> = nums.chunks(2).map(|c| (c[0], c[1])).collect();
                out.push((t.text.clone(), pts, (t.x, t.y)));
            }
        }
        out
    }

    fn graph_with<F: FnOnce(&mut sheaf_core::graph::Layer)>(
        kind: ComparisonPlotType,
        f: F,
    ) -> (Project, sheaf_core::WindowId) {
        let (mut p, g) = report_graph(kind);
        f(&mut p.graph_mut(g).unwrap().layers[0]);
        (p, g)
    }

    #[test]
    fn a_log_axis_stacks_the_brackets_in_log_space_inside_the_extended_range() {
        let (p, g) = graph_with(ComparisonPlotType::Box, |l| l.y_axis.scale = ScaleKind::Log10);
        let graph = p.graph(g).unwrap();
        let data = crate::data::collect_graph(&p, graph);
        let geoms = layout::graph_geometry(&p, graph, &data, &FixedMetrics);
        let l = &graph.layers[0];
        let y = &geoms[0].y;
        let sources: Vec<&crate::data::PlotData> = data[0].plots.iter().collect();
        let plan = plan(&l.brackets, &sources, ScaleKind::Log10, data[0].bracket_step.get()).unwrap();
        // The step is one of log space: every line is a constant factor above the one it clears.
        assert!(plan.step > 0.0);
        let highest = plan.placed.iter().map(|q| q.level).fold(f64::NEG_INFINITY, f64::max);
        assert!(10f64.powf(plan.top) <= y.max * (1.0 + 1e-9), "axis max {} holds {}", y.max, 10f64.powf(plan.top));
        assert!(10f64.powf(highest) < y.max, "the highest line is inside the extended range");
        // A linear-space step of 6 % of the range would put the lines at the wrong height: the first level is the
        // tallest group top times 10^step.
        let (tops, _) = super::tops(&sources, ScaleKind::Log10);
        let tallest_first = plan.placed[0].level - tops[&1].max(tops[&2]);
        assert!((tallest_first - plan.step).abs() < 1e-9, "level one step over the group tops in log space");
        let mut out = Vec::new();
        draw(&mut out, l, &data[0].plots, &geoms[0], (data[0].bracket_step.get(), geoms[0].frame.y), &FixedMetrics);
        let lines = drawn(&out, &["**", "***", "****", "*", "ns"]);
        assert_eq!(lines.len(), 3);
        let f = geoms[0].frame;
        for (_, pts, (_, ty)) in &lines {
            assert!(pts.iter().all(|q| q.1 >= f.y - 1e-6 && q.1 <= f.y + f.h), "line inside the frame: {pts:?}");
            assert!(*ty > f.y, "label inside the frame");
        }
    }

    #[test]
    fn a_fixed_maximum_puts_the_brackets_in_a_band_above_the_frame_that_the_layout_reserves() {
        let plain = {
            let (p, g) = graph_with(ComparisonPlotType::Bar, |l| {
                l.y_axis.max = Some(8.0);
                l.brackets.clear();
            });
            let graph = p.graph(g).unwrap();
            layout::graph_geometry(&p, graph, &crate::data::collect_graph(&p, graph), &FixedMetrics)[0].frame
        };
        for top_axis in [false, true] {
            let (p, g) = graph_with(ComparisonPlotType::Bar, |l| {
                l.y_axis.max = Some(8.0);
                if top_axis {
                    l.x_axis.side = sheaf_core::graph::AxisSide::Opposite;
                    l.x_axis.title = Some("Treatment".to_string());
                    l.x_axis.show_title = true;
                }
            });
            let graph = p.graph(g).unwrap();
            let data = crate::data::collect_graph(&p, graph);
            let geoms = layout::graph_geometry(&p, graph, &data, &FixedMetrics);
            let l = &graph.layers[0];
            assert_eq!(geoms[0].y.max, 8.0, "a fixed limit is not moved");
            assert!(in_band(l));
            let band = band_height_pt(l).unwrap();
            let f = geoms[0].frame;
            let scene = layout::build_scene(&p, g, &FixedMetrics).unwrap();
            let overlay = &scene.layers[0].overlay;
            let lines = drawn(overlay, &["**", "***", "****", "*", "ns"]);
            assert_eq!(lines.len(), 3);
            // The title of the top axis, if any: bracket lines stay above its top edge.
            let title_top = overlay.iter().find_map(|i| match i {
                Item::Text(t) if t.text == "Treatment" => Some(t.y - ASCENT * t.size),
                _ => None,
            });
            assert_eq!(title_top.is_some(), top_axis);
            for (label, pts, (_, ty)) in &lines {
                for q in pts {
                    assert!(q.1 >= -1e-6, "{label}: on the page");
                    match title_top {
                        Some(t) => assert!(q.1 <= t + 1e-6, "{label}: line at {} over the title top {t}", q.1),
                        None => assert!(q.1 <= f.y + 1e-6, "{label}: line at {} above the frame {}", q.1, f.y),
                    }
                }
                assert!(*ty - ASCENT * 9.0 >= 0.0, "label on the page");
            }
            if !top_axis {
                let highest =
                    lines.iter().flat_map(|(_, pts, _)| pts.iter().map(|q| q.1)).fold(f64::INFINITY, f64::min);
                assert!(highest >= f.y - band, "the lines stay inside the band: {highest} vs {}", f.y - band);
                // The layout moved the frame down to make room: its top is lower than without brackets.
                assert!(f.y > plain.y - 1e-9, "frame top {} vs {}", f.y, plain.y);
            }
        }
    }

    #[test]
    fn many_pairs_keep_every_label_clear_of_every_bracket_line() {
        // Six groups, all 15 pairs, p-value labels.
        let mut p = Project::new();
        let b = p.add_workbook("data");
        let data: Vec<Vec<f64>> = (0..6)
            .map(|k| (0..5).map(|i| 3.0 * k as f64 + 1.0 + 0.4 * ((i * 7 + k * 3) % 5) as f64).collect())
            .collect();
        let ids: Vec<_> =
            data.iter().map(|v| p.add_column(b, 0, Designation::Y, ColumnData::numeric(v.clone())).unwrap()).collect();
        let r = p
            .analyze_to(
                &Request { columns: ids, rows: RowRange::All, tool: Tool::Anova(AnovaSpec::default()) },
                &OutputTarget::NewSheet,
            )
            .unwrap();
        let o = ComparisonPlotOptions {
            plot_type: ComparisonPlotType::Box,
            significant_only: false,
            labels: sheaf_core::compplot::BracketLabels::PValues,
            ..Default::default()
        };
        let g = p.comparison_plot(b, r.new_sheet.unwrap(), &o).unwrap();
        // The labels are set in the layer's legend font: 9 pt, so that fifteen levels fit the frame.
        p.graph_mut(g).unwrap().layers[0].legend.font.size_pt = 9.0;
        let graph = p.graph(g).unwrap();
        let pds = crate::data::collect_graph(&p, graph);
        let geoms = layout::graph_geometry(&p, graph, &pds, &FixedMetrics);
        let l = &graph.layers[0];
        assert_eq!(l.brackets.len(), 15);
        let mut out = Vec::new();
        draw(&mut out, l, &pds[0].plots, &geoms[0], (pds[0].bracket_step.get(), geoms[0].frame.y), &FixedMetrics);
        let labels: Vec<String> = l.brackets.iter().map(|b| b.label.clone()).collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let lines = drawn(&out, &label_refs);
        assert_eq!(lines.len(), 15);
        let size = l.legend.font.size_pt;
        for (i, (label, _, (tx, ty))) in lines.iter().enumerate() {
            let half = FixedMetrics.width(label, &l.legend.font, size) / 2.0;
            // The text box: baseline `ty`, ascent up and descent down.
            let (x0, x1, y0, y1) = (tx - half, tx + half, ty - ASCENT * size, ty + DESCENT * size);
            for (j, (_, pts, _)) in lines.iter().enumerate() {
                for seg in pts.windows(2) {
                    let (a, c) = (seg[0], seg[1]);
                    let (sx0, sx1) = (a.0.min(c.0), a.0.max(c.0));
                    let (sy0, sy1) = (a.1.min(c.1), a.1.max(c.1));
                    let hit = sx0 <= x1 && sx1 >= x0 && sy0 <= y1 && sy1 >= y0;
                    assert!(
                        !hit,
                        "label {i} ({label}) box {x0:.1},{y0:.1},{x1:.1},{y1:.1} touches bracket {j}: {seg:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_graph_without_brackets_is_drawn_exactly_as_before() {
        let (mut p, g) = report_graph(ComparisonPlotType::Box);
        let svg = |p: &Project| crate::render_svg_with(p, g, &FixedMetrics).unwrap();
        let with = svg(&p);
        assert!(with.matches("<path").count() > 0);
        {
            let l = &mut p.graph_mut(g).unwrap().layers[0];
            l.brackets.clear();
        }
        let without = svg(&p);
        assert_ne!(with, without, "the brackets add items");
        // The options alone (no brackets) change nothing.
        let mut q = p.clone();
        q.graph_mut(g).unwrap().layers[0].comparison = None;
        assert_eq!(svg(&q), without);
    }
}
