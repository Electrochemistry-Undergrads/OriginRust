//! Graph windows of an `.opj` file turned into [`sheaf_core::graph::Graph`] (spec section 10).
//!
//! What is read: the page size, for each layer the frame, the X and Y axis ranges, scale type,
//! tick settings and tick label size, the axis titles and free text labels, whether a legend
//! exists, and for each plotted curve the worksheet columns, the plot type, colours, line width
//! and style and the symbol shape and size. Anything that cannot be read gets a default and a
//! warning. Nothing here panics on short or odd records.

use super::strings::{c_bytes, decode_cp1252};
use super::window::{WinAnnotation, WinColumn, WinLayer, Window};
use sheaf_core::color::Color;
use sheaf_core::graph::*;
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::template::Template;
use std::collections::HashMap;

/// Page sizes in the window header are in 1/600 inch.
const PAGE_UNITS_PER_INCH: f64 = 600.0;
/// Line widths and symbol sizes in curve records are in 1/500 point.
const CURVE_UNITS_PER_POINT: f64 = 500.0;

// Window header.
const PAGE_WIDTH_AT: usize = 0x23;
const PAGE_HEIGHT_AT: usize = 0x25;
// Layer header.
const LAYER_KIND_AT: usize = 0x02;
const LAYER_GRAPH_BIT: u8 = 0x01;
const X_MIN_AT: usize = 0x0F;
const X_MAX_AT: usize = 0x17;
const X_STEP_AT: usize = 0x1F;
const X_MINOR_AT: usize = 0x37;
const X_SCALE_AT: usize = 0x38;
const Y_MIN_AT: usize = 0x3A;
const Y_MAX_AT: usize = 0x42;
const Y_STEP_AT: usize = 0x4A;
const Y_MINOR_AT: usize = 0x62;
const Y_SCALE_AT: usize = 0x63;
const RECT_AT: usize = 0x71;
// Axis blocks (groups x, y, z of six blocks each).
const AXIS_LABELS: usize = 2;
const AXIS_LINE: usize = 3;
const AXIS_OPPOSITE_LABELS: usize = 4;
const AXIS_OPPOSITE: usize = 5;
/// Colour of the axis line in block 3: `F7 FF` means "automatic" (follows the plot).
const AXIS_COLOR_AT: usize = 0x0F;
const AXIS_AUTO_COLOR: u16 = 0xFFF7;
/// Bit of byte [`AXIS_FLAGS_AT`] in blocks 2 to 5 that shows the part (graphing spec 12.7).
const AXIS_SHOWN_BIT: u8 = 0x40;
const AXIS_FONT_AT: usize = 0x15;
const AXIS_FLAGS_AT: usize = 0x26;
const AXIS_FORMAT_AT: usize = 0x25;
/// Label type "date" in the low nibble of the format byte.
const AXIS_DATE_LABELS: u8 = 3;
const AXIS_FIXED_DECIMALS_BIT: u8 = 0x80;
const AXIS_LINE_WIDTH_AT: usize = 0x15;
const AXIS_TICKS_AT: usize = 0x25;
/// u16 in the axis line block: the major tick length in tenths of a point (opj-format.md 10.3).
const AXIS_MAJOR_LEN_AT: usize = 0x4A;
// Curve record.
const CURVE_Y_AT: usize = 0x04;
/// Plot group bits of curve byte `0x06`: set on the first plot of a group and on the others.
const CURVE_GROUP_AT: usize = 0x06;
const CURVE_GROUP_FIRST_BIT: u8 = 0x20;
const CURVE_GROUP_MEMBER_BIT: u8 = 0x10;
const CURVE_CONNECT_AT: usize = 0x11;
const CURVE_DASH_AT: usize = 0x12;
const CURVE_WIDTH_AT: usize = 0x15;
const CURVE_SHAPE_AT: usize = 0x17;
const CURVE_SIZE_AT: usize = 0x19;
/// In a column plot the size field holds the bar width in percent of the X spacing, times 50.
const CURVE_BAR_WIDTH_PER_PERCENT: f64 = 50.0;
/// Origin's symbol interior number: 0 solid, 1 open, 2 to 11 the others (graphing 5.4, T323).
const CURVE_SYMBOL_FILL_AT: usize = 0x18;
const CURVE_X_AT: usize = 0x23;
const CURVE_TYPE_AT: usize = 0x4C;
const CURVE_FILL_AT: usize = 0xC2;
/// The colour of the symbols (and the outline of column bars): a list position, a free colour or
/// "automatic" (spec 10.5).
const CURVE_LINE_COLOR_AT: usize = 0x132;
/// The colour of the line.
const CURVE_PLOT_LINE_COLOR_AT: usize = 0x16A;
const CURVE_FILL_USED_AT: usize = 0x1E;
const CURVE_FILL_USED_BIT: u8 = 0x10;
/// Offsets of a curve in a stacked layer, in axis units (graphing spec 13.10).
const CURVE_X_OFFSET_AT: usize = 0x1AA;
const CURVE_Y_OFFSET_AT: usize = 0x1CA;
/// Origin's "no value" double.
const NO_VALUE: f64 = -1.23456789e-300;
// Annotation: header holds the rectangle and the name, block 1 the text style, block 2 the text.
const ANN_RECT_AT: usize = 0x03;
const ANN_NAME_AT: usize = 0x46;
/// The layer header byte that holds the 1-based number of the layer this one is linked to.
const LAYER_LINK_AT: usize = 0xAB;
const ANN_FONT_AT: usize = 0x04;
const ANN_ANGLE_AT: usize = 0x02;

/// Origin's colour list: the number stored in a curve record is the position in this list.
pub(crate) const PALETTE: [(u8, u8, u8); 24] = [
    (0, 0, 0),
    (255, 0, 0),
    (0, 255, 0),
    (0, 0, 255),
    (0, 255, 255),
    (255, 0, 255),
    (255, 255, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (128, 0, 0),
    (0, 128, 0),
    (0, 128, 128),
    (0, 0, 160),
    (255, 128, 0),
    (128, 0, 255),
    (255, 0, 128),
    (255, 255, 255),
    (192, 192, 192),
    (128, 128, 128),
    (255, 255, 128),
    (128, 255, 255),
    (255, 128, 255),
    (64, 64, 64),
];

fn palette_color(i: usize) -> Color {
    let (r, g, b) = PALETTE[i % PALETTE.len()];
    Color::rgb(r, g, b)
}

fn u8_at(b: &[u8], at: usize) -> u8 {
    b.get(at).copied().unwrap_or(0)
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    match b.get(at..at + 2) {
        Some(s) => u16::from_le_bytes([s[0], s[1]]),
        None => 0,
    }
}

fn f64_at(b: &[u8], at: usize) -> f64 {
    match b.get(at..at + 8) {
        Some(s) => f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]),
        None => f64::NAN,
    }
}

/// True for a window that holds a graph (spec section 10.0): its first layer header has the
/// graph-layer bit set and the layer carries axis blocks. Origin 8.x and older projects write
/// axis blocks for workbook and matrix layers too, so the blocks alone do not decide; the bit
/// is clear in every workbook and matrix layer seen.
pub(crate) fn is_graph(w: &Window) -> bool {
    w.layers.first().is_some_and(|l| {
        l.header.get(LAYER_KIND_AT).is_some_and(|b| b & LAYER_GRAPH_BIT != 0)
            && l.axes.first().is_some_and(|g| g.len() >= 6)
    })
}

/// Reads one graph window. `column_of_set` maps the 0-based position of a data set to the column
/// that was imported from it.
pub(crate) fn build(w: &Window, column_of_set: &HashMap<usize, ColumnId>, warnings: &mut Vec<String>) -> Graph {
    let mut notes: Vec<String> = Vec::new();
    let mut graph = Template::Line.new_graph(WindowId(0), &w.name);
    graph.long_name = w.label.clone();
    let (pw, ph) = (f64::from(u16_at(&w.header, PAGE_WIDTH_AT)), f64::from(u16_at(&w.header, PAGE_HEIGHT_AT)));
    if pw >= 100.0 && ph >= 100.0 {
        graph.page = Page {
            width_pt: pw / PAGE_UNITS_PER_INCH * 72.0,
            height_pt: ph / PAGE_UNITS_PER_INCH * 72.0,
            background: Color::WHITE,
            style: Default::default(),
        };
    } else {
        notes.push("the page size could not be read; a default size was used".into());
    }
    let template = graph.layers.remove(0);
    for (k, layer) in w.layers.iter().enumerate() {
        graph.layers.push(build_layer(layer, k == 0, &template, &graph.page, column_of_set, &mut notes));
    }
    link_right_y_layers(&mut graph, w);
    link_panels(&mut graph, w);
    if graph.layers.is_empty() {
        graph.layers.push(template);
        notes.push("it has no layers".into());
    }
    notes.sort();
    notes.dedup();
    for n in notes {
        warnings.push(format!("Graph '{}': {n}.", w.name));
    }
    graph
}

