//! Graph to scene (spec section 3): frames, axes, ticks, labels, titles,
//! legend and free text, in drawing order.
//!
//! Layers that are linked to another layer (the right Y axis of a double-Y
//! graph, graphing spec section 12, or the panels of a stack that share X,
//! section 15) are laid out together with it as one group: their frames are
//! placed inside or beside its frame and, when linked, they share its axis.
//!
//! A layer drawn as a waterfall (graphing spec section 13.6) lays its axes out
//! on a smaller front frame and draws each later curve moved right and up.

use crate::data::{self, Dim, LayerData, PlotData};
use crate::fill;
use crate::fonts::TextMeasurer;
use crate::hit;
use crate::legend;
use crate::plots::{draw_plot, line_stroke, push_symbols};
use crate::polar;
use crate::scene::{
    Anchor, Cap, Item, ItemList, Join, LayerScene, ObjectKind, Scene, SceneObject, Stroke, line_to, move_to,
};
use crate::stack;
use crate::ternary;
use crate::text::{self, ASCENT, DESCENT};
use crate::ticks::{ResolvedAxis, resolve_axis_edges};
use crate::{LayerGeometry, PlotError, Rect};
use sheaf_core::Color;
use sheaf_core::graph::{
    Axis, AxisSide, Font, Graph, Layer, Legend, LegendPosition, LegendScope, LineStyle, Page, Plot, PlotKind,
    ScaleLink, StatPlot, TickDirection,
};
use sheaf_core::{Project, WindowId};

/// Builds the scene of a graph.
pub(crate) fn build_scene(project: &Project, graph: WindowId, m: &dyn TextMeasurer) -> Result<Scene, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let pds: Vec<LayerData> = data::collect_graph(project, g);
    let geoms = graph_geometry(project, g, &pds, m);
    let x_titles = title_places(project, g, &geoms, &pds, Dim::X, m);
    let y_titles = title_places(project, g, &geoms, &pds, Dim::Y, m);
    let mut layers: Vec<LayerScene> = (0..g.layers.len())
        .map(|i| {
            // Trendline labels of the layers in one frame (a layer and its right Y layer) stack.
            let first_line: usize = g
                .same_frame(i)
                .into_iter()
                .filter(|&j| j < i && g.layers[j].display.show_data)
                .map(|j| crate::trend::label_lines(&pds[j].plots, &geoms[j]))
                .sum();
            layer_scene(
                project,
                g,
                i,
                &geoms[i],
                &pds[i],
                (x_titles[i], y_titles[i]),
                first_line,
                pds[i].bracket_step.get(),
                m,
            )
        })
        .collect();
    // A legend that lists the plots of every layer drawn in its frame (a layer and its right Y
    // layer) is drawn after all layers, so that no layer's curves cover it.
    for (i, layer) in g.layers.iter().enumerate() {
        if !layer.display.hidden
            && layer.display.show_labels
            && layer.legend.scope == LegendScope::Graph
            && let Some(last) = layers.last_mut()
        {
            let entries: Vec<&PlotData> = g.same_frame(i).into_iter().flat_map(|j| pds[j].plots.iter()).collect();
            let start = last.overlay.len();
            let ids = legend::ids(g, i, &|j| pds[j].plots.len());
            draw_legend(
                &mut last.overlay,
                project,
                &layer.legend,
                geoms[i].frame,
                &entries,
                &ids,
                &edge_ticks(g, i),
                m,
                &layer.plots,
            );
            hit::record(&mut last.objects, &last.overlay, start, ObjectKind::Legend, i, ItemList::Overlay, &[], m);
        }
    }
    // An axis title brought to the front is drawn last in its layer (T153).
    for (i, layer) in g.layers.iter().enumerate() {
        for (on_top, kind) in
            [(layer.x_axis.title_on_top, ObjectKind::XTitle), (layer.y_axis.title_on_top, ObjectKind::YTitle)]
        {
            if on_top {
                for ls in layers.iter_mut() {
                    move_to_top(ls, kind, i);
                }
            }
        }
    }
    // A legend pushed to the back of the data is drawn before the plots of the layer it is in.
    for (i, layer) in g.layers.iter().enumerate() {
        if layer.legend.behind_data {
            for ls in layers.iter_mut() {
                legend_behind_data(ls, i);
            }
        }
    }
    // The page's border line, over everything, inside the page edge.
    if let (Some(border), Some(last)) = (g.page.style.border.as_ref().filter(|b| b.width_pt > 0.0), layers.last_mut()) {
        let inset = border.width_pt / 2.0;
        let rect = Rect { x: inset, y: inset, w: g.page.width_pt - 2.0 * inset, h: g.page.height_pt - 2.0 * inset };
        last.overlay.push(Item::Rect { rect, fill: None, stroke: Some(fill::border_stroke(border)) });
    }
    Ok(Scene { width: g.page.width_pt, height: g.page.height_pt, background: g.page.background, layers })
}

/// Smallest distance in points kept between any label or title and the page edge.
const PAGE_MARGIN: f64 = 3.0;

/// The geometry of every layer of a graph, in layer order.
///
/// Each group (a layer that is not linked, followed by the layers linked to it) is resolved
/// together: an axis linked straight to the group's first layer is that layer's axis, whose
/// automatic range covers the data of every layer that shares it. The rectangle that holds the
/// nominal frames of every member (the first layer's frame, or a whole stack of panels) is moved
/// inwards just enough that the tick labels and titles of every member stay inside the page (spec
/// section 3, "Keeping labels on the page"), and the frames are moved and scaled with it, so the
/// panels of a stack stay aligned. When everything already fits, the frames are the nominal ones.
///
/// A layer drawn as a waterfall gets the front frame of spec 13.6 as its frame and the whole
/// fitted frame as its clip rectangle.
pub(crate) fn graph_geometry(
    project: &Project,
    g: &Graph,
    pds: &[LayerData],
    m: &dyn TextMeasurer,
) -> Vec<LayerGeometry> {
    let mut geoms = graph_geometry_once(project, g, pds, m);
    // Hexbin: the axis range comes from the binning for the nominal frame, the drawing bins for the real
    // one (narrowed by the colour scale). Widen the range to the hexagons as drawn until they fit.
    for _ in 0..6 {
        let mut grew = false;
        for (d, geom) in pds.iter().zip(&geoms) {
            for pd in &d.plots {
                if let Some(h) = &pd.hex {
                    grew |= h.grow_to_drawn(&pd.plot.hexbin, geom);
                }
            }
        }
        if !grew {
            break;
        }
        geoms = graph_geometry_once(project, g, pds, m);
    }
    // Significance brackets: a label needs a fixed height in points, which is a number of data units only at the
    // final scale of the axis. Convert it with the frame just laid out and resolve the axes again until the level
    // step stops growing (the axis grows with the step, and so does the step).
    for _ in 0..30 {
        let mut grew = false;
        for ((layer, d), geom) in g.layers.iter().zip(pds).zip(&geoms) {
            let need = crate::brackets::required_step(layer, geom);
            if need > d.bracket_step.get() * (1.0 + 1e-4) + 1e-12 {
                d.bracket_step.set(need);
                grew = true;
            }
        }
        if !grew {
            break;
        }
        geoms = graph_geometry_once(project, g, pds, m);
    }
    geoms
}

fn graph_geometry_once(project: &Project, g: &Graph, pds: &[LayerData], m: &dyn TextMeasurer) -> Vec<LayerGeometry> {
    let mut out: Vec<Option<LayerGeometry>> = vec![None; g.layers.len()];
    for root in 0..g.layers.len() {
        if g.root_of(root) == root {
            for (i, geom) in group_geometry(project, g, &g.group(root), pds, m) {
                out[i] = Some(geom);
            }
        }
    }
    out.into_iter().map(|geom| geom.expect("every layer belongs to the group of its root")).collect()
}

/// Resolves one axis from the plots it shows (`sources`, the first of which is the axis' first plot).
///
/// - An X axis whose first plot has text in its X column (sample names) is a category axis (see
///   [`data::is_category`]): one tick per row, half a step of room at each end.
/// - An X axis over column plots gets half a bar spacing of room at each end, without rounding, so
///   that the end bars are not cut; a linear Y axis over column plots reaches down to their base.
/// - An X axis over a date or time column gets date tick labels (see [`data::date_axis`]).
/// - An axis with a label column shows that column's cells at whole-number ticks.
pub(crate) fn resolve_dim(
    project: &Project,
    axis: &Axis,
    dim: Dim,
    sources: &[&data::PlotData],
    (layer, bracket_step): (&sheaf_core::graph::Layer, f64),
    fields: &[&crate::field::FieldGrid],
) -> ResolvedAxis {
    let first = sources.first().copied();
    let mut range = data::axis_range(sources.iter().copied(), dim, axis.scale);
    // The extent of a heat map or contour plot counts as data (T192).
    for f in fields {
        let (lo, hi) = f.extent(dim == Dim::X);
        if lo.is_finite() && hi.is_finite() && (axis.scale == sheaf_core::graph::ScaleKind::Linear || lo > 0.0) {
            range = Some(range.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
        }
    }
    // An axis that shows only a heat map or contour plot takes the tick step nearest to a seventh
    // of its range (Origin 2016 puts a tick every 1 on -3..3, every 0.5 on 0..4 and on 0..3).
    let field_axis;
    let axis = match range {
        Some((lo, hi))
            if sources.is_empty()
                && !fields.is_empty()
                && axis.major_step.is_none()
                && axis.scale == sheaf_core::graph::ScaleKind::Linear
                && hi > lo =>
        {
            field_axis = Axis { major_step: Some(crate::ticks::nearest_nice_step((hi - lo) / 7.0)), ..axis.clone() };
            &field_axis
        }
        _ => axis,
    };
    // Significance brackets (T64) are above the data: the Y axis grows to hold them (a log axis in log space),
    // unless its top is fixed, when they stand in a band above the frame.
    if dim == Dim::Y
        && let Some(top) = crate::brackets::needed_top(layer, sources, bracket_step)
    {
        range = range.map(|(lo, hi)| (lo, hi.max(top)));
    }
    let boxes: Vec<&data::BoxStats> = sources.iter().filter_map(|pd| pd.boxed.as_ref()).collect();
    let ridges: Vec<&crate::ridgeline::RidgeData> = sources.iter().filter_map(|pd| pd.ridge.as_ref()).collect();
    let mut resolved = if dim == Dim::Y && !ridges.is_empty() && axis.scale == sheaf_core::graph::ScaleKind::Linear {
        // Ridgelines: one baseline per column at 1, 2, 3, ... named by its column; the axis reaches
        // below the first baseline (and its rug) and above the highest peak (graphing spec 14.23).
        let n = ridges.len() as f64;
        let rug =
            sources.iter().filter_map(|pd| pd.ridge.as_ref().map(|r| r.rug_reach(&pd.plot.ridge))).fold(0.0, f64::max);
        let top = ridges.iter().map(|r| r.pos + r.peak() * r.scale).fold(n, f64::max);
        let (lo, hi) = ((1.0 - rug - 0.1).min(0.5), top + 0.1);
        let mut r = crate::ticks::resolve_category(axis, Some((lo + 0.5, hi - 0.5)));
        let names: Vec<String> = ridges.iter().map(|r| r.label.clone()).collect();
        crate::ticks::label_from_cells(&mut r, &names);
        r
    } else if dim == Dim::X && sources.iter().any(|pd| pd.categories.is_some()) {
        // Indexed data (T76): one category per level of the main group, at 1, 2, ...
        let names = sources.iter().find_map(|pd| pd.categories.clone()).unwrap_or_default();
        let mut r = crate::ticks::resolve_category(axis, Some((1.0, names.len() as f64)));
        crate::ticks::label_from_cells(&mut r, &names);
        r
    } else if dim == Dim::X && sources.iter().any(|pd| pd.slot_label.is_some()) {
        // Interval plot (T83): one category per plotted column, named by it.
        let names: Vec<String> = sources.iter().filter_map(|pd| pd.slot_label.clone()).collect();
        let mut r = crate::ticks::resolve_category(axis, Some((1.0, names.len() as f64)));
        crate::ticks::label_from_cells(&mut r, &names);
        r
    } else if dim == Dim::X && !boxes.is_empty() {
        // Box charts: one category per box, named by its column.
        let mut r = crate::ticks::resolve_category(axis, Some((1.0, boxes.len() as f64)));
        let names: Vec<String> = boxes.iter().map(|b| b.label.clone()).collect();
        crate::ticks::label_from_cells(&mut r, &names);
        r
    } else if dim == Dim::X && sources.iter().any(|pd| pd.column_at.is_some()) {
        // Column scatter: one category per plot, named by its column (0.5 .. n + 0.5).
        let names: Vec<String> = sources.iter().filter_map(|pd| pd.column_at.as_ref().map(|c| c.1.clone())).collect();
        let mut r = crate::ticks::resolve_category(axis, Some((1.0, names.len() as f64)));
        crate::ticks::label_from_cells(&mut r, &names);
        r
    } else if dim == Dim::X && data::is_category(project, axis, first) {
        crate::ticks::resolve_category(axis, range)
    } else {
        let mut drawn = if dim == Dim::X { data::date_axis(axis, sources.iter().copied()) } else { None };
        let linear = axis.scale == sheaf_core::graph::ScaleKind::Linear;
        let mut free = (true, true);
        let bars: Vec<&data::PlotData> = sources.iter().copied().filter(|pd| pd.bar.is_some()).collect();
        if linear && let (Some((lo, hi)), Some(pd)) = (range, bars.first()) {
            // The bar positions run along X, or along Y for horizontal bars.
            let position_dim = if pd.transposed() { Dim::Y } else { Dim::X };
            let dim_role = if dim == position_dim { Dim::X } else { Dim::Y };
            match dim_role {
                Dim::X => {
                    let half = pd.bar.map_or(0.5, |b| b.spacing / 2.0);
                    range = Some((lo - half, hi + half));
                    let base = drawn.as_ref().unwrap_or(axis);
                    drawn = Some(Axis { auto_range: sheaf_core::graph::AutoRange::Tight, ..base.clone() });
                }
                Dim::Y => {
                    // Bars rise from their base: 0, or the plot's offset in a stacked layer.
                    let (blo, bhi) =
                        bars.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), pd| (a.min(pd.oy), b.max(pd.oy)));
                    // A base at or beyond the data limit is the axis limit itself, without a margin.
                    free = (blo > lo, bhi < hi);
                    range = Some((lo.min(blo), hi.max(bhi)));
                    if bars.iter().any(|pd| pd.plot.kind.is_percent_stacked()) {
                        // Piles of 100: the axis ends exactly at 0 and 100.
                        range = Some((0.0, 100.0));
                        free = (false, false);
                    }
                }
            }
        }
        // Tick labels from a column sit on whole-number ticks: the step stays the plain rule (about five ticks)
        // instead of Origin's table, which can skip rows.
        if let (Some((lo, hi)), true) = (range, data::label_column(project, axis, dim, first).is_some())
            && axis.major_step.is_none()
        {
            let base = drawn.as_ref().unwrap_or(axis);
            drawn = Some(Axis { major_step: Some(crate::ticks::nice_step((hi - lo) / 5.0)), ..base.clone() });
        }
        if dim == Dim::X && linear && drawn.is_none() && sources.first().is_some_and(|pd| pd.plot.kind.is_area()) {
            // Area plots reach the frame at both ends: Origin gives them no margin in X (measured).
            drawn = Some(Axis { auto_range: sheaf_core::graph::AutoRange::Tight, ..axis.clone() });
        }
        if dim == Dim::Y
            && linear
            && let Some((lo, hi)) = range
            && lo > 0.0
            && sources.iter().any(|pd| pd.plot.kind.is_area() && pd.plot.y_base.is_none())
        {
            // The fill stands on Y = 0: the axis starts there exactly (measured).
            range = Some((0.0, hi));
            free.0 = false;
        }
        if dim == Dim::Y
            && linear
            && let Some((lo, hi)) = range
            && axis.min.is_none()
            && axis.max.is_none()
            && axis.major_step.is_none()
            && sources.iter().any(|pd| pd.spokes.is_some())
        {
            // A radar chart's centre is the axis' lower limit, which can be anything (graphing spec 14.19). The
            // step is half of the usual table's, and both limits are rounded outwards to whole steps after
            // the margin (measured: 2.1..5.2 gave 1.5..5.5 in steps of 0.5, 0..4 gave -1..5 in steps of 1).
            let span = if hi > lo {
                hi - lo
            } else if hi != 0.0 {
                hi.abs() * 0.2
            } else {
                1.0
            };
            let step = sheaf_core::autorange::linear_step(span) / 2.0;
            let margin = axis.rescale_margin_pct / 100.0 * span;
            let from = ((lo - margin) / step + 1e-9).floor() * step;
            let to = ((hi.max(lo + span) + margin) / step - 1e-9).ceil() * step;
            drawn = Some(Axis { min: Some(from), max: Some(to), major_step: Some(step), ..axis.clone() });
        }
        if dim == Dim::Y && linear && sources.iter().any(|pd| pd.plot.kind == PlotKind::Stacked100Area) {
            // Piles of 100: the axis ends exactly at 0 and 100.
            range = Some((0.0, 100.0));
            free = (false, false);
        }
        resolve_axis_edges(drawn.as_ref().unwrap_or(axis), range, free)
    };
    if let Some(column) = data::label_column(project, axis, dim, first) {
        crate::ticks::label_from_cells(&mut resolved, &data::cell_texts(column));
    }
    resolved
}

