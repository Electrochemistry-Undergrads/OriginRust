//! Drawing the data of one plot: lines, error bars and symbols (spec section 4).

use crate::LayerGeometry;
use crate::Rect;
use crate::data::PlotData;
use crate::scene::{Cap, Item, Join, Stroke, line_to, move_to, push_num};
use sheaf_core::graph::{Connect, ErrorDirection, LineStyle, PlotKind, ScaleKind, SymbolShape, SymbolStyle};

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
    let dash: Vec<f64> = line.dash.pattern().iter().map(|m| m * line.width_pt).collect();
    let cap = if dash.is_empty() { Cap::Round } else { Cap::Butt };
    Stroke { color: line.color, width: line.width_pt, dash, cap, join: Join::Round }
}

/// Appends the items of one plot to `out`.
pub(crate) fn draw_plot(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let plot = pd.plot;
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
    if plot.kind.is_area() {
        draw_area(out, pd, geom);
    }
    if plot.kind.has_line() && plot.line.width_pt > 0.0 {
        draw_line(out, pd, geom);
    }
    if plot.kind == PlotKind::Box {
        draw_box(out, pd, geom);
    } else if plot.kind.is_bar() {
        draw_columns(out, pd, geom);
    }
    if pd.err.is_some() || pd.xerr.is_some() {
        draw_error_bars(out, pd, geom);
    }
    if plot.kind.has_symbols() {
        draw_symbols(out, pd, geom);
    }
    if pd.mask.is_some() && !plot.kind.is_bar() {
        draw_masked(out, pd, geom);
    }
}