fn build_layer(
    layer: &WinLayer,
    first: bool,
    template: &Layer,
    page: &Page,
    column_of_set: &HashMap<usize, ColumnId>,
    notes: &mut Vec<String>,
) -> Layer {
    let mut out = template.clone();
    out.palette =
        Palette { colors: (0..PALETTE.len()).map(palette_color).collect(), mode: PaletteMode::Cycle, list: None };
    let h = &layer.header;

    // Frame.
    let (l, t, r, b) = (
        f64::from(u16_at(h, RECT_AT)),
        f64::from(u16_at(h, RECT_AT + 2)),
        f64::from(u16_at(h, RECT_AT + 4)),
        f64::from(u16_at(h, RECT_AT + 6)),
    );
    let page_units = (page.width_pt / 72.0 * PAGE_UNITS_PER_INCH, page.height_pt / 72.0 * PAGE_UNITS_PER_INCH);
    if r > l && b > t && r <= page_units.0 * 1.5 && b <= page_units.1 * 1.5 {
        out.frame = Frame {
            left: l / page_units.0,
            top: t / page_units.1,
            width: (r - l) / page_units.0,
            height: (b - t) / page_units.1,
        };
    } else {
        notes.push("a layer position could not be read; the default position was used".into());
    }

    // Axes.
    let spec = [
        (X_MIN_AT, X_MAX_AT, X_STEP_AT, X_MINOR_AT, X_SCALE_AT, 0usize),
        (Y_MIN_AT, Y_MAX_AT, Y_STEP_AT, Y_MINOR_AT, Y_SCALE_AT, 1usize),
    ];
    for (min_at, max_at, step_at, minor_at, scale_at, group) in spec {
        let axis = if group == 0 { &mut out.x_axis } else { &mut out.y_axis };
        let (lo, hi, step) = (f64_at(h, min_at), f64_at(h, max_at), f64_at(h, step_at));
        if lo.is_finite() && hi.is_finite() && hi > lo {
            axis.min = Some(lo);
            axis.max = Some(hi);
        }
        let code = u8_at(h, scale_at);
        match ScaleKind::from_file_code(code) {
            Some(ScaleKind::Linear) => {
                if step.is_finite() && step > 0.0 {
                    axis.major_step = Some(step);
                }
                axis.minor_ticks = u32::from(u8_at(h, minor_at)).min(10);
            }
            Some(ScaleKind::Log10) => {
                axis.scale = ScaleKind::Log10;
                axis.minor_ticks = 8;
            }
            Some(kind) => {
                axis.scale = kind;
                axis.minor_ticks = u32::from(u8_at(h, minor_at)).min(10);
                if step.is_finite() && step > 0.0 {
                    axis.major_step = Some(step);
                }
            }
            None => notes.push(format!("scale type {code} is not supported and is drawn as linear")),
        }
        if axis.scale != ScaleKind::Linear
            && !(axis.min.is_some_and(|m| axis.scale.accepts(m)) && axis.max.is_some_and(|m| axis.scale.accepts(m)))
        {
            axis.min = None;
            axis.max = None;
        }
        if let Some(blocks) = layer.axes.get(group).filter(|b| b.len() >= 6) {
            let side = axis_side(blocks, first);
            apply_axis_blocks(axis, blocks, side);
            axis.side = side;
            if side == AxisSide::Opposite && shown(blocks, AXIS_LABELS) {
                notes.push("a layer shows an axis on both sides; only the opposite side is drawn".into());
            }
        } else if let Some(blocks) = layer.axes.get(group) {
            apply_axis_blocks(axis, blocks, AxisSide::Normal);
        }
        axis.show_title = false;
    }

    // Annotations: titles, legend, free text.
    out.legend.visible = false;
    out.labels.clear();
    let frame_pt = (out.frame.left * page.width_pt, out.frame.top * page.height_pt);
    let frame_size = (out.frame.width * page.width_pt, out.frame.height * page.height_pt);
    for a in &layer.annotations {
        let name = annotation_name(a);
        let text = annotation_text(a);
        let size = u16_at(&a.block1, ANN_FONT_AT);
        let size_pt = if (4..=96).contains(&size) { f64::from(size) } else { 12.0 };
        match name.as_str() {
            "XB" | "YL" | "XT" | "YR" => {
                if text.trim().is_empty() {
                    continue;
                }
                // The bottom and left titles, and the right title of a layer whose Y axis is drawn
                // on the right (the second layer of a double-Y graph).
                let axis = match name.as_str() {
                    "XB" if out.x_axis.side != AxisSide::Opposite => &mut out.x_axis,
                    "XT" if out.x_axis.side == AxisSide::Opposite => &mut out.x_axis,
                    "YL" if out.y_axis.side != AxisSide::Opposite => &mut out.y_axis,
                    "YR" if out.y_axis.side == AxisSide::Opposite => &mut out.y_axis,
                    "XB" | "YL" => continue,
                    _ => {
                        notes.push("a title on the top or right axis was not imported".into());
                        continue;
                    }
                };
                // "%(?X)" and "%(?Y)" ask for the title built from the plotted column: a title that is
                // only that token is the automatic title; text around it is kept, and the renderer
                // replaces the token (T246).
                axis.title = if is_only_axis_token(&text) { None } else { Some(clean_markup(&text)) };
                axis.show_title = true;
                axis.title_font.size_pt = size_pt;
                // T291: a title turned away from its default (0 for X, 90 for Y) keeps its angle.
                let angle = f64::from(u16_at(&a.block1, ANN_ANGLE_AT)) / 10.0;
                let default = if name.starts_with('X') { 0.0 } else { 90.0 };
                if (angle - default).abs() > 0.05 && angle < 360.0 {
                    axis.title_rotation_deg = Some(angle);
                }
            }
            // Origin draws nothing outside the page: a legend or text moved off it is hidden there
            // (the owner's XRD project keeps its legends right of the page).
            n if n.eq_ignore_ascii_case("legend") && off_page(a, page) => {}
            n if n.eq_ignore_ascii_case("legend") => {
                out.legend.visible = true;
                out.legend.font.size_pt = size_pt;
                out.legend.text = custom_legend_text(&text);
                out.legend.frame = legend_has_frame(a);
                let (cx, cy) = annotation_center(a, page_units);
                if frame_size.0 > 0.0 && frame_size.1 > 0.0 {
                    let x = (cx / PAGE_UNITS_PER_INCH * 72.0 - frame_pt.0) / frame_size.0;
                    let y = (cy / PAGE_UNITS_PER_INCH * 72.0 - frame_pt.1) / frame_size.1;
                    if x.is_finite() && y.is_finite() {
                        out.legend.position = LegendPosition::Manual { x, y };
                    }
                }
            }
            n if n.starts_with('_') || n.is_empty() || a.block1.len() != 102 => {}
            _ => {
                if text.trim().is_empty() || frame_size.0 <= 0.0 || frame_size.1 <= 0.0 || off_page(a, page) {
                    continue;
                }
                push_text_label(&mut out, a, &text, size_pt, frame_pt, frame_size);
            }
        }
    }

    // Curves.
    let mut y_sets: Vec<usize> = Vec::new();
    let mut run = GroupRun::default();
    for (i, rec) in layer.columns.iter().enumerate() {
        read_curve(&mut out, &mut y_sets, &mut run, i, rec, column_of_set, notes);
    }
    // A group of one plot is no group.
    let sizes: Vec<u32> = out.plots.iter().filter_map(|p| p.group).collect();
    for p in &mut out.plots {
        if p.group.is_some_and(|g| sizes.iter().filter(|&&s| s == g).count() < 2) {
            p.group = None;
        }
    }
    // Curves moved apart by offsets (stacked lines): Origin stores each curve's offset as it drew
    // it, so the layer is drawn with those offsets as they are (graphing spec 13.10).
    if out.plot_offsets.iter().any(|o| o.x != 0.0 || o.y != 0.0) {
        out.stack = Stack::Individual;
    } else {
        out.plot_offsets.clear();
    }
    out
}

/// True when bit [`AXIS_SHOWN_BIT`] is set in block `k` of an axis group.
fn shown(blocks: &[Vec<u8>], k: usize) -> bool {
    blocks.get(k).is_some_and(|b| u8_at(b, AXIS_FLAGS_AT) & AXIS_SHOWN_BIT != 0)
}

