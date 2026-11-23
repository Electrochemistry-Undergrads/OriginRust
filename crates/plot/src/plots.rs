//! Drawing the data of one plot: lines, error bars and symbols (spec section 4).

use crate::LayerGeometry;
use crate::Rect;
use crate::data::PlotData;
use crate::fill::{fill_polygon, fill_rect};
use crate::fonts::TextMeasurer;
use crate::scene::{Anchor, Cap, Item, Join, Stroke, TextItem, line_to, move_to, push_num};
use crate::{markup, text};
use sheaf_core::graph::{
    Connect, DataLabels, DropColorMode, DropTarget, ErrorDirection, FillUnderMode, LabelForm, LabelFrame,
    LabelPosition, LineStyle, PLUS_MINUS_NEGATIVE, PLUS_MINUS_POSITIVE, PlotKind, ScaleKind, SymbolInterior,
    SymbolShape, SymbolStyle,
};

/// Points closer than this (in pt) to the last kept point are dropped.
const DECIMATE_PT: f64 = 0.05;
/// Width (in pt) of the vertical strips used to thin out dense curves: of a
/// run of consecutive points inside one strip only the first, lowest, highest
/// and last are drawn. 0.1 pt is less than one pixel up to 720 dpi.
const STRIP_PT: f64 = 0.1;
/// Runs of up to this many points inside one strip are drawn point by point.
const RUN_EXACT: usize = 4;
/// Plots with more symbols than this leave out symbols that coincide.
const SYMBOL_THIN_ABOVE: usize = 20_000;
/// Two symbols in the same cell of this size (in pt) count as coinciding.
const SYMBOL_CELL_PT: f64 = 0.5;
/// A plot with more points than this is drawn with straight segments instead of a spline (Origin
/// does the same, measured: 900 points are splined, 901 are not, even when zoomed in).
const SPLINE_MAX_POINTS: usize = 900;
/// Smooth connections are drawn as short chords of about this length (pt).
const SMOOTH_STEP_PT: f64 = 2.0;
/// At most this many chords per curve segment.
const SMOOTH_MAX_SUB: usize = 128;

/// The stroke of a data line.
pub(crate) fn line_stroke(line: &LineStyle) -> Stroke {
    let dash = crate::dash::lengths(line.dash, line.width_pt);
    let cap = if dash.is_empty() { Cap::Round } else { Cap::Butt };
    Stroke { color: line.color, width: line.width_pt, dash, cap, join: Join::Round }
}

/// Appends the items of one plot to `out`.
///
/// `next` is the plot drawn after this one in the layer (the first plot for the last one; none
/// when the last plot's partner would only repeat a two-plot segment), the target of drop lines
/// set to Next Data Plot.
pub(crate) fn draw_plot(
    out: &mut Vec<Item>,
    pd: &PlotData,
    geom: &LayerGeometry,
    next: Option<&PlotData>,
    m: &dyn TextMeasurer,
    label_plot_color: &impl Fn(usize) -> Option<sheaf_core::Color>,
) {
    let plot = pd.plot;
    if let Some(markers) = plot.decor.markers
        && pd.marker_display.shows(markers, pd.start, pd.end)
    {
        draw_data_markers(out, pd, geom, markers);
    }
    if plot.kind == PlotKind::Piper {
        // Origin's Piper axis labels (T389) in a 10 pt Arial.
        crate::piper_plot::draw(out, pd, geom, &sheaf_core::graph::Font::new("Arial", 10.0));
        return;
    }
    if plot.kind == PlotKind::Kde2d {
        crate::kde2d::draw(out, pd, geom);
        return;
    }
    if plot.kind == PlotKind::WindRose {
        crate::wind_rose::draw(out, pd, geom);
        return;
    }
    if plot.kind.is_qc() {
        crate::qc_plot::draw_qc(out, pd, geom);
        return;
    }
    if plot.kind.is_marginal() {
        crate::marginal_plot::draw_marginal(out, pd, geom);
        return;
    }
    if plot.kind.is_stock() {
        crate::stock_plot::draw_stock(out, pd, geom);
        return;
    }
    if plot.kind.is_vector() {
        crate::vector_plot::draw_vectors(out, pd, geom);
        return;
    }
    if plot.kind.is_parallel() {
        crate::parallel::draw_parallel(out, pd, geom);
        return;
    }
    if plot.kind == PlotKind::Treemap {
        crate::treemap::draw_treemap(out, pd, geom, m);
        return;
    }
    if plot.kind == PlotKind::Sunburst {
        crate::sunburst::draw_sunburst(out, pd, geom, m);
        return;
    }
    if plot.kind == PlotKind::Sankey {
        crate::sankey::draw_sankey(out, pd, geom, m);
        return;
    }
    if plot.kind == PlotKind::Pie {
        draw_pie(out, pd, geom);
        return;
    }
    if plot.kind == PlotKind::Pie3D {
        crate::pie::draw_pie_3d(out, pd, geom);
        return;
    }
    if plot.kind == PlotKind::PieBw {
        crate::pie::draw_pie_bw(out, pd, geom);
        return;
    }
    if pd.surface.is_some() {
        crate::space3d::draw_surface(out, pd, geom);
        return;
    }
    if pd.ribbon.is_some() {
        crate::space3d::draw_ribbon(out, pd, geom);
        return;
    }
    if plot.kind.is_area() {
        draw_area(out, pd, geom);
    }
    let decor = &plot.decor;
    if let Some(color) = decor.fill_under
        && plot.kind.has_line()
        && !plot.kind.is_area()
    {
        draw_fill_under(out, pd, geom, color);
    }
    // The Drop Lines tab is only offered for line and symbol plots (not bars or areas); the
    // other kinds draw none, whatever was set while the plot was of a kind that has them.
    let offers_drop_lines =
        (plot.kind.has_line() || plot.kind.has_symbols()) && !plot.kind.is_bar() && !plot.kind.is_area();
    if offers_drop_lines && (decor.drop.horizontal.on || decor.drop.vertical.on) {
        draw_drop_lines(out, pd, geom, next);
    }
    let line_in_front =
        plot.kind == PlotKind::LineSymbol && decor.symbol_line.draw_in_front && !decor.symbol_line.gap_to_symbol;
    if plot.kind.has_line() && plot.line.width_pt > 0.0 && !line_in_front {
        draw_line_or_mapped(out, pd, geom);
    }
    if plot.kind == PlotKind::Box {
        draw_box(out, pd, geom);
    } else if plot.kind.is_violin() {
        crate::violin::draw_violin(out, pd, geom);
    } else if plot.kind.is_ridgeline() {
        crate::ridgeline::draw_ridge(out, pd, geom);
    } else if plot.kind.is_hexbin() {
        crate::hexbin::draw_hexbin(out, pd, geom);
    } else if plot.kind.is_bar() {
        draw_columns(out, pd, geom);
    }
    if pd.err.is_some() || pd.xerr.is_some() {
        draw_error_bars(out, pd, geom);
    }
    if plot.kind.has_symbols() {
        draw_symbols(out, pd, geom);
    }
    if line_in_front && plot.line.width_pt > 0.0 {
        draw_line_or_mapped(out, pd, geom);
    }
    if pd.mask.is_some() && !plot.kind.is_bar() {
        draw_masked(out, pd, geom);
    }
    if decor.labels.enable && !plot.kind.is_distribution() {
        draw_data_labels(out, pd, geom, m, label_plot_color);
    }
}

/// Filled modes of "Fill Area Under Curve": Normal fills to the frame bottom (not Y = 0),
/// while Inclusive closes each finite run directly between its endpoints. A missing point
/// breaks the area, as it breaks the line.
fn draw_fill_under(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, color: sheaf_core::Color) {
    if pd.plot.decor.fill_under_mode.is_sidelines() {
        // The outline modes are part of the connecting stroke, avoiding double opacity at joins.
        return;
    }
    let bottom = geom.frame.y + geom.frame.h;
    let style = pd.plot.decor.fill;
    let inclusive = pd.plot.decor.fill_under_mode == FillUnderMode::InclusiveBrokenByMissingValues;
    let connect = match pd.plot.line.connect {
        Connect::Spline if !spline_applies(pd) => Connect::Straight,
        c => c,
    };
    let mut d = String::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    let close = |run: &mut Vec<(f64, f64)>, d: &mut String, out: &mut Vec<Item>| {
        if run.len() >= 2 {
            if inclusive {
                if style.is_plain() {
                    polyline_path(d, run, connect);
                    d.push('Z');
                } else {
                    let mut curve = String::new();
                    polyline_path(&mut curve, run, connect);
                    fill_polygon(out, &crate::hit::path_points(&curve), color, &style);
                }
            } else if style.is_plain() {
                move_to(d, run[0].0, bottom);
                for &(x, y) in run.iter() {
                    line_to(d, x, y);
                }
                line_to(d, run[run.len() - 1].0, bottom);
                d.push('Z');
            } else {
                let mut poly = vec![(run[0].0, bottom)];
                poly.extend(run.iter().copied());
                poly.push((run[run.len() - 1].0, bottom));
                fill_polygon(out, &poly, color, &style);
            }
        }
        run.clear();
    };
    for i in pd.line_rows() {
        match geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) {
            Some(p) => run.push(p),
            None => close(&mut run, &mut d, out),
        }
    }
    close(&mut run, &mut d, out);
    if !d.is_empty() {
        out.push(Item::Path { d, fill: Some(color), stroke: None });
    }
}

