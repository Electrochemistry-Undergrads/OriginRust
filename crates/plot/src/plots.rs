//! Drawing the data of one plot: lines, error bars and symbols (spec section 4).

use crate::LayerGeometry;
use crate::Rect;
use crate::data::PlotData;
use crate::scene::{Cap, Item, Join, Stroke, line_to, move_to, push_num};
use sheaf_core::graph::{Connect, LineStyle, PlotKind, ScaleKind, SymbolShape, SymbolStyle};

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
/// Pieces with more points than this are never drawn as splines.
const SPLINE_MAX_POINTS: usize = 2000;

/// The stroke of a data line.
pub(crate) fn line_stroke(line: &LineStyle) -> Stroke {
    let dash: Vec<f64> = line.dash.pattern().iter().map(|m| m * line.width_pt).collect();
    let cap = if dash.is_empty() { Cap::Round } else { Cap::Butt };
    Stroke { color: line.color, width: line.width_pt, dash, cap, join: Join::Round }
}

/// Appends the items of one plot to `out`.
pub(crate) fn draw_plot(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let plot = pd.plot;
    if plot.kind == PlotKind::Area {
        draw_area(out, pd, geom);
    }
    if plot.kind.has_line() && plot.line.width_pt > 0.0 {
        draw_line(out, pd, geom);
    }
    if plot.kind == PlotKind::Column {
        draw_columns(out, pd, geom);
    }
    if pd.err.is_some() {
        draw_error_bars(out, pd, geom);
    }
    if plot.kind.has_symbols() {
        draw_symbols(out, pd, geom);
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

/// The fill of an area plot: one closed shape per unbroken run of points, from the page height of
/// Y = 0 (or the nearer frame edge when 0 cannot be placed or lies outside) up to the curve. Very
/// long runs are thinned to about 20000 points, which looks the same at any export size.
fn draw_area(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let f = geom.frame;
    let zero = geom.data_to_page(0.0, if pd.oy == 0.0 { 0.0 } else { pd.oy }).1;
    let base = if zero.is_finite() { zero.clamp(f.y, f.y + f.h) } else { f.y + f.h };
    let step = ((pd.end - pd.start) / 20_000).max(1);
    let mut d = String::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    let close = |run: &mut Vec<(f64, f64)>, d: &mut String| {
        if run.len() >= 2 {
            move_to(d, run[0].0, base);
            for &(x, y) in run.iter() {
                line_to(d, x, y);
            }
            line_to(d, run[run.len() - 1].0, base);
            d.push('Z');
        }
        run.clear();
    };
    let mut i = pd.start;
    while i < pd.end {
        match geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) {
            Some(p) => run.push(p),
            None => close(&mut run, &mut d),
        }
        i = if i + step < pd.end || i + 1 == pd.end { i + step } else { pd.end - 1 };
    }
    close(&mut run, &mut d);
    if !d.is_empty() {
        out.push(Item::Path { d, fill: Some(pd.plot.symbol.color), stroke: None });
    }
}

fn draw_line(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let stroke = line_stroke(&pd.plot.line);
    let connect = pd.plot.line.connect;
    let mut piece: Vec<(f64, f64)> = Vec::new();
    let mut pending: Option<(f64, f64)> = None;
    let flush = |piece: &mut Vec<(f64, f64)>, pending: &mut Option<(f64, f64)>, out: &mut Vec<Item>| {
        if let Some(p) = pending.take() {
            piece.push(p);
        }
        if piece.len() >= 2 {
            let mut d = String::with_capacity(piece.len() * 12);
            polyline_path(&mut d, piece, connect);
            out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
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
    for i in pd.start..pd.end {
        match geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) {
            Some(p) => {
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
            None => {
                if let Some(r) = run.take() {
                    r.emit(|q| keep(&mut piece, &mut pending, q));
                }
                flush(&mut piece, &mut pending, out);
            }
        }
    }
    if let Some(r) = run.take() {
        r.emit(|q| keep(&mut piece, &mut pending, q));
    }
    flush(&mut piece, &mut pending, out);
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
        Connect::Spline if pts.len() > 2 && pts.len() <= SPLINE_MAX_POINTS => {
            let n = pts.len();
            for i in 0..n - 1 {
                let p0 = pts[i.saturating_sub(1)];
                let (p1, p2) = (pts[i], pts[i + 1]);
                let p3 = pts[(i + 2).min(n - 1)];
                d.push('C');
                let coords = [
                    p1.0 + (p2.0 - p0.0) / 6.0,
                    p1.1 + (p2.1 - p0.1) / 6.0,
                    p2.0 - (p3.0 - p1.0) / 6.0,
                    p2.1 - (p3.1 - p1.1) / 6.0,
                    p2.0,
                    p2.1,
                ];
                for (k, c) in coords.iter().enumerate() {
                    if k > 0 {
                        d.push(if k % 2 == 0 { ' ' } else { ',' });
                    }
                    push_num(d, *c);
                }
            }
        }
        Connect::Spline => {
            for &(x, y) in &pts[1..] {
                line_to(d, x, y);
            }
        }
    }
}

/// True when the point is within `margin` of the frame.
fn near_frame(geom: &LayerGeometry, p: (f64, f64), margin: f64) -> bool {
    let f = geom.frame;
    p.0 >= f.x - margin && p.0 <= f.x + f.w + margin && p.1 >= f.y - margin && p.1 <= f.y + f.h + margin
}

/// One bar per row from the base to the value, in the plot's slot of the layer's group of bars
/// (graphing spec 14.4). The base is 0 on a linear Y axis (the plot's offset in a stacked layer)
/// and the lower axis limit on a log axis. A bar with a corner that cannot be placed is skipped.
fn draw_columns(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let plot = pd.plot;
    let slot = pd.bar.unwrap_or(crate::data::BarSlot { left: -0.4, width: 0.8, spacing: 1.0 });
    let base = match geom.y.scale {
        ScaleKind::Linear => pd.oy,
        ScaleKind::Log10 => geom.y.min,
    };
    let stroke = (plot.line.width_pt > 0.0).then(|| Stroke::solid(plot.line.color, plot.line.width_pt));
    for i in pd.start..pd.end {
        let (x, y) = (pd.x_at(i), pd.y_at(i));
        let left = x + slot.left;
        let (Some(a), Some(b)) =
            (geom.checked_data_to_page(left, base), geom.checked_data_to_page(left + slot.width, y))
        else {
            continue;
        };
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        out.push(Item::Rect {
            rect: Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 },
            fill: Some(plot.symbol.color),
            stroke: stroke.clone(),
        });
    }
}

fn draw_error_bars(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let plot = pd.plot;
    let column = plot.kind == PlotKind::Column;
    let color = if plot.kind.has_line() || column { plot.line.color } else { plot.symbol.color };
    let half_cap = 0.6 * plot.symbol.size_pt;
    let width = if column { plot.line.width_pt.max(0.5) } else { (plot.line.width_pt / 2.0).max(0.5) };
    // A column plot's error bars stand in the middle of its own bars.
    let shift = pd.bar.map_or(0.0, |b| b.centre());
    let mut d = String::new();
    for i in pd.start..pd.end {
        let x = if shift == 0.0 { pd.x_at(i) } else { pd.x_at(i) + shift };
        let (y, e) = (pd.y_at(i), pd.err_at(i));
        if !e.is_finite() {
            continue;
        }
        let (Some(top), Some(bottom)) = (geom.checked_data_to_page(x, y + e), geom.checked_data_to_page(x, y - e))
        else {
            continue;
        };
        if !(near_frame(geom, top, half_cap + width) || near_frame(geom, bottom, half_cap + width)) {
            continue;
        }
        move_to(&mut d, top.0, top.1);
        line_to(&mut d, bottom.0, bottom.1);
        move_to(&mut d, top.0 - half_cap, top.1);
        line_to(&mut d, top.0 + half_cap, top.1);
        move_to(&mut d, bottom.0 - half_cap, bottom.1);
        line_to(&mut d, bottom.0 + half_cap, bottom.1);
    }
    if !d.is_empty() {
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke::solid(color, width)) });
    }
}