/// The edge an axis is drawn on, from the visibility bits of its blocks (graphing spec 12.7): nothing
/// shown is a hidden axis; tick labels or a line on the opposite side and no labels on the primary
/// side is an axis on the opposite side, unless the primary line is also drawn and the opposite side
/// has no labels (that is a box whose primary labels are off). A layer after the first whose
/// opposite side has tick labels is a right Y layer or like it: its axis is on the opposite side.
fn axis_side(blocks: &[Vec<u8>], first: bool) -> AxisSide {
    let (labels, line) = (shown(blocks, AXIS_LABELS), shown(blocks, AXIS_LINE));
    let (far_labels, far_line) = (shown(blocks, AXIS_OPPOSITE_LABELS), shown(blocks, AXIS_OPPOSITE));
    if !(labels || line || far_labels || far_line) {
        AxisSide::Hidden
    } else if ((far_labels || far_line) && !labels && (far_labels || !line)) || (!first && far_labels) {
        AxisSide::Opposite
    } else {
        AxisSide::Normal
    }
}

/// Reads the link of every other layer that names an earlier layer in its header (a panel of a stack,
/// or a layer of one of Origin's panel templates): the layer is linked to it, its frame becomes
/// fractions of the parent's frame, and an object named `X1` or `Y1` in the layer says that the X or
/// Y scale follows the parent's (spec section 13.13). A parent that is itself linked is not followed:
/// the model links to the first layer of a group only.
fn link_panels(graph: &mut Graph, w: &Window) {
    for k in 1..graph.layers.len().min(w.layers.len()) {
        if graph.layers[k].link.is_some() {
            continue;
        }
        let number = usize::from(u8_at(&w.layers[k].header, LAYER_LINK_AT));
        if number == 0 || number > k {
            continue;
        }
        let parent = number - 1;
        let pf = graph.layers[parent].frame;
        if graph.layers[parent].link.is_some() || pf.width <= 0.0 || pf.height <= 0.0 {
            continue;
        }
        let names: Vec<String> = w.layers[k].annotations.iter().map(annotation_name).collect();
        let follows = |n: &str| if names.iter().any(|a| a == n) { ScaleLink::Straight } else { ScaleLink::None };
        let layer = &mut graph.layers[k];
        layer.link = Some(LayerLink { parent, x: follows("X1"), y: follows("Y1") });
        layer.frame = Frame {
            left: (layer.frame.left - pf.left) / pf.width,
            top: (layer.frame.top - pf.top) / pf.height,
            width: layer.frame.width / pf.width,
            height: layer.frame.height / pf.height,
        };
    }
}

/// Turns the second layer of a double-Y graph (and any layer like it) into a layer linked to the
/// layer it lies on (graphing spec 12.7): a layer after the first whose rectangle is the same as an
/// earlier unlinked layer's and whose Y axis is drawn on the right only. It shares the parent's X
/// scale and has its own Y scale; its X axis is not drawn (a top line it shows closes the parent's
/// box instead), and the parent's legend lists the plots of both layers.
fn link_right_y_layers(graph: &mut Graph, w: &Window) {
    let rect = |k: usize| w.layers.get(k).and_then(|l| l.header.get(RECT_AT..RECT_AT + 8)).map(<[u8]>::to_vec);
    for k in 1..graph.layers.len().min(w.layers.len()) {
        if graph.layers[k].y_axis.side != AxisSide::Opposite || rect(k).is_none() {
            continue;
        }
        let Some(parent) = (0..k).find(|&p| {
            graph.layers[p].link.is_none() && graph.layers[p].y_axis.side != AxisSide::Opposite && rect(p) == rect(k)
        }) else {
            continue;
        };
        let x_blocks = w.layers[k].axes.first().map(Vec::as_slice).unwrap_or(&[]);
        let top_line = shown(x_blocks, AXIS_OPPOSITE);
        let layer = &mut graph.layers[k];
        layer.link = Some(LayerLink { parent, x: ScaleLink::Straight, y: ScaleLink::None });
        layer.frame = Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 };
        layer.x_axis.side = AxisSide::Hidden;
        layer.x_axis.show_title = false;
        let (legend_here, frame_here) = (layer.legend.visible, layer.legend.frame);
        layer.legend.visible = false;
        let p = &mut graph.layers[parent];
        if legend_here && !p.legend.visible {
            p.legend.frame = frame_here;
        }
        if top_line {
            p.x_axis.show_opposite = true;
        }
        p.y_axis.show_opposite = false;
        p.y_axis.opposite_ticks = false;
        if p.legend.visible || legend_here {
            p.legend.visible = true;
            p.legend.scope = LegendScope::Graph;
        }
    }
}

/// Reads the blocks of an axis group: the tick labels and the line of the edge the axis is drawn on
/// (blocks 2 and 3, or 4 and 5 on the opposite side) and whether the other edge has a line too.
fn apply_axis_blocks(axis: &mut Axis, blocks: &[Vec<u8>], side: AxisSide) {
    let (labels, line, far_line) = match side {
        AxisSide::Opposite => (AXIS_OPPOSITE_LABELS, AXIS_OPPOSITE, AXIS_LINE),
        _ => (AXIS_LABELS, AXIS_LINE, AXIS_OPPOSITE),
    };
    if let Some(b) = blocks.get(labels) {
        let size = u8_at(b, AXIS_FONT_AT);
        if (4..=96).contains(&size) {
            axis.tick_font.size_pt = f64::from(size);
            axis.title_font.size_pt = f64::from(size);
        }
        axis.show_tick_labels = u8_at(b, AXIS_FLAGS_AT) & AXIS_SHOWN_BIT != 0;
        axis.tick_format = tick_format(b);
    }
    if let Some(b) = blocks.get(line) {
        axis.color_follows_plot = u16_at(b, AXIS_COLOR_AT) == AXIS_AUTO_COLOR;
        let width = f64::from(u16_at(b, AXIS_LINE_WIDTH_AT)) / CURVE_UNITS_PER_POINT;
        if width > 0.0 && width < 20.0 {
            axis.line_width_pt = width;
        }
        let code = u8_at(b, AXIS_TICKS_AT);
        axis.major_tick = tick_direction((code >> 4) & 3);
        axis.minor_tick = tick_direction((code >> 6) & 3);
        // The major tick length; the minor one is Origin's Auto, half the major (where Origin keeps
        // a chosen minor length was not found).
        let len = f64::from(u16_at(b, AXIS_MAJOR_LEN_AT)) / 10.0;
        if len > 0.0 && len <= 100.0 {
            axis.major_tick_len_pt = len;
            axis.minor_tick_len_pt = len / 2.0;
        }
    }
    // A line on the other edge too closes the box; on an axis drawn on the opposite side that edge
    // is not drawn (the model has one line per axis there).
    axis.show_opposite = side != AxisSide::Opposite && shown(blocks, far_line);
    axis.opposite_ticks = false;
}

/// The number format of the tick labels: bits 4 and 5 of byte 0x25 (0 decimal, 1 scientific; the
/// other kinds are read as decimal), bit 7 "fixed number of decimals" and that number in the low
/// nibble of byte 0x26.
fn tick_format(b: &[u8]) -> TickFormat {
    let kind = u8_at(b, AXIS_FORMAT_AT);
    let fixed = kind & AXIS_FIXED_DECIMALS_BIT != 0;
    let decimals = u8_at(b, AXIS_FLAGS_AT) & 0x0F;
    if kind & 0x0F == AXIS_DATE_LABELS {
        // dates in one of Origin's formats; the model has the patterns of `datetime`
        return TickFormat::DateTime(sheaf_core::datetime::DatePattern::Mdy);
    }
    match (kind >> 4) & 3 {
        1 => TickFormat::Scientific(if fixed { decimals } else { 0 }),
        _ if fixed => TickFormat::Decimal(decimals),
        _ => TickFormat::Auto,
    }
}

fn tick_direction(code: u8) -> TickDirection {
    match code {
        1 => TickDirection::In,
        2 => TickDirection::Out,
        3 => TickDirection::Both,
        _ => TickDirection::None,
    }
}

pub(crate) fn annotation_name(a: &WinAnnotation) -> String {
    match a.header.get(ANN_NAME_AT..) {
        Some(rest) => decode_cp1252(c_bytes(&rest[..rest.len().min(24)])),
        None => String::new(),
    }
}

/// The text of a text object: the bytes of block 2 up to the first NUL.
fn annotation_text(a: &WinAnnotation) -> String {
    decode_cp1252(c_bytes(&a.block2))
}

/// Offset in a legend's style block of the colour of its frame: `FC FF FF FF` is "none", anything
/// else (Origin's own legends have the list position 0, black) is a drawn frame (opj-format 10.4).
const LEGEND_FRAME_COLOR_AT: usize = 0x2A;
const COLOR_NONE: [u8; 4] = [0xFC, 0xFF, 0xFF, 0xFF];

/// True when the legend annotation has a frame. A style block too short to say counts as framed,
/// which is what Origin's own legends are.
fn legend_has_frame(a: &WinAnnotation) -> bool {
    a.block1.get(LEGEND_FRAME_COLOR_AT..LEGEND_FRAME_COLOR_AT + 4).is_none_or(|c| c != COLOR_NONE)
}

