//! Graph windows of a written project: the model of `sheaf_core::graph` turned into the blocks of
//! `glayout.rs` (spec section 13.9). Everything of the model that the file cannot hold is named in
//! a warning.

use super::glayout::{
    AnnKind, AnnSpec, AxisBlocks, AxisScale, Code, CurveKind, CurveSpec, Group, LayerSpec, LinkSpec, UNITS_PER_INCH,
    WindowSpec, annotation, axis_blocks, curve_record, layer_header, link_annotation, window_header,
};
use super::layout::framed;
use super::{SetInfo, Writer, fail};
use crate::error::IoError;
use crate::opj::graph::PALETTE;
use crate::opj::strings::encode_cp1252;
use sheaf_core::Project;
use sheaf_core::autorange;
use sheaf_core::color::Color;
use sheaf_core::graph::*;
use sheaf_core::ids::ColumnId;
use sheaf_core::stack::{Place, places};
use std::collections::HashMap;

const MAX_PAGE_UNITS: f64 = u16::MAX as f64;
/// Width of one character of text as a fraction of the font size; the reader uses the same number to
/// place text objects, so a label comes back where it was.
pub(crate) const CHAR_WIDTH: f64 = 0.58;
/// Height of one line of text as a multiple of the font size.
pub(crate) const LINE_HEIGHT: f64 = 1.2;

fn units(pt: f64) -> f64 {
    pt / 72.0 * UNITS_PER_INCH
}

fn unit_u16(pt: f64) -> u16 {
    units(pt).round().clamp(0.0, MAX_PAGE_UNITS) as u16
}

fn palette_position(c: Color) -> Option<u8> {
    PALETTE.iter().position(|&(r, g, b)| (r, g, b) == (c.r, c.g, c.b)).map(|i| i as u8)
}

/// The colour as the file stores it: a list position when the colour is in Origin's list, else free.
fn code_of(c: Color) -> Code {
    match palette_position(c) {
        Some(i) => Code::index(i),
        None => Code::rgb(c.r, c.g, c.b),
    }
}

/// The nearest colour of Origin's list, for the places that only hold a position.
fn nearest_position(c: Color) -> u8 {
    let d = |&(r, g, b): &(u8, u8, u8)| {
        let (dr, dg, db) =
            (i32::from(r) - i32::from(c.r), i32::from(g) - i32::from(c.g), i32::from(b) - i32::from(c.b));
        dr * dr + dg * dg + db * db
    };
    PALETTE.iter().enumerate().min_by_key(|(_, p)| d(p)).map_or(0, |(i, _)| i as u8)
}

/// Round numbers for an axis: `(from, to, step)` around `lo..hi`. With [`AutoRange::Nice`] these are the
/// limits Origin's own automatic range gives (`margin_pct` is the Rescale Margin; see `sheaf_core::autorange`).
pub(crate) fn nice_range(lo: f64, hi: f64, log: bool, mode: AutoRange, margin_pct: f64) -> (f64, f64, f64) {
    if log {
        if mode == AutoRange::Nice && lo > 0.0 && hi > lo {
            let r = autorange::log_range(lo, hi, margin_pct);
            let step = match r.ticks {
                autorange::LogTicks::Linear(inc) => inc,
                autorange::LogTicks::Decades => 1.0,
            };
            return (r.from, r.to, step);
        }
        let from = 10f64.powf(lo.max(f64::MIN_POSITIVE).log10().floor());
        let mut to = 10f64.powf(hi.max(from).log10().ceil());
        if to <= from {
            to = from * 10.0;
        }
        return (from, to, 1.0);
    }
    if mode == AutoRange::Nice {
        let (mut a, mut b) = (lo, hi);
        if b - a <= 0.0 || (b - a).is_nan() {
            let pad = if a == 0.0 { 1.0 } else { a.abs() * 0.1 };
            (a, b) = (a - pad, b + pad);
        }
        return autorange::linear_range(a, b, margin_pct);
    }
    let (mut lo, mut hi) = (lo, hi);
    if mode == AutoRange::SymmetricZero {
        let m = lo.abs().max(hi.abs());
        (lo, hi) = (-m, m);
    }
    if hi - lo <= 0.0 {
        let pad = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
        (lo, hi) = (lo - pad, hi + pad);
    }
    let span = hi - lo;
    let raw = span / 6.0;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    if mode == AutoRange::Tight {
        return (lo, hi, step);
    }
    let pad = span * 0.02;
    let from = ((lo - pad) / step).floor() * step;
    let to = ((hi + pad) / step).ceil() * step;
    // remove the rounding noise of the multiplication
    let tidy = |v: f64| (v / step).round() * step;
    (tidy(from), tidy(to), step)
}

struct Sets<'a> {
    map: &'a HashMap<ColumnId, SetInfo>,
}

impl Sets<'_> {
    fn of(&self, id: ColumnId) -> Option<SetInfo> {
        self.map.get(&id).copied()
    }
}

/// A plot that can be written, with the data sets it uses.
struct Resolved<'a> {
    plot: &'a Plot,
    y: SetInfo,
    x: u16,
    err: Option<SetInfo>,
    /// Where the plot is drawn against its data (stacked layers).
    place: Place,
}

/// The plots of a layer and the ranges of their data.
struct Plan<'a> {
    plots: Vec<Resolved<'a>>,
    ranges: (Option<(f64, f64)>, Option<(f64, f64)>),
    /// Number of categories when the first plot's X column holds texts.
    text_x: Option<usize>,
}

/// What a layer is in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// An ordinary layer; `has_right` when a right Y layer lies over it.
    Plain { has_right: bool },
    /// The right Y axis of a double-Y graph: linked to `parent`, drawn over its frame.
    RightY { parent: usize, x_straight: bool },
    /// A panel (or any other linked layer): linked to `parent`, with a frame of its own, and the X
    /// and Y scales that follow the parent's when `x` and `y` are set.
    Panel { parent: usize, x: bool, y: bool },
}

impl Role {
    /// The layer whose X scale this layer has, when it is linked straight.
    fn x_parent(&self) -> Option<usize> {
        match *self {
            Role::RightY { parent, x_straight: true } | Role::Panel { parent, x: true, .. } => Some(parent),
            _ => None,
        }
    }

    fn y_parent(&self) -> Option<usize> {
        match *self {
            Role::Panel { parent, y: true, .. } => Some(parent),
            _ => None,
        }
    }
}

/// The parent of layer `index` when it is the second layer of a double-Y graph, and whether its X
/// scale follows the parent's.
fn right_parent(graph: &Graph, index: usize) -> Option<(usize, bool)> {
    let link = graph.valid_link(index)?;
    let layer = &graph.layers[index];
    (layer.y_axis.side == AxisSide::Opposite
        && link.y == ScaleLink::None
        && layer.frame == Frame::WHOLE
        && graph.layers[link.parent].y_axis.side != AxisSide::Opposite)
        .then_some((link.parent, link.x == ScaleLink::Straight))
}