/// Drop lines: from each point across to the left edge of the frame (the Y axis, "Horizontal")
/// and down to its bottom edge (the X axis, "Vertical"); or, with Drop To = Next Data Plot, to
/// the point of `next` (the next plot of the layer) at the same row.
///
/// With the "Y Value: Plus-Minus" colour each segment takes one of two fixed colours by the sign
/// of the value (Sheaf's own, unmeasured): for a drop to the axis, the plotted Y value; for a drop
/// to the next plot, the coordinate of the segment's start along the direction the segment runs
/// (the horizontal value when it runs sideways, as in an exchanged lollipop). A point or a
/// partner point that is missing, NaN or masked gives no segment.
fn draw_drop_lines(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, next: Option<&PlotData>) {
    let f = geom.frame;
    let auto = pd.plot.line.color;
    let drops = &pd.plot.decor.drop;
    for (l, horizontal) in [(&drops.horizontal, true), (&drops.vertical, false)] {
        if !l.on {
            continue;
        }
        // Segments in page coordinates with the data value that Plus-Minus looks at.
        let mut segs: Vec<((f64, f64), (f64, f64), f64)> = Vec::new();
        // The rows the next plot draws a symbol at (row range, row filters, skip points): a
        // segment ends only at a point that is drawn.
        let partner_rows: std::collections::BTreeSet<usize> = match next {
            Some(n) if l.target == DropTarget::NextDataPlot => n.point_rows().collect(),
            _ => Default::default(),
        };
        for i in pd.point_rows() {
            let Some((x, y)) = geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) else { continue };
            if l.target == DropTarget::NextDataPlot {
                let Some(n) = next else { continue };
                let masked = |d: &PlotData| d.mask.is_some_and(|m| m.contains(&i));
                if !partner_rows.contains(&i) || i >= n.y.len() || masked(pd) || masked(n) {
                    continue;
                }
                let Some((nx, ny)) = geom.checked_data_to_page(n.x_at(i), n.y_at(i)) else { continue };
                if (nx, ny) == (x, y) {
                    continue;
                }
                let sideways = (nx - x).abs() > (ny - y).abs();
                let value = if sideways { pd.x_at(i) } else { pd.y_at(i) };
                segs.push(((x, y), (nx, ny), value));
            } else if horizontal {
                segs.push(((f.x, y), (x, y), pd.y_at(i)));
            } else {
                segs.push(((x, y), (x, f.y + f.h), pd.y_at(i)));
            }
        }
        let stroke = |color: sheaf_core::Color| {
            line_stroke(&LineStyle {
                color,
                width_pt: l.width_pt,
                dash: l.dash,
                connect: Connect::Straight,
                sort_x: false,
            })
        };
        let path = |keep: &dyn Fn(f64) -> bool| {
            let mut d = String::new();
            for &((x0, y0), (x1, y1), v) in &segs {
                if keep(v) {
                    move_to(&mut d, x0, y0);
                    line_to(&mut d, x1, y1);
                }
            }
            d
        };
        match (l.color, l.color_mode) {
            (None, DropColorMode::PlusMinus) => {
                for (color, d) in
                    [(PLUS_MINUS_POSITIVE, path(&|v| v >= 0.0)), (PLUS_MINUS_NEGATIVE, path(&|v| v < 0.0))]
                {
                    if !d.is_empty() {
                        out.push(Item::Path { d, fill: None, stroke: Some(stroke(color)) });
                    }
                }
            }
            _ => {
                let d = path(&|_| true);
                if !d.is_empty() {
                    out.push(Item::Path { d, fill: None, stroke: Some(stroke(l.color.unwrap_or(auto))) });
                }
            }
        }
    }
}

/// The text of a data label: the value as the worksheet shows it by default (up to 15
/// significant digits, scientific beyond).
fn label_number(v: f64) -> String {
    sheaf_core::format::format_general(v, 15)
}

/// Data labels (Origin's Label tab): the label form's text next to each point, a quarter of the
/// font size away on the chosen side.
fn draw_data_labels(
    out: &mut Vec<Item>,
    pd: &PlotData,
    geom: &LayerGeometry,
    m: &dyn TextMeasurer,
    plot_color: &impl Fn(usize) -> Option<sheaf_core::Color>,
) {
    let labels = &pd.plot.decor.labels;
    let size = labels.font.size_pt;
    let gap = size * 0.25;
    // From the baseline to the middle of a line of digits.
    let mid = size * 0.35;
    // A column or bar of a group stands in its own slot beside the category: its label is centred on the bar,
    // as Origin's Column + Label draws grouped columns (graphing 18.12).
    let shift = pd.bar.map_or(0.0, |slot| slot.left + slot.width / 2.0);
    for i in pd.point_rows() {
        let (lx, ly) =
            if pd.transposed() { (pd.x_at(i), pd.y_at(i) + shift) } else { (pd.x_at(i) + shift, pd.y_at(i)) };
        let Some((px, py)) = geom.checked_data_to_page(lx, ly) else { continue };
        let (xv, yv) = (pd.source_x_at(i), pd.y.get(i).copied().unwrap_or(f64::NAN));
        if !labels.labels_row(i) {
            continue;
        }
        let number = |v: f64| match labels.decimals {
            Some(d) if v.is_finite() => format!("{v:.*}", usize::from(d)),
            _ => label_number(v),
        };
        let text = match labels.form {
            LabelForm::X => number(xv),
            LabelForm::Y => number(yv),
            LabelForm::Row => (i + 1).to_string(),
            LabelForm::XY => format!("({}, {})", number(xv), number(yv)),
            LabelForm::Column { .. } => pd.label_texts.as_ref().and_then(|t| t.get(i)).cloned().unwrap_or_default(),
        };
        if text.is_empty() {
            continue;
        }
        let runs = (matches!(labels.form, LabelForm::Column { .. }) && text.contains('\\'))
            .then(|| markup::plot_segments(&text, &|name| m.expand_inline_family(name), plot_color));
        if runs.as_ref().is_some_and(Vec::is_empty) {
            continue;
        }
        let (x, y, anchor) = match labels.position {
            LabelPosition::Right => (px + gap, py + mid, Anchor::Start),
            LabelPosition::Left => (px - gap, py + mid, Anchor::End),
            LabelPosition::Above => (px, py - gap, Anchor::Middle),
            LabelPosition::Below => (px, py + gap + 2.0 * mid, Anchor::Middle),
            LabelPosition::Center => (px, py + mid, Anchor::Middle),
        };
        let item = TextItem {
            x: x + labels.dx_pt,
            y: y + labels.dy_pt,
            text,
            family: labels.font.family.clone(),
            size,
            bold: labels.font.bold,
            italic: labels.font.italic,
            color: labels.font.color,
            anchor,
            rotate_ccw: labels.rotate_deg,
        };
        if labels.white_out || !labels.frame.is_none() {
            let width = match &runs {
                Some(runs) => text::segments_width(m, runs, &labels.font, size),
                None => m.width(&item.text, &labels.font, size),
            };
            draw_label_frame(out, &item, width, labels);
            if labels.white_out {
                draw_label_background(out, &item, width);
            }
        }
        if let Some(runs) = runs {
            text::place_segments(out, m, &runs, &labels.font, size, (item.x, item.y), anchor, item.rotate_ccw);
        } else {
            out.push(Item::Text(item));
        }
    }
}

/// White Out follows the text's anchor and rotation. The font box approximates Origin 2016's
/// Arial background (30 pt "12345": 100 x 40 pixels in a 900-pixel-wide page export).
fn draw_label_background(out: &mut Vec<Item>, text: &TextItem, width: f64) {
    if let Some(d) = label_box_path(text, width, 0.05 * text.size, 0.0) {
        out.push(Item::Path { d, fill: Some(sheaf_core::Color::WHITE), stroke: None });
    }
}