fn group_geometry(
    project: &Project,
    g: &Graph,
    members: &[usize],
    pds: &[LayerData],
    m: &dyn TextMeasurer,
) -> Vec<(usize, LayerGeometry)> {
    let root = members[0];
    let rl = &g.layers[root];
    // Does layer `i` use the root's scale on this axis?
    let shares = |i: usize, dim: Dim| {
        i == root || g.valid_link(i).is_some_and(|l| (if dim == Dim::X { l.x } else { l.y }) == ScaleLink::Straight)
    };
    let shared = |dim: Dim, axis: &Axis| {
        let sources: Vec<&data::PlotData> =
            members.iter().filter(|&&i| shares(i, dim)).flat_map(|&i| pds[i].plots.iter()).collect();
        let fields: Vec<&crate::field::FieldGrid> =
            members.iter().filter(|&&i| shares(i, dim)).filter_map(|&i| pds[i].field.as_ref()).collect();
        resolve_dim(project, axis, dim, &sources, (rl, pds[root].bracket_step.get()), &fields)
    };
    let own = |i: usize, dim: Dim, axis: &Axis| {
        let sources: Vec<&data::PlotData> = pds[i].plots.iter().collect();
        let fields: Vec<&crate::field::FieldGrid> = pds[i].field.iter().collect();
        resolve_dim(project, axis, dim, &sources, (&g.layers[i], pds[i].bracket_step.get()), &fields)
    };
    let spoke_axis = |i: usize, axis: &Axis| match g.layers[i].coords.polar() {
        Some(p) if p.radar => {
            polar::radar_axis(pds[i].plots.first().and_then(|pd| pd.spokes.as_deref()).unwrap_or(&[]))
        }
        Some(p) => polar::angle_axis(axis, p.angle_unit),
        None => polar::angle_axis(axis, sheaf_core::graph::AngleUnit::Degrees),
    };
    // The scale of the three axes of a ternary layer.
    let ternary_scale = |i: usize| {
        g.layers[i].coords.ternary().map(|t| {
            let percent = ternary::is_percent(project, &g.layers[i], t);
            (ternary::scale_axis(&g.layers[i].x_axis, percent), ternary::scale_axis(&g.layers[i].y_axis, percent))
        })
    };
    // The radius axis of a polar graph cannot start below 0 (Origin refuses a negative lower limit).
    let polar_floor = |i: usize, y: ResolvedAxis| {
        let l = &g.layers[i];
        if y.min >= 0.0 || l.y_axis.min.is_some() || !l.coords.polar().is_some_and(|p| !p.radar) {
            return y;
        }
        // The rest of the range stays as the data made it (measured: -1 to 2 gave 0 to 2.25, steps of 0.5).
        own(i, Dim::Y, &Axis { min: Some(0.0), max: Some(y.max), major_step: Some(y.step), ..l.y_axis.clone() })
    };
    let (root_x, root_y) = if rl.coords.polar().is_some() {
        (spoke_axis(root, &rl.x_axis), polar_floor(root, shared(Dim::Y, &rl.y_axis)))
    } else if let Some((x, y)) = ternary_scale(root) {
        (x, y)
    } else {
        (shared(Dim::X, &rl.x_axis), shared(Dim::Y, &rl.y_axis))
    };
    let axes: Vec<(ResolvedAxis, ResolvedAxis)> = members
        .iter()
        .map(|&i| {
            let l = &g.layers[i];
            if let Some(xy) = ternary_scale(i) {
                return xy;
            }
            let x = if l.coords.polar().is_some() {
                spoke_axis(i, &l.x_axis)
            } else if shares(i, Dim::X) {
                root_x.clone()
            } else {
                own(i, Dim::X, &l.x_axis)
            };
            let y = if shares(i, Dim::Y) { root_y.clone() } else { own(i, Dim::Y, &l.y_axis) };
            // A zoom layer (T112) shows the extent of the root's region rectangle.
            if i != root
                && let Some(r) = l.zoom.and_then(|k| rl.shapes.get(k))
            {
                let span = |a: &ResolvedAxis, u1: f64, u2: f64| {
                    let (v1, v2) = (a.from_unit(u1), a.from_unit(u2));
                    (v1.min(v2), v1.max(v2))
                };
                let (x0, x1) = span(&root_x, r.x1, r.x2);
                // Frame fractions run down from the top; axis units run up from the bottom.
                let (y0, y1) = span(&root_y, 1.0 - r.y1, 1.0 - r.y2);
                if x1 > x0 && y1 > y0 && [x0, x1, y0, y1].iter().all(|v| v.is_finite()) {
                    let fixed = |a: &Axis, lo: f64, hi: f64| Axis { min: Some(lo), max: Some(hi), ..a.clone() };
                    return (own(i, Dim::X, &fixed(&l.x_axis, x0, x1)), own(i, Dim::Y, &fixed(&l.y_axis, y0, y1)));
                }
            }
            (x, y)
        })
        .collect();

    let frame_of = |root_frame: Rect, i: usize| {
        if i == root { root_frame } else { data::linked_frame(&g.layers[i].frame, root_frame) }
    };
    let mut nominal = data::page_frame(&rl.frame, &g.page);
    // Origin's "Link Axis Length to Scale with X:Y Ratio": the frame keeps its left, top and width and its
    // height follows the axis ranges, even past the page (graphing 18.7a). Not with axis breaks (Origin
    // hides the setting then) nor in polar and ternary layers.
    let cartesian = rl.coords.polar().is_none() && rl.coords.ternary().is_none();
    let linked_height = rl
        .xy_ratio
        .filter(|_| cartesian && root_x.gaps.is_empty() && root_y.gaps.is_empty())
        .and_then(|r| sheaf_core::graph::xy_ratio_height(nominal.w, root_x.scale_span(), root_y.scale_span(), r));
    if let Some(h) = linked_height {
        nominal.h = h;
    }
    let mut edges = LabelEdges::NONE;
    // The rectangle that holds the frames of every member: the root's frame, or the whole stack
    // when the members are panels of their own.
    let mut bounds = nominal;
    for (&i, (x, y)) in members.iter().zip(&axes) {
        let f = frame_of(nominal, i);
        bounds = cover(bounds, f);
        if let Some(t) = g.layers[i].coords.ternary() {
            // The triangle, its tick labels and titles.
            let percent = ternary::is_percent(project, &g.layers[i], t);
            let geom = LayerGeometry {
                frame: f,
                clip: f,
                x: x.clone(),
                y: y.clone(),
                polar: None,
                ternary: Some(ternary::TernaryMap::new(f, percent)),
                connect_across_breaks: false,
                connect_across_missing: false,
            };
            let (l, t, r, b) = ternary::draw_axes(&mut Vec::new(), project, &g.layers[i], &pds[i].plots, &geom, m);
            edges.left = edges.left.min(l);
            edges.top = edges.top.min(t);
            edges.right = edges.right.max(r);
            edges.bottom = edges.bottom.max(b);
            continue;
        }
        if pds[i].plots.iter().any(|pd| pd.plot.kind.is_parallel()) {
            // The axes of a parallel plot are its own: its labels, titles and colour scale or legend.
            crate::parallel::label_edges(&mut edges, &g.layers[i], &pds[i].plots, f, m);
            continue;
        }
        label_edges(&mut edges, &g.layers[i], f, x, y, m, stack::waterfall(g, i).is_some(), &pds[i].plots);
    }
    // A linked X:Y ratio is not given up to keep the labels on the page.
    let fitted = if linked_height.is_some() { bounds } else { fit_on_page(bounds, &edges, &g.page) };
    let f = if bounds == nominal { fitted } else { scale_into(nominal, bounds, fitted) };
    members
        .iter()
        .zip(axes)
        .map(|(&i, (x, y))| {
            let clip = frame_of(f, i);
            let frame = match stack::waterfall(g, i) {
                Some((sx, sy)) => stack::front_frame(clip, pds[i].members, sx, sy),
                None => clip,
            };
            let polar = g.layers[i].coords.polar().map(|p| polar::PolarMap::new(frame, p));
            let ternary = g.layers[i]
                .coords
                .ternary()
                .map(|t| ternary::TernaryMap::new(frame, ternary::is_percent(project, &g.layers[i], t)));
            // A symbol on the outer polygon of a radar or on a side of a triangle sticks out of the
            // frame; the clip leaves room for it.
            let clip = match g.layers[i].coords.polar() {
                Some(p) if p.radar => {
                    let pad = pds[i]
                        .plots
                        .iter()
                        .map(|pd| pd.plot.symbol.size_pt + pd.plot.symbol.edge_width_pt)
                        .fold(0.0, f64::max);
                    Rect { x: clip.x - pad, y: clip.y - pad, w: clip.w + 2.0 * pad, h: clip.h + 2.0 * pad }
                }
                _ if ternary.is_some() => {
                    let e = PAGE_MARGIN * 3.0;
                    Rect { x: clip.x - e, y: clip.y - e, w: clip.w + 2.0 * e, h: clip.h + 2.0 * e }
                }
                _ => clip,
            };
            let connect_across_breaks = g.page.style.connect_across_breaks;
            let connect_across_missing = g.page.style.connect_across_missing;
            (i, LayerGeometry { frame, clip, x, y, polar, ternary, connect_across_breaks, connect_across_missing })
        })
        .collect()
}