/// Origin draws a masked point red: its symbol, and the two line segments that meet at it
/// (measured with a line + symbol plot of five points, one masked). Drawn on top of the plot.
fn draw_masked(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    use sheaf_core::Color;
    let Some(mask) = pd.mask else { return };
    let page = |i: usize| -> Option<(f64, f64)> {
        if i < pd.start || i >= pd.end {
            return None;
        }
        geom.checked_data_to_page(pd.x_at(i), pd.y_at(i))
    };
    let mut lines = String::new();
    let mut symbols = String::new();
    for &i in mask.range(pd.start..pd.end) {
        let Some(p) = page(i) else { continue };
        if pd.plot.kind.has_line() && pd.plot.line.width_pt > 0.0 {
            for j in [i.checked_sub(1), Some(i + 1)].into_iter().flatten() {
                if let Some(q) = page(j) {
                    move_to(&mut lines, q.0, q.1);
                    line_to(&mut lines, p.0, p.1);
                }
            }
        }
        if pd.plot.kind.has_symbols() {
            symbol_path(&mut symbols, pd.plot.symbol.shape, p.0, p.1, pd.plot.symbol.size_pt);
        }
    }
    if !lines.is_empty() {
        let line = crate::plots::line_stroke(&pd.plot.line);
        out.push(Item::Path { d: lines, fill: None, stroke: Some(Stroke { color: Color::RED, ..line }) });
    }
    if !symbols.is_empty() {
        let red = SymbolStyle { color: Color::RED, ..pd.plot.symbol.clone() };
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
    let step = ((pd.end - pd.start) / 20_000).max(1);
    let mut d = String::new();
    // Upper points, and the lower point under each.
    let mut run: Vec<((f64, f64), f64)> = Vec::new();
    let close = |run: &mut Vec<((f64, f64), f64)>, d: &mut String| {
        if run.len() >= 2 {
            move_to(d, run[0].0.0, run[0].1);
            for &((x, y), _) in run.iter() {
                line_to(d, x, y);
            }
            for &((x, _), low) in run.iter().rev() {
                line_to(d, x, low);
            }
            d.push('Z');
        }
        run.clear();
    };
    let mut i = pd.start;
    while i < pd.end {
        let upper = geom.checked_data_to_page(pd.x_at(i), pd.y_at(i));
        // The lower edge of this row; a row without one breaks the shape.
        let lower = match pd.base_at(i) {
            None => Some(flat),
            Some(b) => geom.checked_data_to_page(pd.x_at(i), b).map(|p| p.1),
        };
        match upper.zip(lower) {
            Some((p, low)) => run.push((p, low)),
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
    let connect = match pd.plot.line.connect {
        Connect::Spline if !spline_applies(pd) => Connect::Straight,
        c => c,
    };
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
    if pd.end - pd.start > SPLINE_MAX_POINTS {
        return false;
    }
    let mut last = f64::NEG_INFINITY;
    for i in pd.start..pd.end {
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
    for i in pd.start..pd.end {
        let (cat, top) = (pd.cat_at(i), pd.val_at(i));
        let base = pd.base_at(i).unwrap_or(default_base);
        let left = cat + slot.left;
        let (Some(a), Some(b)) = (place(left, base), place(left + slot.width, top)) else {
            continue;
        };
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        out.push(Item::Rect {
            rect: Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 },
            fill: (!histogram).then_some(plot.symbol.color),
            stroke: stroke.clone(),
        });
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
            text: format!("{:.2}%", share * 100.0),
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
    let Some(b) = &pd.boxed else { return };
    let color = pd.plot.symbol.color;
    let width = pd.plot.line.width_pt.max(0.5);
    let at = |x: f64, y: f64| geom.checked_data_to_page(x, y);
    let (l, r) = (b.pos - crate::data::BOX_WIDTH / 2.0, b.pos + crate::data::BOX_WIDTH / 2.0);
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
    let style =
        |shape, size| SymbolStyle { shape, size_pt: size, color, filled: false, edge_width_pt: width, every: 1 };
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
    for i in pd.start..pd.end {
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
const BUBBLE_MIN_PT: f64 = 2.0;
const BUBBLE_MAX_PT: f64 = 20.0;

/// The colour of a colour mapped symbol at `t` in 0..1: blue, cyan, green, yellow, red (a stand-in:
/// Origin's default colour map was not measured).
fn map_color(t: f64) -> sheaf_core::Color {
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
fn z_fraction(z: &[f64], from: usize, to: usize, v: f64) -> f64 {
    let (lo, hi) = z[from.min(z.len())..to.min(z.len())]
        .iter()
        .filter(|x| x.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
    if hi > lo { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.5 }
}

/// A scatter plot whose symbols differ: bubbles of different sizes and/or colours, one scene item
/// per symbol.
fn draw_varied_symbols(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let sym = &pd.plot.symbol;
    for i in pd.start..pd.end {
        let Some(p) = geom.checked_data_to_page(pd.x_at(i), pd.y_at(i)) else { continue };
        let mut style = sym.clone();
        if let Some(z) = pd.size_z {
            let v = z.get(i).copied().unwrap_or(f64::NAN);
            if !v.is_finite() {
                continue;
            }
            style.size_pt = BUBBLE_MIN_PT + (BUBBLE_MAX_PT - BUBBLE_MIN_PT) * z_fraction(z, pd.start, pd.end, v);
        }
        if let Some(z) = pd.color_z {
            let v = z.get(i).copied().unwrap_or(f64::NAN);
            if !v.is_finite() {
                continue;
            }
            style.color = map_color(z_fraction(z, pd.start, pd.end, v));
        }
        if !near_frame(geom, p, style.size_pt + style.edge_width_pt) {
            continue;
        }
        let mut d = String::new();
        symbol_path(&mut d, style.shape, p.0, p.1, style.size_pt);
        out.push(symbol_item(d, &style));
    }
}

fn draw_symbols(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    if pd.size_z.is_some() || pd.color_z.is_some() {
        return draw_varied_symbols(out, pd, geom);
    }
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
pub(crate) fn smooth_for_test(connect: Connect, pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    match connect {
        Connect::Spline => natural_spline(pts),
        Connect::BSpline => clamped_b_spline(pts),
        Connect::Bezier => bezier_groups(pts),
        _ => pts.to_vec(),
    }
}
