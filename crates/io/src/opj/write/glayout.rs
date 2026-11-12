//! The byte layout of the blocks of a written graph window (spec section 13.9).
//!
//! Like `layout.rs`, every block starts from the constant bytes that Origin 2016 wrote for a plain
//! graph (`graph_tables.rs`), and the fields that depend on the graph are put in on top. Offsets are
//! into the block body.

use super::super::strings::encode_cp1252;
use super::graph_tables::*;
use super::layout::{fill, framed, put, put_name};

/// The page and frame units of a graph: 1/600 inch.
pub(super) const UNITS_PER_INCH: f64 = 600.0;

/// A colour as the file stores it in four bytes: the position in Origin's colour list followed by
/// three zero bytes, red, green, blue and a 1 for a free colour, or "automatic".
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct Code(pub [u8; 4]);

impl Code {
    pub const AUTO: Code = Code([0xf7, 0xff, 0xff, 0xff]);

    pub fn index(i: u8) -> Code {
        Code([i, 0, 0, 0])
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Code {
        Code([r, g, b, 1])
    }

    /// The colour-list position when the code is one.
    pub fn position(&self) -> u8 {
        if self.0[3] == 0 { self.0[0] } else { 0 }
    }
}

// ----- window header ---------------------------------------------------------------------------

pub(super) struct WindowSpec<'a> {
    pub name: &'a str,
    pub label: &'a str,
    /// 0-based position among the windows written, which places the window on the screen.
    pub ordinal: usize,
    /// Page width and height in page units.
    pub page: (u16, u16),
    pub time: f64,
    /// A counter that Origin gives every window (3 for the first graph of a small project).
    pub serial: u8,
}

const WINDOW_LABEL_AT: usize = 0xC3;

pub(super) fn window_header(w: &WindowSpec) -> Vec<u8> {
    let (label, _) = encode_cp1252(w.label);
    let len = if label.is_empty() { WINDOW_LABEL_AT } else { WINDOW_LABEL_AT + label.len() + 1 };
    let mut h = vec![0u8; len];
    fill(&mut h, GRAPH_WINDOW_FIELDS);
    put_name(&mut h, 0x02, 25, w.name);
    let shift = 26 * (w.ordinal % 10) as u16;
    for (i, v) in [46 + shift, 46 + shift, 553 + shift, 395 + shift, w.page.0, w.page.1].into_iter().enumerate() {
        put(&mut h, 0x1B + 2 * i, &v.to_le_bytes());
    }
    put(&mut h, 0x3d, &w.page.0.to_le_bytes());
    put(&mut h, 0x3f, &w.page.1.to_le_bytes());
    put(&mut h, 0x73, &w.time.to_le_bytes());
    put(&mut h, 0x7b, &w.time.to_le_bytes());
    h[0xa1] = w.serial;
    put(&mut h, WINDOW_LABEL_AT, &label);
    h
}

// ----- layer header ----------------------------------------------------------------------------

/// What the layer header says about one axis.
#[derive(Clone, Copy, Debug)]
pub(super) struct AxisScale {
    pub from: f64,
    pub to: f64,
    pub step: f64,
    pub log: bool,
    pub minor: u8,
    /// The limits follow the data (Origin rescales them when the data change).
    pub auto: bool,
}

impl AxisScale {
    /// Number of major ticks the range holds.
    fn major_count(&self) -> u16 {
        let n = if self.log { (self.to / self.from).log10() } else { (self.to - self.from) / self.step };
        if n.is_finite() { n.round().clamp(1.0, 1000.0) as u16 } else { 1 }
    }

    /// Page units per unit of the axis (per decade on a logarithmic axis).
    fn scale(&self, extent: f64) -> f64 {
        let span = if self.log { (self.to / self.from).log10() } else { self.to - self.from };
        if span.is_finite() && span != 0.0 { extent / span } else { 0.0 }
    }
}