/// The outermost extent of the tick labels and titles of a group, in page points.
#[derive(Copy, Clone, Debug)]
pub(crate) struct LabelEdges {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl LabelEdges {
    const NONE: LabelEdges =
        LabelEdges { left: f64::INFINITY, right: f64::NEG_INFINITY, top: f64::INFINITY, bottom: f64::NEG_INFINITY };
}

/// Widens `e` by the tick labels and titles of one layer laid out at frame `f`.
///
/// `waterfall` is true for a layer drawn as a waterfall, which draws no ticks on the opposite edges, so
/// an opposite rug starts at the frame there (the same view of the axes as `layer_scene` draws with).
fn label_edges(
    e: &mut LabelEdges,
    layer: &Layer,
    f: Rect,
    x: &ResolvedAxis,
    y: &ResolvedAxis,
    m: &dyn TextMeasurer,
    waterfall: bool,
    pds: &[PlotData],
) {
    let mut scratch = Vec::new();
    let (xa, ya) = (&layer.x_axis, &layer.y_axis);
    if let Some(p) = layer.coords.polar() {
        // The circle and the angle labels round it.
        let geom = LayerGeometry {
            frame: f,
            clip: f,
            x: x.clone(),
            y: y.clone(),
            polar: Some(polar::PolarMap::new(f, p)),
            ternary: None,
            connect_across_breaks: false,
            connect_across_missing: false,
        };
        let paint = AxisPaint::new(xa, None);
        let (l, t, r, b) = polar::draw_angle_axis(&mut scratch, xa, &paint, &geom, m);
        e.left = e.left.min(l);
        e.top = e.top.min(t);
        e.right = e.right.max(r);
        e.bottom = e.bottom.max(b);
        return;
    }
    let x_size = xa.title_font.size_pt;
    match xa.side {
        AxisSide::Normal => {
            let below = draw_x_labels(&mut scratch, xa, x, f, &xa.tick_font, m);
            let edge = if xa.show_title { below + 0.6 * x_size + (ASCENT + DESCENT) * x_size } else { below };
            e.bottom = e.bottom.max(edge);
        }
        AxisSide::Opposite => {
            let above = draw_x_labels(&mut scratch, xa, x, f, &xa.tick_font, m);
            let edge = if xa.show_title { above - 0.6 * x_size - (ASCENT + DESCENT) * x_size } else { above };
            e.top = e.top.min(edge);
        }
        AxisSide::Hidden => {}
    }
    // The band of significance brackets over the frame (or over the labels and title of a top X axis).
    if layer.display.show_data
        && let Some(h) = crate::brackets::band_height_pt(layer)
    {
        e.top = e.top.min(band_base(layer, x, f, m) - h);
    }
    let across = crate::rug::opposite_reach(xa, f, !waterfall);
    if across > 0.0 {
        match xa.side {
            AxisSide::Normal => e.top = e.top.min(f.y - across),
            _ => e.bottom = e.bottom.max(f.y + f.h + across),
        }
    }
    // The colour scale of a hexbin plot beside the frame.
    if layer.display.show_data && layer.plots.iter().any(|p| p.visible && p.kind.is_hexbin()) {
        e.right = e.right.max(f.x + f.w + crate::hexbin::scale_reserve(ya.tick_font.size_pt));
    }
    // The colour scale of a sunburst plot beside the frame.
    if layer.display.show_data && layer.plots.iter().any(|p| p.visible && p.kind == PlotKind::Sunburst) {
        e.right = e.right.max(f.x + f.w + crate::hexbin::scale_reserve(ya.tick_font.size_pt));
    }
    // The colour scales and bubble scales of the layer, side by side beside the frame.
    if layer.display.show_data && layer.coords.polar().is_none() && layer.coords.ternary().is_none() {
        let room = crate::layer_scale::reserve(layer, pds);
        if room > 0.0 {
            e.right = e.right.max(f.x + f.w + room);
        }
    }
    let y_size = ya.title_font.size_pt;
    match ya.side {
        AxisSide::Normal => {
            let left = draw_y_labels(&mut scratch, ya, y, f, &ya.tick_font, m);
            let edge = if ya.show_title { left - 0.6 * y_size - (ASCENT + DESCENT) * y_size } else { left };
            e.left = e.left.min(edge);
        }
        AxisSide::Opposite => {
            let right = draw_y_labels(&mut scratch, ya, y, f, &ya.tick_font, m);
            let edge = if ya.show_title { right + 0.6 * y_size + (ASCENT + DESCENT) * y_size } else { right };
            e.right = e.right.max(edge);
        }
        AxisSide::Hidden => {}
    }
    let across = crate::rug::opposite_reach(ya, f, !waterfall);
    if across > 0.0 {
        match ya.side {
            AxisSide::Normal => e.right = e.right.max(f.x + f.w + across),
            _ => e.left = e.left.min(f.x - across),
        }
    }
}

/// The page Y a band of significance brackets starts at: the top of the frame, or, with the X axis on the top, the
/// outer edge of its tick labels and title.
fn band_base(layer: &Layer, x: &ResolvedAxis, f: Rect, m: &dyn TextMeasurer) -> f64 {
    let xa = &layer.x_axis;
    if xa.side != AxisSide::Opposite {
        return f.y;
    }
    let above = draw_x_labels(&mut Vec::new(), xa, x, f, &xa.tick_font, m);
    let size = xa.title_font.size_pt;
    let edge = if xa.show_title { above - 0.6 * size - (ASCENT + DESCENT) * size } else { above };
    edge.min(f.y)
}

/// The smallest rectangle that holds `b` and `f`; `b` itself when `f` lies inside it (so that a
/// group whose members all share the root's frame keeps that frame exactly).
fn cover(b: Rect, f: Rect) -> Rect {
    let finite = [f.x, f.y, f.w, f.h].iter().all(|v| v.is_finite());
    if !finite || (f.x >= b.x && f.y >= b.y && f.x + f.w <= b.x + b.w && f.y + f.h <= b.y + b.h) {
        return b;
    }
    let (x0, y0) = (b.x.min(f.x), b.y.min(f.y));
    let (x1, y1) = ((b.x + b.w).max(f.x + f.w), (b.y + b.h).max(f.y + f.h));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// `r`, a rectangle inside `from`, moved and scaled the way `from` became `to`.
fn scale_into(r: Rect, from: Rect, to: Rect) -> Rect {
    let sx = if from.w > 0.0 { to.w / from.w } else { 1.0 };
    let sy = if from.h > 0.0 { to.h / from.h } else { 1.0 };
    Rect { x: to.x + (r.x - from.x) * sx, y: to.y + (r.y - from.y) * sy, w: r.w * sx, h: r.h * sy }
}

/// Moves the sides of `nominal` inwards until the label edges are `PAGE_MARGIN` inside the page.
/// Each move shifts one side only (the opposite side stays) and never takes more than half of
/// the frame, whatever the labels need.
fn fit_on_page(nominal: Rect, e: &LabelEdges, page: &Page) -> Rect {
    let shift = |need: f64, size: f64| need.clamp(0.0, (0.5 * size).max(0.0));
    let mut f = nominal;
    let left = shift(PAGE_MARGIN - e.left, f.w);
    f.x += left;
    f.w -= left;
    let right = shift(e.right - (page.width_pt - PAGE_MARGIN), f.w);
    f.w -= right;
    let bottom = shift(e.bottom - (page.height_pt - PAGE_MARGIN), f.h);
    f.h -= bottom;
    let top = shift(PAGE_MARGIN - e.top, f.h);
    f.y += top;
    f.h -= top;
    f
}

/// Where a layer draws the title of one of its axes.
#[derive(Copy, Clone, Debug, PartialEq)]
enum TitlePlace {
    /// Centred on its own frame.
    Own,
    /// Not drawn: another panel of a run of touching panels with the same title draws it.
    Shared,
    /// Drawn once for a run of touching panels with the same title: centred between `from` and `to`
    /// (page points along the axis), beyond `edge`, the outermost tick label edge of the run.
    Span { from: f64, to: f64, edge: f64 },
}

/// How each layer draws its X (or Y) title. Panels of one linked group that touch along the axis
/// (a column of panels without a gap for Y titles, a row for X titles) and have the same title text
/// show it once, centred on the run, so that a stack of thin panels can carry one title such as
/// "Intensity (a.u.)" (graphing spec 15.5). Every other layer draws its own title.
fn title_places(
    project: &Project,
    g: &Graph,
    geoms: &[LayerGeometry],
    pds: &[LayerData],
    dim: Dim,
    m: &dyn TextMeasurer,
) -> Vec<TitlePlace> {
    let mut out = vec![TitlePlace::Own; g.layers.len()];
    let panels = g.panels();
    let axis = |i: usize| if dim == Dim::X { &g.layers[i].x_axis } else { &g.layers[i].y_axis };
    let title = |i: usize| {
        let a = axis(i);
        resolved_title(project, a, &pds[i].plots, dim == Dim::Y)
    };
    // Along the axis: start and length of a frame; across it: start and length.
    let along = |f: Rect| if dim == Dim::X { (f.x, f.w) } else { (f.y, f.h) };
    let across = |f: Rect| if dim == Dim::X { (f.y, f.h) } else { (f.x, f.w) };
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0);
    let touch = |a: usize, b: usize| {
        let (fa, fb) = (geoms[a].frame, geoms[b].frame);
        let ((sa, la), (sb, _)) = (along(fa), along(fb));
        let ((ca, wa), (cb, wb)) = (across(fa), across(fb));
        close(ca, cb) && close(wa, wb) && close(sa + la, sb)
    };
    for root in (0..g.layers.len()).filter(|&i| g.root_of(i) == i) {
        let mut line: Vec<usize> = g
            .group(root)
            .into_iter()
            .filter(|i| panels.contains(i) && axis(*i).side == AxisSide::Normal && axis(*i).show_title)
            .collect();
        if line.len() < 2 {
            continue;
        }
        line.sort_by(|&a, &b| along(geoms[a].frame).0.total_cmp(&along(geoms[b].frame).0));
        let mut start = 0;
        while start < line.len() {
            let text = title(line[start]);
            let mut end = start + 1;
            while end < line.len() && touch(line[end - 1], line[end]) && title(line[end]) == text {
                end += 1;
            }
            let run = &line[start..end];
            if run.len() > 1 {
                let mut scratch = Vec::new();
                let edges = run.iter().map(|&i| {
                    let (a, geom) = (axis(i), &geoms[i]);
                    match dim {
                        Dim::X => draw_x_labels(&mut scratch, a, &geom.x, geom.frame, &a.tick_font, m),
                        Dim::Y => draw_y_labels(&mut scratch, a, &geom.y, geom.frame, &a.tick_font, m),
                    }
                });
                let edge = match dim {
                    Dim::X => edges.fold(f64::NEG_INFINITY, f64::max),
                    Dim::Y => edges.fold(f64::INFINITY, f64::min),
                };
                let from = along(geoms[run[0]].frame).0;
                let (last_start, last_len) = along(geoms[run[run.len() - 1]].frame);
                let leader = *run.iter().min().expect("a run is not empty");
                for &i in run {
                    out[i] = if i == leader {
                        TitlePlace::Span { from, to: last_start + last_len, edge }
                    } else {
                        TitlePlace::Shared
                    };
                }
            }
            start = end;
        }
    }
    out
}

fn layer_scene(
    project: &Project,
    g: &Graph,
    index: usize,
    geom: &LayerGeometry,
    layer_data: &LayerData,
    (x_title, y_title): (TitlePlace, TitlePlace),
    first_label_line: usize,
    bracket_step: f64,
    m: &dyn TextMeasurer,
) -> LayerScene {
    let pds: &[PlotData] = &layer_data.plots;
    let layer = &g.layers[index];
    let mut scene = LayerScene {
        clip: geom.clip,
        grid: Vec::new(),
        plots: Vec::new(),
        overlay: Vec::new(),
        underlay_len: 0,
        data_on_top: false,
        objects: Vec::new(),
    };
    if layer.display.hidden {
        // Keep scene indices and the resolved geometry of linked layers stable, but expose no
        // drawing or pickable frame. A zero-sized clip is valid in each export backend.
        scene.clip = Rect { x: geom.clip.x, y: geom.clip.y, w: 0.0, h: 0.0 };
        return scene;
    }
    // The layer's Background colour fills the frame behind the grid and the data.
    fill::draw_background(&mut scene.grid, layer.background, &layer.background_style, geom.frame);
    let background_items = scene.grid.len();
    let plot_index = |pd: &PlotData| layer.plots.iter().position(|p| std::ptr::eq(p, pd.plot)).unwrap_or(0);
    let waterfall = stack::waterfall(g, index);
    // A waterfall draws no grid and no lines or ticks on the opposite edges: they would cut
    // through the curves behind the front frame.
    let (x_axis, y_axis);
    let (xa, ya) = if waterfall.is_some() {
        x_axis = Axis { show_opposite: false, opposite_ticks: false, ..layer.x_axis.clone() };
        y_axis = Axis { show_opposite: false, opposite_ticks: false, ..layer.y_axis.clone() };
        (&x_axis, &y_axis)
    } else {
        if pds.iter().any(|pd| pd.plot.kind.is_parallel()) {
            // The vertical axes are drawn with the plots; the layer's own grid has no axes to follow.
        } else if geom.polar.is_some() {
            polar::draw_grid(&mut scene.grid, layer, geom);
        } else if geom.ternary.is_some() {
            ternary::draw_grid(&mut scene.grid, layer, geom);
        } else if layer.coords.smith().is_some() {
            crate::smith::draw_grid(&mut scene.grid, geom);
        } else if let Some(c) = layer.coords.space3d() {
            crate::space3d::draw_box(&mut scene.grid, &mut scene.overlay, project, layer, geom, c, m);
        } else {
            draw_grid(&mut scene.grid, layer, geom);
        }
        (&layer.x_axis, &layer.y_axis)
    };
    // A field that has labels gives its axes the titles of the matrix when they have none (T192).
    let (field_x, field_y);
    let (xa, ya) = match &layer_data.field {
        Some(f) => {
            let titled = |a: &Axis, label: &str| {
                if a.title.is_none() && !label.is_empty() {
                    Axis { title: Some(label.to_string()), ..a.clone() }
                } else {
                    a.clone()
                }
            };
            field_x = titled(xa, &f.x_label);
            field_y = titled(ya, &f.y_label);
            (&field_x, &field_y)
        }
        None => (xa, ya),
    };
    if let (Some(grid), Some(field)) = (&layer_data.field, &layer.field)
        && layer.display.show_data
    {
        let env = crate::field::Env { page: &g.page, measurer: m };
        crate::field::draw(&mut scene.plots, &mut scene.overlay, field, grid, geom, &env);
    }
    // Display tab: Show Elements Data off draws no plot.
    let drawn: &[PlotData] = if layer.display.show_data { pds } else { &[] };
    crate::hexbin::share_max(drawn, geom);
    match waterfall {
        None => {
            // Ridgelines are drawn back to front: the highest baseline first, so that nearer ridges
            // overlap the farther ones.
            let mut order: Vec<usize> = (0..drawn.len()).collect();
            if drawn.iter().any(|pd| pd.ridge.is_some()) {
                order.sort_by(|&a, &b| {
                    let pos = |i: usize| drawn[i].ridge.as_ref().map_or(0.0, |r| r.pos);
                    pos(b).total_cmp(&pos(a))
                });
            }
            for k in order {
                let pd = &drawn[k];
                // Drop To = Next Data Plot: the next plot, the first after the last; with two
                // plots only the first draws the segments (the last would repeat them).
                let next = match drawn.len() {
                    0 | 1 => None,
                    2 if k == 1 => None,
                    n => Some(&drawn[(k + 1) % n]),
                };
                let start = scene.plots.len();
                draw_plot(&mut scene.plots, pd, geom, next, m);
                let kind = ObjectKind::Plot(plot_index(pd));
                hit::record(&mut scene.objects, &scene.plots, start, kind, index, ItemList::Plots, &[], m);
            }
        }
        Some((sx, sy)) => {
            // Back to front, so that the first curve is on top; member k moves by k shifts.
            for pd in drawn.iter().rev() {
                let k = pd.member.unwrap_or(0) as f64;
                let mut moved = geom.clone();
                moved.frame.x += k * sx * geom.frame.w;
                moved.frame.y -= k * sy * geom.frame.h;
                let start = scene.plots.len();
                draw_plot(&mut scene.plots, pd, &moved, None, m);
                let kind = ObjectKind::Plot(plot_index(pd));
                hit::record(&mut scene.objects, &scene.plots, start, kind, index, ItemList::Plots, &[], m);
            }
        }
    }
    if waterfall.is_none() {
        crate::trend::draw(&mut scene.plots, layer, drawn, geom, first_label_line, m);
    }
    // Grid on Top of Data: the grid lines (not the background) are drawn after the plots.
    if layer.display.grid_on_top {
        let keep = background_items.min(scene.grid.len());
        let lines = scene.grid.split_off(keep);
        scene.plots.extend(lines);
    }
    // Clip Data to Frame off: the data may run over the whole page.
    if !layer.display.clip_data {
        scene.clip = Rect { x: 0.0, y: 0.0, w: g.page.width_pt, h: g.page.height_pt };
    }
    let objects = &mut scene.objects;
    let out = &mut scene.overlay;
    let f = geom.frame;
    fill::draw_border(out, &layer.background_style, f);
    // Display tab: X Axes or Y Axes off draws nothing of that axis (line, ticks, labels, title).
    let (hidden_x, hidden_y);
    let hide = |a: &Axis| Axis { side: AxisSide::Hidden, show_opposite: false, opposite_ticks: false, ..a.clone() };
    let xa = if layer.display.show_x_axes {
        xa
    } else {
        hidden_x = hide(xa);
        &hidden_x
    };
    let ya = if layer.display.show_y_axes {
        ya
    } else {
        hidden_y = hide(ya);
        &hidden_y
    };
    let first_color = plot_color(pds);
    let (xp, yp) = (AxisPaint::new(xa, first_color), AxisPaint::new(ya, first_color));
    let rec = |objects: &mut Vec<SceneObject>, out: &Vec<Item>, start: usize, kind: ObjectKind, extra: &[Rect]| {
        hit::record(objects, out, start, kind, index, ItemList::Overlay, extra, m)
    };
    if pds.iter().any(|pd| pd.plot.kind.is_parallel()) {
        // Parallel Plot and Parallel Index: the layer's X and Y axes are not drawn; one vertical axis per column.
        let start = out.len();
        if layer.display.show_data && layer.display.show_y_axes {
            crate::parallel::draw_axes(out, layer, pds, geom, m);
        }
        rec(objects, out, start, ObjectKind::YAxis, &[]);
        if layer.display.show_data {
            // The colour scale and the legend of the groups are the layer's legend: pickable and movable as one.
            let start = out.len();
            crate::parallel::draw_scale(out, layer, pds, geom, m);
            crate::parallel::draw_legend(out, layer, pds, f, m);
            rec(objects, out, start, ObjectKind::Legend, &[]);
        }
        if layer.display.data_on_top {
            scene.underlay_len = out.len();
            scene.data_on_top = true;
        }
        draw_legend_and_labels(out, objects, project, g, index, f, pds, m);
        return scene;
    }
    if geom.ternary.is_some() {
        let start = out.len();
        ternary::draw_axes(out, project, layer, pds, geom, m);
        rec(objects, out, start, ObjectKind::XAxis, &[]);
        if layer.display.data_on_top {
            scene.underlay_len = out.len();
            scene.data_on_top = true;
        }
        draw_legend_and_labels(out, objects, project, g, index, f, pds, m);
        return scene;
    }
    if geom.polar.is_some() {
        let start = out.len();
        polar::draw_angle_axis(out, xa, &xp, geom, m);
        rec(objects, out, start, ObjectKind::XAxis, &[]);
        let start = out.len();
        polar::draw_radius_axis(out, ya, &yp, geom, m);
        rec(objects, out, start, ObjectKind::YAxis, &[]);
        if layer.display.data_on_top {
            scene.underlay_len = out.len();
            scene.data_on_top = true;
        }
        draw_legend_and_labels(out, objects, project, g, index, f, pds, m);
        return scene;
    }
    // Axis Position "At Position =" (T243): the line where the other axis has that value, drawn as an
    // inward offset from the axis' edge (line, ticks, labels and title move together).
    let (at_x, at_y);
    let xa = match crate::axis_offset::at_position_pct(xa, &geom.y) {
        Some(pct) => {
            at_x = Axis { offset_pct: pct, ..xa.clone() };
            &at_x
        }
        None => xa,
    };
    let ya = match crate::axis_offset::at_position_pct(ya, &geom.x) {
        Some(pct) => {
            at_y = Axis { offset_pct: pct, ..ya.clone() };
            &at_y
        }
        None => ya,
    };
    // The edge each axis line is drawn on, as a box of no height or width.
    let x_line = Rect { y: crate::axis_offset::x_line_y(xa, f), h: 0.0, ..f };
    let y_line = Rect { x: crate::axis_offset::y_line_x(ya, f), w: 0.0, ..f };
    let spans = |r: &ResolvedAxis| r.gaps.iter().map(|g| g.span(r.reversed)).collect::<Vec<_>>();
    let gaps = (spans(&geom.x), spans(&geom.y));
    draw_axis_lines(out, xa, ya, f, xp.line, yp.line, &gaps);
    draw_additional_lines(out, xa, ya, geom, xp.line, yp.line);
    let start = out.len();
    draw_ticks(out, xa, &geom.x, f, true, xp.line);
    draw_axis_arrows(out, xa, f, true, xp.line);
    if xa.side != AxisSide::Hidden {
        rec(objects, out, start, ObjectKind::XAxis, &[x_line]);
    }
    let start = out.len();
    draw_ticks(out, ya, &geom.y, f, false, yp.line);
    draw_axis_arrows(out, ya, f, false, yp.line);
    if ya.side != AxisSide::Hidden {
        rec(objects, out, start, ObjectKind::YAxis, &[y_line]);
    }
    crate::rug::draw(out, layer, xa, ya, geom, pds, xp.line, yp.line);
    let start = out.len();
    let x_edge = (xa.side != AxisSide::Hidden).then(|| draw_x_labels(out, xa, &geom.x, f, &xp.labels, m));
    rec(objects, out, start, ObjectKind::XAxis, &[]);
    let start = out.len();
    let y_edge = (ya.side != AxisSide::Hidden).then(|| draw_y_labels(out, ya, &geom.y, f, &yp.labels, m));
    rec(objects, out, start, ObjectKind::YAxis, &[]);
    let plot_color = |slot| layer.plots.get(slot).map(crate::rug::plot_color);
    let place_title = |out: &mut Vec<Item>, title: &str, font: &Font, pos, angle| {
        let segments = crate::markup::plot_segments(title, &|name| m.expand_inline_family(name), &plot_color);
        text::place_segments(out, m, &segments, font, font.size_pt, pos, Anchor::Middle, angle);
    };
    let start = out.len();
    if x_edge.is_some() && xa.offset_pct < 0.0 && xa.offset_pct.is_finite() && x_title == TitlePlace::Own {
        draw_x_title_inward(out, project, xa, f, pds, crate::axis_offset::x_line_y(xa, f), &xp.title, &place_title);
    } else if let Some(edge) = x_edge {
        match x_title {
            TitlePlace::Own => draw_x_title(out, project, xa, f, pds, edge, &xp.title, &place_title),
            TitlePlace::Shared => {}
            TitlePlace::Span { from, to, edge } => {
                draw_x_title(out, project, xa, Rect { x: from, w: to - from, ..f }, pds, edge, &xp.title, &place_title)
            }
        }
    }
    rec(objects, out, start, ObjectKind::XTitle, &[]);
    let start = out.len();
    if y_edge.is_some() && ya.offset_pct < 0.0 && ya.offset_pct.is_finite() && y_title == TitlePlace::Own {
        draw_y_title_inward(out, project, ya, f, pds, crate::axis_offset::y_line_x(ya, f), &yp.title, &place_title);
    } else if let Some(edge) = y_edge {
        match y_title {
            TitlePlace::Own => draw_y_title(out, project, ya, f, pds, edge, &yp.title, &place_title),
            TitlePlace::Shared => {}
            TitlePlace::Span { from, to, edge } => {
                draw_y_title(out, project, ya, Rect { y: from, h: to - from, ..f }, pds, edge, &yp.title, &place_title)
            }
        }
    }
    rec(objects, out, start, ObjectKind::YTitle, &[]);
    if layer.display.data_on_top {
        scene.underlay_len = out.len();
        scene.data_on_top = true;
    }
    if layer.display.show_data {
        crate::hexbin::draw_scale(out, layer, pds, geom, m);
        crate::sunburst::draw_scale(out, layer, pds, geom, m);
        stack::draw_scale(out, layer, pds, geom, m);
    }
    if waterfall.is_none() && layer.display.show_data {
        crate::slope::draw(out, objects, project, layer, index, pds, geom, m);
        crate::brackets::draw(out, layer, pds, geom, (bracket_step, band_base(layer, &geom.x, geom.frame, m)), m);
    }
    if layer.display.show_data {
        crate::layer_scale::draw(out, objects, project, layer, index, pds, geom, m);
    }
    if waterfall.is_none() {
        crate::straight_line::draw(out, objects, project, layer, index, geom, m);
    }
    draw_legend_and_labels(out, objects, project, g, index, f, pds, m);
    scene
}

/// The legend of a layer (when it lists its own plots) and its free text labels.
fn draw_legend_and_labels(
    out: &mut Vec<Item>,
    objects: &mut Vec<SceneObject>,
    project: &Project,
    g: &Graph,
    index: usize,
    f: Rect,
    pds: &[PlotData],
    m: &dyn TextMeasurer,
) {
    let layer = &g.layers[index];
    // View > Show > Labels off (T177): no legend, free labels or drawn objects.
    if !layer.display.show_labels {
        return;
    }
    let label_plot_color = |slot| layer.plots.get(slot).map(crate::rug::plot_color);
    let rec = |objects: &mut Vec<SceneObject>, out: &Vec<Item>, start: usize, kind: ObjectKind, extra: &[Rect]| {
        hit::record(objects, out, start, kind, index, ItemList::Overlay, extra, m)
    };
    // The legend is under the free labels and drawn objects, or over them after Bring to the front (T152).
    let legend = |objects: &mut Vec<SceneObject>, out: &mut Vec<Item>| {
        if layer.legend.scope == LegendScope::Layer {
            let entries: Vec<&PlotData> = pds.iter().collect();
            let start = out.len();
            let ids: Vec<String> = (1..=entries.len()).map(|k| k.to_string()).collect();
            draw_legend(out, project, &layer.legend, f, &entries, &ids, &edge_ticks(g, index), m, &layer.plots);
            rec(objects, out, start, ObjectKind::Legend, &[]);
        }
    };
    if !layer.legend.on_top {
        legend(objects, out);
    }
    for (k, label) in layer.labels.iter().enumerate() {
        let start = out.len();
        if label.table {
            // Graph > New Table (T136): the whole grid is the object, not only its lines and text.
            let r = crate::table::draw_table(out, m, label, f);
            rec(objects, out, start, ObjectKind::Label(k), &[r]);
            continue;
        }
        let size = label.font.size_pt;
        // A label that names plots with \l(n) or %(n), as Origin's text objects may (a pasted copy of
        // the legend, T178): drawn with the legend's line layout and the layer's plots, without the
        // legend's frame or fill, the top of its first row at the anchor's text middle. Turned labels
        // keep the plain text path.
        if label.rotation_deg == 0.0 && !pds.is_empty() && f.h > 0.0 && legend::has_plot_refs(&label.text) {
            let lg = Legend {
                visible: true,
                position: LegendPosition::ManualTopLeft { x: label.x, y: label.y - 0.5 * size / f.h },
                font: label.font.clone(),
                frame: false,
                background: None,
                columns: 1,
                text: Some(label.text.clone()),
                ..layer.legend.clone()
            };
            let entries: Vec<&PlotData> = pds.iter().collect();
            let ids: Vec<String> = (1..=entries.len()).map(|k| k.to_string()).collect();
            draw_legend(out, project, &lg, f, &entries, &ids, &edge_ticks(g, index), m, &layer.plots);
            rec(objects, out, start, ObjectKind::Label(k), &[]);
            continue;
        }
        let anchor = (f.x + label.x * f.w, f.y + label.y * f.h);
        // The anchor is the vertical middle of the text's first character.
        let (sin, cos) = label.rotation_deg.to_radians().sin_cos();
        let dy = (ASCENT - DESCENT) / 2.0 * size;
        let pos = (anchor.0 + dy * sin, anchor.1 + dy * cos);
        text::place_label(out, m, &label.text, &label.font, pos, label.rotation_deg, &label_plot_color);
        rec(objects, out, start, ObjectKind::Label(k), &[]);
    }
    crate::shapes::draw_shapes(out, objects, layer, index, f, m);
    if layer.legend.on_top {
        legend(objects, out);
    }
}

/// Moves object `kind` of model layer `model` to the end of `scene`'s overlay, over everything else of
/// the layer, keeping every object's item range right (T153).
fn move_to_top(scene: &mut LayerScene, kind: ObjectKind, model: usize) {
    let Some(at) = scene.objects.iter().position(|o| o.kind == kind && o.layer == model && o.list == ItemList::Overlay)
    else {
        return;
    };
    let range = scene.objects[at].items.clone();
    if range.is_empty() {
        return;
    }
    let n = range.len();
    let items: Vec<Item> = scene.overlay.drain(range.clone()).collect();
    if range.end <= scene.underlay_len {
        scene.underlay_len -= n;
    }
    let to = scene.overlay.len();
    scene.overlay.extend(items);
    for (i, o) in scene.objects.iter_mut().enumerate() {
        if o.list != ItemList::Overlay {
            continue;
        }
        if i == at {
            o.items = to..to + n;
        } else if o.items.start >= range.end {
            o.items = o.items.start - n..o.items.end - n;
        }
    }
}

/// Moves the legend of model layer `model` into the part of `scene`'s overlay drawn before the data
/// (Push to back of data, T152), keeping every object's item range right.
fn legend_behind_data(scene: &mut LayerScene, model: usize) {
    let Some(at) = scene
        .objects
        .iter()
        .position(|o| o.kind == ObjectKind::Legend && o.layer == model && o.list == ItemList::Overlay)
    else {
        return;
    };
    let range = scene.objects[at].items.clone();
    let to = scene.underlay_len;
    if range.start < to || range.is_empty() {
        return;
    }
    let n = range.len();
    let items: Vec<Item> = scene.overlay.drain(range.clone()).collect();
    scene.overlay.splice(to..to, items);
    scene.underlay_len += n;
    for (i, o) in scene.objects.iter_mut().enumerate() {
        if o.list != ItemList::Overlay {
            continue;
        }
        if i == at {
            o.items = to..to + n;
        } else if o.items.start >= to && o.items.start < range.start {
            o.items = o.items.start + n..o.items.end + n;
        }
    }
}

// ----- axis colors ---------------------------------------------------------------------------

/// The color of the first drawable plot of a layer: its line color, or the symbol (fill) color
/// for scatter and column plots, which have no line of their own to show.
fn plot_color(pds: &[PlotData]) -> Option<Color> {
    pds.first().map(|pd| crate::rug::plot_color(pd.plot))
}

/// The colors an axis is drawn in: its own, or that of the layer's first plot when the axis
/// follows the plot (and the layer has one).
pub(crate) struct AxisPaint {
    /// Axis line and ticks.
    pub line: Color,
    /// The tick label font, in its final color.
    pub labels: Font,
    /// The title font, in its final color.
    pub title: Font,
}

impl AxisPaint {
    pub(crate) fn new(axis: &Axis, plot: Option<Color>) -> AxisPaint {
        let follow = plot.filter(|_| axis.color_follows_plot);
        let title_follow = plot.filter(|_| axis.title_color_follows_plot);
        AxisPaint {
            line: follow.unwrap_or(axis.color),
            labels: Font { color: follow.unwrap_or(axis.tick_font.color), ..axis.tick_font.clone() },
            title: Font { color: title_follow.unwrap_or(axis.title_font.color), ..axis.title_font.clone() },
        }
    }
}

// ----- grid, axis lines, ticks ---------------------------------------------------------------

fn grid_stroke(style: &LineStyle) -> Stroke {
    let mut s = line_stroke(style);
    s.cap = Cap::Butt;
    s
}

fn draw_grid(out: &mut Vec<Item>, layer: &Layer, geom: &LayerGeometry) {
    let f = geom.frame;
    let mut lines = |style: &Option<LineStyle>, ticks: &[f64], axis: &ResolvedAxis, vertical: bool| {
        let Some(style) = style else { return };
        let mut d = String::new();
        for &v in ticks {
            let u = axis.to_unit(v);
            if !u.is_finite() {
                continue;
            }
            if vertical {
                let x = f.x + u * f.w;
                move_to(&mut d, x, f.y);
                line_to(&mut d, x, f.y + f.h);
            } else {
                let y = f.y + (1.0 - u) * f.h;
                move_to(&mut d, f.x, y);
                line_to(&mut d, f.x + f.w, y);
            }
        }
        if !d.is_empty() {
            out.push(Item::Path { d, fill: None, stroke: Some(grid_stroke(style)) });
        }
    };
    // Minor first, then major, so that major lines are on top.
    lines(&layer.x_axis.minor_grid, &geom.x.minor, &geom.x, true);
    lines(&layer.y_axis.minor_grid, &geom.y.minor, &geom.y, false);
    lines(&layer.x_axis.major_grid, &geom.x.major, &geom.x, true);
    lines(&layer.y_axis.major_grid, &geom.y.major, &geom.y, false);
}

/// How far the strokes of an axis break reach to either side of the axis line, in points. Origin 2016's
/// exports showed strokes 35 px across at 2.07 px per point on both axes, about 8 pt from the line
/// counting the stroke width (graphing spec 3.6); whether it follows the tick length is unmeasured.
const BREAK_MARK_PT: f64 = 8.0;

/// Draws the axis lines of a layer: each shown axis on its own edge and, with `show_opposite`,
/// on the edge across. `xc` and `yc` are the axes' line colors.
/// Length and half width in points of the arrowhead of an axis line `width` points thick (T245): 5 and
/// 1.75 times the width, at least 6 and 2 points. Sheaf's own sizes; Origin's are not measured.
pub(crate) fn arrow_size(width: f64) -> (f64, f64) {
    let w = if width.is_finite() { width.max(0.0) } else { 0.0 };
    ((5.0 * w).max(6.0), (1.75 * w).max(2.0))
}

/// Arrow at Begin / Arrow at End (T245): a filled arrowhead whose base sits on the chosen end of the
/// axis line (left or bottom is the beginning, right or top the end, as the page shows them) and whose
/// tip points outward, in the line's colour.
fn draw_axis_arrows(out: &mut Vec<Item>, axis: &Axis, f: Rect, horizontal: bool, color: Color) {
    if axis.side == AxisSide::Hidden || !(axis.arrow_begin || axis.arrow_end) {
        return;
    }
    let (len, half) = arrow_size(axis.line_width_pt);
    let head = |out: &mut Vec<Item>, base: (f64, f64), dir: (f64, f64)| {
        let tip = (base.0 + dir.0 * len, base.1 + dir.1 * len);
        let n = (-dir.1 * half, dir.0 * half);
        let mut d = String::new();
        move_to(&mut d, tip.0, tip.1);
        line_to(&mut d, base.0 + n.0, base.1 + n.1);
        line_to(&mut d, base.0 - n.0, base.1 - n.1);
        d.push('Z');
        out.push(Item::Path { d, fill: Some(color), stroke: None });
    };
    if horizontal {
        let y = crate::axis_offset::x_line_y(axis, f);
        if axis.arrow_begin {
            head(out, (f.x, y), (-1.0, 0.0));
        }
        if axis.arrow_end {
            head(out, (f.x + f.w, y), (1.0, 0.0));
        }
    } else {
        let x = crate::axis_offset::y_line_x(axis, f);
        if axis.arrow_begin {
            head(out, (x, f.y + f.h), (0.0, 1.0));
        }
        if axis.arrow_end {
            head(out, (x, f.y), (0.0, -1.0));
        }
    }
}

fn draw_axis_lines(
    out: &mut Vec<Item>,
    xa: &Axis,
    ya: &Axis,
    f: Rect,
    xc: Color,
    yc: Color,
    gaps: &(Vec<(f64, f64)>, Vec<(f64, f64)>),
) {
    let (l, r, t, b) = (f.x, f.x + f.w, f.y, f.y + f.h);
    let shown = |a: &Axis| a.side != AxisSide::Hidden;
    let same = xa.line_width_pt == ya.line_width_pt && xc == yc;
    let y_main_x = crate::axis_offset::y_line_x(ya, f);
    let displaced = y_main_x != if ya.side == AxisSide::Opposite { r } else { l }
        || crate::axis_offset::x_line_y(xa, f) != if xa.side == AxisSide::Opposite { t } else { b };
    if shown(xa)
        && shown(ya)
        && xa.show_opposite
        && ya.show_opposite
        && same
        && gaps.0.is_empty()
        && gaps.1.is_empty()
        && !displaced
    {
        // All four edges in one stroke, so that the corners join cleanly.
        let mut d = String::new();
        move_to(&mut d, l, t);
        line_to(&mut d, r, t);
        line_to(&mut d, r, b);
        line_to(&mut d, l, b);
        d.push('Z');
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(xc, xa.line_width_pt)) });
        return;
    }
    let line = |out: &mut Vec<Item>,
                (x0, y0): (f64, f64),
                (x1, y1): (f64, f64),
                a: &Axis,
                color: Color,
                gaps: &[(f64, f64)]| {
        let mut d = String::new();
        let stroke = Stroke { color, width: a.line_width_pt, dash: Vec::new(), cap: Cap::Square, join: Join::Miter };
        // The pieces between the gaps, and at each end of a gap a slanted stroke across the axis, as
        // Origin 2016 draws a break (graphing spec 3.6): centred on the end of the piece, as long along
        // the axis as half the gap on either side (the two strokes meet at the middle of the gap) and
        // reaching BREAK_MARK_PT to either side of the line, rising to the right on both axes.
        let at = |u: f64| (x0 + u * (x1 - x0), y0 + u * (y1 - y0));
        let mut cuts: Vec<(f64, f64)> = gaps.to_vec();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut start = 0.0;
        for &(lo, hi) in &cuts {
            let (p, q) = (at(start), at(lo));
            move_to(&mut d, p.0, p.1);
            line_to(&mut d, q.0, q.1);
            start = hi;
        }
        let (p, q) = (at(start), at(1.0));
        move_to(&mut d, p.0, p.1);
        line_to(&mut d, q.0, q.1);
        let horizontal = (y1 - y0).abs() < 1e-9;
        let length = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        for &(lo, hi) in &cuts {
            let half = (hi - lo) * length / 2.0;
            for p in [at(lo), at(hi)] {
                if horizontal {
                    move_to(&mut d, p.0 - half, p.1 + BREAK_MARK_PT);
                    line_to(&mut d, p.0 + half, p.1 - BREAK_MARK_PT);
                } else {
                    move_to(&mut d, p.0 - BREAK_MARK_PT, p.1 + half);
                    line_to(&mut d, p.0 + BREAK_MARK_PT, p.1 - half);
                }
            }
        }
        out.push(Item::Path { d, fill: None, stroke: Some(stroke) });
    };
    // The line of a Y axis runs from the top down, so its gaps are measured from the top.
    let gap_y: Vec<(f64, f64)> = gaps.1.iter().map(|&(lo, hi)| (1.0 - hi, 1.0 - lo)).collect();
    let gap_x = &gaps.0;
    // The edge each axis is drawn on, and the edge across from it.
    let x_other = if xa.side == AxisSide::Opposite { b } else { t };
    let x_main = crate::axis_offset::x_line_y(xa, f);
    let y_other = if ya.side == AxisSide::Opposite { l } else { r };
    let y_main = y_main_x;
    if shown(xa) {
        line(out, (l, x_main), (r, x_main), xa, xc, gap_x);
    }
    if shown(ya) {
        line(out, (y_main, t), (y_main, b), ya, yc, &gap_y);
    }
    if shown(xa) && xa.show_opposite {
        line(out, (l, x_other), (r, x_other), xa, xc, gap_x);
    }
    if shown(ya) && ya.show_opposite {
        line(out, (y_other, t), (y_other, b), ya, yc, &gap_y);
    }
    // An axis moved into the frame (Scatter Central) leaves its own edge bare; with the box shown, that
    // edge is drawn as part of the frame.
    let x_home = if xa.side == AxisSide::Opposite { t } else { b };
    if shown(xa) && xa.show_opposite && xa.offset_pct < 0.0 && x_main != x_home {
        line(out, (l, x_home), (r, x_home), xa, xc, gap_x);
    }
    let y_home = if ya.side == AxisSide::Opposite { r } else { l };
    if shown(ya) && ya.show_opposite && ya.offset_pct < 0.0 && y_main != y_home {
        line(out, (y_home, t), (y_home, b), ya, yc, &gap_y);
    }
}