fn role_of(graph: &Graph, index: usize) -> Role {
    match right_parent(graph, index) {
        Some((parent, x_straight)) => Role::RightY { parent, x_straight },
        None if graph.link_parent(index).is_some() => {
            let link = graph.layers[index].link.expect("a valid link exists");
            Role::Panel { parent: link.parent, x: link.x == ScaleLink::Straight, y: link.y == ScaleLink::Straight }
        }
        None => Role::Plain {
            has_right: (0..graph.layers.len()).any(|j| right_parent(graph, j).is_some_and(|(p, _)| p == index)),
        },
    }
}

/// What `graph_layer` needs to know about the other layers.
struct LayerCtx<'a> {
    graph: &'a Graph,
    index: usize,
    roles: &'a [Role],
    plans: &'a [Plan<'a>],
}

impl<'a> LayerCtx<'a> {
    /// The X axis whose scale this layer has and the range of data it has to cover. Layers whose X
    /// scales are linked straight share the parent's axis and the data of all of them.
    fn shared_x(&self, layer: &'a Layer, own: Option<(f64, f64)>) -> (&'a Axis, Option<(f64, f64)>) {
        let root = self.roles[self.index].x_parent().unwrap_or(self.index);
        let (range, shared) = self.shared_range(root, Role::x_parent, |p| p.ranges.0);
        if root == self.index && !shared {
            return (&layer.x_axis, own);
        }
        (&self.graph.layers[root].x_axis, range)
    }

    /// The Y axis whose scale this layer has and the range of data it has to cover.
    fn shared_y(&self, layer: &'a Layer, own: Option<(f64, f64)>) -> (&'a Axis, Option<(f64, f64)>) {
        let root = self.roles[self.index].y_parent().unwrap_or(self.index);
        let (range, shared) = self.shared_range(root, Role::y_parent, |p| p.ranges.1);
        if root == self.index && !shared {
            return (&layer.y_axis, own);
        }
        (&self.graph.layers[root].y_axis, range)
    }

    /// The range that covers the data of layer `root` and of every layer whose scale follows it, and
    /// whether there is any such layer.
    fn shared_range(
        &self,
        root: usize,
        follows: fn(&Role) -> Option<usize>,
        range_of: fn(&Plan) -> Option<(f64, f64)>,
    ) -> (Option<(f64, f64)>, bool) {
        let mut range = None::<(f64, f64)>;
        let mut grow = |r: Option<(f64, f64)>| {
            if let Some((lo, hi)) = r {
                range = Some(range.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
            }
        };
        grow(range_of(&self.plans[root]));
        let mut shared = false;
        for (j, role) in self.roles.iter().enumerate() {
            if follows(role) == Some(root) {
                shared = true;
                grow(range_of(&self.plans[j]));
            }
        }
        (range, shared)
    }
}

impl Writer {
    /// Writes one graph window. `sets` gives the data-set position of every column of the project.
    pub(super) fn graph(
        &mut self,
        project: &Project,
        graph: &Graph,
        sets: &HashMap<ColumnId, SetInfo>,
    ) -> Result<(), IoError> {
        self.check_name("graph name", &graph.name, super::MAX_DATASET_NAME)?;
        if graph.layers.is_empty() {
            self.notes.graph_notes.push(format!("Graph '{}' has no layer and was not written.", graph.name));
            return Ok(());
        }
        let label = self.label_text(&graph.long_name, "graph long name")?;
        let name = graph.name.clone();
        let mut notes: Vec<String> = Vec::new();
        let (pw, ph) = (units(graph.page.width_pt).round(), units(graph.page.height_pt).round());
        if !(100.0..=MAX_PAGE_UNITS).contains(&pw) || !(100.0..=MAX_PAGE_UNITS).contains(&ph) {
            return fail(format!(
                "the page of graph '{name}' is {:.1} by {:.1} inches; Origin projects hold pages between 0.2 and 109 inches",
                graph.page.width_pt / 72.0,
                graph.page.height_pt / 72.0
            ));
        }
        let page = (pw as u16, ph as u16);
        if graph.page.background != Color::WHITE {
            notes.push("the page background colour was not written (Origin's page is white)".into());
        }
        if !graph.page.style.is_default() {
            notes.push("the page's border, layer icons, window title choice and comments were not written".into());
        }
        let spec = WindowSpec {
            name: &name,
            label: &label,
            ordinal: self.written_windows,
            page,
            time: self.time,
            serial: 3u8.wrapping_add(self.graphs as u8),
            place: self.place,
        };
        self.graphs += 1;
        self.windows.extend(framed(&window_header(&spec)));
        // what each layer is, and its plots, before any layer is written (a legend lists the plots of
        // other layers, and a linked layer's X scale covers its parent's data)
        let roles: Vec<Role> = (0..graph.layers.len()).map(|i| role_of(graph, i)).collect();
        let sets = Sets { map: sets };
        let plans: Vec<Plan> = graph.layers.iter().map(|l| self.plan(project, l, &sets, &mut notes)).collect();
        for (index, layer) in graph.layers.iter().enumerate() {
            unwritten_layer_features(graph, index, &roles, &mut notes);
            // A linked layer's frame is a part of its parent's frame; Origin gets the place on the page.
            let placed;
            let layer = match graph.link_parent(index) {
                Some(parent) => {
                    placed = Layer { frame: within(graph.layers[parent].frame, layer.frame), ..layer.clone() };
                    &placed
                }
                None => layer,
            };
            let ctx = LayerCtx { graph, index, roles: &roles, plans: &plans };
            let bytes = self.graph_layer(project, layer, page, &ctx, &mut notes);
            self.windows.extend(bytes);
        }
        // the end of the layer list
        self.windows.extend(framed(&[]));
        self.written_windows += 1;
        notes.sort();
        notes.dedup();
        for n in notes {
            self.notes.graph_notes.push(format!("Graph '{name}': {n}."));
        }
        Ok(())
    }

    fn next_serial(&mut self) -> u32 {
        self.object_serial += 1;
        self.object_serial
    }

