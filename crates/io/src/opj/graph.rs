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
const AXIS_OPPOSITE: usize = 5;
const AXIS_FONT_AT: usize = 0x15;
const AXIS_FLAGS_AT: usize = 0x26;
const AXIS_LINE_WIDTH_AT: usize = 0x15;
const AXIS_TICKS_AT: usize = 0x25;
// Curve record.
const CURVE_Y_AT: usize = 0x04;
const CURVE_CONNECT_AT: usize = 0x11;
const CURVE_DASH_AT: usize = 0x12;
const CURVE_WIDTH_AT: usize = 0x15;
const CURVE_SHAPE_AT: usize = 0x17;
const CURVE_SIZE_AT: usize = 0x19;
const CURVE_X_AT: usize = 0x23;
const CURVE_TYPE_AT: usize = 0x4C;
const CURVE_FILL_AT: usize = 0xC2;
const CURVE_FILL_FLAG_AT: usize = 0xC5;
const CURVE_LINE_COLOR_AT: usize = 0x132;
// Annotation: header holds the rectangle and the name, block 1 the text style, block 2 the text.
const ANN_RECT_AT: usize = 0x03;
const ANN_NAME_AT: usize = 0x46;
const ANN_FONT_AT: usize = 0x04;
const ANN_ANGLE_AT: usize = 0x02;

/// Origin's colour list: the number stored in a curve record is the position in this list.
pub(crate) const PALETTE: [(u8, u8, u8); 22] = [
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

/// True for a window that holds a graph: its layers carry axis blocks (workbooks and
/// matrices have none).
pub(crate) fn is_graph(w: &Window) -> bool {
    w.layers.iter().any(|l| l.axes.first().is_some_and(|g| g.len() >= 6))
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
        };
    } else {
        notes.push("the page size could not be read; a default size was used".into());
    }
    let template = graph.layers.remove(0);
    for layer in &w.layers {
        graph.layers.push(build_layer(layer, &template, &graph.page, column_of_set, &mut notes));
    }
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
    template: &Layer,
    page: &Page,
    column_of_set: &HashMap<usize, ColumnId>,
    notes: &mut Vec<String>,
) -> Layer {
    let mut out = template.clone();
    out.palette = Palette { colors: (0..PALETTE.len()).map(palette_color).collect(), mode: PaletteMode::Cycle };
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
        match u8_at(h, scale_at) {
            0 => {
                if step.is_finite() && step > 0.0 {
                    axis.major_step = Some(step);
                }
                axis.minor_ticks = u32::from(u8_at(h, minor_at)).min(10);
            }
            1 => {
                axis.scale = ScaleKind::Log10;
                axis.minor_ticks = 8;
            }
            n => notes.push(format!("scale type {n} is not supported and is drawn as linear")),
        }
        if axis.scale == ScaleKind::Log10 && !axis.min.is_some_and(|m| m > 0.0) {
            axis.min = None;
            axis.max = None;
        }
        if let Some(blocks) = layer.axes.get(group) {
            apply_axis_blocks(axis, blocks);
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
                // Only the bottom and left titles are drawn by this model.
                let axis = match name.as_str() {
                    "XB" => &mut out.x_axis,
                    "YL" => &mut out.y_axis,
                    _ => {
                        notes.push("a title on the top or right axis was not imported".into());
                        continue;
                    }
                };
                axis.title = Some(clean_markup(&text));
                axis.show_title = true;
                axis.title_font.size_pt = size_pt;
            }
            "Legend" => {
                out.legend.visible = true;
                out.legend.font.size_pt = size_pt;
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
                if text.trim().is_empty() || frame_size.0 <= 0.0 || frame_size.1 <= 0.0 {
                    continue;
                }
                push_text_label(&mut out, a, &text, size_pt, page_units, frame_pt, frame_size);
            }
        }
    }

    // Curves.
    let mut y_sets: Vec<usize> = Vec::new();
    for (i, rec) in layer.columns.iter().enumerate() {
        read_curve(&mut out, &mut y_sets, i, rec, column_of_set, notes);
    }
    out
}