/// Origin's "Additional Lines" (Grids tab): the X axis' line repeated across the layer at a Y value
/// and the Y axis' line at an X value, in that axis' width and colour. A value outside the axis
/// range draws nothing.
fn draw_additional_lines(out: &mut Vec<Item>, xa: &Axis, ya: &Axis, geom: &LayerGeometry, xc: Color, yc: Color) {
    let f = geom.frame;
    let stroke = |a: &Axis, color: Color| Stroke {
        color,
        width: a.line_width_pt,
        dash: Vec::new(),
        cap: Cap::Butt,
        join: Join::Miter,
    };
    if let Some(v) = xa.additional_line {
        let u = geom.y.to_unit(v);
        if u.is_finite() && (0.0..=1.0).contains(&u) {
            let y = f.y + (1.0 - u) * f.h;
            let mut d = String::new();
            move_to(&mut d, f.x, y);
            line_to(&mut d, f.x + f.w, y);
            out.push(Item::Path { d, fill: None, stroke: Some(stroke(xa, xc)) });
        }
    }
    if let Some(v) = ya.additional_line {
        let u = geom.x.to_unit(v);
        if u.is_finite() && (0.0..=1.0).contains(&u) {
            let x = f.x + u * f.w;
            let mut d = String::new();
            move_to(&mut d, x, f.y);
            line_to(&mut d, x, f.y + f.h);
            out.push(Item::Path { d, fill: None, stroke: Some(stroke(ya, yc)) });
        }
    }
}