/// Origin draws the frame and its shadow before White Out, which clears only the text box.
fn draw_label_frame(out: &mut Vec<Item>, text: &TextItem, width: f64, labels: &DataLabels) {
    if labels.frame.is_none() {
        return;
    }
    let percent = if labels.margin_percent.is_finite() { labels.margin_percent.clamp(0.0, 500.0) } else { 0.0 };
    // Origin 2016's Arial 30 case expands by 19 pixels across both sides at 20% margin,
    // against a 40-pixel text box. Other fonts retain the existing approximate font box.
    let margin = text.size * (4.0 / 3.0) * percent / 100.0;
    let Some(corners) = label_box_corners(text, width, margin, margin) else { return };
    let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for (x, y) in corners {
        (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
    }
    // Origin keeps the frame parallel to the page around the rotated, padded text box.
    // Its shadow stays to the page's lower right, rather than rotating with the text.
    if labels.frame == LabelFrame::Shadow {
        // The same page offset was measured at Arial 15 and 30 pt.
        let shift = 6.0;
        out.push(Item::Rect {
            rect: Rect { x: x0 + shift, y: y0 + shift, w: x1 - x0, h: y1 - y0 },
            fill: Some(sheaf_core::Color::BLACK),
            stroke: None,
        });
    }
    // Inset the path by half the stroke so the outer edge stays on the frame box.
    out.push(Item::Rect {
        rect: Rect { x: x0 + 0.25, y: y0 + 0.25, w: (x1 - x0 - 0.5).max(0.0), h: (y1 - y0 - 0.5).max(0.0) },
        fill: Some(labels.frame_fill),
        stroke: Some(Stroke::solid(labels.frame_border, 0.5)),
    });
}

fn label_box_corners(text: &TextItem, width: f64, pad_x: f64, pad_y: f64) -> Option<[(f64, f64); 4]> {
    if !(width.is_finite()
        && width > 0.0
        && text.size.is_finite()
        && text.size > 0.0
        && text.rotate_ccw.is_finite()
        && text.x.is_finite()
        && text.y.is_finite())
    {
        return None;
    }
    let left = match text.anchor {
        Anchor::Start => 0.0,
        Anchor::Middle => -width / 2.0,
        Anchor::End => -width,
    };
    let (x0, x1, y0, y1) = (left - pad_x, left + width + pad_x, -0.92 * text.size - pad_y, 0.23 * text.size + pad_y);
    let (sin, cos) = text.rotate_ccw.to_radians().sin_cos();
    Some(
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
            .map(|(dx, dy)| (text.x + dx * cos + dy * sin, text.y - dx * sin + dy * cos)),
    )
}

fn label_box_path(text: &TextItem, width: f64, pad_x: f64, pad_y: f64) -> Option<String> {
    let corners = label_box_corners(text, width, pad_x, pad_y)?;
    let mut d = String::new();
    for (i, (x, y)) in corners.into_iter().enumerate() {
        if i == 0 {
            move_to(&mut d, x, y);
        } else {
            line_to(&mut d, x, y);
        }
    }
    d.push('Z');
    Some(d)
}

/// Origin draws a masked point in the mask colour (red at first): its symbol, and the two line
/// segments that meet at it (measured with a line + symbol plot of five points, one masked; the
/// colour follows Column > Mask > Change Color, measured for indices 1 to 5). Drawn on top of the plot.
fn draw_masked(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(mask) = pd.mask else { return };
    let sampled_masks = (pd.plot.kind.has_symbols() && pd.point_interval() > 1)
        .then(|| pd.point_rows().filter(|i| mask.contains(i)).collect::<std::collections::BTreeSet<_>>());
    let page = |i: usize| -> Option<(f64, f64)> {
        if i < pd.start || i >= pd.end || !pd.row_shown(i) {
            return None;
        }
        geom.checked_data_to_page(pd.x_at(i), pd.y_at(i))
    };
    let mut lines = String::new();
    let mut symbols = String::new();
    let line = line_stroke(&pd.plot.line);
    let gaps = symbol_line_gaps(pd, geom, &line);
    for &i in mask.range(pd.start..pd.end - usize::from(pd.closing.is_some())) {
        let Some(p) = page(i) else { continue };
        // The first point of a radar polygon is also the end of the closing segment.
        let wrap = (pd.closing == Some(i)).then_some(pd.end - 1);
        if pd.plot.kind.has_line() && pd.plot.line.width_pt > 0.0 {
            let previous = (pd.start..i).rev().find(|&r| pd.row_shown(r));
            let next = (i + 1..pd.end).find(|&r| pd.row_shown(r));
            for j in [previous, next].into_iter().flatten().chain(wrap) {
                if let Some(q) = page(j) {
                    if let Some(gaps) = &gaps {
                        gaps.write_polyline(&[q, p], &mut lines);
                    } else {
                        move_to(&mut lines, q.0, q.1);
                        line_to(&mut lines, p.0, p.1);
                    }
                }
            }
        }
        if pd.plot.kind.has_symbols() && sampled_masks.as_ref().is_none_or(|rows| rows.contains(&i)) {
            symbol_path(&mut symbols, pd.plot.symbol.shape, p.0, p.1, pd.plot.symbol.size_pt);
        }
    }
    if !lines.is_empty() {
        out.push(Item::Path { d: lines, fill: None, stroke: Some(Stroke { color: pd.mask_color, ..line }) });
    }
    if !symbols.is_empty() {
        let red = SymbolStyle { color: pd.mask_color, ..pd.plot.symbol.clone() };
        out.push(symbol_item(symbols, &red));
    }
}

/// Consecutive points that fall into the same narrow vertical strip of the
/// page. A long run is drawn as its first point, its lowest and highest
/// points and its last point, which looks the same as drawing every point.
struct Run {
    strip: i64,
    /// The first points of the run, kept so that short runs are drawn exactly.
    head: [(f64, f64); RUN_EXACT],
    count: usize,
    /// Position in the run and page point of the smallest and largest page y, and of the last point.
    low: (usize, (f64, f64)),
    high: (usize, (f64, f64)),
    last: (usize, (f64, f64)),
}

impl Run {
    fn new(strip: i64, p: (f64, f64)) -> Self {
        Run { strip, head: [p; RUN_EXACT], count: 1, low: (0, p), high: (0, p), last: (0, p) }
    }

    fn add(&mut self, p: (f64, f64)) {
        let k = self.count;
        if k < RUN_EXACT {
            self.head[k] = p;
        }
        if p.1 < self.low.1.1 {
            self.low = (k, p);
        }
        if p.1 > self.high.1.1 {
            self.high = (k, p);
        }
        self.last = (k, p);
        self.count += 1;
    }

    /// Hands the points that represent the run to `push`, in drawing order.
    fn emit(&self, mut push: impl FnMut((f64, f64))) {
        if self.count <= RUN_EXACT {
            self.head[..self.count].iter().for_each(|p| push(*p));
            return;
        }
        let mut keep = [(0, self.head[0]), self.low, self.high, self.last];
        keep.sort_by_key(|(k, _)| *k);
        let mut previous = usize::MAX;
        for (k, p) in keep {
            if k != previous {
                push(p);
            }
            previous = k;
        }
    }
}

/// The fill of an area plot: one closed shape per unbroken run of points, from the lower edge up
/// to the curve. The lower edge is the page height of Y = 0 (or the nearer frame edge when 0
/// cannot be placed or lies outside); for a stacked area plot it is the curve of the plots below,
/// for an area plot with a base column (`Plot::y_base`) that column's curve. Very long runs are
/// thinned to about 20000 points, which looks the same at any export size.
fn draw_area(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let f = geom.frame;
    let zero = geom.data_to_page(0.0, if pd.oy == 0.0 { 0.0 } else { pd.oy }).1;
    let flat = if zero.is_finite() { zero.clamp(f.y, f.y + f.h) } else { f.y + f.h };
    let step = (pd.rows().count() / 20_000).max(1);
    let style = pd.plot.decor.fill;
    let color = pd.plot.symbol.color;
    let mut d = String::new();
    // Upper points, and the lower point under each.
    let mut run: Vec<((f64, f64), f64)> = Vec::new();
    let close = |run: &mut Vec<((f64, f64), f64)>, d: &mut String, out: &mut Vec<Item>| {
        if run.len() >= 2 {
            if style.is_plain() {
                move_to(d, run[0].0.0, run[0].1);
                for &((x, y), _) in run.iter() {
                    line_to(d, x, y);
                }
                for &((x, _), low) in run.iter().rev() {
                    line_to(d, x, low);
                }
                d.push('Z');
            } else {
                // The upper edge from left to right, then the lower edge back.
                let mut poly: Vec<(f64, f64)> = run.iter().map(|&(p, _)| p).collect();
                poly.extend(run.iter().rev().map(|&((x, _), low)| (x, low)));
                fill_polygon(out, &poly, color, &style);
            }
        }
        run.clear();
    };
    let mut previous = None;
    let last = pd.rows().next_back();
    for i in pd.rows().step_by(step).chain(last) {
        if previous == Some(i) {
            continue;
        }
        previous = Some(i);
        let upper = geom.checked_data_to_page(pd.x_at(i), pd.y_at(i));
        // The lower edge of this row; a row without one breaks the shape.
        let lower = match pd.base_at(i) {
            None => Some(flat),
            Some(b) => geom.checked_data_to_page(pd.x_at(i), b).map(|p| p.1),
        };
        match upper.zip(lower) {
            Some((p, low)) => run.push((p, low)),
            None => close(&mut run, &mut d, out),
        }
    }
    close(&mut run, &mut d, out);
    if !d.is_empty() {
        out.push(Item::Path { d, fill: Some(color), stroke: None });
    }
}

fn symbol_line_gaps(pd: &PlotData, geom: &LayerGeometry, stroke: &Stroke) -> Option<crate::symbol_gaps::Gaps> {
    let interface = pd.plot.decor.symbol_line;
    let size = pd.plot.symbol.size_pt;
    if pd.plot.kind != PlotKind::LineSymbol || !interface.gap_to_symbol || !(size.is_finite() && size > 0.0) {
        return None;
    }
    // Use 30% symbol-diameter clearance, matching the measured Origin exports.
    // Keep it beyond the symbol's half-size, including round stroke end caps.
    let cap = if stroke.cap == Cap::Round { stroke.width.max(0.0) / 2.0 } else { 0.0 };
    let radius = size * (0.5 + 0.3) + cap;
    let rows: Box<dyn Iterator<Item = usize> + '_> = if !interface.no_break_skipped {
        pd.line_rows()
    } else if pd.plot.line.sort_x {
        let shown: std::collections::BTreeSet<usize> = pd.point_rows().collect();
        Box::new(pd.line_rows().filter(move |r| shown.contains(r)))
    } else {
        Box::new(pd.point_rows())
    };
    crate::symbol_gaps::Gaps::new(rows.filter_map(|i| geom.checked_data_to_page(pd.x_at(i), pd.y_at(i))), radius)
}

/// The line of a plot: along its points in the colours of Y mapping on a colour mapped waterfall, else the
/// ordinary line (in the curve's Z colour with Z mapping).
fn draw_line_or_mapped(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    match pd.wf {
        Some(wf) if wf.mode == sheaf_core::graph::WaterfallColor::Y => draw_line_y_mapped(out, pd, geom, &wf),
        _ => draw_line(out, pd, geom),
    }
}

/// Waterfall Y:Color Mapping (graphing spec 13.17): the line is built exactly as the plain line (connect mode,
/// axis breaks, missing data, symbol gaps); every straight piece of the resulting path is then coloured by the
/// mean drawn Y value of its ends through the shared ramp, and runs of one colour are one path.
fn draw_line_y_mapped(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, wf: &crate::data::WaterfallColors) {
    draw_line_with(out, pd, geom, Some(wf));
}