/// The legend text of a file as the model keeps it: `None` when it is the automatic text (a
/// symbol and a name per plot, `\l(n) %(n)`), otherwise the text with LF line ends.
fn custom_legend_text(text: &str) -> Option<String> {
    let normal = text.replace("\r\n", "\n").replace('\r', "\n");
    let normal = normal.trim_end_matches('\n');
    let automatic = |line: &str| {
        let Some(rest) = line.strip_prefix("\\l(").or_else(|| line.strip_prefix("\\L(")) else { return false };
        let Some((id, name)) = rest.split_once(") ") else { return false };
        name == format!("%({id})")
    };
    if normal.is_empty() || normal.split('\n').all(automatic) { None } else { Some(normal.to_string()) }
}

/// Centre of the annotation rectangle in page units.
fn annotation_center(a: &WinAnnotation, page_units: (f64, f64)) -> (f64, f64) {
    let (l, t, r, b) = (
        f64::from(u16_at(&a.header, ANN_RECT_AT)),
        f64::from(u16_at(&a.header, ANN_RECT_AT + 2)),
        f64::from(u16_at(&a.header, ANN_RECT_AT + 4)),
        f64::from(u16_at(&a.header, ANN_RECT_AT + 6)),
    );
    let _ = page_units;
    ((l + r) / 2.0, (t + b) / 2.0)
}

/// True when the annotation's rectangle lies wholly outside the page (an empty rectangle is not).
fn off_page(a: &WinAnnotation, page: &Page) -> bool {
    let (l, t, r, b) = annotation_rect_pt(a);
    (r > l || b > t) && (l >= page.width_pt || t >= page.height_pt || r <= 0.0 || b <= 0.0)
}

/// The rectangle of an annotation in points: left, top, right, bottom.
fn annotation_rect_pt(a: &WinAnnotation) -> (f64, f64, f64, f64) {
    let pt = |k: usize| f64::from(u16_at(&a.header, ANN_RECT_AT + k)) / PAGE_UNITS_PER_INCH * 72.0;
    (pt(0), pt(2), pt(4), pt(6))
}

/// A free text object becomes one label per line of text (the model draws single lines).
///
/// Origin turns a text object about the top-left corner of its unturned box, and the rectangle
/// stored in the file has that corner as its own top-left corner, with width and height swapped
/// for a quarter or three-quarter turn (opj-format.md 10.4). Measured on a 90-degree label: the
/// stored rectangle runs from y 946 to 1187 (1/600 inch) while Origin draws the text from 667 up to
/// 932, ending at the rectangle's top. So each line starts at that corner, moved down the turned
/// text by half a line height per line, which is where the model anchors a label (the vertical
/// middle of the first character).
fn push_text_label(
    out: &mut Layer,
    a: &WinAnnotation,
    text: &str,
    size_pt: f64,
    frame_pt: (f64, f64),
    frame_size: (f64, f64),
) {
    let (l, t, r, b) = annotation_rect_pt(a);
    let angle = f64::from(u16_at(&a.block1, ANN_ANGLE_AT)) / 10.0;
    let lines: Vec<String> = text.split(['\r', '\n']).filter(|l| !l.trim().is_empty()).map(clean_markup).collect();
    if lines.is_empty() {
        return;
    }
    // The height of the unturned box: the rectangle's height, or its width when a quarter turn
    // made the rectangle tall.
    let quarter = ((angle / 90.0).round() as i64).rem_euclid(4);
    let square = (angle / 90.0 - (angle / 90.0).round()).abs() < 1e-6;
    let box_height = match (square, quarter) {
        (true, 1 | 3) => r - l,
        (true, _) => b - t,
        _ => 0.0,
    };
    let line_height = if box_height > 0.0 { box_height / lines.len() as f64 } else { 1.2 * size_pt };
    // Down the turned text: the screen direction of "down" in the unturned box (y grows downwards).
    let (sin, cos) = angle.to_radians().sin_cos();
    for (k, line) in lines.iter().enumerate() {
        let along = line_height * (k as f64 + 0.5);
        let (x, y) = (l + along * sin, t + along * cos);
        let mut font = out.legend.font.clone();
        font.size_pt = size_pt;
        out.labels.push(TextLabel {
            text: line.clone(),
            x: (x - frame_pt.0) / frame_size.0,
            y: (y - frame_pt.1) / frame_size.1,
            font,
            rotation_deg: angle,
            table: false,
            group: None,
        });
    }
}

/// Drops the size escape `\p<digits>(...)` (the model has no relative text size) and keeps the
/// escapes it can draw.
/// True when `text` is a single `%(?X...)` or `%(?Y...)` token and nothing else (T246).
fn is_only_axis_token(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("%(?") && t.ends_with(')') && t.matches(')').count() == 1
}