/// The part of a tick on one side of the axis line: `(from, to)` offsets in
/// the direction pointing into the frame (negative is outwards).
pub(crate) fn tick_span(dir: TickDirection, len: f64) -> Option<(f64, f64)> {
    match dir {
        TickDirection::In => Some((0.0, len)),
        TickDirection::Out => Some((0.0, -len)),
        TickDirection::Both => Some((-len, len)),
        TickDirection::None => None,
    }
}

/// How far ticks reach out of the frame on the axis' outer side.
pub(crate) fn outward_extent(axis: &Axis) -> f64 {
    let ext = |dir: TickDirection, len: f64| match dir {
        TickDirection::Out | TickDirection::Both => len.max(0.0),
        _ => 0.0,
    };
    let mut e = ext(axis.major_tick, axis.major_tick_len_pt);
    if axis.minor_ticks > 0 {
        e = e.max(ext(axis.minor_tick, axis.minor_tick_len_pt));
    }
    e
}

/// Draws the ticks of one axis on its own edge and, with `opposite_ticks`, on the edge across
/// (mirrored, so an `In` tick still points into the frame). `horizontal` is true for X.
fn draw_ticks(out: &mut Vec<Item>, axis: &Axis, resolved: &ResolvedAxis, f: Rect, horizontal: bool, color: Color) {
    let f =
        if horizontal { crate::axis_offset::x_axis_frame(axis, f) } else { crate::axis_offset::y_axis_frame(axis, f) };
    // (position of the edge, +1 when "into the frame" is the positive page direction).
    let (main, other) = match (horizontal, axis.side) {
        (_, AxisSide::Hidden) => return,
        (true, AxisSide::Normal) => ((f.y + f.h, -1.0), (f.y, 1.0)),
        (true, AxisSide::Opposite) => ((f.y, 1.0), (f.y + f.h, -1.0)),
        (false, AxisSide::Normal) => ((f.x, 1.0), (f.x + f.w, -1.0)),
        (false, AxisSide::Opposite) => ((f.x + f.w, -1.0), (f.x, 1.0)),
    };
    let side = |out: &mut Vec<Item>, (line_pos, inward): (f64, f64)| {
        let mut d = String::new();
        let sets = [
            (&resolved.major, axis.major_tick, axis.major_tick_len_pt),
            (&resolved.minor, axis.minor_tick, if axis.minor_ticks > 0 { axis.minor_tick_len_pt } else { 0.0 }),
        ];
        for (values, dir, len) in sets {
            let Some((a, b)) = tick_span(dir, len) else { continue };
            for &v in values {
                let u = resolved.to_unit(v);
                if !u.is_finite() {
                    continue;
                }
                if horizontal {
                    let x = f.x + u * f.w;
                    move_to(&mut d, x, line_pos + inward * a);
                    line_to(&mut d, x, line_pos + inward * b);
                } else {
                    let y = f.y + (1.0 - u) * f.h;
                    move_to(&mut d, line_pos + inward * a, y);
                    line_to(&mut d, line_pos + inward * b, y);
                }
            }
        }
        if !d.is_empty() {
            let stroke =
                Stroke::solid(axis.tick_color.unwrap_or(color), axis.tick_width_pt.unwrap_or(axis.line_width_pt));
            out.push(Item::Path { d, fill: None, stroke: Some(stroke) });
        }
    };
    side(out, main);
    if axis.opposite_ticks {
        side(out, other);
    }
}