pub(super) struct LayerSpec {
    /// Left, top, right and bottom of the frame in page units.
    pub frame: [u16; 4],
    pub page: (u16, u16),
    pub x: AxisScale,
    pub y: AxisScale,
}

pub(super) fn layer_header(l: &LayerSpec) -> Vec<u8> {
    let mut h = vec![0u8; 361];
    fill(&mut h, GRAPH_LAYER_FIELDS);
    let frame_w = f64::from(l.frame[2].saturating_sub(l.frame[0]));
    let frame_h = f64::from(l.frame[3].saturating_sub(l.frame[1]));
    // (from, counts, flags, scale, minor, scale kind) of X and Y
    for (axis, at, count_at, minor_at, extent, sign) in
        [(&l.x, 0x0f, 0x2b, 0x37, frame_w, 1.0), (&l.y, 0x3a, 0x56, 0x62, frame_h, -1.0)]
    {
        put(&mut h, at, &axis.from.to_le_bytes());
        put(&mut h, at + 8, &axis.to.to_le_bytes());
        put(&mut h, at + 16, &axis.step.to_le_bytes());
        put(&mut h, count_at, &axis.major_count().to_le_bytes());
        h[count_at + 3] = u8::from(axis.auto);
        put(&mut h, count_at + 4, &(sign * axis.scale(extent)).to_le_bytes());
        h[minor_at] = axis.minor;
        h[minor_at + 1] = u8::from(axis.log);
    }
    for (i, v) in l.frame.iter().enumerate() {
        put(&mut h, 0x71 + 2 * i, &v.to_le_bytes());
    }
    // the rectangle around the frame that holds the tick labels and titles; Origin works it out again
    let outer = [
        l.frame[0].saturating_sub(728),
        l.frame[1].saturating_sub(354),
        l.frame[2].saturating_add(543).min(l.page.0),
        l.frame[3].saturating_add(537).min(l.page.1),
    ];
    for (i, v) in outer.iter().enumerate() {
        put(&mut h, 0x79 + 2 * i, &v.to_le_bytes());
    }
    h
}

// ----- axis blocks -----------------------------------------------------------------------------

/// What the six blocks of one axis say.
#[derive(Clone, Copy, Debug)]
pub(super) struct AxisBlocks {
    pub label_pt: u8,
    pub labels_shown: bool,
    /// Axis line width in 1/500 point.
    pub line_width: u16,
    /// Direction codes of the major and minor ticks: 0 none, 1 in, 2 out, 3 both.
    pub major_ticks: u8,
    pub minor_ticks: u8,
    pub opposite: bool,
    pub grid_major: bool,
    pub grid_minor: bool,
    pub line_color: Code,
    pub label_color: Code,
    /// The tick labels are the texts of the X column of the first plot.
    pub text_labels: bool,
}

const SHOWN: u8 = 0x40;

fn show(b: &mut [u8], on: bool) {
    if on {
        b[0x26] |= SHOWN;
    } else {
        b[0x26] &= !SHOWN;
    }
}