pub(crate) fn clean_markup(s: &str) -> String {
    #[derive(Clone, Copy, PartialEq)]
    enum Open {
        Plain,
        Dropped,
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut stack: Vec<Open> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && chars.get(i + 1) == Some(&'p') {
            let mut j = i + 2;
            while chars.get(j).is_some_and(|d| d.is_ascii_digit()) {
                j += 1;
            }
            if j > i + 2 && chars.get(j) == Some(&'(') {
                stack.push(Open::Dropped);
                i = j + 1;
                continue;
            }
        }
        if c == '(' {
            stack.push(Open::Plain);
        } else if c == ')' && stack.pop() == Some(Open::Dropped) {
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Text without any escape, for measuring.
pub(crate) fn strip_escapes(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    // true for a bracket that opened an escape (its closing bracket is not text).
    let mut stack: Vec<bool> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' {
            let mut j = i + 1;
            while chars.get(j).is_some_and(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '-') {
                j += 1;
            }
            if j > i + 1 && chars.get(j) == Some(&'(') {
                stack.push(true);
                i = j + 1;
                continue;
            }
        }
        if chars[i] == '(' {
            stack.push(false);
        } else if chars[i] == ')' && stack.pop() == Some(true) {
            i += 1;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// A colour written as four bytes (a list position and three zero bytes, or red, green, blue and
/// 1); `None` for "automatic" and anything else.
fn color_code(b: &[u8], at: usize) -> Option<Color> {
    let g = |k: usize| u8_at(b, at + k);
    match g(3) {
        0 => Some(palette_color(usize::from(g(0)))),
        1 => Some(Color::rgb(g(0), g(1), g(2))),
        _ => None,
    }
}

/// The line connection: 1 straight, 5 to 10 smooth curves (8 is read as a B-spline), 15 Bezier, 11 and 12 steps,
/// 13 and 14 the horizontal and vertical steps at the middle (spec 10.5, graphing 2.x; T268).
fn connect_kind(code: u8, notes: &mut Vec<String>) -> Connect {
    match code {
        8 => Connect::BSpline,
        15 => Connect::Bezier,
        5..=10 => Connect::Spline,
        11 => Connect::StepHorizontal,
        12 => Connect::StepVertical,
        13 => Connect::StepHorizontalCenter,
        14 => Connect::StepVerticalCenter,
        // LabTalk's line connection codes 2 and 3 (measured); the file field uses the same codes
        2 => Connect::Segment2,
        3 => Connect::Segment3,
        0 | 1 => Connect::Straight,
        _ => {
            notes.push("some line connections (segments, other steps) are not supported and are drawn straight".into());
            Connect::Straight
        }
    }
}

fn symbol_shape(code: u8, notes: &mut Vec<String>) -> SymbolShape {
    SymbolShape::from_origin_code(code).unwrap_or_else(|| {
        notes.push("some symbol shapes are not known and are drawn as squares".into());
        SymbolShape::Square
    })
}

/// Where the curves of a layer are in a plot group while they are read: the group's number and
/// how many members came before.
#[derive(Default)]
struct GroupRun {
    last: u32,
    current: Option<u32>,
    members: usize,
}

impl GroupRun {
    /// The group number and position in the group of the curve with this flag byte, `None` for a
    /// curve outside a group. A curve with the first bit starts a group, one with the member bit
    /// that follows a member joins it.
    fn place(&mut self, flags: u8) -> Option<(u32, usize)> {
        if flags & CURVE_GROUP_FIRST_BIT != 0 {
            self.last += 1;
            self.current = Some(self.last);
            self.members = 0;
        } else if flags & CURVE_GROUP_MEMBER_BIT != 0 && self.current.is_some() {
            self.members += 1;
        } else {
            self.current = None;
        }
        self.current.map(|g| (g, self.members))
    }
}

fn read_curve(
    layer: &mut Layer,
    y_sets: &mut Vec<usize>,
    run: &mut GroupRun,
    index: usize,
    rec: &WinColumn,
    column_of_set: &HashMap<usize, ColumnId>,
    notes: &mut Vec<String>,
) {
    let r = &rec.record;
    if r.len() < CURVE_PLOT_LINE_COLOR_AT + 4 {
        notes.push("a plot record is too short and was skipped".into());
        return;
    }
    let y_set = usize::from(u16_at(r, CURVE_Y_AT));
    let x_set = usize::from(u16_at(r, CURVE_X_AT));
    let ptype = u16_at(r, CURVE_TYPE_AT);
    let column = |set: usize| set.checked_sub(1).and_then(|i| column_of_set.get(&i)).copied();

    if ptype >= 0x1E0 {
        // An error plot: it names the plot it belongs to in the X slot.
        let owner = y_sets.iter().rposition(|&s| s == x_set);
        match (owner, column(y_set)) {
            (Some(k), Some(err)) if k < layer.plots.len() => layer.plots[k].y_error = Some(err),
            _ => notes.push("an error bar column could not be attached to a plot".into()),
        }
        y_sets.push(y_set);
        return;
    }
    y_sets.push(y_set);
    let Some(y) = column(y_set) else {
        notes.push("a plot uses data that is not a worksheet column and was skipped".into());
        return;
    };
    let connect = u8_at(r, CURVE_CONNECT_AT);
    let shape_code = u8_at(r, CURVE_SHAPE_AT);
    let kind = match ptype {
        200 => PlotKind::Line,
        201 => PlotKind::Scatter,
        202 => PlotKind::LineSymbol,
        203 => PlotKind::Column,
        204 => PlotKind::Area,
        // LabTalk's documented plot-type IDs (T223) of plots that read one Y column; the record layout
        // of these kinds was not compared byte for byte with Origin's.
        206 => PlotKind::Box,
        213 => PlotKind::StackedColumn,
        214 => PlotKind::StackedArea,
        215 => PlotKind::Bar,
        216 => PlotKind::StackedBar,
        219 => PlotKind::Histogram,
        225 => PlotKind::Pie,
        _ => {
            notes.push(format!("plot type {ptype} is not supported and is drawn as a line with symbols"));
            PlotKind::LineSymbol
        }
    };
    let symbol_code = color_code(r, CURVE_LINE_COLOR_AT);
    let line_code = color_code(r, CURVE_PLOT_LINE_COLOR_AT);
    let automatic = palette_color(index);
    // A scatter plot has no line: its colour is the symbol colour. Elsewhere the line has its own.
    let line_color = match kind {
        PlotKind::Scatter => symbol_code.or(line_code),
        _ => line_code.or(symbol_code),
    }
    .unwrap_or(automatic);
    let width = f64::from(u16_at(r, CURVE_WIDTH_AT)) / CURVE_UNITS_PER_POINT;
    let width = if width > 0.0 && width < 20.0 { width } else { 1.0 };
    let dash_code = usize::from(u8_at(r, CURVE_DASH_AT));
    let dash = Dash::ALL.get(dash_code).copied().unwrap_or(Dash::Solid);
    let size = f64::from(u16_at(r, CURVE_SIZE_AT)) / CURVE_UNITS_PER_POINT;
    let size = if size > 0.0 && size < 60.0 { size } else { 5.0 };
    let shape = if shape_code == 0 { SymbolShape::Square } else { symbol_shape(shape_code, notes) };
    let fill = if matches!(kind, PlotKind::Column | PlotKind::Area) {
        if u8_at(r, CURVE_FILL_USED_AT) & CURVE_FILL_USED_BIT != 0 {
            color_code(r, CURVE_FILL_AT).unwrap_or_else(|| palette_color(index))
        } else {
            palette_color(index)
        }
    } else {
        symbol_code.unwrap_or(line_color)
    };
    let auto = u8_at(r, CURVE_LINE_COLOR_AT + 3) != 0 && u8_at(r, CURVE_LINE_COLOR_AT + 3) != 1;
    let mut plot = Plot {
        x: column(x_set),
        y,
        y_error: None,
        rows: None,
        kind,
        line: LineStyle {
            color: line_color,
            width_pt: width,
            dash,
            connect: if kind == PlotKind::Column { Connect::Straight } else { connect_kind(connect, notes) },
            sort_x: false,
        },
        symbol: SymbolStyle {
            shape,
            size_pt: size,
            color: fill,
            filled: matches!(kind, PlotKind::Column | PlotKind::Area) || u8_at(r, CURVE_SYMBOL_FILL_AT) == 0,
            // The byte holds Origin's interior number (checked on a file Origin 2016 saved, T323):
            // solid stays on `filled`, the others (Open is white inside) become an interior.
            interior: match u8_at(r, CURVE_SYMBOL_FILL_AT) {
                0 => None,
                code => SymbolInterior::from_origin_code(code),
            }
            .filter(|_| !matches!(kind, PlotKind::Column | PlotKind::Area)),
            fill_color: None,
            edge_width_pt: 0.75,
            every: 1,
        },
        label: None,
        visible: true,
        auto_color: false,
        group: None,
        x_error: None,
        y_base: None,
        error: ErrorStyle::default(),
        hist: Default::default(),
        ridge: Default::default(),
        hexbin: Default::default(),
        parallel: Default::default(),
        treemap: Default::default(),
        indexed: None,
        stat: None,
        sunburst: Default::default(),
        size_from: None,
        color_from: None,
        decor: Default::default(),
    };
    // A plot group colours its members from a colour list by their place in the group. The list
    // is not always Origin's base palette (the owner's XRD figure draws black, red, blue, dark
    // cyan, magenta, dark yellow), and where the list is stored was not found; but Origin writes the
    // colour it drew into each record when it saves, so a colour written there wins, and only an
    // automatic one takes the base list (opj-format.md 10.9). A lone column plot or area keeps its
    // outline.
    if let Some((group, pos)) = run.place(u8_at(r, CURVE_GROUP_AT)) {
        let recorded = match kind {
            PlotKind::Column | PlotKind::Area => (u8_at(r, CURVE_FILL_USED_AT) & CURVE_FILL_USED_BIT != 0)
                .then(|| color_code(r, CURVE_FILL_AT))
                .flatten(),
            PlotKind::Scatter => symbol_code.or(line_code),
            _ => line_code.or(symbol_code),
        };
        plot.group = Some(group);
        if recorded.is_none() {
            let c = palette_color(pos);
            plot.symbol.color = c;
            if !matches!(kind, PlotKind::Column | PlotKind::Area) {
                plot.line.color = c;
            }
        }
        plot.auto_color = false;
    }
    if kind == PlotKind::Area && auto {
        // an area is outlined in black unless the line colour was chosen
        plot.line.color = Color::BLACK;
    }
    if kind == PlotKind::Column {
        // Bars are outlined in black unless the line colour was chosen.
        plot.line.color = if auto { Color::BLACK } else { line_color };
        plot.line.width_pt = width.min(2.0);
        // A column plot keeps the width of its bars in the size field: (100 - gap) * 50, so 4000 is
        // Origin's default gap of 20 percent (graphing spec 14.7). The layer's first column plot
        // sets the gap of the layer's bars.
        let bar_width = f64::from(u16_at(r, CURVE_SIZE_AT)) / CURVE_BAR_WIDTH_PER_PERCENT;
        if !layer.plots.iter().any(|p| p.kind == PlotKind::Column) && (0.0..=100.0).contains(&bar_width) {
            layer.bars.gap_pct = 100.0 - bar_width;
        }
    }
    if x_set != 0 && column(x_set).is_none() {
        notes.push("a plot's X data is not a worksheet column; it was plotted against the row number".into());
    }
    let offset = |at: usize| {
        let v = f64_at(r, at);
        if v.is_finite() && v != NO_VALUE && v.abs() < 1e300 { v } else { 0.0 }
    };
    let (dx, dy) = (offset(CURVE_X_OFFSET_AT), offset(CURVE_Y_OFFSET_AT));
    if layer.plot_offset(y).is_none() {
        // Every curve gets an entry (an offset of 0 included); entries are dropped again when no
        // curve of the layer is moved.
        let o = layer.plot_offset_mut(y);
        o.x = dx;
        o.y = dy;
    }
    layer.plots.push(plot);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u16(b: &mut [u8], at: usize, v: u16) {
        b[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }

    fn put_f64(b: &mut [u8], at: usize, v: f64) {
        b[at..at + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn window_header() -> Vec<u8> {
        let mut h = vec![0u8; 195];
        put_u16(&mut h, PAGE_WIDTH_AT, 6000);
        put_u16(&mut h, PAGE_HEIGHT_AT, 3000);
        h
    }

    fn layer_header() -> Vec<u8> {
        let mut h = vec![0u8; 361];
        put_f64(&mut h, X_MIN_AT, 0.0);
        put_f64(&mut h, X_MAX_AT, 10.0);
        put_f64(&mut h, X_STEP_AT, 2.0);
        put_f64(&mut h, Y_MIN_AT, 1.0);
        put_f64(&mut h, Y_MAX_AT, 1000.0);
        put_f64(&mut h, Y_STEP_AT, 1.0);
        h[Y_SCALE_AT] = 1;
        h[Y_MINOR_AT] = 8;
        h[X_MINOR_AT] = 1;
        put_u16(&mut h, RECT_AT, 600);
        put_u16(&mut h, RECT_AT + 2, 300);
        put_u16(&mut h, RECT_AT + 4, 5400);
        put_u16(&mut h, RECT_AT + 6, 2400);
        h
    }

    fn curve(y: u16, x: u16, ptype: u16, color: [u8; 4]) -> WinColumn {
        let mut r = vec![0u8; 515];
        put_u16(&mut r, CURVE_Y_AT, y);
        put_u16(&mut r, CURVE_X_AT, x);
        put_u16(&mut r, CURVE_TYPE_AT, ptype);
        put_u16(&mut r, CURVE_WIDTH_AT, 1500);
        put_u16(&mut r, CURVE_SIZE_AT, 4500);
        r[CURVE_SHAPE_AT] = 2;
        r[CURVE_CONNECT_AT] = 1;
        r[CURVE_DASH_AT] = 2;
        r[CURVE_LINE_COLOR_AT..CURVE_LINE_COLOR_AT + 4].copy_from_slice(&color);
        r[CURVE_PLOT_LINE_COLOR_AT..CURVE_PLOT_LINE_COLOR_AT + 4].copy_from_slice(&color);
        WinColumn {
            dataset: y.into(),
            designation: 0,
            short_name: String::new(),
            value_type: 0,
            note: Vec::new(),
            record: r,
        }
    }

    fn text_annotation(name: &str, text: &str, size: u16, rect: [u16; 4]) -> WinAnnotation {
        let mut header = vec![0u8; 131];
        for (k, v) in rect.iter().enumerate() {
            put_u16(&mut header, ANN_RECT_AT + 2 * k, *v);
        }
        header[ANN_NAME_AT..ANN_NAME_AT + name.len()].copy_from_slice(name.as_bytes());
        let mut block1 = vec![0u8; 102];
        put_u16(&mut block1, ANN_FONT_AT, size);
        WinAnnotation { header, block1, block2: text.as_bytes().to_vec(), block3: Vec::new() }
    }

    fn axis_blocks(font: u8, labels: bool, ticks: u8, opposite: bool) -> Vec<Vec<u8>> {
        let mut blocks = vec![vec![0u8; 515]; 6];
        blocks[AXIS_LABELS][AXIS_FONT_AT] = font;
        blocks[AXIS_LABELS][AXIS_FLAGS_AT] = if labels { 0x40 } else { 0 };
        put_u16(&mut blocks[AXIS_LINE], AXIS_LINE_WIDTH_AT, 1500);
        blocks[AXIS_LINE][AXIS_TICKS_AT] = ticks;
        blocks[AXIS_LINE][AXIS_FLAGS_AT] = 0x40;
        blocks[AXIS_OPPOSITE][AXIS_FLAGS_AT] = if opposite { 0x40 } else { 0 };
        blocks
    }

    fn window() -> Window {
        let layer = WinLayer {
            name: String::new(),
            header: layer_header(),
            annotations: vec![
                text_annotation("YL", "\\b(\\p89(Current (mA)))", 28, [10, 500, 80, 2000]),
                text_annotation("Text3", "Peak\r\nA", 20, [2000, 100, 2400, 200]),
                text_annotation("Legend", "\\l(1) %(1)", 18, [3000, 400, 4000, 700]),
                text_annotation("_hidden", "x", 10, [0, 0, 1, 1]),
            ],
            columns: vec![
                curve(2, 1, 202, [3, 0, 0, 0]),
                curve(3, 1, 200, [0xF7, 0xFF, 0xFF, 0xFF]),
                curve(4, 3, 0x1E7, [0xF7, 0xFF, 0xFF, 0xFF]),
            ],
            axes: vec![axis_blocks(24, false, 0xE0, true), axis_blocks(21, true, 0x50, false), Vec::new()],
        };
        Window {
            name: "Graph7".into(),
            label: "My figure".into(),
            comments: String::new(),
            layers: vec![layer],
            header: window_header(),
        }
    }

    fn ids() -> HashMap<usize, ColumnId> {
        (0..6).map(|i| (i, ColumnId(100 + i as u64))).collect()
    }

    #[test]
    fn page_frame_axes_and_scale_are_read() {
        let mut warnings = Vec::new();
        let g = build(&window(), &ids(), &mut warnings);
        assert_eq!((g.name.as_str(), g.long_name.as_str()), ("Graph7", "My figure"));
        assert!((g.page.width_pt - 720.0).abs() < 1e-9 && (g.page.height_pt - 360.0).abs() < 1e-9);
        let l = &g.layers[0];
        assert!((l.frame.left - 0.1).abs() < 1e-9 && (l.frame.top - 0.1).abs() < 1e-9);
        assert!((l.frame.width - 0.8).abs() < 1e-9 && (l.frame.height - 0.7).abs() < 1e-9);
        assert_eq!((l.x_axis.min, l.x_axis.max, l.x_axis.major_step), (Some(0.0), Some(10.0), Some(2.0)));
        assert_eq!(l.x_axis.scale, ScaleKind::Linear);
        assert_eq!((l.y_axis.min, l.y_axis.max, l.y_axis.major_step), (Some(1.0), Some(1000.0), None));
        assert_eq!(l.y_axis.scale, ScaleKind::Log10);
        assert_eq!(l.y_axis.minor_ticks, 8);
    }

    #[test]
    fn axis_blocks_give_ticks_labels_fonts_and_box() {
        let mut warnings = Vec::new();
        let g = build(&window(), &ids(), &mut warnings);
        let l = &g.layers[0];
        assert!(!l.x_axis.show_tick_labels && l.y_axis.show_tick_labels);
        assert_eq!((l.x_axis.major_tick, l.x_axis.minor_tick), (TickDirection::Out, TickDirection::Both));
        assert_eq!((l.y_axis.major_tick, l.y_axis.minor_tick), (TickDirection::In, TickDirection::In));
        assert_eq!((l.x_axis.tick_font.size_pt, l.y_axis.tick_font.size_pt), (24.0, 21.0));
        assert!(l.x_axis.show_opposite && !l.y_axis.show_opposite);
        assert!((l.x_axis.line_width_pt - 3.0).abs() < 1e-9);
    }

    #[test]
    fn titles_labels_and_legend_come_from_annotations() {
        let mut warnings = Vec::new();
        let g = build(&window(), &ids(), &mut warnings);
        let l = &g.layers[0];
        assert_eq!(l.y_axis.title.as_deref(), Some("\\b(Current (mA))"));
        assert!(l.y_axis.show_title && !l.x_axis.show_title);
        assert!(l.legend.visible);
        assert!(matches!(l.legend.position, LegendPosition::Manual { .. }));
        // Two lines of text become two labels; the hidden object is ignored.
        assert_eq!(l.labels.len(), 2);
        assert_eq!(l.labels[0].text, "Peak");
        assert!(l.labels[1].y > l.labels[0].y);
    }

    #[test]
    fn legend_text_is_kept_only_when_it_is_not_the_automatic_text() {
        let legend_text = |text: &str| {
            let mut w = window();
            w.layers[0].annotations[2] = text_annotation("Legend", text, 18, [3000, 400, 4000, 700]);
            build(&w, &ids(), &mut Vec::new()).layers[0].legend.text.clone()
        };
        assert_eq!(legend_text("\\l(1) %(1)"), None);
        assert_eq!(legend_text("\\l(1.1) %(1.1)\r\n\\l(2.1) %(2.1)\r\n"), None);
        assert_eq!(legend_text("\\l(1) Fe\r\n\\l(2) %(2)").as_deref(), Some("\\l(1) Fe\n\\l(2) %(2)"));
        assert_eq!(legend_text("\\l(1) %(2)").as_deref(), Some("\\l(1) %(2)"));
        assert_eq!(legend_text("T = 5 K").as_deref(), Some("T = 5 K"));
    }

    #[test]
    fn curves_have_columns_type_colors_widths_and_error_bars() {
        let mut warnings = Vec::new();
        let g = build(&window(), &ids(), &mut warnings);
        let p = &g.layers[0].plots;
        assert_eq!(p.len(), 2, "the error plot is attached, not listed");
        assert_eq!((p[0].y, p[0].x), (ColumnId(101), Some(ColumnId(100))));
        assert_eq!(p[0].kind, PlotKind::LineSymbol);
        assert_eq!(p[0].line.color, Color::rgb(0, 0, 255));
        assert!((p[0].line.width_pt - 3.0).abs() < 1e-9 && (p[0].symbol.size_pt - 9.0).abs() < 1e-9);
        assert_eq!((p[0].line.dash, p[0].symbol.shape), (Dash::Dot, SymbolShape::Circle));
        assert_eq!(p[1].kind, PlotKind::Line);
        assert_eq!(p[1].line.color, palette_color(1), "automatic colour follows the plot position");
        assert_eq!(p[1].y_error, Some(ColumnId(103)));
    }

    #[test]
    fn the_first_column_plot_gives_the_gap_between_bars() {
        let mut w = window();
        // Bar widths as Origin saved them for gaps of 40 and 55 percent.
        let mut bars = curve(2, 1, 203, [0, 0, 0, 0]);
        put_u16(&mut bars.record, CURVE_SIZE_AT, 3000);
        let mut second = curve(3, 1, 203, [0, 0, 0, 0]);
        put_u16(&mut second.record, CURVE_SIZE_AT, 2250);
        w.layers[0].columns = vec![bars, second];
        let mut warnings = Vec::new();
        let g = build(&w, &ids(), &mut warnings);
        let l = &g.layers[0];
        assert_eq!(l.plots.len(), 2);
        assert_eq!(l.bars.gap_pct, 40.0);
        assert_eq!(l.bars.overlap_pct, 0.0);
        // A layer without column plots keeps the default gap.
        let g = build(&window(), &ids(), &mut warnings);
        assert_eq!(g.layers[0].bars, sheaf_core::graph::BarSpacing::default());
        // A width outside 0..100 percent is not a gap.
        let mut w = window();
        let mut odd = curve(2, 1, 203, [0, 0, 0, 0]);
        put_u16(&mut odd.record, CURVE_SIZE_AT, 6000);
        w.layers[0].columns = vec![odd];
        assert_eq!(build(&w, &ids(), &mut warnings).layers[0].bars.gap_pct, 20.0);
    }

    #[test]
    fn unknown_data_and_short_records_give_warnings_not_panics() {
        let mut w = window();
        w.layers[0].columns.push(curve(9, 1, 200, [0, 0, 0, 0]));
        w.layers[0].columns.push(WinColumn {
            dataset: 0,
            designation: 0,
            short_name: String::new(),
            value_type: 0,
            note: Vec::new(),
            record: vec![1, 2, 3],
        });
        w.layers[0].header.truncate(20);
        w.header.truncate(10);
        let mut warnings = Vec::new();
        let g = build(&w, &ids(), &mut warnings);
        assert_eq!(g.layers[0].plots.len(), 2);
        assert!(warnings.iter().any(|m| m.contains("page size")));
        assert!(warnings.iter().any(|m| m.contains("not a worksheet column")));
    }

    #[test]
    fn size_escapes_are_dropped_and_other_text_kept() {
        assert_eq!(clean_markup("\\b(\\p89(Nitrate (ppm)))"), "\\b(Nitrate (ppm))");
        assert_eq!(clean_markup("plain \\p12 text"), "plain \\p12 text");
        assert_eq!(strip_escapes("\\b(Lady Bird)"), "Lady Bird");
    }

    #[test]
    fn palette_has_the_default_colour_order() {
        assert_eq!(palette_color(0), Color::BLACK);
        assert_eq!(palette_color(1), Color::RED);
        assert_eq!(palette_color(3), Color::BLUE);
        assert_eq!(palette_color(22), Color::rgb(255, 128, 255));
        assert_eq!(palette_color(24), Color::BLACK);
    }

    /// Axis blocks with the visibility bits of graphing spec 12.7: primary-side tick labels,
    /// primary line, opposite-side tick labels, opposite line.
    fn axis_sides(labels: bool, line: bool, opposite_labels: bool, opposite_line: bool) -> Vec<Vec<u8>> {
        let mut blocks = axis_blocks(20, labels, 0x50, opposite_line);
        blocks[AXIS_LINE][AXIS_FLAGS_AT] = if line { 0x40 } else { 0 };
        blocks[AXIS_OPPOSITE_LABELS][AXIS_FLAGS_AT] = if opposite_labels { 0x40 } else { 0 };
        blocks
    }

    fn with_offset(mut c: WinColumn, y: f64) -> WinColumn {
        put_f64(&mut c.record, CURVE_Y_OFFSET_AT, y);
        c
    }

    #[test]
    fn curve_offsets_give_an_individual_stack() {
        let mut w = window();
        w.layers[0].columns =
            vec![with_offset(curve(2, 1, 200, [0, 0, 0, 0]), 0.0), with_offset(curve(3, 1, 200, [1, 0, 0, 0]), 378.3)];
        let g = build(&w, &ids(), &mut Vec::new());
        let l = &g.layers[0];
        assert_eq!(l.stack, Stack::Individual);
        assert_eq!(l.plot_offset(ColumnId(101)).map(|o| o.y), Some(0.0));
        assert_eq!(l.plot_offset(ColumnId(102)).map(|o| (o.x, o.y)), Some((0.0, 378.3)));
        // Origin's "no value" and numbers that are not finite count as no offset.
        for odd in [NO_VALUE, f64::NAN, f64::INFINITY, 0.0] {
            let mut w = window();
            w.layers[0].columns =
                vec![curve(2, 1, 200, [0, 0, 0, 0]), with_offset(curve(3, 1, 200, [1, 0, 0, 0]), odd)];
            let l = &build(&w, &ids(), &mut Vec::new()).layers[0];
            assert_eq!(l.stack, Stack::None, "{odd}");
            assert!(l.plot_offsets.is_empty(), "{odd}");
        }
    }

    /// The window of [`window`] with a second layer on the same frame that shows only a right Y
    /// axis (automatic colour) and the top line, as Origin's double-Y graph does.
    fn double_y_window() -> Window {
        let mut w = window();
        let mut second = w.layers[0].clone();
        second.annotations = vec![text_annotation("YR", "Sine (mA)", 22, [5500, 500, 5600, 2000])];
        second.columns = vec![curve(5, 1, 200, [0xF7, 0xFF, 0xFF, 0xFF])];
        let mut y = axis_sides(false, false, true, true);
        put_u16(&mut y[AXIS_LINE], AXIS_COLOR_AT, AXIS_AUTO_COLOR);
        put_u16(&mut y[AXIS_OPPOSITE], AXIS_COLOR_AT, AXIS_AUTO_COLOR);
        second.axes = vec![axis_sides(false, false, false, true), y, Vec::new()];
        w.layers[0].annotations[2] = text_annotation("legend", "\\l(1.1) %(1.1)", 18, [3000, 400, 4000, 700]);
        w.layers[0].axes[0] = axis_sides(true, true, false, false);
        w.layers.push(second);
        w
    }

    #[test]
    fn a_second_layer_with_a_right_axis_on_the_same_frame_becomes_a_linked_layer() {
        let mut warnings = Vec::new();
        let g = build(&double_y_window(), &ids(), &mut warnings);
        assert_eq!(g.layers.len(), 2);
        let (root, right) = (&g.layers[0], &g.layers[1]);
        assert_eq!(right.link, Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None }));
        assert_eq!(right.frame, Frame { left: 0.0, top: 0.0, width: 1.0, height: 1.0 });
        assert_eq!((right.y_axis.side, right.x_axis.side), (AxisSide::Opposite, AxisSide::Hidden));
        assert!(right.y_axis.show_tick_labels && right.y_axis.color_follows_plot);
        assert_eq!(right.y_axis.title.as_deref(), Some("Sine (mA)"));
        assert!(right.y_axis.show_title && !right.legend.visible);
        assert!(root.legend.visible && root.legend.scope == LegendScope::Graph, "the lower-case legend is found");
        assert!(root.x_axis.show_opposite, "the right layer's top line closes the box");
        assert!(!root.y_axis.show_opposite && !root.y_axis.color_follows_plot);
        assert_eq!(right.plots.len(), 1);
        assert!(!warnings.iter().any(|m| m.contains("top or right")), "{warnings:?}");
    }

    #[test]
    fn a_right_axis_layer_elsewhere_or_a_plain_second_layer_stays_unlinked() {
        // Another frame: an inset with its axis on the right is drawn there, but not linked.
        let mut w = double_y_window();
        put_u16(&mut w.layers[1].header, RECT_AT, 3000);
        let g = build(&w, &ids(), &mut Vec::new());
        assert!(g.layers[1].link.is_none());
        assert_eq!(g.layers[1].y_axis.side, AxisSide::Opposite);
        // The same frame with the axis on the left and a box: two plain overlapping layers.
        let mut w = double_y_window();
        w.layers[1].axes[1] = axis_sides(true, true, false, true);
        let g = build(&w, &ids(), &mut Vec::new());
        assert!(g.layers[1].link.is_none());
        assert_eq!(g.layers[1].y_axis.side, AxisSide::Normal);
        // Tick labels on both sides: linked and drawn on the right; the import says what is left out.
        let mut w = double_y_window();
        w.layers[1].axes[1] = axis_sides(true, true, true, true);
        let mut warnings = Vec::new();
        let g = build(&w, &ids(), &mut warnings);
        assert!(g.layers[1].link.is_some());
        assert!(warnings.iter().any(|m| m.contains("both sides")), "{warnings:?}");
    }

    #[test]
    fn the_legend_frame_is_read_from_the_frame_colour() {
        let frame = |colour: Option<[u8; 4]>| {
            let mut w = window();
            if let Some(c) = colour {
                w.layers[0].annotations[2].block1[LEGEND_FRAME_COLOR_AT..LEGEND_FRAME_COLOR_AT + 4].copy_from_slice(&c);
            }
            build(&w, &ids(), &mut Vec::new()).layers[0].legend.frame
        };
        assert!(frame(None), "a black frame colour (0) is a frame");
        assert!(frame(Some([3, 0, 0, 0])));
        assert!(!frame(Some([0xFC, 0xFF, 0xFF, 0xFF])), "the colour 'none' is no frame");
    }

    #[test]
    fn plot_groups_are_read_with_the_colours_of_the_list() {
        let mut w = window();
        let rec = |flags: u8, y: u16| {
            let mut c = curve(y, 1, 200, [0xF7, 0xFF, 0xFF, 0xFF]);
            c.record[CURVE_GROUP_AT] = flags;
            c
        };
        // three plots in a group, one outside, a lone first-bit plot (no group), two more in a group
        w.layers[0].columns = vec![
            rec(0x29, 2),
            rec(0x19, 3),
            rec(0x19, 4),
            rec(0x09, 5),
            rec(0x29, 6),
            rec(0x09, 2),
            rec(0x29, 3),
            rec(0x19, 4),
        ];
        let g = build(&w, &ids(), &mut Vec::new());
        let plots = &g.layers[0].plots;
        let groups: Vec<Option<u32>> = plots.iter().map(|p| p.group).collect();
        assert_eq!(groups, vec![Some(1), Some(1), Some(1), None, None, None, Some(3), Some(3)]);
        let (r, gr) = (Color::rgb(255, 0, 0), Color::rgb(0, 255, 0));
        assert_eq!((plots[0].line.color, plots[1].line.color, plots[2].line.color), (Color::BLACK, r, gr));
        assert_eq!(
            (plots[6].line.color, plots[7].line.color),
            (Color::BLACK, r),
            "a second group starts at black again"
        );
        assert_eq!(plots[1].symbol.color, r);
    }

    #[test]
    fn a_grouped_column_plot_is_filled_from_the_list_and_keeps_its_outline() {
        let mut w = window();
        let rec = |flags: u8, y: u16| {
            let mut c = curve(y, 1, 203, [0xF7, 0xFF, 0xFF, 0xFF]);
            c.record[CURVE_GROUP_AT] = flags;
            c
        };
        w.layers[0].columns = vec![rec(0x29, 2), rec(0x19, 3)];
        let g = build(&w, &ids(), &mut Vec::new());
        let plots = &g.layers[0].plots;
        assert_eq!((plots[0].symbol.color, plots[1].symbol.color), (Color::BLACK, Color::rgb(255, 0, 0)));
        assert_eq!(plots[1].line.color, Color::BLACK);
        assert_eq!((plots[0].group, plots[1].group), (Some(1), Some(1)));
    }

    /// The owner's XRD figure: a group whose records hold black, red, blue and dark cyan (list
    /// positions 0, 1, 3, 12) is drawn with those colours by Origin, not with the base list.
    #[test]
    fn a_group_keeps_the_colours_written_in_its_records() {
        let mut w = window();
        let rec = |flags: u8, color: [u8; 4]| {
            let mut c = curve(2, 1, 200, color);
            c.record[CURVE_GROUP_AT] = flags;
            c
        };
        w.layers[0].columns = vec![
            rec(0x29, [0, 0, 0, 0]),
            rec(0x19, [1, 0, 0, 0]),
            rec(0x19, [3, 0, 0, 0]),
            rec(0x19, [12, 0, 0, 0]),
            rec(0x19, [0xF7, 0xFF, 0xFF, 0xFF]),
        ];
        let g = build(&w, &ids(), &mut Vec::new());
        let colors: Vec<Color> = g.layers[0].plots.iter().map(|p| p.line.color).collect();
        assert_eq!(
            colors,
            vec![Color::BLACK, Color::RED, Color::BLUE, Color::rgb(0, 128, 128), palette_color(4)],
            "written colours win; an automatic one takes the list colour of its place"
        );
    }

    /// A label turned 90 degrees starts at the top-left corner of its stored rectangle and runs up
    /// from it (measured in Origin on the owner's XRD figure: rectangle y 946..1187, text drawn from
    /// 667 to 932); the anchor is the middle of the first character, half the box width to the right.
    #[test]
    fn a_turned_label_starts_at_the_top_left_corner_of_its_rectangle() {
        let mut w = window();
        let mut a = text_annotation("Text8", "\\b(111)", 17, [1729, 946, 1897, 1187]);
        put_u16(&mut a.block1, ANN_ANGLE_AT, 900);
        let mut flat =
            text_annotation("Text9", "\\b(2\\g(q) \\f:Default: Arial((degree)))", 30, [2735, 2000, 4112, 2320]);
        put_u16(&mut flat.block1, ANN_ANGLE_AT, 0);
        w.layers[0].annotations = vec![a, flat];
        let g = build(&w, &ids(), &mut Vec::new());
        let l = &g.layers[0];
        let to_pt = |u: f64| u / PAGE_UNITS_PER_INCH * 72.0;
        let at = |k: usize| {
            let lb = &l.labels[k];
            (
                (l.frame.left + lb.x * l.frame.width) * g.page.width_pt,
                (l.frame.top + lb.y * l.frame.height) * g.page.height_pt,
            )
        };
        let (x, y) = at(0);
        assert!((x - to_pt(1729.0 + 84.0)).abs() < 1e-6, "{x}");
        assert!((y - to_pt(946.0)).abs() < 1e-6, "{y}");
        assert_eq!(l.labels[0].rotation_deg, 90.0);
        // An upright label starts at the rectangle's left edge, whatever escapes its text holds.
        let (x, y) = at(1);
        assert!((x - to_pt(2735.0)).abs() < 1e-6, "{x}");
        assert!((y - to_pt(2160.0)).abs() < 1e-6, "{y}");
    }

    /// A legend or text moved wholly off the page is not drawn by Origin, so it is not shown.
    #[test]
    fn a_legend_off_the_page_is_hidden() {
        let mut w = window();
        w.layers[0].annotations = vec![
            text_annotation("Legend", "\\l(1) %(1)", 18, [60000, 400, 61000, 700]),
            text_annotation("Text3", "away", 18, [60000, 400, 61000, 700]),
        ];
        let g = build(&w, &ids(), &mut Vec::new());
        assert!(!g.layers[0].legend.visible);
        assert!(g.layers[0].labels.is_empty());
    }

    #[test]
    fn documented_single_column_plot_types_are_read_as_their_kind() {
        // T223: LabTalk's plot-type IDs; a multi-column type (205 High-Low-Close) keeps the fallback.
        for (ptype, kind) in [
            (206, PlotKind::Box),
            (213, PlotKind::StackedColumn),
            (214, PlotKind::StackedArea),
            (215, PlotKind::Bar),
            (216, PlotKind::StackedBar),
            (219, PlotKind::Histogram),
            (225, PlotKind::Pie),
            (205, PlotKind::LineSymbol),
        ] {
            let mut w = window();
            w.layers[0].columns = vec![curve(2, 1, ptype, [3, 0, 0, 0])];
            let mut warnings = Vec::new();
            let g = build(&w, &ids(), &mut warnings);
            assert_eq!(g.layers[0].plots[0].kind, kind, "type {ptype}");
            let warned = warnings.iter().any(|w| w.contains(&format!("plot type {ptype}")));
            assert_eq!(warned, ptype == 205, "type {ptype}: {warnings:?}");
        }
    }

    #[test]
    fn only_a_lone_axis_token_is_the_automatic_title() {
        // T246: text around %(?X) is kept for the renderer to fill in.
        assert!(is_only_axis_token("%(?X)"));
        assert!(is_only_axis_token(" %(?Y,@LG) "));
        assert!(!is_only_axis_token("%(?X) shifted"));
        assert!(!is_only_axis_token("Time %(?X)"));
        assert!(!is_only_axis_token("Potential (V)"));
    }
}
