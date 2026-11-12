//! Graph windows of a written project: the model of `sheaf_core::graph` turned into the blocks of
//! `glayout.rs` (spec section 13.9). Everything of the model that the file cannot hold is named in
//! a warning.

use super::glayout::{
    AnnKind, AnnSpec, AxisBlocks, AxisScale, Code, CurveKind, CurveSpec, LayerSpec, UNITS_PER_INCH, WindowSpec,
    annotation, axis_blocks, curve_record, layer_header, window_header,
};
use super::layout::framed;
use super::{SetInfo, Writer, fail};
use crate::error::IoError;
use crate::opj::graph::PALETTE;
use crate::opj::strings::encode_cp1252;
use sheaf_core::Project;
use sheaf_core::color::Color;
use sheaf_core::graph::*;
use sheaf_core::ids::ColumnId;
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

/// Round numbers for an axis: `(from, to, step)` around `lo..hi`.
pub(crate) fn nice_range(lo: f64, hi: f64, log: bool, mode: AutoRange) -> (f64, f64, f64) {
    if log {
        let from = 10f64.powf(lo.max(f64::MIN_POSITIVE).log10().floor());
        let mut to = 10f64.powf(hi.max(from).log10().ceil());
        if to <= from {
            to = from * 10.0;
        }
        return (from, to, 1.0);
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
        let spec = WindowSpec {
            name: &name,
            label: &label,
            ordinal: self.written_windows,
            page,
            time: self.time,
            serial: 3u8.wrapping_add(self.graphs as u8),
        };
        self.graphs += 1;
        self.windows.extend(framed(&window_header(&spec)));
        for (index, layer) in graph.layers.iter().enumerate() {
            unwritten_layer_features(graph, index, &mut notes);
            // A linked layer's frame is a part of its parent's frame; Origin gets the place on the page.
            let placed;
            let layer = match graph.link_parent(index) {
                Some(parent) => {
                    placed = Layer { frame: within(graph.layers[parent].frame, layer.frame), ..layer.clone() };
                    &placed
                }
                None => layer,
            };
            let bytes = self.graph_layer(project, layer, page, &Sets { map: sets }, &mut notes);
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

    fn graph_layer(
        &mut self,
        project: &Project,
        layer: &Layer,
        page: (u16, u16),
        sets: &Sets,
        notes: &mut Vec<String>,
    ) -> Vec<u8> {
        // the plots that can be written
        struct Resolved<'a> {
            plot: &'a Plot,
            y: SetInfo,
            x: u16,
            err: Option<SetInfo>,
        }
        let mut plots: Vec<Resolved> = Vec::new();
        for p in &layer.plots {
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
            if p.symbol.every != 1 && p.kind.has_symbols() {
                notes.push("symbols on every n-th point only were not written; every point has a symbol".into());
            }
            plots.push(Resolved { plot: p, y, x, err });
        }

        // frame and axes
        let frame = sane_frame(layer.frame, notes);
        let left = unit_u16(frame.left * f64::from(page.0) / UNITS_PER_INCH * 72.0);
        let top = unit_u16(frame.top * f64::from(page.1) / UNITS_PER_INCH * 72.0);
        let right = unit_u16((frame.left + frame.width) * f64::from(page.0) / UNITS_PER_INCH * 72.0);
        let bottom = unit_u16((frame.top + frame.height) * f64::from(page.1) / UNITS_PER_INCH * 72.0);
        let plain: Vec<&Plot> = plots.iter().map(|r| r.plot).collect();
        let ranges = data_ranges(project, layer, &plain);
        // a text column as X gives categories: the plots use the row numbers and the axis shows the texts
        let text_x =
            plain.first().and_then(|p| p.x).and_then(|c| project.column(c)).and_then(|c| c.text().map(<[String]>::len));
        let mut x_scale = axis_scale(&layer.x_axis, ranges.0, notes);
        if let Some(n) = text_x.filter(|_| layer.x_axis.min.is_none() && layer.x_axis.max.is_none()) {
            x_scale.from = 0.5;
            x_scale.to = n as f64 + 0.5;
            x_scale.step = 1.0;
        }
        let y_scale = axis_scale(&layer.y_axis, ranges.1, notes);
        let mut out =
            framed(&layer_header(&LayerSpec { frame: [left, top, right, bottom], page, x: x_scale, y: y_scale }));

        // annotations: titles, free text, legend
        let frame_pt = FramePt {
            left: frame.left * f64::from(page.0) / UNITS_PER_INCH * 72.0,
            top: frame.top * f64::from(page.1) / UNITS_PER_INCH * 72.0,
            width: frame.width * f64::from(page.0) / UNITS_PER_INCH * 72.0,
            height: frame.height * f64::from(page.1) / UNITS_PER_INCH * 72.0,
        };
        let mut annotations: Vec<Vec<u8>> = Vec::new();
        for (kind, axis) in [(AnnKind::XTitle, &layer.x_axis), (AnnKind::YTitle, &layer.y_axis)] {
            if !axis.show_title {
                continue;
            }
            let auto = if kind == AnnKind::XTitle { "%(?X,@LG)" } else { "%(?Y,@LG)" };
            let text = axis.title.clone().unwrap_or_else(|| auto.to_string());
            let text = styled(&text, &axis.title_font);
            let size = axis.title_font.size_pt;
            let (w, h) = text_size(&text, size);
            let (cx, cy, angle) = match kind {
                AnnKind::XTitle => (
                    frame_pt.left + frame_pt.width / 2.0,
                    frame_pt.top + frame_pt.height + axis.tick_font.size_pt * 1.6 + h,
                    0u16,
                ),
                _ => {
                    (frame_pt.left - axis.tick_font.size_pt * 3.4 - h / 2.0, frame_pt.top + frame_pt.height / 2.0, 900)
                }
            };
            let (rw, rh) = if angle == 0 { (w, h) } else { (h, w) };
            let name = if kind == AnnKind::XTitle { "XB" } else { "YL" };
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
        let mut used_names: Vec<String> = Vec::new();
        for (i, l) in layer.labels.iter().enumerate() {
            let text = styled(&l.text, &l.font);
            let (w, h) = text_size(&text, l.font.size_pt);
            let cx = frame_pt.left + l.x * frame_pt.width + w / 2.0;
            let cy = frame_pt.top + l.y * frame_pt.height;
            let angle = (((l.rotation_deg % 360.0) + 360.0) % 360.0 * 10.0).round() as u16;
            let rotated = (450..1350).contains(&angle) || (2250..3150).contains(&angle);
            let (rw, rh) = if rotated { (h, w) } else { (w, h) };
            let name = if i == 0 { "Text".to_string() } else { format!("Text{i}") };
            used_names.push(name.clone());
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
        if layer.legend.visible && !plots.is_empty() {
            let mut lines = Vec::new();
            for (i, r) in plots.iter().enumerate() {
                match &r.plot.label {
                    Some(t) => lines.push(format!("\\l({}) {}", i + 1, t.replace(['\r', '\n'], " "))),
                    None => lines.push(format!("\\l({}) %({})", i + 1, i + 1)),
                }
            }
            let text = lines.join("\r\n");
            let size = layer.legend.font.size_pt;
            let longest =
                plots.iter().map(|r| r.plot.label.as_ref().map_or(12, |t| t.chars().count())).max().unwrap_or(1);
            let w = CHAR_WIDTH * size * longest as f64 + 2.6 * size;
            let h = LINE_HEIGHT * size * plots.len() as f64;
            let margin = 0.4 * size;
            let (cx, cy) = match layer.legend.position {
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
        out.extend(framed(&[]));

        // curves
        let mut first = true;
        for r in &plots {
            let p = r.plot;
            let kind = match p.kind {
                PlotKind::Line => CurveKind::Line,
                PlotKind::Scatter => CurveKind::Scatter,
                PlotKind::LineSymbol => CurveKind::LineSymbol,
                PlotKind::Column => CurveKind::Column,
            };
            let connect = match p.line.connect {
                Connect::Straight => 1,
                Connect::Spline => 5,
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
                serial: self.next_serial(),
            };
            first = false;
            out.extend(framed(&curve_record(&spec)));
            out.extend(framed(&[]));
            if let Some(err) = r.err {
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
                    serial: self.next_serial(),
                };
                out.extend(framed(&curve_record(&spec)));
                out.extend(framed(&[]));
            }
        }
        // the end of the curves and the extra block
        out.extend(framed(&[]));
        out.extend(framed(&[]));

        // the three groups of axis blocks
        for (group, axis) in [(0, &layer.x_axis), (1, &layer.y_axis)] {
            let mut spec = axis_spec(axis, notes);
            spec.text_labels = group == 0 && text_x.is_some();
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
        let _ = used_names;
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
fn data_ranges(project: &Project, layer: &Layer, plots: &[&Plot]) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
    let mut x: Option<(f64, f64)> = None;
    let mut y: Option<(f64, f64)> = None;
    let grow = |r: &mut Option<(f64, f64)>, v: f64| {
        if v.is_finite() {
            *r = Some(r.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        }
    };
    let log_x = layer.x_axis.scale == ScaleKind::Log10;
    let log_y = layer.y_axis.scale == ScaleKind::Log10;
    for p in plots {
        let ys = project.numeric(p.y).unwrap_or(&[]);
        let errs = p.y_error.and_then(|e| project.numeric(e));
        for (i, v) in ys.iter().enumerate() {
            if !v.is_finite() || (log_y && *v <= 0.0) {
                continue;
            }
            let e = errs.and_then(|e| e.get(i)).copied().filter(|e| e.is_finite()).unwrap_or(0.0).abs();
            grow(&mut y, *v);
            if !log_y {
                grow(&mut y, v - e);
                grow(&mut y, v + e);
            }
        }
        if p.kind == PlotKind::Column {
            grow(&mut y, 0.0);
        }
        match p.x.and_then(|c| project.numeric(c)) {
            Some(xs) => {
                for v in xs {
                    if !log_x || *v > 0.0 {
                        grow(&mut x, *v);
                    }
                }
            }
            None => {
                grow(&mut x, 1.0);
                grow(&mut x, ys.len().max(1) as f64);
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

/// Notes for the parts of layer `index` that the writer does not store yet (the double-Y, stack,
/// column and panel features of graphing spec sections 12 to 15). Each note makes the save keep a
/// companion file with the whole project.
fn unwritten_layer_features(graph: &Graph, index: usize, notes: &mut Vec<String>) {
    let layer = &graph.layers[index];
    if graph.link_parent(index).is_some() {
        notes.push(
            "a layer linked to another one (the right Y axis of a double-Y graph, or a panel of a stack) was written as an independent layer at the same place; the link and the shared axis are not kept"
                .into(),
        );
    }
    if layer.stack != Stack::None || !layer.plot_offsets.is_empty() {
        notes.push(
            "offsets of stacked or waterfall curves were not written; the curves are drawn where their data lies"
                .into(),
        );
    }
    if layer.bars != BarSpacing::default() {
        notes.push("the gap and overlap of column bars were not written; Origin's defaults are used".into());
    }
    if layer.legend.scope != LegendScope::Layer {
        notes
            .push("a legend that lists the plots of several layers was written with its own layer's plots only".into());
    }
    for axis in [&layer.x_axis, &layer.y_axis] {
        if axis.side != AxisSide::Normal {
            notes.push("an axis on the opposite side, or a hidden axis, was written as a normal axis".into());
        }
        if axis.tick_label_source != TickLabelSource::Auto {
            notes.push("the choice of tick labels (numbers, or the cells of a column) was not written".into());
        }
        if axis.tick_label_angle_deg != 0.0 {
            notes.push("rotated tick labels were not written".into());
        }
        if axis.hide_end_label {
            notes.push("a hidden tick label at the end of an axis was not written".into());
        }
        if axis.color_follows_plot || axis.title_color_follows_plot {
            notes.push("axis colours that follow the first plot's colour were not written".into());
        }
    }
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
    let (dlo, dhi) = data.unwrap_or(if log { (1.0, 10.0) } else { (0.0, 1.0) });
    let (alo, ahi, astep) = nice_range(dlo, dhi, log, axis.auto_range);
    let (mut from, mut to) = (axis.min.unwrap_or(alo), axis.max.unwrap_or(ahi));
    if log && (from <= 0.0 || to <= 0.0) {
        from = alo;
        to = ahi;
    }
    if to.partial_cmp(&from) != Some(std::cmp::Ordering::Greater) {
        (from, to) = (alo, ahi);
    }
    let step = if log { 1.0 } else { axis.major_step.filter(|s| *s > 0.0).unwrap_or(astep) };
    if axis.auto_range != AutoRange::Nice && (axis.min.is_none() || axis.max.is_none()) {
        notes.push(
            "an axis range that follows the data in another way than Origin's own (tight or symmetric about zero) was written as the limits it gives for the current data"
                .into(),
        );
    }
    AxisScale {
        from,
        to,
        step,
        log,
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
    if axis.tick_format != TickFormat::Auto {
        notes.push("a tick label number format was not written; Origin's automatic format is used".into());
    }
    if (axis.tick_label_divisor - 1.0).abs() > f64::EPSILON {
        notes.push("a tick label divisor was not written".into());
    }
    if (axis.major_tick_len_pt - 5.0).abs() > 1e-9 || (axis.minor_tick_len_pt - 2.5).abs() > 1e-9 {
        notes.push("tick lengths were not written; Origin's default lengths are used".into());
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
    AxisBlocks {
        label_pt: axis.tick_font.size_pt.round().clamp(1.0, 255.0) as u8,
        labels_shown: axis.show_tick_labels,
        line_width: (axis.line_width_pt * 500.0).round().clamp(1.0, f64::from(u16::MAX)) as u16,
        major_ticks: tick_code(axis.major_tick),
        minor_ticks: tick_code(axis.minor_tick),
        opposite: axis.show_opposite,
        grid_major: axis.major_grid.is_some(),
        grid_minor: axis.minor_grid.is_some(),
        line_color: code_of(axis.color),
        label_color: code_of(axis.tick_font.color),
        text_labels: false,
    }
}