/// The six blocks of axis group X (`group` 0) or Y (1); the Z group has fixed blocks.
pub(super) fn axis_blocks(group: usize, a: &AxisBlocks) -> Vec<Vec<u8>> {
    let tables: [super::layout::Fields; 6] = match group {
        0 => [AXIS_X_MINOR_GRID, AXIS_X_MAJOR_GRID, AXIS_X_LABELS, AXIS_X_LINE, AXIS_X_TITLE, AXIS_X_OPPOSITE],
        1 => [AXIS_Y_MINOR_GRID, AXIS_Y_MAJOR_GRID, AXIS_Y_LABELS, AXIS_Y_LINE, AXIS_Y_TITLE, AXIS_Y_OPPOSITE],
        _ => [AXIS_Z_MINOR_GRID, AXIS_Z_MAJOR_GRID, AXIS_Z_LABELS, AXIS_Z_LINE, AXIS_Z_TITLE, AXIS_Z_OPPOSITE],
    };
    let mut blocks: Vec<Vec<u8>> = tables
        .iter()
        .map(|t| {
            let mut b = vec![0u8; 515];
            fill(&mut b, t);
            b
        })
        .collect();
    if group > 1 {
        return blocks;
    }
    show(&mut blocks[0], a.grid_minor);
    show(&mut blocks[1], a.grid_major);
    {
        let b = &mut blocks[2];
        b[0x15] = a.label_pt;
        show(b, a.labels_shown);
        if a.label_color != Code::AUTO && a.label_color != Code::index(0) {
            b[0x0f] = a.label_color.position();
            b[0x10] = 0;
            put(b, 0x92, &a.label_color.0);
            put(b, 0x16a, &a.label_color.0);
        }
    }
    {
        let b = &mut blocks[3];
        put(b, 0x15, &a.line_width.to_le_bytes());
        b[0x25] = (a.major_ticks & 3) << 4 | (a.minor_ticks & 3) << 6;
        if a.line_color != Code::index(0) {
            b[0x0f] = a.line_color.position();
            put(b, 0x92, &a.line_color.0);
            put(b, 0x16a, &a.line_color.0);
        }
    }
    if a.text_labels {
        blocks[2][0x23] = 1;
        blocks[2][0x25] = 1;
        blocks[4][0x23] = 1;
        blocks[4][0x25] = 1;
    }
    show(&mut blocks[5], a.opposite);
    blocks
}

// ----- curve records ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum CurveKind {
    Line,
    Scatter,
    LineSymbol,
    Column,
    /// A Y error plot attached to the curve whose Y data set is `x_set`.
    Error,
}

pub(super) struct CurveSpec {
    pub kind: CurveKind,
    /// 1-based position of the Y data set (of the error column for an error plot).
    pub y_set: u16,
    /// 1-based position of the X data set, 0 for none; for an error plot the Y data set of its curve.
    pub x_set: u16,
    /// Position of the first value and after the last value of the Y data set.
    pub used: (u32, u32),
    pub first: bool,
    pub line: Code,
    /// Line width in 1/500 point.
    pub width: u16,
    pub dash: u8,
    /// Line connection: 1 straight.
    pub connect: u8,
    pub shape: u8,
    /// Symbol size in 1/500 point.
    pub size: u16,
    pub open_symbol: bool,
    pub symbol: Code,
    /// Fill of a column.
    pub fill: Code,
    pub serial: u32,
}

pub(super) fn curve_record(c: &CurveSpec) -> Vec<u8> {
    let mut r = vec![0u8; 515];
    let (table, ptype) = match c.kind {
        CurveKind::Line => (CURVE_LINE, 200u16),
        CurveKind::Scatter => (CURVE_SCATTER, 201),
        CurveKind::LineSymbol => (CURVE_LINE_SYMBOL, 202),
        CurveKind::Column => (CURVE_COLUMN, 203),
        CurveKind::Error => (CURVE_ERROR, 487),
    };
    fill(&mut r, table);
    put(&mut r, 0x04, &c.y_set.to_le_bytes());
    put(&mut r, 0x07, &c.used.0.to_le_bytes());
    put(&mut r, 0x0b, &c.used.1.to_le_bytes());
    // not in a plot group: every curve keeps its own colour
    r[0x06] = 0x09;
    put(&mut r, 0x23, &c.x_set.to_le_bytes());
    put(&mut r, 0x4c, &ptype.to_le_bytes());
    put(&mut r, 0x1e2, &c.serial.to_le_bytes());
    r[0x1e9] = 8;
    if c.kind == CurveKind::Error {
        return r;
    }
    r[0x39] = if c.first { 0x20 } else { 0x60 };
    r[0x0f] = c.line.position();
    put(&mut r, 0x16a, &c.line.0);
    r[0x12] = c.dash;
    put(&mut r, 0x15, &c.width.to_le_bytes());
    if matches!(c.kind, CurveKind::Line | CurveKind::LineSymbol) {
        r[0x11] = c.connect;
    }
    if matches!(c.kind, CurveKind::Scatter | CurveKind::LineSymbol) {
        r[0x17] = c.shape;
        put(&mut r, 0x19, &c.size.to_le_bytes());
        r[0x18] = u8::from(c.open_symbol);
        put(&mut r, 0x132, &c.symbol.0);
    }
    if c.kind == CurveKind::Column {
        put(&mut r, 0xc2, &c.fill.0);
        r[0x1e] |= 0x10;
    }
    r
}