/// Pushes the path `d` as runs of segments coloured by the mean Y (data units, without the curve's stack offset)
/// of their ends.
fn push_y_mapped(
    out: &mut Vec<Item>,
    d: &str,
    geom: &LayerGeometry,
    wf: &crate::data::WaterfallColors,
    offset: f64,
    base: &Stroke,
) {
    let value = |p: (f64, f64)| geom.page_to_data(p.0, p.1).1 - offset;
    for sub in crate::hit::subpaths(d) {
        let mut run: Option<(sheaf_core::Color, String)> = None;
        let mut flush = |run: &mut Option<(sheaf_core::Color, String)>| {
            if let Some((color, d)) = run.take() {
                out.push(Item::Path { d, fill: None, stroke: Some(Stroke { color, ..base.clone() }) });
            }
        };
        if let [single] = sub.as_slice() {
            let mut d = String::new();
            move_to(&mut d, single.0, single.1);
            line_to(&mut d, single.0, single.1);
            out.push(Item::Path {
                d,
                fill: None,
                stroke: Some(Stroke { color: map_color(wf.fraction(value(*single))), ..base.clone() }),
            });
            continue;
        }
        for w in sub.windows(2) {
            let color = map_color(wf.fraction((value(w[0]) + value(w[1])) / 2.0));
            match &mut run {
                Some((c, d)) if *c == color => line_to(d, w[1].0, w[1].1),
                _ => {
                    flush(&mut run);
                    let mut d = String::new();
                    move_to(&mut d, w[0].0, w[0].1);
                    line_to(&mut d, w[1].0, w[1].1);
                    run = Some((color, d));
                }
            }
        }
        flush(&mut run);
    }
}

fn draw_line(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    draw_line_with(out, pd, geom, None);
}

fn draw_line_with(
    out: &mut Vec<Item>,
    pd: &PlotData,
    geom: &LayerGeometry,
    mapped: Option<&crate::data::WaterfallColors>,
) {
    let mut style = pd.plot.line.clone();
    if let Some(wf) = pd.wf.filter(|w| w.mode == sheaf_core::graph::WaterfallColor::Z) {
        style.color = map_color(wf.fraction(wf.z));
    }
    let stroke = line_stroke(&style);
    // The stack offset of the curve (constant over its rows), to read the source Y back off the page.
    let y_offset = pd
        .line_rows()
        .find(|&i| pd.source_y_at(i).is_finite() && pd.y_at(i).is_finite())
        .map_or(0.0, |i| pd.y_at(i) - pd.source_y_at(i));
    let gaps = symbol_line_gaps(pd, geom, &stroke);
    let inclusive = pd.plot.decor.fill_under.is_some()
        && !pd.plot.kind.is_area()
        && pd.plot.decor.fill_under_mode == FillUnderMode::InclusiveBrokenByMissingValues;
    let side_mode =
        (pd.plot.decor.fill_under.is_some() && !pd.plot.kind.is_area() && pd.plot.decor.fill_under_mode.is_sidelines())
            .then_some(pd.plot.decor.fill_under_mode);
    let ends = side_mode.and_then(|_| {
        let mut points =
            pd.line_rows().filter_map(|i| geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)).map(|p| (i, p)));
        let first = points.next()?;
        Some((first, points.next_back().unwrap_or(first)))
    });
    let bottom = geom.frame.y + geom.frame.h;
    if let Some(n) = segment_size(pd.plot.line.connect) {
        draw_segments(out, pd, geom, n, &stroke);
        return;
    }
    let connect = match pd.plot.line.connect {
        Connect::Spline if !spline_applies(pd) => Connect::Straight,
        c => c,
    };
    let mut piece: Vec<(f64, f64)> = Vec::new();
    let mut pending: Option<(f64, f64)> = None;
    let (mut first_row, mut last_row) = (None, None);
    let mut outline = String::new();
    let mut flush =
        |piece: &mut Vec<(f64, f64)>, pending: &mut Option<(f64, f64)>, out: &mut Vec<Item>, first_row, last_row| {
            if let Some(p) = pending.take() {
                piece.push(p);
            }
            let at_end = ends.is_some_and(|((a, _), (b, _))| first_row == Some(a) || last_row == Some(b));
            if piece.len() >= 2 || ((at_end || inclusive) && !piece.is_empty()) {
                let mut d = String::with_capacity(piece.len() * 12);
                polyline_path(&mut d, piece, connect);
                if inclusive {
                    // Origin outlines each closed finite run with the connecting stroke, including
                    // the endpoint chord. A one-point run has no area but shows the round stroke cap.
                    // One path keeps transparency uniform at the joins.
                    if piece.len() == 1 {
                        line_to(&mut d, piece[0].0, piece[0].1);
                    } else {
                        d.push('Z');
                    }
                }
                if let Some(((first, first_point), (last, _))) = ends {
                    // Origin adds boundaries only at the plot's outer endpoints, including across
                    // missing-data breaks. It adds none at internal breaks or isolated middle points.
                    if first_row == Some(first) {
                        let mut prefix = String::new();
                        move_to(&mut prefix, piece[0].0, bottom);
                        d.replace_range(..1, "L");
                        prefix.push_str(&d);
                        d = prefix;
                    }
                    if last_row == Some(last) {
                        let last_x = crate::hit::path_points(&d).last().map_or(piece[piece.len() - 1].0, |p| p.0);
                        line_to(&mut d, last_x, bottom);
                        if side_mode == Some(FillUnderMode::SidelinesAndBase) {
                            line_to(&mut d, first_point.0, bottom);
                        }
                    }
                }
                if let Some(gaps) = &gaps {
                    let points = crate::hit::path_points(&d);
                    d.clear();
                    gaps.write_polyline(&points, &mut d);
                }
                let d = crate::break_cut::cut_at_breaks(d, geom);
                if !d.is_empty() {
                    if side_mode.is_some() {
                        outline.push_str(&d);
                    } else if let Some(wf) = mapped {
                        push_y_mapped(out, &d, geom, wf, y_offset, &stroke);
                    } else {
                        out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
                    }
                }
            }
            piece.clear();
        };
    // Adds a point to the piece unless it is closer than DECIMATE_PT to the last kept one.
    let keep = |piece: &mut Vec<(f64, f64)>, pending: &mut Option<(f64, f64)>, p: (f64, f64)| match piece.last() {
        None => piece.push(p),
        Some(&(lx, ly)) => {
            let (dx, dy) = (p.0 - lx, p.1 - ly);
            if dx * dx + dy * dy < DECIMATE_PT * DECIMATE_PT {
                *pending = Some(p);
            } else {
                *pending = None;
                piece.push(p);
            }
        }
    };
    let mut run: Option<Run> = None;
    for i in pd.line_rows() {
        match geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) {
            Some(p) => {
                first_row.get_or_insert(i);
                last_row = Some(i);
                let strip = (p.0 / STRIP_PT).floor() as i64;
                match &mut run {
                    Some(r) if r.strip == strip => r.add(p),
                    _ => {
                        if let Some(r) = run.take() {
                            r.emit(|q| keep(&mut piece, &mut pending, q));
                        }
                        run = Some(Run::new(strip, p));
                    }
                }
            }
            // With the page's "Connect line across Missing Data" a row without a value does not break the
            // line (a value that cannot be placed still does).
            None if geom.connect_across_missing && !(pd.x_at(i).is_finite() && pd.y_at(i).is_finite()) => {}
            None => {
                if let Some(r) = run.take() {
                    r.emit(|q| keep(&mut piece, &mut pending, q));
                }
                flush(&mut piece, &mut pending, out, first_row.take(), last_row.take());
            }
        }
    }
    if let Some(r) = run.take() {
        r.emit(|q| keep(&mut piece, &mut pending, q));
    }
    flush(&mut piece, &mut pending, out, first_row.take(), last_row.take());
    if !outline.is_empty() {
        out.push(Item::Path { d: outline, fill: None, stroke: Some(stroke) });
    }
}

/// The group size of a segment connection (2 Point Segment, 3 Point Segment, k-point segments).
fn segment_size(c: Connect) -> Option<usize> {
    match c {
        Connect::Segment2 => Some(2),
        Connect::Segment3 => Some(3),
        Connect::Segments(k) => Some(usize::from(k).max(2)),
        _ => None,
    }
}

/// Draws a segment connection: the plot's rows, counted from its first row, form groups of `n`
/// (rows 1..n, n+1..2n, ...), the points of a group are joined in order and the groups are not. A row
/// whose point cannot be placed (a missing value) breaks the line inside its group but does not shift
/// the groups, so a Line Series row never joins the next row. No decimation: groups are short.
fn draw_segments(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, n: usize, stroke: &Stroke) {
    let mut d = String::new();
    let mut last: Option<(usize, (f64, f64))> = None;
    for (k, i) in pd.line_rows().enumerate() {
        let group = k / n;
        match geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) {
            Some(p) => {
                match last {
                    Some((g, _)) if g == group => line_to(&mut d, p.0, p.1),
                    _ => move_to(&mut d, p.0, p.1),
                }
                last = Some((group, p));
            }
            None => last = None,
        }
    }
    let d = crate::break_cut::cut_at_breaks(d, geom);
    if !d.is_empty() {
        out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
    }
}

