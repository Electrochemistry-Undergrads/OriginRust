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
use crate::fonts::TextMeasurer;
use crate::hit;
use crate::legend;
use crate::plots::{draw_plot, line_stroke, symbol_item, symbol_path};
use crate::scene::{
    Anchor, Cap, Item, ItemList, Join, LayerScene, ObjectKind, Scene, SceneObject, Stroke, line_to, move_to,
};
use crate::stack;
use crate::text::{self, ASCENT, DESCENT};
use crate::ticks::{ResolvedAxis, resolve_axis_edges};
use crate::{LayerGeometry, PlotError, Rect};
use sheaf_core::Color;
use sheaf_core::graph::{
    Axis, AxisSide, Font, Graph, Layer, Legend, LegendPosition, LegendScope, LineStyle, Page, PlotKind, ScaleLink,
    TickDirection,
};
use sheaf_core::{Project, WindowId};

/// Builds the scene of a graph.
pub(crate) fn build_scene(project: &Project, graph: WindowId, m: &dyn TextMeasurer) -> Result<Scene, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let pds: Vec<LayerData> = g.layers.iter().map(|layer| data::collect(project, layer)).collect();
    let geoms = graph_geometry(project, g, &pds, m);
    let x_titles = title_places(project, g, &geoms, &pds, Dim::X, m);
    let y_titles = title_places(project, g, &geoms, &pds, Dim::Y, m);
    let mut layers: Vec<LayerScene> = (0..g.layers.len())
        .map(|i| layer_scene(project, g, i, &geoms[i], &pds[i].plots, (x_titles[i], y_titles[i]), m))
        .collect();
    // A legend that lists the plots of every layer drawn in its frame (a layer and its right Y
    // layer) is drawn after all layers, so that no layer's curves cover it.
    for (i, layer) in g.layers.iter().enumerate() {
        if layer.legend.scope == LegendScope::Graph
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
            );
            hit::record(&mut last.objects, &last.overlay, start, ObjectKind::Legend, i, ItemList::Overlay, &[], m);
        }
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
fn resolve_dim(project: &Project, axis: &Axis, dim: Dim, sources: &[&data::PlotData]) -> ResolvedAxis {
    let first = sources.first().copied();
    let mut range = data::axis_range(sources.iter().copied(), dim, axis.scale);
    let mut resolved = if dim == Dim::X && data::is_category(project, axis, first) {
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
                    if bars.iter().any(|pd| pd.plot.kind == PlotKind::Stacked100Column) {
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
        resolve_dim(project, axis, dim, &sources)
    };
    let own = |i: usize, dim: Dim, axis: &Axis| {
        let sources: Vec<&data::PlotData> = pds[i].plots.iter().collect();
        resolve_dim(project, axis, dim, &sources)
    };
    let (root_x, root_y) = (shared(Dim::X, &rl.x_axis), shared(Dim::Y, &rl.y_axis));
    let axes: Vec<(ResolvedAxis, ResolvedAxis)> = members
        .iter()
        .map(|&i| {
            let l = &g.layers[i];
            let x = if shares(i, Dim::X) { root_x.clone() } else { own(i, Dim::X, &l.x_axis) };
            let y = if shares(i, Dim::Y) { root_y.clone() } else { own(i, Dim::Y, &l.y_axis) };
            (x, y)
        })
        .collect();

    let frame_of = |root_frame: Rect, i: usize| {
        if i == root { root_frame } else { data::linked_frame(&g.layers[i].frame, root_frame) }
    };
    let nominal = data::page_frame(&rl.frame, &g.page);
    let mut edges = LabelEdges::NONE;
    // The rectangle that holds the frames of every member: the root's frame, or the whole stack
    // when the members are panels of their own.
    let mut bounds = nominal;
    for (&i, (x, y)) in members.iter().zip(&axes) {
        let f = frame_of(nominal, i);
        bounds = cover(bounds, f);
        label_edges(&mut edges, &g.layers[i], f, x, y, m);
    }
    let fitted = fit_on_page(bounds, &edges, &g.page);
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
            (i, LayerGeometry { frame, clip, x, y })
        })
        .collect()
}