// ----- tick labels and titles ----------------------------------------------------------------

pub(crate) fn label_gap(axis: &Axis) -> f64 {
    0.35 * axis.tick_font.size_pt
}

/// Whether the tick at unit position `u` is at the far end of an axis that leaves out its end
/// label (`hide_end_label`): the top of a Y axis or the right end of an X axis, whatever the
/// direction of the values.
fn at_hidden_end(axis: &Axis, u: f64) -> bool {
    axis.hide_end_label && u >= 1.0 - 1e-9
}

/// The rotation of an X axis' tick labels in degrees, counter-clockwise, in (-180, 180]; 0 when
/// the stored angle is not a number.
fn label_angle(axis: &Axis) -> f64 {
    let a = axis.tick_label_angle_deg;
    if !a.is_finite() {
        return 0.0;
    }
    let a = a.rem_euclid(360.0);
    if a > 180.0 { a - 360.0 } else { a }
}

/// Draws the X tick labels in `font` below the frame, or above it for an axis on the top edge.
/// Returns the outer edge of the label block: its bottom (top), or the tick ends when the
/// labels are hidden.
///
/// Rotated labels (graphing spec 14.5) hang from the corner of their text box that is nearest to
/// the axis, which sits right under (over) the tick; the block reaches as far as the deepest label.
fn draw_x_labels(
    out: &mut Vec<Item>,
    axis: &Axis,
    resolved: &ResolvedAxis,
    f: Rect,
    font: &Font,
    m: &dyn TextMeasurer,
) -> f64 {
    // A displaced axis (graphing spec 21.3) carries its labels with it.
    let f = crate::axis_offset::x_axis_frame(axis, f);
    let size = axis.tick_font.size_pt;
    let top_side = axis.side == AxisSide::Opposite;
    let rug = crate::rug::main_extent(axis, f);
    let tick_end = if top_side { f.y - outward_extent(axis) - rug } else { f.y + f.h + outward_extent(axis) + rug };
    if !axis.show_tick_labels {
        return tick_end;
    }
    // The side of the label block that faces the axis.
    let near = if top_side { tick_end - label_gap(axis) } else { tick_end + label_gap(axis) };
    let labels = resolved.labelled().filter_map(|(v, label)| {
        let u = resolved.to_unit(v);
        (u.is_finite() && !at_hidden_end(axis, u)).then_some((f.x + u * f.w, label))
    });
    let angle = label_angle(axis);
    if angle == 0.0 {
        let baseline = if top_side { near - DESCENT * size } else { near + ASCENT * size };
        for (x, label) in labels {
            text::place(out, m, label, font, size, (x, baseline), Anchor::Middle, 0.0);
        }
        let depth = (ASCENT + DESCENT) * size;
        return if top_side { near - depth } else { near + depth };
    }
    let (sin, cos) = angle.to_radians().sin_cos();
    let mut depth: f64 = 0.0;
    for (x, label) in labels {
        if label.is_empty() {
            continue;
        }
        let w = text::width(m, label, &axis.tick_font, size);
        // The text box in label coordinates (along the text, down from the baseline start),
        // turned to page offsets from the baseline start.
        let corners = [(0.0, -ASCENT * size), (w, -ASCENT * size), (w, DESCENT * size), (0.0, DESCENT * size)]
            .map(|(a, b)| (a * cos + b * sin, -a * sin + b * cos));
        let ymin = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
        let ymax = corners.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max);
        // The corner (or edge) nearest to the axis goes right under (over) the tick.
        let target = if top_side { ymax } else { ymin };
        let tol = 1e-9 * target.abs().max(1.0);
        let touching: Vec<f64> = corners.iter().filter(|c| (c.1 - target).abs() <= tol).map(|c| c.0).collect();
        let xt = touching.iter().sum::<f64>() / touching.len() as f64;
        text::place(out, m, label, font, size, (x - xt, near - target), Anchor::Start, angle);
        depth = depth.max(ymax - ymin);
    }
    if top_side { near - depth } else { near + depth }
}

/// Draws the Y tick labels in `font`: right-aligned left of the frame, or left-aligned right of
/// it for an axis on the right edge. Returns the outer edge of the widest label (or of the tick
/// ends when the labels are hidden).
fn draw_y_labels(
    out: &mut Vec<Item>,
    axis: &Axis,
    resolved: &ResolvedAxis,
    f: Rect,
    font: &Font,
    m: &dyn TextMeasurer,
) -> f64 {
    // A displaced axis (graphing spec 21.3) carries its labels outward with it.
    let f = crate::axis_offset::y_axis_frame(axis, f);
    let right_side = axis.side == AxisSide::Opposite;
    let rug = crate::rug::main_extent(axis, f);
    let tick_end = if right_side { f.x + f.w + outward_extent(axis) + rug } else { f.x - outward_extent(axis) - rug };
    if !axis.show_tick_labels {
        return tick_end;
    }
    let size = axis.tick_font.size_pt;
    let (start, anchor) = if right_side {
        (tick_end + label_gap(axis), Anchor::Start)
    } else {
        (tick_end - label_gap(axis), Anchor::End)
    };
    let mut edge = start;
    for (v, label) in resolved.labelled() {
        let u = resolved.to_unit(v);
        if !u.is_finite() || at_hidden_end(axis, u) {
            continue;
        }
        let y = f.y + (1.0 - u) * f.h;
        let w = text::width(m, label, &axis.tick_font, size);
        edge = if right_side { edge.max(start + w) } else { edge.min(start - w) };
        text::place(out, m, label, font, size, (start, y + (ASCENT - DESCENT) / 2.0 * size), anchor, 0.0);
    }
    edge
}