// ----- annotations -----------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum AnnKind {
    Text,
    XTitle,
    YTitle,
    Legend,
}

pub(super) struct AnnSpec<'a> {
    pub kind: AnnKind,
    /// The object name (`Text`, `Text1`, ...; the titles and the legend have fixed names).
    pub name: &'a str,
    /// The text in Windows-1252 with Origin's escapes.
    pub text: &'a [u8],
    /// Left, top, right and bottom in page units.
    pub rect: [u16; 4],
    /// Left and top as fractions of the layer frame.
    pub relative: (f64, f64),
    pub size_pt: u16,
    /// Angle in tenths of a degree, counter-clockwise.
    pub angle: u16,
    pub color: u8,
    /// Width and height of the text in page units.
    pub extent: (u16, u16),
    /// A box around a legend.
    pub frame: bool,
    pub serial: u16,
}

/// The four blocks of one annotation (header, style, text, empty).
pub(super) fn annotation(a: &AnnSpec) -> Vec<u8> {
    let (header_table, style_table) = match a.kind {
        AnnKind::Text => (ANN_TEXT_HEADER, ANN_TEXT_STYLE),
        AnnKind::XTitle => (ANN_XB_HEADER, ANN_XB_STYLE),
        AnnKind::YTitle => (ANN_YL_HEADER, ANN_YL_STYLE),
        AnnKind::Legend => (ANN_LEGEND_HEADER, ANN_LEGEND_STYLE),
    };
    let mut h = vec![0u8; 131];
    fill(&mut h, header_table);
    for (i, v) in a.rect.iter().enumerate() {
        put(&mut h, 0x03 + 2 * i, &v.to_le_bytes());
    }
    put(&mut h, 0x13, &a.relative.0.to_le_bytes());
    put(&mut h, 0x1b, &a.relative.1.to_le_bytes());
    put(&mut h, 0x25, &((a.text.len() + 10) as u16).to_le_bytes());
    put_name(&mut h, 0x46, 24, a.name);
    h[0x33] = a.color;
    h[0x6b] = a.color;
    put(&mut h, 0x67, &a.serial.to_le_bytes());
    let mut b1 = vec![0u8; 102];
    fill(&mut b1, style_table);
    put(&mut b1, 0x02, &a.angle.to_le_bytes());
    put(&mut b1, 0x04, &a.size_pt.to_le_bytes());
    put(&mut b1, 0x12, &a.extent.0.to_le_bytes());
    put(&mut b1, 0x14, &a.extent.1.to_le_bytes());
    if a.kind == AnnKind::Legend && !a.frame {
        // the legend without its box: the frame style and the background colour are "none"
        h[0x29] = 0;
        b1[0x1a..0x1e].copy_from_slice(&[0xfc, 0xff, 0xff, 0xff]);
        b1[0x28] = 0;
        b1[0x2a..0x2e].copy_from_slice(&[0xfc, 0xff, 0xff, 0xff]);
        b1[0x32..0x36].copy_from_slice(&[0, 0, 0, 0]);
    }
    let mut text = a.text.to_vec();
    text.extend_from_slice(&[0u8; 10]);
    let mut out = framed(&h);
    out.extend(framed(&b1));
    out.extend(framed(&text));
    out.extend(framed(&[]));
    out
}