/// Writes one piece as path data.
fn polyline_path(d: &mut String, pts: &[(f64, f64)], connect: Connect) {
    let (x0, y0) = pts[0];
    move_to(d, x0, y0);
    match connect {
        Connect::Straight => {
            for &(x, y) in &pts[1..] {
                line_to(d, x, y);
            }
        }
        Connect::StepHorizontal => {
            for w in pts.windows(2) {
                line_to(d, w[1].0, w[0].1);
                line_to(d, w[1].0, w[1].1);
            }
        }
        Connect::StepVertical => {
            for w in pts.windows(2) {
                line_to(d, w[0].0, w[1].1);
                line_to(d, w[1].0, w[1].1);
            }
        }
        // T252: the step halfway between the two points.
        Connect::StepHorizontalCenter => {
            for w in pts.windows(2) {
                let mid = 0.5 * (w[0].0 + w[1].0);
                line_to(d, mid, w[0].1);
                line_to(d, mid, w[1].1);
                line_to(d, w[1].0, w[1].1);
            }
        }
        Connect::StepVerticalCenter => {
            for w in pts.windows(2) {
                let mid = 0.5 * (w[0].1 + w[1].1);
                line_to(d, w[0].0, mid);
                line_to(d, w[1].0, mid);
                line_to(d, w[1].0, w[1].1);
            }
        }
        Connect::Segment2 | Connect::Segment3 | Connect::Segments(_) => {
            // groups of 2 (3) points counted from the first point of the piece; a last group with a
            // single point draws no line
            let n = match connect {
                Connect::Segment2 => 2,
                Connect::Segments(k) => usize::from(k).max(2),
                _ => 3,
            };
            for (k, group) in pts.chunks(n).enumerate() {
                if k > 0 {
                    move_to(d, group[0].0, group[0].1);
                }
                for &(x, y) in &group[1..] {
                    line_to(d, x, y);
                }
            }
        }
        Connect::Spline => {
            let mono = pts.windows(2).all(|w| w[1].0 > w[0].0) || pts.windows(2).all(|w| w[1].0 < w[0].0);
            if pts.len() < 3 || !mono {
                for &(x, y) in &pts[1..] {
                    line_to(d, x, y);
                }
            } else {
                for (x, y) in natural_spline(pts).into_iter().skip(1) {
                    line_to(d, x, y);
                }
            }
        }
        Connect::BSpline => {
            let curve = if pts.len() < 3 { pts.to_vec() } else { clamped_b_spline(pts) };
            for &(x, y) in &curve[1..] {
                line_to(d, x, y);
            }
        }
        Connect::Bezier => {
            let curve = if pts.len() < 4 { pts.to_vec() } else { bezier_groups(pts) };
            for &(x, y) in &curve[1..] {
                line_to(d, x, y);
            }
        }
    }
}

/// True when Origin draws this plot as a spline: at most 900 points and X strictly increasing
/// (a plot that fails either test is drawn with straight segments).
fn spline_applies(pd: &PlotData) -> bool {
    if pd.rows().take(SPLINE_MAX_POINTS + 1).count() > SPLINE_MAX_POINTS {
        return false;
    }
    let mut last = f64::NEG_INFINITY;
    for i in pd.line_rows() {
        let x = pd.x_at(i);
        if !x.is_finite() {
            continue;
        }
        if x <= last {
            return false;
        }
        last = x;
    }
    true
}

/// Number of chords for a stretch of `len` pt.
fn sub_count(len: f64) -> usize {
    ((len / SMOOTH_STEP_PT).ceil() as usize).clamp(1, SMOOTH_MAX_SUB)
}

/// Natural cubic spline y(x) through the points (x strictly monotone), sampled along x.
fn natural_spline(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = pts.len();
    let h: Vec<f64> = pts.windows(2).map(|w| w[1].0 - w[0].0).collect();
    // Second derivatives m: tridiagonal system with m[0] = m[n-1] = 0.
    let mut m = vec![0.0; n];
    if n > 2 {
        let k = n - 2;
        let mut diag = vec![0.0; k];
        let mut rhs = vec![0.0; k];
        for i in 0..k {
            diag[i] = 2.0 * (h[i] + h[i + 1]);
            rhs[i] = 6.0 * ((pts[i + 2].1 - pts[i + 1].1) / h[i + 1] - (pts[i + 1].1 - pts[i].1) / h[i]);
        }
        for i in 1..k {
            let w = h[i] / diag[i - 1];
            diag[i] -= w * h[i];
            rhs[i] -= w * rhs[i - 1];
        }
        let mut sol = vec![0.0; k];
        for i in (0..k).rev() {
            let up = if i + 1 < k { h[i + 1] * sol[i + 1] } else { 0.0 };
            sol[i] = (rhs[i] - up) / diag[i];
        }
        m[1..=k].copy_from_slice(&sol);
    }
    let mut out = vec![pts[0]];
    for i in 0..n - 1 {
        let (x0, y0) = pts[i];
        let hi = h[i];
        let subs = sub_count(hi.abs());
        for s in 1..=subs {
            if s == subs {
                out.push(pts[i + 1]);
                break;
            }
            let t = s as f64 / subs as f64;
            let (a, b) = (1.0 - t, t);
            let y = a * y0 + b * pts[i + 1].1 + ((a * a * a - a) * m[i] + (b * b * b - b) * m[i + 1]) * hi * hi / 6.0;
            out.push((x0 + t * hi, y));
        }
    }
    out
}