fn draw_symbols(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let sym = &pd.plot.symbol;
    let every = sym.every.max(1) as usize;
    let mut d = String::new();
    // With very many points, symbols that would sit on top of an already drawn
    // one (same small cell of the page) are left out; the picture is the same.
    let thin = (pd.end - pd.start) / every > SYMBOL_THIN_ABOVE;
    let mut taken: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();
    for i in (pd.start..pd.end).step_by(every) {
        let Some(p) = geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) else { continue };
        if !near_frame(geom, p, sym.size_pt + sym.edge_width_pt) {
            continue;
        }
        if thin && !taken.insert(((p.0 / SYMBOL_CELL_PT).floor() as i64, (p.1 / SYMBOL_CELL_PT).floor() as i64)) {
            continue;
        }
        symbol_path(&mut d, sym.shape, p.0, p.1, sym.size_pt);
    }
    if d.is_empty() {
        return;
    }
    out.push(symbol_item(d, sym));
}

/// The scene item for symbol path data written by [`symbol_path`].
pub(crate) fn symbol_item(d: String, sym: &SymbolStyle) -> Item {
    let stroke_only = matches!(sym.shape, SymbolShape::Plus | SymbolShape::Cross | SymbolShape::Star);
    let stroke =
        Stroke { color: sym.color, width: sym.edge_width_pt, dash: Vec::new(), cap: Cap::Butt, join: Join::Miter };
    if sym.filled && !stroke_only {
        Item::Path { d, fill: Some(sym.color), stroke: None }
    } else {
        Item::Path { d, fill: None, stroke: Some(stroke) }
    }
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
pub(crate) fn polyline_path_for_test(d: &mut String, pts: &[(f64, f64)]) {
    polyline_path(d, pts, Connect::Spline);
}