fn apply_axis_blocks(axis: &mut Axis, blocks: &[Vec<u8>]) {
    if let Some(b) = blocks.get(AXIS_LABELS) {
        let size = u8_at(b, AXIS_FONT_AT);
        if (4..=96).contains(&size) {
            axis.tick_font.size_pt = f64::from(size);
            axis.title_font.size_pt = f64::from(size);
        }
        axis.show_tick_labels = u8_at(b, AXIS_FLAGS_AT) & 0x40 != 0;
    }
    if let Some(b) = blocks.get(AXIS_LINE) {
        let width = f64::from(u16_at(b, AXIS_LINE_WIDTH_AT)) / CURVE_UNITS_PER_POINT;
        if width > 0.0 && width < 20.0 {
            axis.line_width_pt = width;
        }
        let code = u8_at(b, AXIS_TICKS_AT);
        axis.major_tick = tick_direction((code >> 4) & 3);
        axis.minor_tick = tick_direction((code >> 6) & 3);
    }
    if let Some(b) = blocks.get(AXIS_OPPOSITE) {
        let on = u8_at(b, AXIS_FLAGS_AT) & 0x40 != 0;
        axis.show_opposite = on;
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

fn annotation_name(a: &WinAnnotation) -> String {
    match a.header.get(ANN_NAME_AT..) {
        Some(rest) => decode_cp1252(c_bytes(&rest[..rest.len().min(24)])),
        None => String::new(),
    }
}

/// The text of a text object: the bytes of block 2 up to the first NUL.
fn annotation_text(a: &WinAnnotation) -> String {
    decode_cp1252(c_bytes(&a.block2))
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

/// A free text object becomes one label per line of text (the model draws single lines), each
/// line starting at an estimated left edge so that the text is centred on the object.
fn push_text_label(
    out: &mut Layer,
    a: &WinAnnotation,
    text: &str,
    size_pt: f64,
    page_units: (f64, f64),
    frame_pt: (f64, f64),
    frame_size: (f64, f64),
) {
    let (cx, cy) = annotation_center(a, page_units);
    let (cx, cy) = (cx / PAGE_UNITS_PER_INCH * 72.0, cy / PAGE_UNITS_PER_INCH * 72.0);
    let angle = f64::from(u16_at(&a.block1, ANN_ANGLE_AT)) / 10.0;
    let lines: Vec<String> = text.split(['\r', '\n']).filter(|l| !l.trim().is_empty()).map(clean_markup).collect();
    let line_height = 1.2 * size_pt;
    let first = cy - line_height * (lines.len() as f64 - 1.0) / 2.0;
    for (k, line) in lines.iter().enumerate() {
        let plain = strip_escapes(line);
        let width = 0.58 * size_pt * plain.chars().count() as f64;
        let left = cx - width / 2.0;
        let mid = first + line_height * k as f64;
        let mut font = out.legend.font.clone();
        font.size_pt = size_pt;
        out.labels.push(TextLabel {
            text: line.clone(),
            x: (left - frame_pt.0) / frame_size.0,
            y: (mid - frame_pt.1) / frame_size.1,
            font,
            rotation_deg: angle,
        });
    }
}

/// Drops the size escape `\p<digits>(...)` (the model has no relative text size) and keeps the
/// escapes it can draw.
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
fn strip_escapes(s: &str) -> String {
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

/// A colour written as four bytes: a palette position (last byte 0), a red, green, blue triple
/// (last byte 1), or "automatic" (anything else).
fn color4(b: &[u8], at: usize, automatic: usize) -> Color {
    let g = |k: usize| u8_at(b, at + k);
    match g(3) {
        0 => palette_color(usize::from(g(0))),
        1 => Color::rgb(g(0), g(1), g(2)),
        _ => palette_color(automatic),
    }
}

fn symbol_shape(code: u8, notes: &mut Vec<String>) -> SymbolShape {
    match code {
        1 => SymbolShape::Square,
        2 => SymbolShape::Circle,
        3 => SymbolShape::TriangleUp,
        4 => SymbolShape::TriangleDown,
        5 => SymbolShape::Diamond,
        6 => SymbolShape::Plus,
        7 => SymbolShape::Cross,
        8 => SymbolShape::Star,
        _ => {
            notes.push("some symbol shapes are not known and are drawn as squares".into());
            SymbolShape::Square
        }
    }
}

fn read_curve(
    layer: &mut Layer,
    y_sets: &mut Vec<usize>,
    index: usize,
    rec: &WinColumn,
    column_of_set: &HashMap<usize, ColumnId>,
    notes: &mut Vec<String>,
) {
    let r = &rec.record;
    if r.len() < CURVE_LINE_COLOR_AT + 4 {
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
        _ => {
            notes.push(format!("plot type {ptype} is not supported and is drawn as a line with symbols"));
            PlotKind::LineSymbol
        }
    };
    let line_color = color4(r, CURVE_LINE_COLOR_AT, index);
    let width = f64::from(u16_at(r, CURVE_WIDTH_AT)) / CURVE_UNITS_PER_POINT;
    let width = if width > 0.0 && width < 20.0 { width } else { 1.0 };
    let dash_code = usize::from(u8_at(r, CURVE_DASH_AT));
    let dash = Dash::ALL.get(dash_code).copied().unwrap_or(Dash::Solid);
    let size = f64::from(u16_at(r, CURVE_SIZE_AT)) / CURVE_UNITS_PER_POINT;
    let size = if size > 0.0 && size < 60.0 { size } else { 5.0 };
    let shape = if shape_code == 0 { SymbolShape::Square } else { symbol_shape(shape_code, notes) };
    let fill = if kind == PlotKind::Column {
        if u8_at(r, CURVE_FILL_FLAG_AT) == 1 {
            Color::rgb(u8_at(r, CURVE_FILL_AT), u8_at(r, CURVE_FILL_AT + 1), u8_at(r, CURVE_FILL_AT + 2))
        } else {
            palette_color(index)
        }
    } else {
        line_color
    };
    let auto = u8_at(r, CURVE_LINE_COLOR_AT + 3) != 0 && u8_at(r, CURVE_LINE_COLOR_AT + 3) != 1;
    let mut plot = Plot {
        x: column(x_set),
        y,
        y_error: None,
        rows: None,
        kind,
        line: LineStyle { color: line_color, width_pt: width, dash, connect: Connect::Straight },
        symbol: SymbolStyle { shape, size_pt: size, color: fill, filled: true, edge_width_pt: 0.75, every: 1 },
        label: None,
        visible: true,
        auto_color: false,
    };
    if kind == PlotKind::Column {
        // Bars are outlined in black unless the line colour was chosen.
        plot.line.color = if auto { Color::BLACK } else { line_color };
        plot.line.width_pt = width.min(2.0);
    }
    if x_set != 0 && column(x_set).is_none() {
        notes.push("a plot's X data is not a worksheet column; it was plotted against the row number".into());
    }
    let _ = connect;
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
        Window { name: "Graph7".into(), label: "My figure".into(), layers: vec![layer], header: window_header() }
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
        assert_eq!(palette_color(22), Color::BLACK);
    }
}