/// The title of an axis without a title of its own, from the layer's first plot: its Y column for
/// a Y axis; for an X axis its X column (numbers or text), else the column that labels the ticks,
/// else `Row`.
/// The title an axis shows: its own text with Origin's `%(?X)` and `%(?Y)` (with any options after
/// a comma, such as `%(?X,@LG)`) replaced by the title built from the plotted X or Y column (T246),
/// or that built title when the axis has no text of its own.
pub(crate) fn resolved_title(project: &Project, axis: &Axis, pds: &[PlotData], y_axis: bool) -> String {
    match &axis.title {
        None => first_title(project, axis, pds, y_axis),
        Some(text) => substitute_axis_tokens(text, |y, option| match title_source(project, axis, pds, y) {
            TitleSource::Column(c) => token_option_text(project, c, option),
            TitleSource::Text(s) => s,
        }),
    }
}

/// What `%(?X,option)` / `%(?Y,option)` shows for the column that names the axis, as Origin 2016
/// draws it (T281, measured with `yl.text$` and exported pictures, `re-workspace/scratch/t281`):
/// no option, `@LG` and `@U` the built title (long name, else short name, then ` (units)`); `@LL`
/// and `@L` the long name; `@LU` the units; `@LC` the comments; `@LS` and `@C` the short name; `@LA`
/// the long name, else the short name; `@LN` the comments, else the long name, else the short name,
/// then ` (units)`; `@D` `Book_Column`; `@W` the book's short name, `@WL` its long name, `@WS` the
/// sheet's name; `@LP` (a parameter row Sheaf has not) nothing; any other option `###`.
pub(crate) fn token_option_text(project: &Project, c: &sheaf_core::Column, option: &str) -> String {
    let units = |name: &str| if c.units.is_empty() { name.to_string() } else { format!("{name} ({})", c.units) };
    let first = |names: &[&str]| names.iter().find(|n| !n.is_empty()).map(|n| n.to_string()).unwrap_or_default();
    let place = project.locate(c.id);
    match option.trim().to_ascii_uppercase().as_str() {
        "" | "@LG" | "@U" => c.axis_title(),
        "@LL" | "@L" => c.long_name.clone(),
        "@LU" => c.units.clone(),
        "@LC" => c.comments.clone(),
        "@LS" | "@C" => c.short_name.clone(),
        "@LA" => first(&[&c.long_name, &c.short_name]),
        "@LN" => units(&first(&[&c.comments, &c.long_name, &c.short_name])),
        "@D" => place.map(|(b, _, _)| format!("{}_{}", b.name, c.short_name)).unwrap_or_default(),
        "@W" => place.map(|(b, _, _)| b.name.clone()).unwrap_or_default(),
        "@WL" => place.map(|(b, _, _)| b.long_name.clone()).unwrap_or_default(),
        "@WS" => place.map(|(_, s, _)| s.name.clone()).unwrap_or_default(),
        "@LP" => String::new(),
        _ => "###".to_string(),
    }
}

/// What `%(n)` or `%(n, option)` shows in a legend for plot `plot`, as Origin 2016 substitutes it in a
/// graph (T339, measured with `string s$ = "%(1,@LL)"` on a graph window, `re-workspace/scratch/t338`):
/// without an option the plot's name ([`Project::plot_label`]: the comment's first line, else the long
/// name, else the short name); `@LM` the same from the column alone; `@R` the plotted range,
/// `[Book1]Sheet1!Col("Volt")[1:5]` (the long name quoted, else the short name); every other option as
/// in an axis title ([`token_option_text`]: `@LL`, `@LU`, `@LC`, `@LS`, `@LG`, `@LA`, `@LN`, `@L`, `@U`,
/// `@C`, `@D`, `@W`, `@WS`, `@WL`, `@LP`; unknown ones `###`).
pub(crate) fn legend_name(project: &Project, plot: &sheaf_core::graph::Plot, option: &str) -> String {
    let option = option.trim();
    if option.is_empty() {
        return project.plot_label(plot);
    }
    let Some(c) = project.column(plot.y) else { return project.plot_label(plot) };
    match option.to_ascii_uppercase().as_str() {
        "@LM" => c.comments.lines().next().filter(|l| !l.is_empty()).unwrap_or(c.display_name()).to_string(),
        "@R" => {
            let name = if c.long_name.is_empty() { c.short_name.clone() } else { format!("\"{}\"", c.long_name) };
            let (from, to) = plot.rows.map_or((1, c.len()), |(a, b)| (a + 1, b.min(c.len())));
            match project.locate(c.id) {
                Some((b, s, _)) => format!("[{}]{}!Col({name})[{from}:{to}]", b.name, s.name),
                None => format!("Col({name})[{from}:{to}]"),
            }
        }
        _ => token_option_text(project, c, option),
    }
}