    /// The plots of a layer that can be written, with where each is drawn, and the data ranges.
    fn plan<'a>(&self, project: &Project, layer: &'a Layer, sets: &Sets, notes: &mut Vec<String>) -> Plan<'a> {
        let (mut places, _) = places(project, layer);
        if let Stack::Waterfall { dx_percent, dy_percent } = layer.stack {
            waterfall_places(project, layer, dx_percent, dy_percent, &mut places, notes);
        }
        if layer.background.is_some() {
            notes.push("a layer background colour was not written (the companion file keeps it)".into());
        }
        if !layer.background_style.is_default() {
            notes.push("the gradient and border of a layer's background were not written".into());
        }
        if !layer.group_style.is_default() {
            notes.push("the Group tab of a layer (edit mode, increments, subgroups) was not written".into());
        }
        if !layer.display.is_default() {
            notes.push("the Display tab of a layer (data hidden, not clipped, grid on top) was not written".into());
        }
        if layer.xy_ratio.is_some() {
            notes.push(
                "a layer's axis lengths linked to the scale with an X:Y ratio (Size/Speed) were not written".into(),
            );
        }
        let mut plots: Vec<Resolved> = Vec::new();
        for (plot_index, p) in layer.plots.iter().enumerate() {
            if !p.visible {
                notes.push("a hidden plot was left out".into());
                continue;
            }
            let Some(y) = sets.of(p.y) else {
                notes.push("a plot whose data is not a worksheet column of the project was left out".into());
                continue;
            };
            let x = match p.x {
                Some(c) => match sets.of(c) {
                    Some(s) => s.position,
                    None => {
                        notes.push(
                            "a plot's X column is not in the project; it was plotted against the row number".into(),
                        );
                        0
                    }
                },
                None => 0,
            };
            let err = p.y_error.and_then(|e| {
                let s = sets.of(e);
                if s.is_none() {
                    notes.push("an error bar column is not in the project and was left out".into());
                }
                s
            });
            if p.rows.is_some() {
                notes.push("a plot's row range was not written; all rows of its columns are plotted".into());
            }
            if p.symbol.every > 1 && p.kind.supports_skip_points() {
                notes.push("Skip Points (symbols, drop lines, error bars and labels on every n-th point only) was not written; the companion file keeps it".into());
            }
            if p.line.sort_x
                && matches!(p.kind, sheaf_core::graph::PlotKind::Line | sheaf_core::graph::PlotKind::LineSymbol)
            {
                notes.push("Sort X for line connection (the line joins the points in increasing X order) was not written; the companion file keeps it".into());
            }
            if !p.decor.trendlines.is_empty() {
                notes.push("trendlines (fitted curves and moving averages with their equation and R-squared labels) were not written; the companion file keeps them".into());
            }
            let mut other_decor = p.decor.clone();
            other_decor.trendlines.clear();
            if !other_decor.is_default() {
                notes.push(
                    "drop lines, symbol/line gaps and draw order, fills under curves (with their pattern, transparency and gradient) and data labels were not written (the companion file keeps them)"
                        .into(),
                );
            }
            let drops = &p.decor.drop;
            if [&drops.horizontal, &drops.vertical].iter().any(|d| {
                d.on && (d.target != sheaf_core::graph::DropTarget::Axis
                    || d.color_mode != sheaf_core::graph::DropColorMode::Auto)
            }) {
                notes.push("drop lines to the next data plot and the Y Value: Plus-Minus colour (lollipop plots) were not written; the companion file keeps them".into());
            }
            let place = places[plot_index];
            plots.push(Resolved { plot: p, y, x, err, place });
        }
        let plain: Vec<(&Plot, Place)> = plots.iter().map(|r| (r.plot, r.place)).collect();
        let ranges = data_ranges(project, layer, &plain);
        // a text column as X gives categories: the plots use the row numbers and the axis shows the texts
        let text_x = plain
            .first()
            .and_then(|(p, _)| p.x)
            .and_then(|c| project.column(c))
            .and_then(|c| c.text().map(<[String]>::len));
        Plan { plots, ranges, text_x }
    }

    fn graph_layer(
        &mut self,
        _project: &Project,
        layer: &Layer,
        page: (u16, u16),
        ctx: &LayerCtx,
        notes: &mut Vec<String>,
    ) -> Vec<u8> {
        let plan = &ctx.plans[ctx.index];
        let plots = &plan.plots;
        let ranges = plan.ranges;
        let role = ctx.roles[ctx.index];
        let text_x = match role.x_parent() {
            Some(parent) => ctx.plans[parent].text_x,
            None => plan.text_x,
        };
        let graph = ctx.graph;

        // frame and axes
        let frame = sane_frame(layer.frame, notes);
        let left = unit_u16(frame.left * f64::from(page.0) / UNITS_PER_INCH * 72.0);
        let top = unit_u16(frame.top * f64::from(page.1) / UNITS_PER_INCH * 72.0);
        let right = unit_u16((frame.left + frame.width) * f64::from(page.0) / UNITS_PER_INCH * 72.0);
        let bottom = unit_u16((frame.top + frame.height) * f64::from(page.1) / UNITS_PER_INCH * 72.0);
        // The X scale of a layer that shares its X axis with others covers the data of all of them.
        let (x_axis, x_data) = ctx.shared_x(layer, ranges.0);
        let mut x_scale = axis_scale(x_axis, x_data, notes);
        if let Some(n) = text_x.filter(|_| x_axis.min.is_none() && x_axis.max.is_none()) {
            x_scale.from = 0.5;
            x_scale.to = n as f64 + 0.5;
            x_scale.step = 1.0;
        }
        let (y_axis, y_data) = ctx.shared_y(layer, ranges.1);
        let y_scale = axis_scale(y_axis, y_data, notes);
        let link = match role {
            Role::RightY { parent, .. } => Some(LinkSpec {
                parent: (parent + 1).min(255) as u8,
                index: ctx.index.min(255) as u8,
                percent_of_parent: false,
            }),
            Role::Panel { parent, .. } => Some(LinkSpec {
                parent: (parent + 1).min(255) as u8,
                index: ctx.index.min(255) as u8,
                percent_of_parent: true,
            }),
            Role::Plain { .. } => None,
        };
        let offsets = plots.iter().any(|r| r.place.oy != 0.0 || r.place.ox != 0.0);
        let x_offsets = plots.iter().any(|r| r.place.ox != 0.0);
        let mut out = framed(&layer_header(&LayerSpec {
            frame: [left, top, right, bottom],
            page,
            x: x_scale,
            y: y_scale,
            link,
            offsets,
            x_offsets,
        }));

        // annotations: titles, free text, legend
        let frame_pt = FramePt {
            left: frame.left * f64::from(page.0) / UNITS_PER_INCH * 72.0,
            top: frame.top * f64::from(page.1) / UNITS_PER_INCH * 72.0,
            width: frame.width * f64::from(page.0) / UNITS_PER_INCH * 72.0,
            height: frame.height * f64::from(page.1) / UNITS_PER_INCH * 72.0,
        };
        let mut annotations: Vec<Vec<u8>> = Vec::new();
        for (kind, axis) in [(AnnKind::XTitle, &layer.x_axis), (AnnKind::YTitle, &layer.y_axis)] {
            if !axis.show_title || axis.side == AxisSide::Hidden {
                continue;
            }
            let opposite = axis.side == AxisSide::Opposite;
            let auto = if kind == AnnKind::XTitle { "%(?X,@LG)" } else { "%(?Y,@LG)" };
            let text = axis.title.clone().unwrap_or_else(|| auto.to_string());
            let text = styled(&text, &axis.title_font);
            let size = axis.title_font.size_pt;
            let (w, h) = text_size(&text, size);
            let tick = axis.tick_font.size_pt;
            let (cx, cy, angle) = match (kind, opposite) {
                (AnnKind::XTitle, false) => {
                    (frame_pt.left + frame_pt.width / 2.0, frame_pt.top + frame_pt.height + tick * 1.6 + h, 0u16)
                }
                (AnnKind::XTitle, true) => (frame_pt.left + frame_pt.width / 2.0, frame_pt.top - tick * 1.6 - h, 0),
                (_, false) => (frame_pt.left - tick * 3.4 - h / 2.0, frame_pt.top + frame_pt.height / 2.0, 900),
                (_, true) => {
                    (frame_pt.left + frame_pt.width + tick * 3.4 + h / 2.0, frame_pt.top + frame_pt.height / 2.0, 900)
                }
            };
            let (rw, rh) = if angle == 0 { (w, h) } else { (h, w) };
            let name = match (kind == AnnKind::XTitle, opposite) {
                (true, false) => "XB",
                (true, true) => "XT",
                (false, false) => "YL",
                (false, true) => "YR",
            };
            annotations.push(self.annotation_bytes(
                kind,
                name,
                &text,
                (cx, cy),
                (rw, rh),
                (w, h),
                size,
                angle,
                &axis.title_font,
                &frame_pt,
                page,
                false,
                notes,
            ));
        }
        for (i, l) in layer.labels.iter().enumerate() {
            let text = styled(&l.text, &l.font);
            let (w, h) = text_size(&text, l.font.size_pt);
            let angle = (((l.rotation_deg % 360.0) + 360.0) % 360.0 * 10.0).round() as u16;
            let rotated = (450..1350).contains(&angle) || (2250..3150).contains(&angle);
            let (rw, rh) = if rotated { (h, w) } else { (w, h) };
            // Origin turns the text about the top-left corner of its unturned box, which is also the
            // top-left corner of the stored rectangle (the reader's rule, opj-format.md 10.4): the
            // label's anchor lies half a line height down the turned text from that corner.
            let (sin, cos) = l.rotation_deg.to_radians().sin_cos();
            let anchor = (frame_pt.left + l.x * frame_pt.width, frame_pt.top + l.y * frame_pt.height);
            let corner = (anchor.0 - h / 2.0 * sin, anchor.1 - h / 2.0 * cos);
            let (cx, cy) = (corner.0 + rw / 2.0, corner.1 + rh / 2.0);
            let name = if i == 0 { "Text".to_string() } else { format!("Text{i}") };
            annotations.push(self.annotation_bytes(
                AnnKind::Text,
                &name,
                &text,
                (cx, cy),
                (rw, rh),
                (w, h),
                l.font.size_pt,
                angle,
                &l.font,
                &frame_pt,
                page,
                false,
                notes,
            ));
        }
        // the plots a legend lists: its own layer's, or those of every layer drawn in its frame
        let mut entries: Vec<(String, &Plot)> = Vec::new();
        if layer.legend.scope == LegendScope::Graph && graph.same_frame(ctx.index).len() > 1 {
            for l in graph.same_frame(ctx.index) {
                for (k, r) in ctx.plans[l].plots.iter().enumerate() {
                    entries.push((format!("{}.{}", l + 1, k + 1), r.plot));
                }
            }
        } else {
            for (k, r) in plots.iter().enumerate() {
                entries.push(((k + 1).to_string(), r.plot));
            }
        }
        if layer.legend.visible && !entries.is_empty() {
            // The legend's own text when it has one (its `\l(n)` and `%(n)` count the plots the
            // legend lists, as here), otherwise a line per plot.
            let (lines, longest): (Vec<String>, usize) = match &layer.legend.text {
                Some(custom) => {
                    let lines: Vec<String> =
                        custom.replace("\r\n", "\n").replace('\r', "\n").split('\n').map(str::to_string).collect();
                    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1);
                    (lines, longest)
                }
                None => {
                    let mut lines = Vec::new();
                    for (id, plot) in &entries {
                        match &plot.label {
                            Some(t) => lines.push(format!("\\l({id}) {}", t.replace(['\r', '\n'], " "))),
                            None => lines.push(format!("\\l({id}) %({id})")),
                        }
                    }
                    let longest = entries
                        .iter()
                        .map(|(_, p)| p.label.as_ref().map_or(12, |t| t.chars().count()))
                        .max()
                        .unwrap_or(1);
                    (lines, longest)
                }
            };
            let text = lines.join("\r\n");
            let size = layer.legend.font.size_pt;
            let w = CHAR_WIDTH * size * longest as f64 + 2.6 * size;
            let h = LINE_HEIGHT * size * lines.len().max(1) as f64;
            let margin = 0.4 * size;
            let (cx, cy) = match layer.legend.position {
                LegendPosition::Automatic => {
                    // The rectangle is positioned for Origin; the companion retains the dynamic
                    // anchor for later Sheaf text/column changes rather than importing a centre.
                    notes.push("the automatic legend anchor was written as a fixed position".into());
                    let (x, y) = Legend::AUTOMATIC_ANCHOR;
                    (frame_pt.left + x * frame_pt.width + w / 2.0, frame_pt.top + y * frame_pt.height + h / 2.0)
                }
                LegendPosition::TopRight => {
                    (frame_pt.left + frame_pt.width - w / 2.0 - margin, frame_pt.top + h / 2.0 + margin)
                }
                LegendPosition::TopLeft => (frame_pt.left + w / 2.0 + margin, frame_pt.top + h / 2.0 + margin),
                LegendPosition::BottomRight => (
                    frame_pt.left + frame_pt.width - w / 2.0 - margin,
                    frame_pt.top + frame_pt.height - h / 2.0 - margin,
                ),
                LegendPosition::BottomLeft => {
                    (frame_pt.left + w / 2.0 + margin, frame_pt.top + frame_pt.height - h / 2.0 - margin)
                }
                LegendPosition::Manual { x, y } => {
                    (frame_pt.left + x * frame_pt.width, frame_pt.top + y * frame_pt.height)
                }
            };
            if layer.legend.background.is_some() {
                notes.push("the legend background colour was not written".into());
            }
            annotations.push(self.annotation_bytes(
                AnnKind::Legend,
                "Legend",
                &text,
                (cx, cy),
                (w, h),
                (w, h),
                size,
                0,
                &layer.legend.font,
                &frame_pt,
                page,
                layer.legend.frame,
                notes,
            ));
        }
        for a in annotations {
            out.extend(a);
        }
        if matches!(role, Role::RightY { x_straight: true, .. }) {
            // the X scale follows the parent's
            out.extend(link_annotation("X2"));
            out.extend(link_annotation("X1"));
        }
        if let Role::Panel { x, y, .. } = role {
            // scales that follow the parent's: an object named Y1 and one named X1
            if y {
                out.extend(link_annotation("Y1"));
            }
            if x {
                out.extend(link_annotation("X1"));
            }
        }
        out.extend(framed(&[]));

        // curves
        let mut first = true;
        let mut later_errors = Vec::new();
        let n_columns = plots.iter().filter(|r| r.plot.kind.is_bar()).count();
        // The plot groups of the layer: the plots with a group number, and all column plots that
        // stand side by side as one group. A group is a run of neighbouring curves; a run of one
        // is no group.
        let ids: Vec<Option<u32>> = plots
            .iter()
            .map(|r| r.plot.group.or((r.plot.kind.is_bar() && n_columns >= 2).then_some(u32::MAX)))
            .collect();
        let places: Vec<Group> = (0..ids.len())
            .map(|i| match ids[i] {
                Some(g) if i > 0 && ids[i - 1] == Some(g) => Group::Member,
                Some(g) if ids.get(i + 1) == Some(&Some(g)) => Group::First,
                _ => Group::None,
            })
            .collect();
        let grouped = places.iter().any(|g| *g != Group::None);
        if grouped {
            // Origin colours the members of a group from its own colour list (the first is black,
            // the second red, ...). Colours that come from the layer's palette were not chosen by
            // hand and are not named.
            let mut at = 0;
            let replaced = plots.iter().zip(&places).any(|(r, g)| {
                at = if *g == Group::First { 0 } else { at + 1 };
                let c = if r.plot.kind.is_bar() || r.plot.kind == PlotKind::Scatter {
                    r.plot.symbol.color
                } else {
                    r.plot.line.color
                };
                *g != Group::None && !r.plot.auto_color && palette_position(c) != Some((at % PALETTE.len()) as u8)
            });
            let bars_with_errors =
                plots.iter().zip(&places).any(|(r, g)| *g != Group::None && r.plot.kind.is_bar() && r.err.is_some());
            if bars_with_errors {
                notes.push(
                    "error bars on column plots that stand side by side were written, but Origin did not place them on their bars"
                        .into(),
                );
            }
            if replaced {
                notes.push(
                    "the colours of plots in a plot group were not written; Origin colours the plots of a group from its own colour list"
                        .into(),
                );
            }
        }
        // Plot types the writer cannot store go out as plain columns; the companion file keeps them.
        let has = |f: &dyn Fn(PlotKind) -> bool| plots.iter().any(|r| f(r.plot.kind));
        if has(&|k| k == PlotKind::Histogram || k == PlotKind::Box || is_pie(k)) {
            notes.push("histogram, box and pie plots were written as plain columns of their data".into());
        }
        if has(&|k| k.is_violin()) {
            notes.push("violin plots were written as plain columns of their data".into());
        }
        if has(&|k| k.is_hexbin()) {
            notes.push("hexbin plots were written as scatter plots of their data".into());
        }
        if has(&|k| k.is_parallel()) {
            notes.push(
                "parallel plots and parallel indexes were written as line plots of their columns; their axes, colours and groups were not written"
                    .into(),
            );
        }
        if has(&|k| k.is_ridgeline()) {
            notes.push("ridgeline charts were written as plain columns of their data".into());
        }
        if plots
            .iter()
            .any(|r| (r.plot.size_from.is_some() && layer.coords.ternary().is_none()) || r.plot.color_from.is_some())
        {
            notes.push("bubble sizes and colour mapped symbol colours were not written".into());
        }
        if has(&|k| k.is_stacked_column()) {
            notes.push("stacked columns and bars were written as columns side by side".into());
        }
        if has(&|k| k.is_stacked_area()) {
            notes.push("stacked areas were written as areas from zero".into());
        }
        if plots.iter().any(|r| r.plot.kind == PlotKind::Area && r.plot.y_base.is_some()) {
            notes.push("fill areas between two curves were written as areas down to zero".into());
        }
        if has(&|k| k.is_horizontal()) {
            notes.push("horizontal bars were written as vertical columns".into());
        }
        if has(&|k| matches!(k, PlotKind::FloatingColumn | PlotKind::FloatingBar)) {
            notes.push("floating columns were written as columns from zero".into());
        }
        if plots.iter().any(|r| r.plot.x_error.is_some() || !r.plot.error.is_default()) {
            notes.push("X error bars, error bar directions and cap widths were not written".into());
        }
        for (r, place) in plots.iter().zip(places.iter().copied()) {
            let p = r.plot;
            let kind = match p.kind {
                PlotKind::Line | PlotKind::Parallel | PlotKind::ParallelIndex => CurveKind::Line,
                PlotKind::Scatter | PlotKind::Hexbin => CurveKind::Scatter,
                PlotKind::LineSymbol => CurveKind::LineSymbol,
                PlotKind::Column
                | PlotKind::StackedColumn
                | PlotKind::Stacked100Column
                | PlotKind::Bar
                | PlotKind::FloatingColumn
                | PlotKind::StackedBar
                | PlotKind::Stacked100Bar
                | PlotKind::FloatingBar
                | PlotKind::Histogram
                | PlotKind::Box
                | PlotKind::Violin
                | PlotKind::ViolinBox
                | PlotKind::ViolinPoint
                | PlotKind::ViolinQuartile
                | PlotKind::ViolinStick
                | PlotKind::HalfViolin
                | PlotKind::Ridgeline
                | PlotKind::Pie
                | PlotKind::Pie3D
                | PlotKind::PieBw => CurveKind::Column,
                PlotKind::Area | PlotKind::StackedArea | PlotKind::Stacked100Area => CurveKind::Area,
            };
            let connect = match p.line.connect {
                Connect::Straight => 1,
                Connect::Spline => 5,
                Connect::BSpline => 8,
                Connect::Bezier => 15,
                Connect::StepHorizontal => 11,
                Connect::StepVertical => 12,
            };
            let line_color = if kind == CurveKind::Scatter { p.symbol.color } else { p.line.color };
            let spec = CurveSpec {
                kind,
                y_set: r.y.position,
                used: (r.y.first, r.y.last),
                x_set: r.x,
                first,
                line: code_of(line_color),
                width: (p.line.width_pt * 500.0).round().clamp(1.0, f64::from(u16::MAX)) as u16,
                dash: Dash::ALL.iter().position(|d| *d == p.line.dash).unwrap_or(0) as u8,
                connect,
                shape: symbol_code(p.symbol.shape),
                size: (p.symbol.size_pt * 500.0).round().clamp(1.0, f64::from(u16::MAX)) as u16,
                open_symbol: !p.symbol.filled,
                symbol: code_of(p.symbol.color),
                fill: code_of(p.symbol.color),
                bar_width_pct: 100.0 - layer.bars.gap_pct,
                offset: (r.place.ox, r.place.oy),
                group: place,
                serial: self.next_serial(),
            };
            first = false;
            out.extend(framed(&curve_record(&spec)));
            out.extend(framed(&[]));
            if grouped {
                // the members of a plot group follow one another; the error plots come after them
                later_errors.push(r);
            } else if let Some(err) = r.err {
                out.extend(self.error_record(r, err));
            }
        }
        for r in later_errors {
            if let Some(err) = r.err {
                out.extend(self.error_record(r, err));
            }
        }
        // the end of the curves and the extra block
        out.extend(framed(&[]));
        out.extend(framed(&[]));

        // the three groups of axis blocks
        for (group, axis) in [(0, &layer.x_axis), (1, &layer.y_axis)] {
            let mut spec = match (role, group) {
                // a right Y layer shows the parent's top line, and nothing else of the X axis
                (Role::RightY { parent, .. }, 0) => {
                    let parent_x = &graph.layers[parent].x_axis;
                    let mut s = axis_spec(parent_x, &mut Vec::new());
                    s.second_edge = true;
                    s.labels_shown = false;
                    s.line_shown = parent_x.show_opposite && parent_x.side == AxisSide::Normal;
                    s.far_line = false;
                    s
                }
                _ => axis_spec(axis, notes),
            };
            if role == (Role::Plain { has_right: true }) {
                // the far edges belong to the right Y layer
                spec.far_line = false;
            }
            spec.text_labels = group == 0 && text_x.is_some() && !matches!(role, Role::RightY { .. });
            let blocks = axis_blocks(group, &spec);
            for b in blocks {
                out.extend(framed(&b));
            }
            out.extend(framed(&[]));
        }
        for b in axis_blocks(2, &axis_spec(&layer.x_axis, &mut Vec::new())) {
            out.extend(framed(&b));
        }
        out.extend(framed(&[]));
        out
    }

    /// The record and the empty block of the Y error plot `err` that belongs to the curve of `r`.
    fn error_record(&mut self, r: &Resolved, err: SetInfo) -> Vec<u8> {
        let spec = CurveSpec {
            kind: CurveKind::Error,
            y_set: err.position,
            used: (err.first, err.last),
            x_set: r.y.position,
            first: false,
            line: Code::index(0),
            width: 0,
            dash: 0,
            connect: 0,
            shape: 0,
            size: 0,
            open_symbol: false,
            symbol: Code::AUTO,
            fill: Code::AUTO,
            bar_width_pct: 0.0,
            offset: (0.0, 0.0),
            group: Group::None,
            serial: self.next_serial(),
        };
        let mut out = framed(&curve_record(&spec));
        out.extend(framed(&[]));
        out
    }

    /// One annotation: the centre of its rectangle is `centre` (points on the page), `size` the
    /// rectangle's width and height, `text_extent` the unrotated size of the text.
    #[allow(clippy::too_many_arguments)]
    fn annotation_bytes(
        &mut self,
        kind: AnnKind,
        name: &str,
        text: &str,
        centre: (f64, f64),
        size: (f64, f64),
        text_extent: (f64, f64),
        size_pt: f64,
        angle: u16,
        font: &Font,
        frame: &FramePt,
        page: (u16, u16),
        legend_frame: bool,
        notes: &mut Vec<String>,
    ) -> Vec<u8> {
        let (bytes, lossy) = encode_cp1252(text);
        if lossy {
            self.notes.lossy_texts += 1;
        }
        if font.family != sheaf_core::template::DEFAULT_FONT {
            notes.push(format!("the font '{}' of a text was not written; Origin's default font is used", font.family));
        }
        let rect = [
            unit_u16(centre.0 - size.0 / 2.0).min(page.0),
            unit_u16(centre.1 - size.1 / 2.0).min(page.1),
            unit_u16(centre.0 + size.0 / 2.0).min(page.0),
            unit_u16(centre.1 + size.1 / 2.0).min(page.1),
        ];
        let relative = (
            if frame.width > 0.0 { (centre.0 - size.0 / 2.0 - frame.left) / frame.width } else { 0.0 },
            if frame.height > 0.0 { (centre.1 - size.1 / 2.0 - frame.top) / frame.height } else { 0.0 },
        );
        let color = if palette_position(font.color).is_some() {
            nearest_position(font.color)
        } else {
            notes.push(
                "a text colour that is not in Origin's colour list was written as the nearest listed colour".into(),
            );
            nearest_position(font.color)
        };
        let spec = AnnSpec {
            kind,
            name,
            text: &bytes,
            rect,
            relative,
            size_pt: size_pt.round().clamp(1.0, 255.0) as u16,
            angle,
            color,
            extent: (unit_u16(text_extent.0), unit_u16(text_extent.1)),
            frame: legend_frame,
            serial: self.next_serial().min(u32::from(u16::MAX)) as u16,
        };
        annotation(&spec)
    }
}