/// The outermost extent of the tick labels and titles of a group, in page points.
#[derive(Copy, Clone, Debug)]
struct LabelEdges {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl LabelEdges {
    const NONE: LabelEdges =
        LabelEdges { left: f64::INFINITY, right: f64::NEG_INFINITY, top: f64::INFINITY, bottom: f64::NEG_INFINITY };
}

/// Widens `e` by the tick labels and titles of one layer laid out at frame `f`.
fn label_edges(e: &mut LabelEdges, layer: &Layer, f: Rect, x: &ResolvedAxis, y: &ResolvedAxis, m: &dyn TextMeasurer) {
    let mut scratch = Vec::new();
    let (xa, ya) = (&layer.x_axis, &layer.y_axis);
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
        a.title.clone().unwrap_or_else(|| first_title(project, a, &pds[i].plots, dim == Dim::Y))
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
    pds: &[PlotData],
    (x_title, y_title): (TitlePlace, TitlePlace),
    m: &dyn TextMeasurer,
) -> LayerScene {
    let layer = &g.layers[index];
    let mut scene =
        LayerScene { clip: geom.clip, grid: Vec::new(), plots: Vec::new(), overlay: Vec::new(), objects: Vec::new() };
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
        draw_grid(&mut scene.grid, layer, geom);
        (&layer.x_axis, &layer.y_axis)
    };
    match waterfall {
        None => {
            for pd in pds {
                let start = scene.plots.len();
                draw_plot(&mut scene.plots, pd, geom);
                let kind = ObjectKind::Plot(plot_index(pd));
                hit::record(&mut scene.objects, &scene.plots, start, kind, index, ItemList::Plots, &[], m);
            }
        }
        Some((sx, sy)) => {
            // Back to front, so that the first curve is on top; member k moves by k shifts.
            for pd in pds.iter().rev() {
                let k = pd.member.unwrap_or(0) as f64;
                let mut moved = geom.clone();
                moved.frame.x += k * sx * geom.frame.w;
                moved.frame.y -= k * sy * geom.frame.h;
                let start = scene.plots.len();
                draw_plot(&mut scene.plots, pd, &moved);
                let kind = ObjectKind::Plot(plot_index(pd));
                hit::record(&mut scene.objects, &scene.plots, start, kind, index, ItemList::Plots, &[], m);
            }
        }
    }
    let objects = &mut scene.objects;
    let out = &mut scene.overlay;
    let f = geom.frame;
    let first_color = plot_color(pds);
    let (xp, yp) = (AxisPaint::new(xa, first_color), AxisPaint::new(ya, first_color));
    let rec = |objects: &mut Vec<SceneObject>, out: &Vec<Item>, start: usize, kind: ObjectKind, extra: &[Rect]| {
        hit::record(objects, out, start, kind, index, ItemList::Overlay, extra, m)
    };
    // The edge each axis line is drawn on, as a box of no height or width.
    let x_line = Rect { y: if xa.side == AxisSide::Opposite { f.y } else { f.y + f.h }, h: 0.0, ..f };
    let y_line = Rect { x: if ya.side == AxisSide::Opposite { f.x + f.w } else { f.x }, w: 0.0, ..f };
    let gaps = (geom.x.gap.map(|g| g.span(geom.x.reversed)), geom.y.gap.map(|g| g.span(geom.y.reversed)));
    draw_axis_lines(out, xa, ya, f, xp.line, yp.line, gaps);
    let start = out.len();
    draw_ticks(out, xa, &geom.x, f, true, xp.line);
    if xa.side != AxisSide::Hidden {
        rec(objects, out, start, ObjectKind::XAxis, &[x_line]);
    }
    let start = out.len();
    draw_ticks(out, ya, &geom.y, f, false, yp.line);
    if ya.side != AxisSide::Hidden {
        rec(objects, out, start, ObjectKind::YAxis, &[y_line]);
    }
    let start = out.len();
    let x_edge = (xa.side != AxisSide::Hidden).then(|| draw_x_labels(out, xa, &geom.x, f, &xp.labels, m));
    rec(objects, out, start, ObjectKind::XAxis, &[]);
    let start = out.len();
    let y_edge = (ya.side != AxisSide::Hidden).then(|| draw_y_labels(out, ya, &geom.y, f, &yp.labels, m));
    rec(objects, out, start, ObjectKind::YAxis, &[]);
    let start = out.len();
    if let Some(edge) = x_edge {
        match x_title {
            TitlePlace::Own => draw_x_title(out, project, xa, f, pds, edge, &xp.title, m),
            TitlePlace::Shared => {}
            TitlePlace::Span { from, to, edge } => {
                draw_x_title(out, project, xa, Rect { x: from, w: to - from, ..f }, pds, edge, &xp.title, m)
            }
        }
    }
    rec(objects, out, start, ObjectKind::XTitle, &[]);
    let start = out.len();
    if let Some(edge) = y_edge {
        match y_title {
            TitlePlace::Own => draw_y_title(out, project, ya, f, pds, edge, &yp.title, m),
            TitlePlace::Shared => {}
            TitlePlace::Span { from, to, edge } => {
                draw_y_title(out, project, ya, Rect { y: from, h: to - from, ..f }, pds, edge, &yp.title, m)
            }
        }
    }
    rec(objects, out, start, ObjectKind::YTitle, &[]);
    if layer.legend.scope == LegendScope::Layer {
        let entries: Vec<&PlotData> = pds.iter().collect();
        let start = out.len();
        let ids: Vec<String> = (1..=entries.len()).map(|k| k.to_string()).collect();
        draw_legend(out, project, &layer.legend, f, &entries, &ids, &edge_ticks(g, index), m);
        rec(objects, out, start, ObjectKind::Legend, &[]);
    }
    for (k, label) in layer.labels.iter().enumerate() {
        let start = out.len();
        let size = label.font.size_pt;
        let anchor = (f.x + label.x * f.w, f.y + label.y * f.h);
        // The anchor is the vertical middle of the text's first character.
        let (sin, cos) = label.rotation_deg.to_radians().sin_cos();
        let dy = (ASCENT - DESCENT) / 2.0 * size;
        let pos = (anchor.0 + dy * sin, anchor.1 + dy * cos);
        text::place(out, m, &label.text, &label.font, size, pos, Anchor::Start, label.rotation_deg);
        rec(objects, out, start, ObjectKind::Label(k), &[]);
    }
    scene
}

