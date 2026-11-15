//! Significance brackets of a multiple comparisons plot (T58, graphing spec 17.7): a horizontal line over two groups
//! with a short vertical end down to each group, and the label centred above it.
//!
//! The groups stand at X = 1, 2, 3, ... The brackets are stacked: the pairs are sorted by span (the shortest first,
//! then in report order) and each bracket gets the lowest level that clears every element of the groups it spans (bar
//! with error bar, whisker, outlier, violin tip or point) and every earlier bracket whose span touches its own. One
//! level step is 6 % of the data range (Sheaf's choice, unmeasured). The Y axis range grows to hold the brackets and
//! one step of room for the topmost label, unless the axis' upper limit is fixed.

use std::collections::BTreeMap;

use crate::LayerGeometry;
use crate::data::{Dim, PlotData};
use crate::fonts::TextMeasurer;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Item};
use sheaf_core::Color;
use sheaf_core::compplot::SignificanceBracket;
use sheaf_core::graph::{Layer, LineStyle};

/// One level step as a fraction of the data range.
pub(crate) const STEP_FRACTION: f64 = 0.06;
/// Length of the vertical ends and gap under the label, in points.
const END_PT: f64 = 5.0;
const LABEL_GAP_PT: f64 = 2.5;

/// A bracket with its place: the X positions of its ends and the Y of its horizontal line, in data units.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    /// Index into the layer's bracket list.
    pub index: usize,
    pub x0: f64,
    pub x1: f64,
    pub level: f64,
}

/// What stacking found: every bracket's place, the level step and the highest line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Plan {
    pub placed: Vec<Placed>,
    pub step: f64,
    /// The highest line plus one step for its label.
    pub top: f64,
}

/// The top of each group (key: its X position rounded to a whole number) and the data range `(lo, hi)`.
fn tops(sources: &[&PlotData]) -> (BTreeMap<i64, f64>, Option<(f64, f64)>) {
    let mut tops: BTreeMap<i64, f64> = BTreeMap::new();
    let mut add = |pos: f64, y: f64| {
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
    let range = crate::data::axis_range(sources.iter().copied(), Dim::Y, sheaf_core::graph::ScaleKind::Linear);
    let range = range.map(|(lo, hi)| {
        let lo = if sources.iter().any(|pd| pd.bar.is_some()) { lo.min(0.0) } else { lo };
        (lo, hi)
    });
    (tops, range)
}

/// Stacks `brackets` over the groups of `sources`. `None` without brackets or without data.
pub(crate) fn plan(brackets: &[SignificanceBracket], sources: &[&PlotData]) -> Option<Plan> {
    if brackets.is_empty() {
        return None;
    }
    let (tops, range) = tops(sources);
    let (lo, hi) = range?;
    let span = hi - lo;
    let step = STEP_FRACTION * if span > 0.0 { span } else { hi.abs().max(1.0) };
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

/// The upper end the Y axis must reach for the brackets of `layer` (`None` without brackets).
pub(crate) fn needed_top(layer: &Layer, sources: &[&PlotData]) -> Option<f64> {
    plan(&layer.brackets, sources).map(|p| p.top)
}

/// Draws the brackets of a layer into its overlay.
pub(crate) fn draw(out: &mut Vec<Item>, layer: &Layer, pds: &[PlotData], geom: &LayerGeometry, m: &dyn TextMeasurer) {
    if geom.polar.is_some() || geom.ternary.is_some() || layer.brackets.is_empty() {
        return;
    }
    let sources: Vec<&PlotData> = pds.iter().collect();
    let Some(plan) = plan(&layer.brackets, &sources) else { return };
    let style = LineStyle::solid(Color::BLACK, 1.0);
    let font = layer.legend.font.clone();
    let size = font.size_pt;
    for p in &plan.placed {
        let Some(a) = geom.checked_data_to_page(p.x0, p.level) else { continue };
        let Some(b) = geom.checked_data_to_page(p.x1, p.level) else { continue };
        let mut d = String::new();
        crate::scene::move_to(&mut d, a.0, a.1 + END_PT);
        crate::scene::line_to(&mut d, a.0, a.1);
        crate::scene::line_to(&mut d, b.0, b.1);
        crate::scene::line_to(&mut d, b.0, b.1 + END_PT);
        out.push(Item::Path { d, fill: None, stroke: Some(line_stroke(&style)) });
        let label = &layer.brackets[p.index].label;
        // An asterisk sits high in its line of text: lower it so that stars hug the bracket as words do.
        let drop = if label.starts_with('*') { size * 0.35 } else { 0.0 };
        crate::text::place(
            out,
            m,
            label,
            &font,
            size,
            ((a.0 + b.0) / 2.0, a.1 - LABEL_GAP_PT + drop),
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
            let geoms = layout::graph_geometry(&p, graph, &crate::data::collect_graph(&p, graph), &FixedMetrics);
            let top = geoms[0].y.max;
            let l = &p.graph(g).unwrap().layers[0];
            let data = crate::data::collect(&p, l);
            let sources: Vec<&crate::data::PlotData> = data.plots.iter().collect();
            let plan = plan(&l.brackets, &sources).unwrap();
            assert!(top >= plan.top - 1e-9, "{kind:?}: axis top {top} holds the labels up to {}", plan.top);
            let (tops, _) = super::tops(&sources);
            let tallest = tops.values().copied().fold(f64::NEG_INFINITY, f64::max);
            assert!(plan.placed.iter().all(|q| q.level > tallest - 1e-12 || q.x1 - q.x0 < 2.0), "{kind:?}");
            let lowest = plan.placed.iter().map(|q| q.level).fold(f64::INFINITY, f64::min);
            assert!(lowest > tops[&1].min(tallest), "{kind:?}");
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