struct FramePt {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

/// Text with the bold and italic of its font as Origin's escapes.
fn styled(text: &str, font: &Font) -> String {
    let mut t = text.to_string();
    if font.italic {
        t = format!("\\i({t})");
    }
    if font.bold {
        t = format!("\\b({t})");
    }
    t
}

/// Width and height in points of text that the reader would measure the same way.
fn text_size(text: &str, size_pt: f64) -> (f64, f64) {
    let plain = crate::opj::graph::strip_escapes(text);
    let lines = plain.split(['\r', '\n']).filter(|l| !l.is_empty()).count().max(1);
    let longest = plain.split(['\r', '\n']).map(|l| l.chars().count()).max().unwrap_or(0);
    (CHAR_WIDTH * size_pt * longest as f64, LINE_HEIGHT * size_pt * lines as f64)
}

fn symbol_code(s: SymbolShape) -> u8 {
    match s {
        SymbolShape::Square => 1,
        SymbolShape::Circle => 2,
        SymbolShape::TriangleUp => 3,
        SymbolShape::TriangleDown => 4,
        SymbolShape::Diamond => 5,
        SymbolShape::Plus => 6,
        SymbolShape::Cross => 7,
        SymbolShape::Star => 8,
    }
}

/// Smallest and largest value of the data of the plots, X and Y.
fn data_ranges(project: &Project, layer: &Layer, plots: &[(&Plot, Place)]) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
    let mut x: Option<(f64, f64)> = None;
    let mut y: Option<(f64, f64)> = None;
    let grow = |r: &mut Option<(f64, f64)>, v: f64| {
        if v.is_finite() {
            *r = Some(r.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        }
    };
    // Data that the scale cannot place (not positive on a log axis, outside 0..100 on a percent axis) are left out.
    let (sx, sy) = (layer.x_axis.scale, layer.y_axis.scale);
    let log_y = sy != ScaleKind::Linear;
    for (p, place) in plots {
        let (ox, oy) = (place.ox, place.oy);
        let ys = project.numeric(p.y).unwrap_or(&[]);
        let errs = p.y_error.and_then(|e| project.numeric(e));
        for (i, v) in ys.iter().enumerate() {
            if !v.is_finite() || !sy.accepts(*v) {
                continue;
            }
            let e = errs.and_then(|e| e.get(i)).copied().filter(|e| e.is_finite()).unwrap_or(0.0).abs();
            if sy.accepts(*v + oy) {
                grow(&mut y, *v + oy);
            }
            if !log_y {
                grow(&mut y, v + oy - e);
                grow(&mut y, v + oy + e);
            }
        }
        if p.kind.is_bar() {
            grow(&mut y, oy);
        }
        match p.x.and_then(|c| project.numeric(c)) {
            Some(xs) => {
                for v in xs {
                    if sx.accepts(*v) {
                        grow(&mut x, *v + ox);
                    }
                }
            }
            None => {
                grow(&mut x, 1.0 + ox);
                grow(&mut x, ys.len().max(1) as f64 + ox);
            }
        }
    }
    (x, y)
}

/// A frame that lies inside the page. Origin loads a frame with no width, a negative corner or a
/// size beyond the page but draws a blank picture, so such a frame is replaced and named in a warning.
/// The page place of a frame given as fractions of `parent` (a page frame).
fn within(parent: sheaf_core::graph::Frame, f: sheaf_core::graph::Frame) -> sheaf_core::graph::Frame {
    sheaf_core::graph::Frame {
        left: parent.left + f.left * parent.width,
        top: parent.top + f.top * parent.height,
        width: f.width * parent.width,
        height: f.height * parent.height,
    }
}

/// Notes for the parts of layer `index` that the writer does not store (spec section 13.12): a layer
/// linked in any way other than as the right Y axis of a double-Y graph, waterfall offsets, the
/// overlap of bars, and some axis details. Each note makes the save keep a companion file with the
/// whole project.
fn unwritten_layer_features(graph: &Graph, index: usize, roles: &[Role], notes: &mut Vec<String>) {
    let layer = &graph.layers[index];
    if !layer.shapes.is_empty() {
        let count = |k| layer.shapes.iter().filter(|s| s.kind == k).count();
        let names: Vec<String> = sheaf_core::shape::ShapeKind::ALL
            .iter()
            .filter(|k| count(**k) > 0)
            .map(|k| {
                let noun =
                    if *k == sheaf_core::shape::ShapeKind::Ellipse { "ellipse" } else { &k.name().to_lowercase() };
                format!("{} {noun}", count(*k))
            })
            .collect();
        notes.push(format!(
            "layer {} has drawn objects ({}) that an Origin project file written here cannot hold; they were not written (the .opj.sheaf companion keeps them)",
            index + 1,
            names.join(", ")
        ));
    }
    if matches!(roles[index], Role::RightY { x_straight: false, .. }) {
        notes.push(
            "the X scale of a right Y layer that is not linked to its parent's was written with the link only".into(),
        );
    }
    if matches!(layer.stack, Stack::Waterfall { .. }) {
        notes.push(
            "a waterfall was written as curves moved by X and Y offsets on axes that cover them (Origin's own waterfall plot, with its Z axis, was not written)"
                .into(),
        );
    }
    if layer.plot_offsets.iter().any(|o| o.x_multiplier != 1.0 || o.y_multiplier != 1.0) {
        notes.push("plot X/Y multipliers were not written; Origin shows the source values with offsets only (the .opj.sheaf companion keeps the multipliers)".into());
    }
    if layer.coords.polar().is_some_and(|p| p.radar) {
        notes.push(
            "the radar layer was not written; its plots went out as plain line + symbol plots on rectangular axes"
                .into(),
        );
    } else if layer.coords.polar().is_some() {
        notes
            .push("the polar layer was not written; its plots went out as plain line plots on rectangular axes".into());
    }
    if layer.coords.ternary().is_some() {
        notes.push(
            "the ternary layer was not written; its plots went out as plain scatter plots on rectangular axes \
             (the Z column is not kept)"
                .into(),
        );
    }
    if layer.bars.overlap_pct != 0.0 {
        notes.push("the overlap of column bars was not written; bars stand side by side".into());
    }
    if layer.bars.width_pct.is_some() {
        notes.push("the width of column bars (in percent) was not written; the bars reopen with the layer's gap setting instead".into());
    }
    if layer.legend.visible && layer.legend.columns > 1 {
        notes.push("the number of columns of the legend was not written; its entries stand in one column".into());
    }
    for axis in [&layer.x_axis, &layer.y_axis] {
        if axis.tick_label_source != TickLabelSource::Auto {
            notes.push("the choice of tick labels (numbers, or the cells of a column) was not written".into());
        }
        if axis.tick_label_angle_deg != 0.0 {
            notes.push("rotated tick labels were not written".into());
        }
        if axis.hide_end_label {
            notes.push("a hidden tick label at the end of an axis was not written".into());
        }
        if axis.reverse {
            notes.push("a reversed axis was written in the normal direction".into());
        }
        if !axis.axis_breaks.is_empty() {
            notes.push("an axis break was not written".into());
        }
        if !axis.rug.is_default() || !axis.rug_opposite.is_default() {
            notes.push("an axis rug (Rug tab, Scatter + Rug) was not written; the companion file keeps it".into());
        }
        if axis.additional_line.is_some() {
            notes.push("an additional line at a value of the other axis (Grids tab) was not written".into());
        }
        if !axis.special_ticks.is_empty() || !axis.tick_label_formula.is_empty() || axis.minor_tick_labels {
            notes.push("special ticks, tick label formulas and minor tick labels were not written".into());
        }
        if axis.tick_color.is_some() || axis.tick_width_pt.is_some() {
            notes.push("the colour and thickness of ticks (other than the axis line) were not written".into());
        }
        if axis.first_tick.is_some() {
            notes.push("the first tick (anchor) of an axis was not written".into());
        }
        if !axis.tick_prefix.is_empty() || !axis.tick_suffix.is_empty() {
            notes.push("the prefix and suffix of tick labels were not written".into());
        }
        if axis.title_color_follows_plot {
            notes.push("axis title colours that follow the first plot's colour were not written".into());
        }
    }
}

/// A waterfall as offsets: Sheaf draws member k moved right and up by k times the given percentages
/// of the front frame, which spans the axes of the unmoved data. Origin has no such shift, so the
/// same positions are written as per-curve offsets in axis units (graphing spec 13.6); the axis
/// range written then covers the moved curves.
fn waterfall_places(
    project: &Project,
    layer: &Layer,
    dx_percent: f64,
    dy_percent: f64,
    places: &mut [Place],
    notes: &mut Vec<String>,
) {
    if layer.x_axis.scale != ScaleKind::Linear || layer.y_axis.scale != ScaleKind::Linear {
        notes.push("the shifts of a waterfall on a logarithmic axis were not written".into());
        return;
    }
    let plain: Vec<(&Plot, Place)> = layer.plots.iter().map(|p| (p, Place::default())).collect();
    let (x, y) = data_ranges(project, layer, &plain);
    let span = |axis: &Axis, data: Option<(f64, f64)>| {
        let s = axis_scale(axis, data, &mut Vec::new());
        s.to - s.from
    };
    let (sx, sy) = (dx_percent / 100.0 * span(&layer.x_axis, x), dy_percent / 100.0 * span(&layer.y_axis, y));
    if !(sx.is_finite() && sy.is_finite()) {
        return;
    }
    let shifted: Vec<Place> = places
        .iter()
        .map(|pl| pl.member.map_or(*pl, |k| Place { ox: k as f64 * sx, oy: k as f64 * sy, ..*pl }))
        .collect();
    places.copy_from_slice(&shifted);
}

fn sane_frame(f: sheaf_core::graph::Frame, notes: &mut Vec<String>) -> sheaf_core::graph::Frame {
    use sheaf_core::graph::Frame;
    let ok = [f.left, f.top, f.width, f.height].iter().all(|v| v.is_finite())
        && f.left >= 0.0
        && f.top >= 0.0
        && f.width >= 0.01
        && f.height >= 0.01
        && f.left + f.width <= 1.0 + 1e-9
        && f.top + f.height <= 1.0 + 1e-9;
    if ok {
        return f;
    }
    notes.push("a layer frame that does not lie inside the page was replaced by Origin's default frame".into());
    Frame { left: 0.15, top: 0.12, width: 0.78, height: 0.76 }
}

/// The limits that go in the layer header: the fixed ones, and for an automatic limit one that
/// the data fit in.
fn axis_scale(axis: &Axis, data: Option<(f64, f64)>, notes: &mut Vec<String>) -> AxisScale {
    let log = axis.scale == ScaleKind::Log10;
    let other = !matches!(axis.scale, ScaleKind::Linear | ScaleKind::Log10);
    let (dlo, dhi) = data.unwrap_or(if log { (1.0, 10.0) } else { (0.0, 1.0) });
    let (alo, ahi, astep) = match sheaf_core::scales::auto_range(axis.scale, dlo, dhi, axis.rescale_margin_pct) {
        Some(r) if axis.scale.accepts(dlo) && axis.scale.accepts(dhi) => (r.from, r.to, r.inc),
        Some(_) => {
            let (a, b) = if axis.scale.is_percent() { (10.0, 90.0) } else { (1.0, 10.0) };
            sheaf_core::scales::auto_range(axis.scale, a, b, axis.rescale_margin_pct)
                .map_or((a, b, 1.0), |r| (r.from, r.to, r.inc))
        }
        None => nice_range(dlo, dhi, log, axis.auto_range, axis.rescale_margin_pct),
    };
    let (mut from, mut to) = (axis.min.unwrap_or(alo), axis.max.unwrap_or(ahi));
    if (log || other) && !(axis.scale.accepts(from) && axis.scale.accepts(to)) {
        from = alo;
        to = ahi;
    }
    if to.partial_cmp(&from) != Some(std::cmp::Ordering::Greater) {
        (from, to) = (alo, ahi);
    }
    let step = if log {
        1.0
    } else if other {
        axis.major_step.filter(|s| *s > 0.0).unwrap_or(astep)
    } else {
        axis.major_step.filter(|s| *s > 0.0).unwrap_or(astep)
    };
    if axis.auto_range != AutoRange::Nice && (axis.min.is_none() || axis.max.is_none()) {
        notes.push(
            "an axis range that follows the data in another way than Origin's own (tight or symmetric about zero) was written as the limits it gives for the current data"
                .into(),
        );
    }
    if axis.auto_range == AutoRange::Nice
        && (axis.min.is_none() || axis.max.is_none())
        && (axis.rescale_margin_pct - autorange::DEFAULT_RESCALE_MARGIN_PCT).abs() > 1e-9
    {
        notes.push(
            "the Rescale Margin of an axis is not stored in the project file; the axis range was written as the limits it gives for the current data"
                .into(),
        );
    }
    AxisScale {
        from,
        to,
        step,
        log,
        code: axis.scale.file_code(),
        minor: if log { axis.minor_ticks.clamp(0, 9) as u8 } else { axis.minor_ticks.min(100) as u8 },
        auto: axis.min.is_none() && axis.max.is_none() && axis.auto_range == AutoRange::Nice,
    }
}

fn tick_code(t: TickDirection) -> u8 {
    match t {
        TickDirection::None => 0,
        TickDirection::In => 1,
        TickDirection::Out => 2,
        TickDirection::Both => 3,
    }
}

fn axis_spec(axis: &Axis, notes: &mut Vec<String>) -> AxisBlocks {
    if axis.side == AxisSide::Opposite && axis.show_opposite {
        notes.push("the line on the near side of an axis that is drawn on the opposite side was not written".into());
    }
    let (mut scientific, mut decimals, mut date_format) = (false, None, None);
    match axis.tick_format {
        TickFormat::Auto => {}
        TickFormat::Decimal(d) => decimals = Some(d.min(15)),
        TickFormat::Scientific(d) => (scientific, decimals) = (true, Some(d.min(15))),
        TickFormat::DateTime(pattern) => {
            // Origin offers a fixed list of date formats; only month/day/year is in it
            date_format = Some(0);
            if pattern != sheaf_core::datetime::DatePattern::Mdy {
                notes.push(format!(
                    "the date format {} of tick labels has no counterpart in Origin's list; month/day/year was written",
                    pattern.label()
                ));
            }
        }
    }
    if (axis.tick_label_divisor - 1.0).abs() > f64::EPSILON {
        notes.push("a tick label divisor was not written".into());
    }
    // The major tick length is written (opj-format.md 10.3); the minor one only as Origin's Auto,
    // half the major, since where Origin keeps a chosen minor length was not found.
    if (axis.minor_tick_len_pt - axis.major_tick_len_pt / 2.0).abs() > 1e-9 {
        notes.push(
            "the minor tick length was not written; Origin draws minor ticks half as long as the major ones".into(),
        );
    }
    if axis.opposite_ticks && !axis.show_opposite {
        notes.push("ticks on the opposite side without its axis line were not written".into());
    }
    if axis.tick_font.bold || axis.tick_font.italic || axis.tick_font.family != sheaf_core::template::DEFAULT_FONT {
        notes.push("the font family, bold and italic of tick labels were not written".into());
    }
    if axis.major_grid.is_some() || axis.minor_grid.is_some() {
        notes.push(
            "grid lines are written with Origin's default grid style (colour, width and dash were not written)".into(),
        );
    }
    let hidden = axis.side == AxisSide::Hidden;
    AxisBlocks {
        label_pt: axis.tick_font.size_pt.round().clamp(1.0, 255.0) as u8,
        labels_shown: axis.show_tick_labels && !hidden,
        line_shown: !hidden,
        second_edge: axis.side == AxisSide::Opposite,
        line_width: (axis.line_width_pt * 500.0).round().clamp(1.0, f64::from(u16::MAX)) as u16,
        major_ticks: tick_code(axis.major_tick),
        // tenths of a point; Origin's dialog offers 3 to 15 pt, the block holds a u16
        major_len: (axis.major_tick_len_pt * 10.0).round().clamp(1.0, 1000.0) as u16,
        minor_ticks: tick_code(axis.minor_tick),
        far_line: axis.show_opposite && axis.side == AxisSide::Normal,
        grid_major: axis.major_grid.is_some(),
        grid_minor: axis.minor_grid.is_some(),
        line_color: code_of(axis.color),
        label_color: code_of(axis.tick_font.color),
        follows_plot: axis.color_follows_plot,
        text_labels: false,
        scientific,
        decimals,
        date_format,
    }
}

/// The three pie kinds (flat colour, 3D colour, black and white), which the writer cannot store.
fn is_pie(k: PlotKind) -> bool {
    matches!(k, PlotKind::Pie | PlotKind::Pie3D | PlotKind::PieBw)
}