// ----- axis colors ---------------------------------------------------------------------------

/// The color of the first drawable plot of a layer: its line color, or the symbol (fill) color
/// for scatter and column plots, which have no line of their own to show.
fn plot_color(pds: &[PlotData]) -> Option<Color> {
    pds.first().map(|pd| match pd.plot.kind {
        PlotKind::Line | PlotKind::LineSymbol | PlotKind::Area => pd.plot.line.color,
        _ => pd.plot.symbol.color,
    })
}

/// The colors an axis is drawn in: its own, or that of the layer's first plot when the axis
/// follows the plot (and the layer has one).
struct AxisPaint {
    /// Axis line and ticks.
    line: Color,
    /// The tick label font, in its final color.
    labels: Font,
    /// The title font, in its final color.
    title: Font,
}

impl AxisPaint {
    fn new(axis: &Axis, plot: Option<Color>) -> AxisPaint {
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

/// Draws the axis lines of a layer: each shown axis on its own edge and, with `show_opposite`,
/// on the edge across. `xc` and `yc` are the axes' line colors.
fn draw_axis_lines(
    out: &mut Vec<Item>,
    xa: &Axis,
    ya: &Axis,
    f: Rect,
    xc: Color,
    yc: Color,
    gaps: (Option<(f64, f64)>, Option<(f64, f64)>),
) {
    let (l, r, t, b) = (f.x, f.x + f.w, f.y, f.y + f.h);
    let shown = |a: &Axis| a.side != AxisSide::Hidden;
    let same = xa.line_width_pt == ya.line_width_pt && xc == yc;
    if shown(xa) && shown(ya) && xa.show_opposite && ya.show_opposite && same && gaps == (None, None) {
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
                gap: Option<(f64, f64)>| {
        let mut d = String::new();
        let stroke = Stroke { color, width: a.line_width_pt, dash: Vec::new(), cap: Cap::Square, join: Join::Miter };
        match gap {
            None => {
                move_to(&mut d, x0, y0);
                line_to(&mut d, x1, y1);
            }
            Some((lo, hi)) => {
                // Two pieces, and a short slash across the axis at each end of the gap.
                let at = |u: f64| (x0 + u * (x1 - x0), y0 + u * (y1 - y0));
                let (a0, a1, b0, b1) = (at(0.0), at(lo), at(hi), at(1.0));
                move_to(&mut d, a0.0, a0.1);
                line_to(&mut d, a1.0, a1.1);
                move_to(&mut d, b0.0, b0.1);
                line_to(&mut d, b1.0, b1.1);
                let horizontal = (y1 - y0).abs() < 1e-9;
                let k = 3.0;
                for p in [a1, b0] {
                    if horizontal {
                        move_to(&mut d, p.0 - k * 0.5, p.1 + k);
                        line_to(&mut d, p.0 + k * 0.5, p.1 - k);
                    } else {
                        move_to(&mut d, p.0 - k, p.1 - k * 0.5);
                        line_to(&mut d, p.0 + k, p.1 + k * 0.5);
                    }
                }
            }
        }
        out.push(Item::Path { d, fill: None, stroke: Some(stroke) });
    };
    // The line of a Y axis runs from the top down, so its gap is measured from the top.
    let gap_y = gaps.1.map(|(lo, hi)| (1.0 - hi, 1.0 - lo));
    // The edge each axis is drawn on, and the edge across from it.
    let (x_main, x_other) = if xa.side == AxisSide::Opposite { (t, b) } else { (b, t) };
    let (y_main, y_other) = if ya.side == AxisSide::Opposite { (r, l) } else { (l, r) };
    if shown(xa) {
        line(out, (l, x_main), (r, x_main), xa, xc, gaps.0);
    }
    if shown(ya) {
        line(out, (y_main, t), (y_main, b), ya, yc, gap_y);
    }
    if shown(xa) && xa.show_opposite {
        line(out, (l, x_other), (r, x_other), xa, xc, gaps.0);
    }
    if shown(ya) && ya.show_opposite {
        line(out, (y_other, t), (y_other, b), ya, yc, gap_y);
    }
}

/// The part of a tick on one side of the axis line: `(from, to)` offsets in
/// the direction pointing into the frame (negative is outwards).
fn tick_span(dir: TickDirection, len: f64) -> Option<(f64, f64)> {
    match dir {
        TickDirection::In => Some((0.0, len)),
        TickDirection::Out => Some((0.0, -len)),
        TickDirection::Both => Some((-len, len)),
        TickDirection::None => None,
    }
}

/// How far ticks reach out of the frame on the axis' outer side.
fn outward_extent(axis: &Axis) -> f64 {
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

fn label_gap(axis: &Axis) -> f64 {
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
    let size = axis.tick_font.size_pt;
    let top_side = axis.side == AxisSide::Opposite;
    let tick_end = if top_side { f.y - outward_extent(axis) } else { f.y + f.h + outward_extent(axis) };
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
    let right_side = axis.side == AxisSide::Opposite;
    let tick_end = if right_side { f.x + f.w + outward_extent(axis) } else { f.x - outward_extent(axis) };
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
fn first_title(project: &Project, axis: &Axis, pds: &[PlotData], y_axis: bool) -> String {
    let Some(pd) = pds.first() else { return String::new() };
    // Horizontal bars show their values on the X axis and their positions on the Y axis.
    if y_axis != pd.transposed() {
        return project.column(pd.plot.y).map(|c| c.axis_title()).unwrap_or_default();
    }
    let x = pd.plot.x.and_then(|id| project.column(id));
    match x.or_else(|| data::label_column(project, axis, Dim::X, Some(pd))) {
        Some(c) => c.axis_title(),
        None => "Row".to_string(),
    }
}

/// X title, centred on the frame, `0.6 * size` beyond the labels' outer edge `labels_edge`.
#[allow(clippy::too_many_arguments)]
fn draw_x_title(
    out: &mut Vec<Item>,
    project: &Project,
    axis: &Axis,
    f: Rect,
    pds: &[PlotData],
    labels_edge: f64,
    font: &Font,
    m: &dyn TextMeasurer,
) {
    if !axis.show_title {
        return;
    }
    let title = axis.title.clone().unwrap_or_else(|| first_title(project, axis, pds, false));
    let size = axis.title_font.size_pt;
    let baseline = if axis.side == AxisSide::Opposite {
        // The text's bottom edge (baseline plus descent) faces the labels below it.
        labels_edge - 0.6 * size - DESCENT * size
    } else {
        labels_edge + 0.6 * size + ASCENT * size
    };
    text::place(out, m, &title, font, size, (f.x + f.w / 2.0, baseline), Anchor::Middle, 0.0);
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
    m: &dyn TextMeasurer,
) {
    if !axis.show_title {
        return;
    }
    let title = axis.title.clone().unwrap_or_else(|| first_title(project, axis, pds, true));
    let size = axis.title_font.size_pt;
    let x = if axis.side == AxisSide::Opposite {
        // The tops of the letters (the ascent side) face the labels on the left.
        labels_edge + 0.6 * size + ASCENT * size
    } else {
        // The text's bottom edge (baseline plus descent) faces the axis.
        labels_edge - 0.6 * size - DESCENT * size
    };
    text::place(out, m, &title, font, size, (x, f.y + f.h / 2.0), Anchor::Middle, 90.0);
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

/// Draws legend `lg` inside frame `f`: one row per line of its text (the automatic text has one
/// line per plot), in `lg.columns` columns filled top to bottom. `ids` name the entries as
/// `\l(n)` and `%(n)` refer to them.
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
) {
    if !lg.visible || entries.is_empty() {
        return;
    }
    let fs = lg.font.size_pt;
    let sample = 2.2 * fs;
    let gap = 0.5 * fs;
    let pad = 0.4 * fs;
    let row_h = 1.3 * fs;
    let col_gap = fs;

    // Piled and lying bars list the last plot first, as they appear on the page.
    let reversed = entries.first().is_some_and(|pd| pd.plot.kind.is_stacked() || pd.plot.kind.is_horizontal());
    let text = lg.text.clone().unwrap_or_else(|| {
        if reversed {
            let rev: Vec<String> = ids.iter().rev().cloned().collect();
            Legend::automatic_text(&rev)
        } else {
            Legend::automatic_text(ids)
        }
    });
    let mut rows: Vec<(Vec<Cell>, f64)> = Vec::new();
    for line in legend::lines(&text) {
        let mut cells: Vec<Cell> = Vec::new();
        let mut pending = String::new();
        let mut after_symbol = false;
        let flush = |pending: &mut String, cells: &mut Vec<Cell>, after_symbol: &mut bool| {
            let t = if *after_symbol { pending.trim_start() } else { pending.as_str() };
            if !t.is_empty() {
                let w = text::width(m, t, &lg.font, fs);
                cells.push(Cell::Text(t.to_string(), w));
            }
            pending.clear();
            *after_symbol = false;
        };
        for part in legend::parse_line(&line) {
            match part {
                legend::Part::Text(t) => pending.push_str(t),
                legend::Part::Name(id) => {
                    if let Some(k) = legend::resolve(ids, id) {
                        pending.push_str(&project.plot_label(entries[k].plot));
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
                *w = text::width(m, t, &lg.font, fs);
            }
        }
        let mut width = 0.0;
        for (i, c) in cells.iter().enumerate() {
            width += match c {
                Cell::Sample(_) => sample,
                Cell::Text(_, w) => *w + if i > 0 && matches!(cells[i - 1], Cell::Sample(_)) { gap } else { 0.0 },
            };
        }
        rows.push((cells, width));
    }
    if rows.is_empty() {
        return;
    }
    let per_col = rows.len().div_ceil((lg.columns.max(1) as usize).min(rows.len()));
    let n_cols = rows.len().div_ceil(per_col);
    let col_w: Vec<f64> = (0..n_cols)
        .map(|c| rows[c * per_col..((c + 1) * per_col).min(rows.len())].iter().map(|r| r.1).fold(0.0, f64::max))
        .collect();
    let w = 2.0 * pad + col_w.iter().sum::<f64>() + col_gap * (n_cols - 1) as f64;
    let h = 2.0 * pad + row_h * per_col as f64;

    // Margin from the nearest frame edges; ticks pointing in push the legend further.
    let m_left = 0.8 * fs + ticks.left.unwrap_or(0.0);
    let m_bottom = 0.8 * fs + ticks.bottom.unwrap_or(0.0);
    let m_right = 0.8 * fs + ticks.right.unwrap_or(0.0);
    let m_top = 0.8 * fs + ticks.top.unwrap_or(0.0);
    let (x0, y0) = match lg.position {
        LegendPosition::TopLeft => (f.x + m_left, f.y + m_top),
        LegendPosition::TopRight => (f.x + f.w - m_right - w, f.y + m_top),
        LegendPosition::BottomLeft => (f.x + m_left, f.y + f.h - m_bottom - h),
        LegendPosition::BottomRight => (f.x + f.w - m_right - w, f.y + f.h - m_bottom - h),
        LegendPosition::Manual { x, y } => (f.x + x * f.w - w / 2.0, f.y + y * f.h - h / 2.0),
    };
    if lg.background.is_some() || lg.frame {
        let stroke = lg.frame.then(|| Stroke::solid(lg.font.color, 0.75));
        out.push(Item::Rect { rect: Rect { x: x0, y: y0, w, h }, fill: lg.background, stroke });
    }
    for (r, (cells, _)) in rows.iter().enumerate() {
        let (col, row) = (r / per_col, r % per_col);
        let col_x = x0 + pad + col_w[..col].iter().sum::<f64>() + col_gap * col as f64;
        let cy = y0 + pad + row_h * (row as f64 + 0.5);
        let baseline = cy + (ASCENT - DESCENT) / 2.0 * fs;
        let mut x = col_x;
        let mut after_symbol = false;
        for cell in cells {
            match cell {
                Cell::Sample(k) => {
                    let (sx0, sx1) = (x, x + sample);
                    let plot = entries[*k].plot;
                    if plot.kind.has_line() && plot.line.width_pt > 0.0 {
                        let mut d = String::new();
                        move_to(&mut d, sx0, cy);
                        line_to(&mut d, sx1, cy);
                        out.push(Item::Path { d, fill: None, stroke: Some(line_stroke(&plot.line)) });
                    }
                    if plot.kind.has_symbols() {
                        let mut d = String::new();
                        symbol_path(&mut d, plot.symbol.shape, (sx0 + sx1) / 2.0, cy, plot.symbol.size_pt);
                        out.push(symbol_item(d, &plot.symbol));
                    }
                    if plot.kind.is_bar() || plot.kind == PlotKind::Area {
                        // A small bar: as long as a line sample, one font size high, filled and
                        // outlined like the bars (an area plot is shown the same way).
                        out.push(Item::Rect {
                            rect: Rect { x: sx0, y: cy - fs / 2.0, w: sample, h: fs },
                            fill: Some(plot.symbol.color),
                            stroke: (plot.line.width_pt > 0.0)
                                .then(|| Stroke::solid(plot.line.color, plot.line.width_pt)),
                        });
                    }
                    x = sx1;
                    after_symbol = true;
                }
                Cell::Text(t, w) => {
                    if after_symbol {
                        x += gap;
                    }
                    text::place(out, m, t, &lg.font, fs, (x, baseline), Anchor::Start, 0.0);
                    x += w;
                    after_symbol = false;
                }
            }
        }
    }
}