/// Uniform cubic B-spline whose first and last point are repeated three times, so the curve starts
/// and ends at the end points (measured against Origin, 0.4 px).
fn clamped_b_spline(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut q = vec![pts[0], pts[0]];
    q.extend_from_slice(pts);
    q.push(pts[pts.len() - 1]);
    q.push(pts[pts.len() - 1]);
    let mut out = vec![pts[0]];
    for w in q.windows(4) {
        let len = (w[1].0 - w[2].0).hypot(w[1].1 - w[2].1);
        if len == 0.0 {
            continue;
        }
        let subs = sub_count(len);
        for s in 1..=subs {
            let t = s as f64 / subs as f64;
            let (t2, t3) = (t * t, t * t * t);
            let c = [
                (1.0 - t).powi(3) / 6.0,
                (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0,
                (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0,
                t3 / 6.0,
            ];
            let x: f64 = w.iter().zip(c).map(|(p, c)| p.0 * c).sum();
            let y: f64 = w.iter().zip(c).map(|(p, c)| p.1 * c).sum();
            out.push((x, y));
        }
    }
    out.push(pts[pts.len() - 1]);
    out
}

/// Cubic Bezier segments over points 0-3, 3-6, 6-9 and so on; points after the last full group
/// are not used (as in Origin).
fn bezier_groups(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = vec![pts[0]];
    let mut i = 0;
    while i + 3 < pts.len() {
        let p = &pts[i..i + 4];
        let len = (p[1].0 - p[0].0).hypot(p[1].1 - p[0].1)
            + (p[2].0 - p[1].0).hypot(p[2].1 - p[1].1)
            + (p[3].0 - p[2].0).hypot(p[3].1 - p[2].1);
        let subs = sub_count(len);
        for s in 1..=subs {
            let t = s as f64 / subs as f64;
            let u = 1.0 - t;
            let c = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            let x: f64 = p.iter().zip(c).map(|(p, c)| p.0 * c).sum();
            let y: f64 = p.iter().zip(c).map(|(p, c)| p.1 * c).sum();
            out.push((x, y));
        }
        i += 3;
    }
    out
}

/// True when the point is within `margin` of the frame.
fn near_frame(geom: &LayerGeometry, p: (f64, f64), margin: f64) -> bool {
    let f = geom.frame;
    p.0 >= f.x - margin && p.0 <= f.x + f.w + margin && p.1 >= f.y - margin && p.1 <= f.y + f.h + margin
}

/// One bar per row from the base to the value, in the plot's slot of the layer's group of bars
/// (graphing spec 14.4). The base is 0 on a linear value axis (the plot's offset in a stacked
/// layer, the end of the bar below in a stacked column plot, the base column of a floating
/// column) and the lower axis limit on a log axis. Horizontal bars lie down: values along X,
/// positions along Y. A bar with a corner that cannot be placed is skipped.
fn draw_columns(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let plot = pd.plot;
    let slot = pd.bar.unwrap_or(crate::data::BarSlot { left: -0.4, width: 0.8, spacing: 1.0 });
    let transposed = pd.transposed();
    let value_axis = if transposed { &geom.x } else { &geom.y };
    let default_base = match value_axis.scale {
        ScaleKind::Linear => pd.oy,
        _ => value_axis.min,
    };
    let histogram = plot.kind == PlotKind::Histogram;
    // A histogram's bars are outlined in the plot's colour and not filled (Origin hatches them).
    let stroke = if histogram {
        Some(Stroke::solid(plot.symbol.color, plot.line.width_pt.max(0.5)))
    } else {
        (plot.line.width_pt > 0.0).then(|| Stroke::solid(plot.line.color, plot.line.width_pt))
    };
    let place = |cat: f64, val: f64| {
        if transposed { geom.checked_data_to_page(val, cat) } else { geom.checked_data_to_page(cat, val) }
    };
    for i in pd.rows() {
        let (cat, top) = (pd.cat_at(i), pd.val_at(i));
        let base = pd.base_at(i).unwrap_or(default_base);
        let left = cat + slot.left;
        let (Some(a), Some(b)) = (place(left, base), place(left + slot.width, top)) else {
            continue;
        };
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let rect = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
        let style = &plot.decor.fill;
        if histogram && style.is_plain() {
            out.push(Item::Rect { rect, fill: None, stroke: stroke.clone() });
        } else {
            fill_rect(out, rect, plot.symbol.color, style, stroke.clone());
        }
    }
}

/// Width of a box's whisker caps, in X axis units (measured: a tenth of the box spacing).
const BOX_CAP: f64 = 0.1;
/// Size of the cross at the whisker ends and of the outliers, and of the square at the mean, in pt.
const BOX_CROSS_PT: f64 = 7.0;
const BOX_MEAN_PT: f64 = 5.0;

/// Origin's pie: a pie of 70 % of the frame height, a little left of the middle, slices clockwise
/// from 12 o'clock outlined in black, each labelled outside with its share to two decimals, a
/// legend of squares and names at the top right, and a black frame around the layer (measured).
fn draw_pie(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    use crate::scene::{Anchor, TextItem};
    use sheaf_core::Color;
    let Some(pie) = &pd.pie else { return };
    let f = geom.frame;
    out.push(Item::Rect { rect: f, fill: None, stroke: Some(Stroke::solid(Color::BLACK, 1.0)) });
    let total: f64 = pie.values.iter().sum();
    if total <= 0.0 {
        return;
    }
    let r = 0.35 * f.h.min(f.w);
    let (cx, cy) = (f.x + 0.42 * f.w, f.y + 0.5 * f.h);
    let point = |angle: f64, radius: f64| (cx + radius * angle.sin(), cy - radius * angle.cos());
    let outline = Stroke::solid(Color::BLACK, 0.5);
    let mut from = 0.0;
    for (i, v) in pie.values.iter().enumerate() {
        let share = v / total;
        let to = from + share * std::f64::consts::TAU;
        let mut d = String::new();
        if share >= 1.0 - 1e-12 {
            move_to(&mut d, cx, cy - r);
            for (x, y) in [(cx, cy + r), (cx, cy - r)] {
                d.push('A');
                push_num(&mut d, r);
                d.push(' ');
                push_num(&mut d, r);
                d.push_str(" 0 1 1 ");
                push_num(&mut d, x);
                d.push(' ');
                push_num(&mut d, y);
            }
        } else {
            let (a, b) = (point(from, r), point(to, r));
            move_to(&mut d, cx, cy);
            line_to(&mut d, a.0, a.1);
            d.push('A');
            push_num(&mut d, r);
            d.push(' ');
            push_num(&mut d, r);
            d.push_str(if share > 0.5 { " 0 1 1 " } else { " 0 0 1 " });
            push_num(&mut d, b.0);
            d.push(' ');
            push_num(&mut d, b.1);
        }
        d.push('Z');
        out.push(Item::Path { d, fill: Some(pie.colors[i]), stroke: Some(outline.clone()) });
        // The share, outside the slice, next to its middle.
        let mid = (from + to) / 2.0;
        let (x, y) = point(mid, r * 1.12);
        let size = pie.label_font.size_pt;
        let anchor = if mid.sin() > 0.3 {
            Anchor::Start
        } else if mid.sin() < -0.3 {
            Anchor::End
        } else {
            Anchor::Middle
        };
        out.push(Item::Text(TextItem {
            x,
            y: y + 0.35 * size,
            text: crate::pie::share_text(share * 100.0),
            family: pie.label_font.family.clone(),
            size,
            bold: pie.label_font.bold,
            italic: pie.label_font.italic,
            color: pie.label_font.color,
            anchor,
            rotate_ccw: 0.0,
        }));
        from = to;
    }
    // The legend: a square and the name of each slice, from the top right corner down.
    let fs = pie.legend_font.size_pt;
    let row = 1.3 * fs;
    let width = pie.names.iter().map(|n| n.chars().count()).max().unwrap_or(0) as f64 * 0.55 * fs + 2.4 * fs;
    let x0 = f.x + f.w - width - 0.5 * fs;
    for (i, name) in pie.names.iter().enumerate() {
        let y = f.y + 0.5 * fs + i as f64 * row;
        out.push(Item::Rect {
            rect: Rect { x: x0, y, w: fs, h: fs },
            fill: Some(pie.colors[i]),
            stroke: Some(outline.clone()),
        });
        out.push(Item::Text(TextItem {
            x: x0 + 1.4 * fs,
            y: y + 0.8 * fs,
            text: name.clone(),
            family: pie.legend_font.family.clone(),
            size: fs,
            bold: pie.legend_font.bold,
            italic: pie.legend_font.italic,
            color: pie.legend_font.color,
            anchor: Anchor::Start,
            rotate_ccw: 0.0,
        }));
    }
}

/// One box of a box chart: the box from the 25th to the 75th percentile with the median across it,
/// whiskers with caps and a cross at their ends, the mean as a small open square and the outliers
/// as crosses, all in the plot's colour (measured in Origin).
fn draw_box(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    for b in pd.boxed.iter().chain(&pd.gboxes) {
        draw_one_box(out, pd, geom, b);
    }
}

fn draw_one_box(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, b: &crate::data::BoxStats) {
    let color = pd.plot.symbol.color;
    let width = pd.plot.line.width_pt.max(0.5);
    let at = |x: f64, y: f64| geom.checked_data_to_page(pd.display_x(x), pd.display_y(y));
    let (l, r) = (b.pos - b.width / 2.0, b.pos + b.width / 2.0);
    if let (Some(a), Some(c)) = (at(l, b.q1), at(r, b.q3)) {
        out.push(Item::Rect {
            rect: Rect { x: a.0.min(c.0), y: a.1.min(c.1), w: (c.0 - a.0).abs(), h: (c.1 - a.1).abs() },
            fill: None,
            stroke: Some(Stroke::solid(color, width)),
        });
    }
    let mut d = String::new();
    let seg = |d: &mut String, p: Option<(f64, f64)>, q: Option<(f64, f64)>| {
        if let (Some(p), Some(q)) = (p, q) {
            move_to(d, p.0, p.1);
            line_to(d, q.0, q.1);
        }
    };
    seg(&mut d, at(l, b.median), at(r, b.median));
    seg(&mut d, at(b.pos, b.q3), at(b.pos, b.whisker_high));
    seg(&mut d, at(b.pos, b.q1), at(b.pos, b.whisker_low));
    for end in [b.whisker_low, b.whisker_high] {
        seg(&mut d, at(b.pos - BOX_CAP / 2.0, end), at(b.pos + BOX_CAP / 2.0, end));
    }
    out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, width)) });
    let mut crosses = String::new();
    for v in [b.whisker_low, b.whisker_high].iter().chain(b.outliers.iter()) {
        if let Some(p) = at(b.pos, *v) {
            symbol_path(&mut crosses, SymbolShape::Cross, p.0, p.1, BOX_CROSS_PT);
        }
    }
    let style = |shape, size| SymbolStyle {
        shape,
        size_pt: size,
        color,
        filled: false,
        interior: None,
        fill_color: None,
        edge_width_pt: width,
        every: 1,
    };
    if !crosses.is_empty() {
        out.push(symbol_item(crosses, &style(SymbolShape::Cross, BOX_CROSS_PT)));
    }
    if let Some(p) = at(b.pos, b.mean) {
        let mut d = String::new();
        symbol_path(&mut d, SymbolShape::Square, p.0, p.1, BOX_MEAN_PT);
        out.push(symbol_item(d, &style(SymbolShape::Square, BOX_MEAN_PT)));
    }
}

/// Error bars: along the Y values (along X for horizontal bars) from the Y error column, and along
/// X from the X error column of a plot that is not a bar plot. Each bar reaches to the side or sides
/// of the plot's direction and ends in a cap of the plot's cap width. A bar plot's error bars stand
/// in the middle of its own bars, at the top of a stacked bar.
fn draw_error_bars(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let plot = pd.plot;
    let bar = plot.kind.is_bar();
    let color = if plot.kind.has_line() || bar { plot.line.color } else { plot.symbol.color };
    let half_cap = plot.error.cap_width_pt.max(0.0) / 2.0;
    let width = if bar { plot.line.width_pt.max(0.5) } else { (plot.line.width_pt / 2.0).max(0.5) };
    let dir = plot.error.direction;
    let shift = pd.bar.map_or(0.0, |b| b.centre());
    let transposed = pd.transposed();
    let mut d = String::new();
    // One bar from page point `a` to `b`; the caps lie across it, horizontally when `vertical`.
    let mut segment = |a: (f64, f64), b: (f64, f64), vertical: bool, cap_a: bool, cap_b: bool| {
        if !(near_frame(geom, a, half_cap + width) || near_frame(geom, b, half_cap + width)) {
            return;
        }
        move_to(&mut d, a.0, a.1);
        line_to(&mut d, b.0, b.1);
        for (p, on) in [(a, cap_a), (b, cap_b)] {
            if !on || half_cap <= 0.0 {
                continue;
            }
            if vertical {
                move_to(&mut d, p.0 - half_cap, p.1);
                line_to(&mut d, p.0 + half_cap, p.1);
            } else {
                move_to(&mut d, p.0, p.1 - half_cap);
                line_to(&mut d, p.0, p.1 + half_cap);
            }
        }
    };
    let cap_low = dir != ErrorDirection::Plus;
    let cap_high = dir != ErrorDirection::Minus;
    for i in pd.point_rows() {
        if let Some((below, above)) = pd.reach(pd.err_at(i), dir) {
            let (cat, val) = (pd.cat_at(i) + shift, pd.val_at(i));
            let place = |v: f64| {
                if transposed { geom.checked_data_to_page(v, cat) } else { geom.checked_data_to_page(cat, v) }
            };
            if let (Some(lo), Some(hi)) = (place(val - below), place(val + above)) {
                // The bar is vertical on the page unless the bars lie down.
                segment(hi, lo, !transposed, cap_high, cap_low);
            }
        }
        if !bar && let Some((below, above)) = pd.reach(pd.xerr_at(i), dir) {
            let (x, y) = (pd.x_at(i), pd.y_at(i));
            if let (Some(lo), Some(hi)) =
                (geom.checked_data_to_page(x - below, y), geom.checked_data_to_page(x + above, y))
            {
                segment(hi, lo, false, cap_high, cap_low);
            }
        }
    }
    if !d.is_empty() {
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, width)) });
    }
}

/// Smallest and largest symbol width of a bubble plot, in points: Origin scales the width linearly
/// with the Z value, from the smallest to the largest (measured: 8 points with Z from 1 to 9 gave
/// rings 2.4 and 21 pt wide, 5 pt at Z = 2, 15 pt at Z = 6).
pub(crate) const BUBBLE_MIN_PT: f64 = 2.0;
pub(crate) const BUBBLE_MAX_PT: f64 = 20.0;