/// Replaces each `%(?X...)` and `%(?Y...)` of `text` (letter case ignored) by `built(is_y, option)`,
/// `option` being the text after the comma (empty without one); other text, and a `%(?` not followed
/// by X or Y and a closing parenthesis, stay as they are.
pub(crate) fn substitute_axis_tokens(text: &str, mut built: impl FnMut(bool, &str) -> String) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("%(?") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 3..];
        let which = after.chars().next().map(|c| c.to_ascii_uppercase());
        match (which, after.find(')')) {
            (Some(w @ ('X' | 'Y')), Some(close)) if after[1..close].is_empty() || after[1..close].starts_with(',') => {
                let option = after[1..close].strip_prefix(',').unwrap_or("");
                out.push_str(&built(w == 'Y', option));
                rest = &after[close + 1..];
            }
            _ => {
                out.push_str("%(?");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub(crate) fn first_title(project: &Project, axis: &Axis, pds: &[PlotData], y_axis: bool) -> String {
    match title_source(project, axis, pds, y_axis) {
        TitleSource::Column(c) => c.axis_title(),
        TitleSource::Text(s) => s,
    }
}

/// What names an axis: a column (its labels make the title) or a fixed text.
pub(crate) enum TitleSource<'a> {
    Column(&'a sheaf_core::Column),
    Text(String),
}

pub(crate) fn title_source<'a>(
    project: &'a Project,
    axis: &Axis,
    pds: &[PlotData<'a>],
    y_axis: bool,
) -> TitleSource<'a> {
    let column = |id| project.column(id).map_or(TitleSource::Text(String::new()), TitleSource::Column);
    let text = |s: &str| TitleSource::Text(s.to_string());
    let Some(pd) = pds.first() else { return text("") };
    if pd.plot.kind == PlotKind::Histogram {
        // The counts up the page, the binned column along it.
        return if y_axis { text("Count") } else { column(pd.plot.y) };
    }
    if !y_axis && let Some(ix) = &pd.plot.indexed {
        // Indexed data: the X axis is named by the main group column.
        return column(ix.group);
    }
    if (pd.boxed.is_some() || pd.column_at.is_some()) && !y_axis {
        return text("");
    }
    match (&pd.plot.stat, y_axis) {
        (Some(StatPlot::Interval { .. }), false) => return text(""),
        (Some(StatPlot::Pareto | StatPlot::ParetoCumulative), false) => return column(pd.plot.y),
        _ => {}
    }
    if pd.column_at.is_some() {
        return text("Range");
    }
    if pd.ridge.is_some() {
        // The columns name the baselines; the values along X have no column of their own.
        return if y_axis { text("") } else { text("Value") };
    }
    // Horizontal bars show their values on the X axis and their positions on the Y axis.
    if y_axis != pd.transposed() {
        return column(pd.plot.y);
    }
    if let Some(s) = project.column(pd.plot.y).and_then(|c| c.sampling.as_ref()) {
        // A sampling interval names the X axis by its long name and units (T84), even beside an X
        // column (T304).
        return TitleSource::Text(s.title());
    }
    let x = pd.plot.x.and_then(|id| project.column(id));
    match x.or_else(|| data::label_column(project, axis, Dim::X, Some(pd))) {
        Some(c) => TitleSource::Column(c),
        None => text("Row"),
    }
}

type PlaceTitle<'a> = dyn Fn(&mut Vec<Item>, &str, &Font, (f64, f64), f64) + 'a;

/// X title, centred on the frame, `0.6 * size` beyond the labels' outer edge `labels_edge`.
#[allow(clippy::too_many_arguments)]
/// Where a title placed at `pos` (the middle of its baseline) with the default angle `default_deg`
/// goes, and at which angle, once the axis' rotation and offset are applied (T291): it turns about
/// its own middle, as Origin turns a title, then moves by the offset.
pub(crate) fn turned_title(axis: &Axis, pos: (f64, f64), default_deg: f64) -> ((f64, f64), f64) {
    let angle = axis.title_rotation_deg.filter(|a| a.is_finite()).unwrap_or(default_deg);
    // From the baseline to the middle of the letters, along "up" of the text (y grows downwards).
    let half = 0.5 * (ASCENT - DESCENT) * axis.title_font.size_pt;
    let up = |deg: f64| {
        let (sin, cos) = deg.to_radians().sin_cos();
        (-sin, -cos)
    };
    let (u0, u1) = (up(default_deg), up(angle));
    let middle = (pos.0 + half * u0.0, pos.1 + half * u0.1);
    let (dx, dy) = axis.title_offset_pt;
    let (dx, dy) = (if dx.is_finite() { dx } else { 0.0 }, if dy.is_finite() { dy } else { 0.0 });
    ((middle.0 - half * u1.0 + dx, middle.1 - half * u1.1 + dy), angle)
}

fn draw_x_title(
    out: &mut Vec<Item>,
    project: &Project,
    axis: &Axis,
    f: Rect,
    pds: &[PlotData],
    labels_edge: f64,
    font: &Font,
    place: &PlaceTitle<'_>,
) {
    if !axis.show_title {
        return;
    }
    let title = resolved_title(project, axis, pds, false);
    let size = axis.title_font.size_pt;
    let baseline = if axis.side == AxisSide::Opposite {
        // The text's bottom edge (baseline plus descent) faces the labels below it.
        labels_edge - 0.6 * size - DESCENT * size
    } else {
        labels_edge + 0.6 * size + ASCENT * size
    };
    let (pos, angle) = turned_title(axis, (f.x + f.w / 2.0, baseline), 0.0);
    place(out, &title, font, pos, angle);
}

/// Y title, rotated 90 degrees counter-clockwise (read from bottom to top) on either side, centred
/// on the frame height, `0.6 * size` beyond the labels' outer edge `labels_edge`.
#[allow(clippy::too_many_arguments)]
fn draw_y_title(
    out: &mut Vec<Item>,
    project: &Project,
    axis: &Axis,
    f: Rect,
    pds: &[PlotData],
    labels_edge: f64,
    font: &Font,
    place: &PlaceTitle<'_>,
) {
    if !axis.show_title {
        return;
    }
    let title = resolved_title(project, axis, pds, true);
    let size = axis.title_font.size_pt;
    let x = if axis.side == AxisSide::Opposite {
        // The tops of the letters (the ascent side) face the labels on the left.
        labels_edge + 0.6 * size + ASCENT * size
    } else {
        // The text's bottom edge (baseline plus descent) faces the axis.
        labels_edge - 0.6 * size - DESCENT * size
    };
    let (pos, angle) = turned_title(axis, (x, f.y + f.h / 2.0), 90.0);
    place(out, &title, font, pos, angle);
}

/// Title of an X axis moved into the frame (Origin's Scatter Central): rotated 90 degrees like a Y
/// title, just left of the frame's left edge, centred on the axis line's page y `line_y`.
#[allow(clippy::too_many_arguments)]
fn draw_x_title_inward(
    out: &mut Vec<Item>,
    project: &Project,
    axis: &Axis,
    f: Rect,
    pds: &[PlotData],
    line_y: f64,
    font: &Font,
    place: &PlaceTitle<'_>,
) {
    if !axis.show_title {
        return;
    }
    let title = resolved_title(project, axis, pds, false);
    let size = axis.title_font.size_pt;
    let x = f.x - 0.6 * size - DESCENT * size;
    let (pos, angle) = turned_title(axis, (x, line_y), 90.0);
    place(out, &title, font, pos, angle);
}

/// Title of a Y axis moved into the frame (Origin's Scatter Central): horizontal, centred on the axis
/// line's page x `line_x`, just above the frame's top edge.
#[allow(clippy::too_many_arguments)]
fn draw_y_title_inward(
    out: &mut Vec<Item>,
    project: &Project,
    axis: &Axis,
    f: Rect,
    pds: &[PlotData],
    line_x: f64,
    font: &Font,
    place: &PlaceTitle<'_>,
) {
    if !axis.show_title {
        return;
    }
    let title = resolved_title(project, axis, pds, true);
    let size = axis.title_font.size_pt;
    let baseline = f.y - 0.6 * size - DESCENT * size;
    let (pos, angle) = turned_title(axis, (line_x, baseline), 0.0);
    place(out, &title, font, pos, angle);
}

// ----- legend --------------------------------------------------------------------------------

/// How far the major ticks reach into the frame on each edge, over every axis drawn in the frame
/// (a layer and its right Y layer, not the other panels of a stack); a legend in a corner keeps
/// clear of them. `None` where no axis draws ticks on that edge.
#[derive(Copy, Clone, Debug, Default)]
struct EdgeTicks {
    left: Option<f64>,
    right: Option<f64>,
    top: Option<f64>,
    bottom: Option<f64>,
}

fn edge_ticks(g: &Graph, index: usize) -> EdgeTicks {
    fn add(slot: &mut Option<f64>, v: f64) {
        *slot = Some(slot.map_or(v, |old| old.max(v)));
    }
    // `across` is `None` when the axis draws no ticks on the edge across (a waterfall never does).
    fn axis(a: &Axis, main: &mut Option<f64>, across: Option<&mut Option<f64>>) {
        let inward =
            if matches!(a.major_tick, TickDirection::In | TickDirection::Both) { a.major_tick_len_pt } else { 0.0 };
        add(main, inward);
        if let Some(across) = across.filter(|_| a.opposite_ticks) {
            add(across, inward);
        }
    }
    let mut e = EdgeTicks::default();
    for i in g.same_frame(index) {
        let (xa, ya) = (&g.layers[i].x_axis, &g.layers[i].y_axis);
        let opposite = stack::waterfall(g, i).is_none();
        match xa.side {
            AxisSide::Normal => axis(xa, &mut e.bottom, opposite.then_some(&mut e.top)),
            AxisSide::Opposite => axis(xa, &mut e.top, opposite.then_some(&mut e.bottom)),
            AxisSide::Hidden => {}
        }
        match ya.side {
            AxisSide::Normal => axis(ya, &mut e.left, opposite.then_some(&mut e.right)),
            AxisSide::Opposite => axis(ya, &mut e.right, opposite.then_some(&mut e.left)),
            AxisSide::Hidden => {}
        }
    }
    e
}

/// One piece of a legend row, resolved against the plots.
enum Cell {
    /// The line and symbol sample of entry `n`.
    Sample(usize),
    /// Text with the label markup, and its width in points.
    Text(String, f64),
}

struct LegendRow {
    cells: Vec<Cell>,
    width: f64,
    separation_before: f64,
}

/// Draws legend `lg` inside frame `f`: one row per entry line (the automatic text has one
/// line per plot), in `lg.columns` columns filled top to bottom. `ids` name the entries as
/// `\l(n)` and `%(n)` refer to them. `owner_plots` supplies the layer-local colors for `\cpN`.
#[allow(clippy::too_many_arguments)]
fn draw_legend(
    out: &mut Vec<Item>,
    project: &Project,
    lg: &Legend,
    f: Rect,
    entries: &[&PlotData],
    ids: &[String],
    ticks: &EdgeTicks,
    m: &dyn TextMeasurer,
    owner_plots: &[Plot],
) {
    // A parallel plot's legend lists its groups, not its axes (`parallel::draw_legend`).
    if !lg.visible || entries.is_empty() || entries.iter().any(|pd| pd.plot.kind.is_parallel()) {
        return;
    }
    if let LegendPosition::ManualTopLeft { x, y } = lg.position {
        crate::legend_anchor::place(out, m, f, (x, y), |out, position| {
            let mut legend = lg.clone();
            legend.position = position;
            draw_legend(out, project, &legend, f, entries, ids, ticks, m, owner_plots);
        });
        return;
    }
    let fs = lg.font.size_pt;
    let sample = 2.2 * fs;
    let gap = 0.5 * fs;
    let pad = 0.4 * fs;
    let row_h = 1.3 * fs;
    let col_gap = fs;
    let plot_color = |slot| owner_plots.get(slot).map(crate::rug::plot_color);
    let segments = |t: &str| crate::markup::plot_segments(t, &|name| m.expand_inline_family(name), &plot_color);
    let text_width = |t: &str| text::segments_width(m, &segments(t), &lg.font, fs);

    // Piled, floating and lying bars list the last plot first (measured, spec 14.12).
    let reversed = entries.first().is_some_and(|pd| {
        pd.plot.kind.is_stacked_column() || pd.plot.kind.is_floating() || pd.plot.kind == PlotKind::Bar
    });
    let text = lg.text.clone().unwrap_or_else(|| {
        if reversed {
            let rev: Vec<String> = ids.iter().rev().cloned().collect();
            Legend::automatic_text(&rev)
        } else {
            Legend::automatic_text(ids)
        }
    });
    let mut rows = Vec::new();
    let mut separation_before = 0.0;
    for line in legend::lines(&text) {
        if let Some(percent) = legend::separation_percent(&line) {
            separation_before += f64::from(percent) / 100.0 * fs;
            continue;
        }
        let mut cells: Vec<Cell> = Vec::new();
        let mut pending = String::new();
        let mut after_symbol = false;
        let flush = |pending: &mut String, cells: &mut Vec<Cell>, after_symbol: &mut bool| {
            let t = if *after_symbol { pending.trim_start() } else { pending.as_str() };
            if !t.is_empty() {
                cells.push(Cell::Text(t.to_string(), text_width(t)));
            }
            pending.clear();
            *after_symbol = false;
        };
        for part in legend::parse_line(&line) {
            match part {
                legend::Part::Text(t) => pending.push_str(t),
                legend::Part::Name(id, option) => {
                    if let Some(k) = legend::resolve(ids, id) {
                        pending.push_str(&legend_name(project, entries[k].plot, option));
                    }
                }
                legend::Part::Symbol(id) => {
                    flush(&mut pending, &mut cells, &mut after_symbol);
                    if let Some(k) = legend::resolve(ids, id) {
                        cells.push(Cell::Sample(k));
                    }
                    // The space after an escape is dropped even when the escape drew nothing.
                    after_symbol = true;
                }
            }
        }
        flush(&mut pending, &mut cells, &mut after_symbol);
        if let Some(Cell::Text(t, w)) = cells.last_mut() {
            let trimmed = t.trim_end();
            if trimmed.len() != t.len() {
                *t = trimmed.to_string();
                *w = text_width(t);
            }
        }
        let mut width = 0.0;
        for (i, c) in cells.iter().enumerate() {
            width += match c {
                Cell::Sample(_) => sample,
                Cell::Text(_, w) => *w + if i > 0 && matches!(cells[i - 1], Cell::Sample(_)) { gap } else { 0.0 },
            };
        }
        rows.push(LegendRow { cells, width, separation_before: std::mem::take(&mut separation_before) });
    }
    if rows.is_empty() {
        return;
    }
    let per_col = rows.len().div_ceil((lg.columns.max(1) as usize).min(rows.len()));
    let n_cols = rows.len().div_ceil(per_col);
    let mut col_w = Vec::with_capacity(n_cols);
    let mut row_centers = Vec::with_capacity(rows.len());
    let mut content_height: f64 = 0.0;
    for column in rows.chunks(per_col) {
        let mut extra = 0.0;
        for (i, row) in column.iter().enumerate() {
            if i > 0 {
                extra += row.separation_before;
            }
            row_centers.push(row_h * (i as f64 + 0.5) + extra);
        }
        col_w.push(column.iter().map(|row| row.width).fold(0.0, f64::max));
        content_height = content_height.max(row_h * column.len() as f64 + extra);
    }
    let w = 2.0 * pad + col_w.iter().sum::<f64>() + col_gap * (n_cols - 1) as f64;
    let h = 2.0 * pad + content_height;

    // Margin from the nearest frame edges; ticks pointing in push the legend further.
    let m_left = 0.8 * fs + ticks.left.unwrap_or(0.0);
    let m_bottom = 0.8 * fs + ticks.bottom.unwrap_or(0.0);
    let m_right = 0.8 * fs + ticks.right.unwrap_or(0.0);
    let m_top = 0.8 * fs + ticks.top.unwrap_or(0.0);
    let (x0, y0) = match lg.position {
        LegendPosition::Automatic => {
            let (x, y) = Legend::AUTOMATIC_ANCHOR;
            (f.x + x * f.w, f.y + y * f.h)
        }
        LegendPosition::TopLeft => (f.x + m_left, f.y + m_top),
        LegendPosition::TopRight => (f.x + f.w - m_right - w, f.y + m_top),
        LegendPosition::BottomLeft => (f.x + m_left, f.y + f.h - m_bottom - h),
        LegendPosition::BottomRight => (f.x + f.w - m_right - w, f.y + f.h - m_bottom - h),
        LegendPosition::ManualTopLeft { x, y } => (f.x + x * f.w, f.y + y * f.h),
        LegendPosition::Manual { x, y } => (f.x + x * f.w - w / 2.0, f.y + y * f.h - h / 2.0),
    };
    if lg.background.is_some() || lg.frame {
        let stroke = lg.frame.then(|| Stroke::solid(lg.font.color, 0.75));
        out.push(Item::Rect { rect: Rect { x: x0, y: y0, w, h }, fill: lg.background, stroke });
    }
    for (r, row) in rows.iter().enumerate() {
        let col = r / per_col;
        let col_x = x0 + pad + col_w[..col].iter().sum::<f64>() + col_gap * col as f64;
        let cy = y0 + pad + row_centers[r];
        let baseline = cy + (ASCENT - DESCENT) / 2.0 * fs;
        let mut x = col_x;
        let mut after_symbol = false;
        for cell in &row.cells {
            match cell {
                Cell::Sample(k) => {
                    let (sx0, sx1) = (x, x + sample);
                    let entry = entries[*k];
                    let plot = entry.plot;
                    x = sx1;
                    after_symbol = true;
                    if let Some(vector) = &entry.vector {
                        crate::vector_plot::draw_legend_sample(out, (sx0, cy), sample, fs, &plot.line, &vector.spec);
                        continue;
                    }
                    // A line plot filled under its curve shows a filled box, as Origin's legend does.
                    let filled = plot.decor.fill_under.filter(|_| {
                        plot.kind.has_line() && !plot.kind.is_area() && !plot.decor.fill_under_mode.is_sidelines()
                    });
                    let mut front_line = None;
                    if let Some(fill) = filled {
                        out.push(Item::Rect {
                            rect: Rect { x: sx0, y: cy - fs / 2.0, w: sample, h: fs },
                            fill: Some(fill),
                            stroke: (plot.line.width_pt > 0.0)
                                .then(|| Stroke::solid(plot.line.color, plot.line.width_pt.min(fs / 4.0))),
                        });
                    } else if plot.kind.has_line() && plot.line.width_pt > 0.0 {
                        let mut d = String::new();
                        move_to(&mut d, sx0, cy);
                        line_to(&mut d, sx1, cy);
                        let line = Item::Path { d, fill: None, stroke: Some(line_stroke(&plot.line)) };
                        if plot.kind == PlotKind::LineSymbol
                            && plot.decor.symbol_line.draw_in_front
                            && !plot.decor.symbol_line.gap_to_symbol
                        {
                            front_line = Some(line);
                        } else {
                            out.push(line);
                        }
                    }
                    if plot.kind.has_symbols() {
                        // Origin 2016's legend shows "1" for Number and "a" for both letter shapes
                        // (measured, T323).
                        let mut sym = plot.symbol.clone();
                        if sym.shape == sheaf_core::graph::SymbolShape::CapitalLetter {
                            sym.shape = sheaf_core::graph::SymbolShape::SmallLetter;
                        }
                        push_symbols(out, &sym, &[((sx0 + sx1) / 2.0, cy)], &[0]);
                    }
                    out.extend(front_line);
                    if matches!(plot.kind, PlotKind::Histogram | PlotKind::Box) {
                        // An outlined box in the plot's colour, without a fill.
                        out.push(Item::Rect {
                            rect: Rect { x: sx0, y: cy - fs / 2.0, w: sample, h: fs },
                            fill: None,
                            stroke: Some(Stroke::solid(plot.symbol.color, plot.line.width_pt.max(0.5))),
                        });
                    } else if plot.kind.is_violin() {
                        // A small filled box like the silhouette: the tinted fill, outlined in the plot colour.
                        fill::fill_rect(
                            out,
                            Rect { x: sx0, y: cy - fs / 2.0, w: sample, h: fs },
                            crate::violin::violin_fill_color(plot),
                            &plot.decor.fill,
                            Some(Stroke::solid(plot.symbol.color, plot.line.width_pt.max(0.5))),
                        );
                    } else if plot.kind.is_ridgeline() {
                        // A small box in the ridge's fill and outline.
                        fill::fill_rect(
                            out,
                            Rect { x: sx0, y: cy - fs / 2.0, w: sample, h: fs },
                            plot.symbol.color,
                            &sheaf_core::graph::FillStyle {
                                transparency_pct: plot.ridge.fill_transparency_pct.round().clamp(0.0, 100.0) as u8,
                                ..plot.decor.fill
                            },
                            Some(Stroke::solid(plot.symbol.color, plot.line.width_pt.max(0.5))),
                        );
                    } else if plot.kind.is_bar() || plot.kind.is_area() || plot.kind == PlotKind::WindRose {
                        // A small bar: as long as a line sample, one font size high, filled and
                        // outlined like the bars (an area plot is shown the same way).
                        fill::fill_rect(
                            out,
                            Rect { x: sx0, y: cy - fs / 2.0, w: sample, h: fs },
                            plot.symbol.color,
                            &plot.decor.fill,
                            (plot.line.width_pt > 0.0).then(|| Stroke::solid(plot.line.color, plot.line.width_pt)),
                        );
                    }
                }
                Cell::Text(t, w) => {
                    if after_symbol {
                        x += gap;
                    }
                    text::place_segments(out, m, &segments(t), &lg.font, fs, (x, baseline), Anchor::Start, 0.0);
                    x += w;
                    after_symbol = false;
                }
            }
        }
    }
}