/// The colour of a colour mapped symbol at `t` in 0..1: blue, cyan, green, yellow, red (a stand-in:
/// Origin's default colour map was not measured).
pub(crate) fn map_color(t: f64) -> sheaf_core::Color {
    const STOPS: [(f64, f64, f64); 5] =
        [(0.0, 0.0, 255.0), (0.0, 255.0, 255.0), (0.0, 255.0, 0.0), (255.0, 255.0, 0.0), (255.0, 0.0, 0.0)];
    let x = t.clamp(0.0, 1.0) * (STOPS.len() - 1) as f64;
    let i = (x.floor() as usize).min(STOPS.len() - 2);
    let f = x - i as f64;
    let mix = |a: f64, b: f64| (a + (b - a) * f).round() as u8;
    sheaf_core::Color::rgb(
        mix(STOPS[i].0, STOPS[i + 1].0),
        mix(STOPS[i].1, STOPS[i + 1].1),
        mix(STOPS[i].2, STOPS[i + 1].2),
    )
}

/// The position of `v` between the smallest and largest finite value of `z[from..to]`, 0..1; 0.5
/// when all values are equal.
pub(crate) fn z_range(z: &[f64], pd: &PlotData) -> (f64, f64) {
    pd.rows()
        .filter_map(|r| z.get(r))
        .filter(|x| x.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)))
}

pub(crate) fn z_fraction((lo, hi): (f64, f64), v: f64) -> f64 {
    if hi > lo { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.5 }
}

/// A scatter plot whose symbols differ: bubbles of different sizes and/or colours, one scene item
/// per symbol.
fn draw_varied_symbols(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let sym = &pd.plot.symbol;
    let size_range = pd.size_z.map(|z| z_range(z, pd));
    let color_range = pd.color_z.map(|z| z_range(z, pd));
    for i in pd.point_rows() {
        let Some(p) = geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) else { continue };
        let mut style = sym.clone();
        if let (Some(z), Some(range)) = (pd.size_z, size_range) {
            let v = z.get(i).copied().unwrap_or(f64::NAN);
            if !v.is_finite() {
                continue;
            }
            style.size_pt = BUBBLE_MIN_PT + (BUBBLE_MAX_PT - BUBBLE_MIN_PT) * z_fraction(range, v);
        }
        if let (Some(z), Some(range)) = (pd.color_z, color_range) {
            let v = z.get(i).copied().unwrap_or(f64::NAN);
            if !v.is_finite() {
                continue;
            }
            style.color = map_color(z_fraction(range, v));
        }
        if !near_frame(geom, p, style.size_pt + style.edge_width_pt) {
            continue;
        }
        push_symbols(out, &style, &[p], &[i]);
    }
}

fn draw_symbols(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    if pd.size_z.is_some() || pd.color_z.is_some() {
        return draw_varied_symbols(out, pd, geom);
    }
    let sym = &pd.plot.symbol;
    let every = pd.point_interval();
    let mut d = String::new();
    // With very many points, symbols that would sit on top of an already drawn
    // one (same small cell of the page) are left out; the picture is the same.
    let end = pd.end - usize::from(pd.closing.is_some());
    let thin = (end - pd.start) / every > SYMBOL_THIN_ABOVE;
    let mut taken: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();
    let plain = !sym.shape.is_text()
        && sym.shape != SymbolShape::Sphere
        && (sym.shape.is_stroke_only()
            || matches!(sym.effective_interior(), SymbolInterior::Solid | SymbolInterior::Hollow));
    let (mut centres, mut rows) = (Vec::new(), Vec::new());
    for i in pd.point_rows() {
        let Some(p) = geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) else { continue };
        if !near_frame(geom, p, sym.size_pt + sym.edge_width_pt) {
            continue;
        }
        if thin && !taken.insert(((p.0 / SYMBOL_CELL_PT).floor() as i64, (p.1 / SYMBOL_CELL_PT).floor() as i64)) {
            continue;
        }
        if plain {
            symbol_path(&mut d, sym.shape, p.0, p.1, sym.size_pt);
        } else {
            centres.push(p);
            rows.push(i);
        }
    }
    if !plain {
        return push_symbols(out, sym, &centres, &rows);
    }
    if d.is_empty() {
        return;
    }
    out.push(symbol_item(d, sym));
}

/// The scene item for symbol path data written by [`symbol_path`].
pub(crate) fn symbol_item(d: String, sym: &SymbolStyle) -> Item {
    let stroke_only = sym.shape.is_stroke_only();
    let stroke = symbol_stroke(sym);
    if sym.effective_interior() == SymbolInterior::Solid && !stroke_only {
        Item::Path { d, fill: Some(sym.color), stroke: None }
    } else {
        Item::Path { d, fill: None, stroke: Some(stroke) }
    }
}

fn symbol_stroke(sym: &SymbolStyle) -> Stroke {
    Stroke { color: sym.color, width: sym.edge_width_pt, dash: Vec::new(), cap: Cap::Butt, join: Join::Miter }
}

/// Draws the symbols of `sym` at the page points `centres` (graphing 5.4): the outline of
/// [`symbol_path`] with Origin's interior (Open filled with the fill colour, white by default; a
/// centre mark; a half in the edge colour), the sphere's shading, or (for the text shapes) the
/// row number or letter of `rows[k]`, the source row of `centres[k]`.
pub(crate) fn push_symbols(out: &mut Vec<Item>, sym: &SymbolStyle, centres: &[(f64, f64)], rows: &[usize]) {
    if centres.is_empty() {
        return;
    }
    let size = sym.size_pt;
    if sym.shape.is_text() {
        for (k, &(cx, cy)) in centres.iter().enumerate() {
            let row = rows.get(k).copied().unwrap_or(0);
            let text = sym.shape.text_at(row).unwrap_or_default();
            out.push(Item::Text(TextItem {
                x: cx,
                y: cy + 0.36 * size,
                text,
                family: "Arial".into(),
                size,
                bold: false,
                italic: false,
                color: sym.color,
                anchor: Anchor::Middle,
                rotate_ccw: 0.0,
            }));
        }
        return;
    }
    let mut d = String::new();
    for &(cx, cy) in centres {
        symbol_path(&mut d, sym.shape, cx, cy, size);
    }
    if sym.shape == SymbolShape::Sphere {
        // Origin shades the sphere with a light spot to the upper left; drawn here as two lighter
        // discs over the coloured one (no gradients in the scene).
        out.push(Item::Path { d, fill: Some(sym.color), stroke: None });
        let r = size / 2.0;
        for (f, rr) in [(0.45, 0.42), (0.8, 0.18)] {
            let mut h = String::new();
            for &(cx, cy) in centres {
                symbol_path(&mut h, SymbolShape::Circle, cx - 0.32 * r, cy - 0.32 * r, 2.0 * rr * r);
            }
            out.push(Item::Path { d: h, fill: Some(mix_white(sym.color, f)), stroke: None });
        }
        return;
    }
    let interior = sym.effective_interior();
    if sym.shape.is_stroke_only() || matches!(interior, SymbolInterior::Solid | SymbolInterior::Hollow) {
        out.push(symbol_item(d, sym));
        return;
    }
    let fill = sym.fill_color.unwrap_or(sheaf_core::Color::WHITE);
    out.push(Item::Path { d: d.clone(), fill: Some(fill), stroke: None });
    let r = size / 2.0;
    let mut marks = String::new();
    let mut solid = String::new();
    for &(cx, cy) in centres {
        match interior {
            SymbolInterior::DotCenter => {
                symbol_path(&mut solid, SymbolShape::Circle, cx, cy, 2.0 * sym.edge_width_pt.max(size * 0.025))
            }
            SymbolInterior::PlusCenter => plus(&mut marks, cx, cy, r),
            SymbolInterior::XCenter => cross(&mut marks, cx, cy, r),
            SymbolInterior::MinusCenter => {
                move_to(&mut marks, cx - r, cy);
                line_to(&mut marks, cx + r, cy);
            }
            SymbolInterior::BarCenter => {
                move_to(&mut marks, cx, cy - r);
                line_to(&mut marks, cx, cy + r);
            }
            SymbolInterior::HalfUp
            | SymbolInterior::HalfDown
            | SymbolInterior::HalfLeft
            | SymbolInterior::HalfRight => half_path(&mut solid, sym.shape, interior, cx, cy, size),
            _ => {}
        }
    }
    if !solid.is_empty() {
        out.push(Item::Path { d: solid, fill: Some(sym.color), stroke: None });
    }
    if !marks.is_empty() {
        out.push(Item::Path { d: marks, fill: None, stroke: Some(symbol_stroke(sym)) });
    }
    out.push(Item::Path { d, fill: None, stroke: Some(symbol_stroke(sym)) });
}

/// `c` moved the fraction `f` of the way to white.
fn mix_white(c: sheaf_core::Color, f: f64) -> sheaf_core::Color {
    let m = |v: u8| (f64::from(v) + (255.0 - f64::from(v)) * f).round() as u8;
    sheaf_core::Color { r: m(c.r), g: m(c.g), b: m(c.b), ..c }
}

/// The corners of a polygon shape, or `None` for a circle, a stroke or a text shape.
fn shape_polygon(shape: SymbolShape, cx: f64, cy: f64, size: f64) -> Option<Vec<(f64, f64)>> {
    let r = size / 2.0;
    let h = size * 0.866;
    let regular = |n: usize, radius: f64| -> Vec<(f64, f64)> {
        (0..n)
            .map(|k| {
                let a = (-90.0 + 360.0 * k as f64 / n as f64).to_radians();
                (cx + radius * a.cos(), cy + radius * a.sin())
            })
            .collect()
    };
    Some(match shape {
        SymbolShape::Square => vec![(cx - r, cy - r), (cx + r, cy - r), (cx + r, cy + r), (cx - r, cy + r)],
        SymbolShape::TriangleUp => vec![(cx, cy - 2.0 * h / 3.0), (cx + r, cy + h / 3.0), (cx - r, cy + h / 3.0)],
        SymbolShape::TriangleDown => vec![(cx, cy + 2.0 * h / 3.0), (cx + r, cy - h / 3.0), (cx - r, cy - h / 3.0)],
        SymbolShape::TriangleLeft => vec![(cx - 2.0 * h / 3.0, cy), (cx + h / 3.0, cy - r), (cx + h / 3.0, cy + r)],
        SymbolShape::TriangleRight => vec![(cx + 2.0 * h / 3.0, cy), (cx - h / 3.0, cy + r), (cx - h / 3.0, cy - r)],
        SymbolShape::Diamond => vec![(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)],
        SymbolShape::Hexagon => regular(6, r),
        SymbolShape::Pentagon => regular(5, r),
        SymbolShape::FivePointStar => {
            let outer = regular(5, r);
            let inner: Vec<(f64, f64)> = (0..5)
                .map(|k| {
                    let a = (-90.0 + 36.0 + 72.0 * k as f64).to_radians();
                    (cx + 0.382 * r * a.cos(), cy + 0.382 * r * a.sin())
                })
                .collect();
            outer.into_iter().zip(inner).flat_map(|(o, i)| [o, i]).collect()
        }
        _ => return None,
    })
}

fn polygon_path(d: &mut String, pts: &[(f64, f64)]) {
    if let Some((first, rest)) = pts.split_first() {
        move_to(d, first.0, first.1);
        for p in rest {
            line_to(d, p.0, p.1);
        }
        d.push('Z');
    }
}

/// The half of a symbol that a half interior fills, cut along the line through its centre.
fn half_path(d: &mut String, shape: SymbolShape, half: SymbolInterior, cx: f64, cy: f64, size: f64) {
    // keep(p) says whether the point is on the filled side; the cut is x = cx or y = cy.
    let (vertical_cut, keep_low) = match half {
        SymbolInterior::HalfUp => (false, true),
        SymbolInterior::HalfDown => (false, false),
        SymbolInterior::HalfLeft => (true, true),
        _ => (true, false),
    };
    let coord = |p: (f64, f64)| if vertical_cut { p.0 - cx } else { p.1 - cy };
    let inside = |p: (f64, f64)| if keep_low { coord(p) <= 0.0 } else { coord(p) >= 0.0 };
    if let Some(poly) = shape_polygon(shape, cx, cy, size) {
        // Sutherland-Hodgman against one half plane.
        let mut out = Vec::new();
        for k in 0..poly.len() {
            let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
            if inside(a) {
                out.push(a);
            }
            if inside(a) != inside(b) {
                let t = coord(a) / (coord(a) - coord(b));
                out.push((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)));
            }
        }
        polygon_path(d, &out);
        return;
    }
    // A circle: a half disc.
    let r = size / 2.0;
    let (start, end, sweep) = match half {
        SymbolInterior::HalfUp => ((cx - r, cy), (cx + r, cy), 1),
        SymbolInterior::HalfDown => ((cx - r, cy), (cx + r, cy), 0),
        SymbolInterior::HalfLeft => ((cx, cy - r), (cx, cy + r), 0),
        _ => ((cx, cy - r), (cx, cy + r), 1),
    };
    move_to(d, start.0, start.1);
    d.push('A');
    push_num(d, r);
    d.push(' ');
    push_num(d, r);
    d.push_str(&format!(" 0 0 {sweep} "));
    push_num(d, end.0);
    d.push(' ');
    push_num(d, end.1);
    d.push('Z');
}

/// Appends the outline of one symbol, centred on `(cx, cy)`, `size` wide.
pub(crate) fn symbol_path(d: &mut String, shape: SymbolShape, cx: f64, cy: f64, size: f64) {
    let r = size / 2.0;
    let close = |d: &mut String| d.push('Z');
    match shape {
        SymbolShape::Square => {
            move_to(d, cx - r, cy - r);
            line_to(d, cx + r, cy - r);
            line_to(d, cx + r, cy + r);
            line_to(d, cx - r, cy + r);
            close(d);
        }
        SymbolShape::Circle => {
            move_to(d, cx - r, cy);
            for (x, y) in [(cx + r, cy), (cx - r, cy)] {
                d.push('A');
                push_num(d, r);
                d.push(' ');
                push_num(d, r);
                d.push_str(" 0 1 0 ");
                push_num(d, x);
                d.push(' ');
                push_num(d, y);
            }
            close(d);
        }
        SymbolShape::TriangleUp | SymbolShape::TriangleDown => {
            // Equilateral triangle, centred on its centroid.
            let h = size * 0.866;
            let s = if shape == SymbolShape::TriangleUp { 1.0 } else { -1.0 };
            move_to(d, cx, cy - s * 2.0 * h / 3.0);
            line_to(d, cx + r, cy + s * h / 3.0);
            line_to(d, cx - r, cy + s * h / 3.0);
            close(d);
        }
        SymbolShape::Diamond => {
            move_to(d, cx, cy - r);
            line_to(d, cx + r, cy);
            line_to(d, cx, cy + r);
            line_to(d, cx - r, cy);
            close(d);
        }
        SymbolShape::Plus => plus(d, cx, cy, r),
        SymbolShape::Cross => cross(d, cx, cy, r),
        SymbolShape::Star => {
            plus(d, cx, cy, r);
            cross(d, cx, cy, r);
        }
        SymbolShape::HBar => {
            move_to(d, cx - r, cy);
            line_to(d, cx + r, cy);
        }
        SymbolShape::VBar => {
            move_to(d, cx, cy - r);
            line_to(d, cx, cy + r);
        }
        SymbolShape::Sphere => symbol_path(d, SymbolShape::Circle, cx, cy, size),
        // Drawn as text by `push_symbols`; nothing to outline.
        SymbolShape::Number | SymbolShape::CapitalLetter | SymbolShape::SmallLetter => {}
        SymbolShape::TriangleLeft
        | SymbolShape::TriangleRight
        | SymbolShape::Hexagon
        | SymbolShape::FivePointStar
        | SymbolShape::Pentagon => {
            if let Some(pts) = shape_polygon(shape, cx, cy, size) {
                polygon_path(d, &pts);
            }
        }
    }
}

fn plus(d: &mut String, cx: f64, cy: f64, r: f64) {
    move_to(d, cx - r, cy);
    line_to(d, cx + r, cy);
    move_to(d, cx, cy - r);
    line_to(d, cx, cy + r);
}

fn cross(d: &mut String, cx: f64, cy: f64, r: f64) {
    move_to(d, cx - r, cy - r);
    line_to(d, cx + r, cy + r);
    move_to(d, cx - r, cy + r);
    line_to(d, cx + r, cy - r);
}

#[cfg(test)]
pub(crate) fn smooth_for_test(connect: Connect, pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    match connect {
        Connect::Spline => natural_spline(pts),
        Connect::BSpline => clamped_b_spline(pts),
        Connect::Bezier => bezier_groups(pts),
        _ => pts.to_vec(),
    }
}

/// Data > Mark Data Range (T118): an I-beam at each of the two marked points, 24 pt high at the Medium
/// size of Data > Analysis Markers (T135).
/// The data markers of a plot as Origin 2016 draws them (measured, T390): a red line across the whole layer at
/// each marked point, with a short bar at the point whose length follows Data > Analysis Markers' size (a
/// third of its height).
fn draw_data_markers(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry, (a, b): (usize, usize)) {
    let stroke = Stroke::solid(sheaf_core::Color::rgb(255, 0, 0), 1.0);
    let bar = pd.marker_display.size.half_height_pt() / 3.0;
    let (top, bottom) = (geom.frame.y, geom.frame.y + geom.frame.h);
    for row in [a, b] {
        if !(pd.start..pd.end).contains(&row) {
            continue;
        }
        let (x, y) = geom.data_to_page(pd.x_at(row), pd.y_at(row));
        if !(x.is_finite() && y.is_finite()) {
            continue;
        }
        let mut d = String::new();
        move_to(&mut d, x, top);
        line_to(&mut d, x, bottom);
        move_to(&mut d, x - bar, y);
        line_to(&mut d, x + bar, y);
        out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
    }
}

#[cfg(test)]
mod step_center_tests {
    use super::*;

    #[test]
    fn centred_steps_turn_halfway_between_the_points() {
        // T252: Step Horizontal Center (Origin's line connection 13) and Step Vertical Center.
        let pts = [(0.0, 0.0), (2.0, 4.0), (6.0, 2.0)];
        let mut h = String::new();
        polyline_path(&mut h, &pts, Connect::StepHorizontalCenter);
        let mut want = String::new();
        move_to(&mut want, 0.0, 0.0);
        for (x, y) in [(1.0, 0.0), (1.0, 4.0), (2.0, 4.0), (4.0, 4.0), (4.0, 2.0), (6.0, 2.0)] {
            line_to(&mut want, x, y);
        }
        assert_eq!(h, want);
        let mut v = String::new();
        polyline_path(&mut v, &pts, Connect::StepVerticalCenter);
        let mut want = String::new();
        move_to(&mut want, 0.0, 0.0);
        for (x, y) in [(0.0, 2.0), (2.0, 2.0), (2.0, 4.0), (2.0, 3.0), (6.0, 3.0), (6.0, 2.0)] {
            line_to(&mut want, x, y);
        }
        assert_eq!(v, want);
    }
}
