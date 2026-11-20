//! Heat maps and contour plots of matrices and of XYZ columns. The rules (levels, colours, the
//! colour scale, what was measured in Origin 2016) are in `docs/spec/matrices.md`, section
//! "Graphs".

use crate::LayerGeometry;
use crate::Rect;
use crate::fonts::TextMeasurer;
use crate::plots::line_stroke;
use crate::scene::{Anchor, Item, Stroke, line_to, move_to};
use crate::text;
use sheaf_core::graph::{FieldKind, FieldPalette, FieldPlot, FieldSource, Font, Page};
use sheaf_core::matrix::spaced;
use sheaf_core::matrix_convert::{scatter_to_matrix, xyz_to_matrix};
use sheaf_core::{Color, Project};
use std::collections::HashMap;

/// The numbers of a field with the coordinates of its columns and rows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FieldGrid {
    pub rows: usize,
    pub cols: usize,
    pub x_from: f64,
    pub x_to: f64,
    pub y_from: f64,
    pub y_to: f64,
    /// Row by row.
    pub z: Vec<f64>,
    pub x_label: String,
    pub y_label: String,
    pub z_label: String,
    /// Blocks around the cell centres (a heat map) instead of contours.
    pub cells: bool,
}

impl FieldGrid {
    pub fn x_at(&self, c: usize) -> f64 {
        spaced(self.x_from, self.x_to, c, self.cols)
    }

    pub fn y_at(&self, r: usize) -> f64 {
        spaced(self.y_from, self.y_to, r, self.rows)
    }

    fn step(from: f64, to: f64, n: usize) -> f64 {
        if n > 1 { (to - from) / (n - 1) as f64 } else { 1.0 }
    }

    /// The part of an axis the field covers: the coordinates of the first and last column (or
    /// row), widened by half a cell on both sides for blocks.
    pub fn extent(&self, x_axis: bool) -> (f64, f64) {
        let (from, to, n) =
            if x_axis { (self.x_from, self.x_to, self.cols) } else { (self.y_from, self.y_to, self.rows) };
        let (lo, hi) = (from.min(to), from.max(to));
        if self.cells {
            let half = Self::step(from, to, n).abs() / 2.0;
            (lo - half, hi + half)
        } else {
            (lo, hi)
        }
    }

    fn at(&self, r: usize, c: usize) -> f64 {
        self.z[r * self.cols + c]
    }

    fn z_range(&self) -> Option<(f64, f64)> {
        let mut r: Option<(f64, f64)> = None;
        for &v in self.z.iter().filter(|v| v.is_finite()) {
            r = Some(r.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
        }
        r
    }
}

/// Reads the field's numbers from the project; `None` when the source is missing, empty, or XYZ
/// columns that do not form a grid for a heat map.
pub(crate) fn resolve(project: &Project, field: &FieldPlot) -> Option<FieldGrid> {
    let cells = field.kind == FieldKind::Cells;
    match &field.source {
        FieldSource::Matrix { window, sheet } => {
            let s = project.matrix_book(*window)?.sheets.get(*sheet)?;
            let z = s.values();
            if z.is_empty() {
                return None;
            }
            Some(FieldGrid {
                rows: s.rows,
                cols: s.cols,
                x_from: s.x_from,
                x_to: s.x_to,
                y_from: s.y_from,
                y_to: s.y_to,
                z: z.to_vec(),
                x_label: s.x.title(),
                y_label: s.y.title(),
                z_label: s.z.title(),
                cells,
            })
        }
        FieldSource::Ternary { a, b, c, z } => {
            // Each point's compositions as fractions of their sum; then scattered points over
            // the fractions of A and B (C is the rest), triangulated as for XYZ columns.
            let (av, bv, cv, zs) =
                (project.numeric(*a)?, project.numeric(*b)?, project.numeric(*c)?, project.numeric(*z)?);
            let n = av.len().min(bv.len()).min(cv.len()).min(zs.len());
            let (mut xs, mut ys, mut vs) = (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n));
            for k in 0..n {
                let sum = av[k] + bv[k] + cv[k];
                if sum.is_finite() && sum > 0.0 && zs[k].is_finite() {
                    xs.push(av[k] / sum);
                    ys.push(bv[k] / sum);
                    vs.push(zs[k]);
                }
            }
            let m = scatter_to_matrix(&xs, &ys, &vs, 0, 0, "").ok()?;
            let label = |id| project.column(id).map(|c| c.axis_title()).unwrap_or_default();
            Some(FieldGrid {
                rows: m.rows,
                cols: m.cols,
                x_from: m.x_from,
                x_to: m.x_to,
                y_from: m.y_from,
                y_to: m.y_to,
                z: m.values().to_vec(),
                x_label: label(*a),
                y_label: label(*b),
                z_label: label(*z),
                cells,
            })
        }
        FieldSource::Xyz { x, y, z } => {
            let (xs, ys, zs) = (project.numeric(*x)?, project.numeric(*y)?, project.numeric(*z)?);
            let m = match xyz_to_matrix(xs, ys, zs, "") {
                Ok(m) => m,
                Err(_) if !cells => scatter_to_matrix(xs, ys, zs, 0, 0, "").ok()?,
                Err(_) => return None,
            };
            let label = |id| project.column(id).map(|c| c.axis_title()).unwrap_or_default();
            Some(FieldGrid {
                rows: m.rows,
                cols: m.cols,
                x_from: m.x_from,
                x_to: m.x_to,
                y_from: m.y_from,
                y_to: m.y_to,
                z: m.values().to_vec(),
                x_label: label(*x),
                y_label: label(*y),
                z_label: label(*z),
                cells,
            })
        }
    }
}

/// The cells of a layer's heat map or contour plot (T189, the 2D Integrate gadget): each grid
/// point's X, Y and Z (missing values left out), and the area one cell covers (|dX dY|). `None`
/// for a layer without a field or with an empty one.
pub fn field_cells(project: &Project, layer: &sheaf_core::graph::Layer) -> Option<(Vec<[f64; 3]>, f64)> {
    let grid = resolve(project, layer.field.as_ref()?)?;
    let dx = FieldGrid::step(grid.x_from, grid.x_to, grid.cols).abs();
    let dy = FieldGrid::step(grid.y_from, grid.y_to, grid.rows).abs();
    let cells = (0..grid.rows)
        .flat_map(|r| (0..grid.cols).map(move |c| (r, c)))
        .map(|(r, c)| [grid.x_at(c), grid.y_at(r), grid.at(r, c)])
        .filter(|p| p[2].is_finite())
        .collect();
    Some((cells, dx * dy))
}

// ----- levels -------------------------------------------------------------------------------

/// The largest of 1, 2, 5 times a power of ten that is not above `v` (`v` > 0).
fn nice_floor(v: f64) -> f64 {
    let e = v.log10().floor();
    let base = 10f64.powf(e);
    let m = v / base;
    // Guard against 10^e being a hair too large or small for `v` (log10 rounding).
    let m = if m >= 10.0 - 1e-12 { 10.0 } else { m };
    base * if m >= 5.0 {
        5.0
    } else if m >= 2.0 {
        2.0
    } else {
        1.0
    }
}

/// The range of Z that the colour scale covers when the user did not set one: the lowest value
/// rounded down and the highest rounded up to a multiple of a quantum, the largest of 1, 2, 5 times
/// a power of ten that is at most 1/200 of the span of the data (measured in Origin 2016).
pub(crate) fn auto_range(lo: f64, hi: f64) -> (f64, f64) {
    let span = hi - lo;
    if !(span > 0.0 && span.is_finite()) {
        let pad = if lo.is_finite() && lo != 0.0 { lo.abs() * 0.5 } else { 1.0 };
        return (lo - pad, hi + pad);
    }
    let q = nice_floor(span / 200.0);
    // The lowest value goes down to a multiple of the quantum. The highest goes up to one, unless
    // it is one already: then it stays as it is (Origin's labels show the float noise that
    // follows from this: z from -3.3 to 3.3 has a middle label of -4.441E-16).
    let top = hi / q;
    let hi = if (top - top.round()).abs() < 1e-9 { hi } else { top.ceil() * q };
    ((lo / q + 1e-9).floor() * q, hi)
}

/// The levels of a field: `n = major * minor` bands between `lo` and `hi`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Levels {
    pub lo: f64,
    pub hi: f64,
    pub major: usize,
    pub minor: usize,
}

impl Levels {
    pub fn bands(&self) -> usize {
        self.major * self.minor
    }

    /// The value at the lower edge of band `k` (`k = bands` is the highest level).
    pub fn boundary(&self, k: usize) -> f64 {
        self.lo + k as f64 * (self.hi - self.lo) / self.bands() as f64
    }

    /// The value of major level `j` (0 = lowest, `major` = highest).
    pub fn major_level(&self, j: usize) -> f64 {
        self.lo + j as f64 * (self.hi - self.lo) / self.major as f64
    }

    /// The band a value is in: `Err(false)` below the scale, `Err(true)` above it.
    pub fn band(&self, v: f64) -> Result<usize, bool> {
        if v < self.lo {
            return Err(false);
        }
        if v > self.hi {
            return Err(true);
        }
        let n = self.bands();
        let k = ((v - self.lo) / (self.hi - self.lo) * n as f64).floor();
        Ok((k.max(0.0) as usize).min(n - 1))
    }
}

pub(crate) fn levels(field: &FieldPlot, grid: &FieldGrid) -> Option<Levels> {
    let (dlo, dhi) = grid.z_range()?;
    let (alo, ahi) = auto_range(dlo, dhi);
    let (lo, hi) = (field.levels.min.unwrap_or(alo), field.levels.max.unwrap_or(ahi));
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    if !(lo.is_finite() && hi.is_finite()) || lo == hi {
        return None;
    }
    Some(Levels { lo, hi, major: field.levels.major.max(1) as usize, minor: field.levels.minor.max(1) as usize })
}

/// A level as the colour scale writes it: `digits` significant digits (trailing zeros kept), in
/// the form 1.234E+05 when the exponent is below -3 or reaches `digits`; an exact zero is `0.000`.
pub(crate) fn format_level(v: f64, digits: usize) -> String {
    if v == 0.0 {
        return "0.000".to_string();
    }
    if !v.is_finite() {
        return String::new();
    }
    let sci = format!("{:.*e}", digits - 1, v);
    let (mantissa, exp) = sci.split_once('e').expect("exponent form");
    let e: i32 = exp.parse().expect("integer exponent");
    if e < -3 || e >= digits as i32 {
        format!("{mantissa}E{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
    } else {
        let decimals = (digits as i32 - 1 - e).max(0) as usize;
        format!("{v:.decimals$}")
    }
}

// ----- colours ------------------------------------------------------------------------------

fn lerp_color(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

/// The colour of band `k` of `n` (0 = lowest): the palette read at `k / (n - 1)`.
pub(crate) fn band_color(palette: FieldPalette, k: usize, n: usize) -> Color {
    let t = if n > 1 { k as f64 / (n - 1) as f64 } else { 0.5 };
    let stops: &[(f64, f64, f64)] = match palette {
        FieldPalette::Rainbow => {
            &[(0.0, 0.0, 255.0), (0.0, 255.0, 255.0), (0.0, 255.0, 0.0), (255.0, 255.0, 0.0), (255.0, 0.0, 0.0)]
        }
        FieldPalette::BlueWhiteRed => &[(0.0, 0.0, 255.0), (255.0, 255.0, 255.0), (255.0, 0.0, 0.0)],
        FieldPalette::Gray => &[(0.0, 0.0, 0.0), (255.0, 255.0, 255.0)],
    };
    let x = t * (stops.len() - 1) as f64;
    let i = (x.floor() as usize).min(stops.len() - 2);
    let c = lerp_color(stops[i], stops[i + 1], x - i as f64);
    Color::rgb(c.0.round() as u8, c.1.round() as u8, c.2.round() as u8)
}

/// The colour of values below and above the scale.
fn out_of_range_color(palette: FieldPalette, above: bool) -> Color {
    match (palette, above) {
        (FieldPalette::Rainbow, false) => Color::BLACK,
        _ => Color::WHITE,
    }
}

// ----- geometry helpers ---------------------------------------------------------------------

type P = (f64, f64);

fn page(geom: &LayerGeometry, p: P) -> P {
    geom.data_to_page(p.0, p.1)
}

fn push_poly(d: &mut String, pts: &[P], geom: &LayerGeometry) {
    for (i, p) in pts.iter().enumerate() {
        let q = page(geom, *p);
        if i == 0 {
            move_to(d, q.0, q.1);
        } else {
            line_to(d, q.0, q.1);
        }
    }
    d.push('Z');
}

/// The polygon of band `[a, b]` inside the cell with corners `c` (counter-clockwise: lower left,
/// lower right, upper right, upper left) and values `v`: the corners that are in the band and, on
/// every edge, the places where the edge reaches `a` or `b`, in order around the cell.
fn band_polygon(c: [P; 4], v: [f64; 4], a: f64, b: f64) -> Vec<P> {
    let mut out: Vec<P> = Vec::with_capacity(8);
    for i in 0..4 {
        let j = (i + 1) % 4;
        let (zi, zj) = (v[i], v[j]);
        if zi >= a && zi <= b {
            out.push(c[i]);
        }
        let mut ts: Vec<f64> = Vec::new();
        for level in [a, b] {
            if (zi < level && zj > level) || (zi > level && zj < level) {
                ts.push((level - zi) / (zj - zi));
            }
        }
        ts.sort_by(f64::total_cmp);
        for t in ts {
            out.push((c[i].0 + (c[j].0 - c[i].0) * t, c[i].1 + (c[j].1 - c[i].1) * t));
        }
    }
    out.dedup_by(|p, q| (p.0 - q.0).abs() < 1e-12 && (p.1 - q.1).abs() < 1e-12);
    out
}

// ----- drawing ------------------------------------------------------------------------------

/// What `draw` needs besides the plot.
pub(crate) struct Env<'a> {
    pub page: &'a Page,
    pub measurer: &'a dyn TextMeasurer,
}

/// Draws the field into `plots` (inside the layer's clip) and its colour scale into `overlay`.
pub(crate) fn draw(
    plots: &mut Vec<Item>,
    overlay: &mut Vec<Item>,
    field: &FieldPlot,
    grid: &FieldGrid,
    geom: &LayerGeometry,
    env: &Env,
) {
    let Some(lv) = levels(field, grid) else { return };
    if field.fill {
        match field.kind {
            FieldKind::Cells => draw_cells(plots, field, grid, geom, &lv),
            FieldKind::Contour => draw_bands(plots, field, grid, geom, &lv),
        }
    }
    if field.lines && field.kind == FieldKind::Contour {
        draw_lines(plots, field, grid, geom, &lv, env);
    }
    if field.scale.visible && field.fill {
        draw_scale(overlay, field, grid, geom, &lv, env);
    }
}

fn draw_cells(out: &mut Vec<Item>, field: &FieldPlot, grid: &FieldGrid, geom: &LayerGeometry, lv: &Levels) {
    let n = lv.bands();
    let half_x = FieldGrid::step(grid.x_from, grid.x_to, grid.cols) / 2.0;
    let half_y = FieldGrid::step(grid.y_from, grid.y_to, grid.rows) / 2.0;
    // A little more than the cell to the right and down, so that no hairline of the background
    // shows between blocks (the blocks are painted from the top left, so the next ones cover the
    // extra).
    const OVERLAP: f64 = 0.8;
    // The page extent of a cell along X (left, right) and along Y (top, bottom).
    let across = |c: usize| {
        let (a, b) = (geom.data_to_page(grid.x_at(c) - half_x, 0.0).0, geom.data_to_page(grid.x_at(c) + half_x, 0.0).0);
        (a.min(b), a.max(b))
    };
    let down = |r: usize| {
        let (a, b) = (geom.data_to_page(0.0, grid.y_at(r) - half_y).1, geom.data_to_page(0.0, grid.y_at(r) + half_y).1);
        (a.min(b), a.max(b))
    };
    let mut rows: Vec<usize> = (0..grid.rows).collect();
    rows.sort_by(|&a, &b| down(a).0.total_cmp(&down(b).0));
    let mut cols: Vec<usize> = (0..grid.cols).collect();
    cols.sort_by(|&a, &b| across(a).0.total_cmp(&across(b).0));
    for &r in &rows {
        let (top, bottom) = down(r);
        let mut i = 0;
        while i < cols.len() {
            let v = grid.at(r, cols[i]);
            if v.is_nan() {
                i += 1;
                continue;
            }
            let band = lv.band(v);
            let mut end = i + 1;
            while end < cols.len() && !grid.at(r, cols[end]).is_nan() && lv.band(grid.at(r, cols[end])) == band {
                end += 1;
            }
            let color = match band {
                Ok(k) => band_color(field.palette, k, n),
                Err(above) => out_of_range_color(field.palette, above),
            };
            let (left, right) = (across(cols[i]).0, across(cols[end - 1]).1);
            if [left, right, top, bottom].iter().all(|v| v.is_finite()) {
                let rect = Rect { x: left, y: top, w: right - left + OVERLAP, h: bottom - top + OVERLAP };
                out.push(Item::Rect { rect, fill: Some(color), stroke: None });
            }
            i = end;
        }
    }
}

fn draw_bands(out: &mut Vec<Item>, field: &FieldPlot, grid: &FieldGrid, geom: &LayerGeometry, lv: &Levels) {
    let n = lv.bands();
    let width = (lv.hi - lv.lo) / n as f64;
    // Each band reaches a little into the next one, which is painted after it: no hairline of the
    // background shows between bands.
    let reach = width * 0.05;
    let mut paths: Vec<String> = vec![String::new(); n];
    for r in 0..grid.rows.saturating_sub(1) {
        for c in 0..grid.cols.saturating_sub(1) {
            let corners = [
                (grid.x_at(c), grid.y_at(r)),
                (grid.x_at(c + 1), grid.y_at(r)),
                (grid.x_at(c + 1), grid.y_at(r + 1)),
                (grid.x_at(c), grid.y_at(r + 1)),
            ];
            let v = [grid.at(r, c), grid.at(r, c + 1), grid.at(r + 1, c + 1), grid.at(r + 1, c)];
            if v.iter().any(|z| z.is_nan()) {
                continue;
            }
            let (zmin, zmax) = v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), z| (a.min(*z), b.max(*z)));
            let k_lo = match lv.band(zmin) {
                Ok(k) => k,
                Err(false) => 0,
                Err(true) => continue,
            };
            let k_hi = match lv.band(zmax) {
                Ok(k) => k,
                Err(true) => n - 1,
                Err(false) => continue,
            };
            for (k, path) in paths.iter_mut().enumerate().take(k_hi + 1).skip(k_lo) {
                let a = lv.boundary(k);
                let b = if k + 1 < n { lv.boundary(k + 1) + reach } else { lv.hi };
                let a = if k == 0 { f64::NEG_INFINITY } else { a };
                let b = if k + 1 == n { f64::INFINITY } else { b };
                let poly = band_polygon(corners, v, a, b);
                if poly.len() >= 3 {
                    push_poly(path, &poly, geom);
                }
            }
        }
    }
    for (k, d) in paths.into_iter().enumerate() {
        if !d.is_empty() {
            out.push(Item::Path { d, fill: Some(band_color(field.palette, k, n)), stroke: None });
        }
    }
}

/// An end of a marching-squares segment: the grid edge it lies on. `(horizontal, row, col)`: a
/// horizontal edge joins `(row, col)` and `(row, col + 1)`, a vertical one `(row, col)` and `(row + 1, col)`.
type EdgeId = (bool, usize, usize);

/// The polylines of the level `level`: each is a list of points in data coordinates and whether it
/// closes on itself.
fn level_lines(grid: &FieldGrid, level: f64) -> Vec<(Vec<P>, bool)> {
    let point = |e: EdgeId| -> P {
        let (horizontal, r, c) = e;
        let (r2, c2) = if horizontal { (r, c + 1) } else { (r + 1, c) };
        let (z1, z2) = (grid.at(r, c), grid.at(r2, c2));
        let t = (level - z1) / (z2 - z1);
        let (x1, y1) = (grid.x_at(c), grid.y_at(r));
        let (x2, y2) = (grid.x_at(c2), grid.y_at(r2));
        (x1 + (x2 - x1) * t, y1 + (y2 - y1) * t)
    };
    let mut segs: Vec<(EdgeId, EdgeId)> = Vec::new();
    for r in 0..grid.rows.saturating_sub(1) {
        for c in 0..grid.cols.saturating_sub(1) {
            let z = [grid.at(r, c), grid.at(r, c + 1), grid.at(r + 1, c + 1), grid.at(r + 1, c)];
            if z.iter().any(|v| v.is_nan()) {
                continue;
            }
            let above = z.map(|v| v >= level);
            // Edges in order around the cell: bottom, right, top, left.
            let edges: [EdgeId; 4] = [(true, r, c), (false, r, c + 1), (true, r + 1, c), (false, r, c)];
            let crossing: Vec<usize> = (0..4).filter(|&i| above[i] != above[(i + 1) % 4]).collect();
            match crossing.len() {
                2 => segs.push((edges[crossing[0]], edges[crossing[1]])),
                4 => {
                    // A saddle: the centre decides which corners are joined.
                    let centre = z.iter().sum::<f64>() / 4.0;
                    if (centre >= level) == above[0] {
                        // The centre is like corner 0: corners 1 and 3 are cut off.
                        segs.push((edges[0], edges[1]));
                        segs.push((edges[2], edges[3]));
                    } else {
                        segs.push((edges[3], edges[0]));
                        segs.push((edges[1], edges[2]));
                    }
                }
                _ => {}
            }
        }
    }
    // Join the segments end to end.
    let mut by_end: HashMap<EdgeId, Vec<usize>> = HashMap::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        by_end.entry(*a).or_default().push(i);
        by_end.entry(*b).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut out = Vec::new();
    let mut starts: Vec<usize> = (0..segs.len()).collect();
    // Open lines start at an end that no other segment shares.
    starts.sort_by_key(|&i| {
        let (a, b) = segs[i];
        let open = by_end[&a].len() == 1 || by_end[&b].len() == 1;
        (!open, i)
    });
    for s in starts {
        if used[s] {
            continue;
        }
        used[s] = true;
        let (a, b) = segs[s];
        // Start from the unshared end when there is one.
        let (first, mut tail) = if by_end[&a].len() == 1 {
            (a, b)
        } else if by_end[&b].len() == 1 {
            (b, a)
        } else {
            (a, b)
        };
        let mut ids = vec![first, tail];
        loop {
            let next = by_end[&tail].iter().copied().find(|&i| !used[i]);
            let Some(i) = next else { break };
            used[i] = true;
            let (p, q) = segs[i];
            tail = if p == tail { q } else { p };
            ids.push(tail);
            if tail == first {
                break;
            }
        }
        let closed = ids.len() > 2 && ids.first() == ids.last();
        if closed {
            ids.pop();
        }
        out.push((ids.into_iter().map(point).collect(), closed));
    }
    out
}

fn draw_lines(out: &mut Vec<Item>, field: &FieldPlot, grid: &FieldGrid, geom: &LayerGeometry, lv: &Levels, env: &Env) {
    let stroke = line_stroke(&field.line);
    if stroke.width <= 0.0 {
        return;
    }
    let mut labels: Vec<(f64, P, f64)> = Vec::new();
    let mut d = String::new();
    for j in 1..lv.major {
        let level = lv.major_level(j);
        let lines = level_lines(grid, level);
        let mut best: Option<(f64, Vec<P>)> = None;
        for (pts, closed) in &lines {
            let page_pts: Vec<P> = pts.iter().map(|p| page(geom, *p)).collect();
            if page_pts.len() < 2 || page_pts.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
                continue;
            }
            for (i, p) in page_pts.iter().enumerate() {
                if i == 0 {
                    move_to(&mut d, p.0, p.1);
                } else {
                    line_to(&mut d, p.0, p.1);
                }
            }
            if *closed {
                d.push('Z');
            }
            let mut length: f64 = page_pts.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum();
            if *closed {
                let (a, b) = (page_pts[0], page_pts[page_pts.len() - 1]);
                length += (a.0 - b.0).hypot(a.1 - b.1);
            }
            if best.as_ref().is_none_or(|(l, _)| length > *l) {
                best = Some((length, page_pts));
            }
        }
        if field.labels
            && let Some((length, pts)) = best
        {
            // Neighbouring levels are labelled at different places along their lines, so that the
            // labels of tightly nested lines do not run into each other.
            let fraction = 0.2 + 0.6 * (j as f64 * 0.618_033_988_75).fract();
            labels.push((level, point_at(&pts, length, fraction), length));
        }
    }
    if !d.is_empty() {
        out.push(Item::Path { d, fill: None, stroke: Some(Stroke { ..stroke }) });
    }
    let font = field.label_font.clone();
    for (level, at, length) in labels {
        let text_w = text::width(env.measurer, &format_level(level, 2), &font, font.size_pt);
        if length < text_w * 1.5 {
            continue;
        }
        let h = font.size_pt;
        let pad = 1.5;
        out.push(Item::Rect {
            rect: Rect {
                x: at.0 - text_w / 2.0 - pad,
                y: at.1 - h / 2.0 - pad / 2.0,
                w: text_w + 2.0 * pad,
                h: h + pad,
            },
            fill: Some(Color::WHITE),
            stroke: None,
        });
        let baseline = at.1 + (text::ASCENT - text::DESCENT) / 2.0 * h;
        text::place(out, env.measurer, &format_level(level, 2), &font, h, (at.0, baseline), Anchor::Middle, 0.0);
    }
}

/// The point `fraction` of the way along a polyline of total length `length`.
fn point_at(pts: &[P], length: f64, fraction: f64) -> P {
    let mut left = length * fraction;
    for w in pts.windows(2) {
        let seg = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        if seg >= left && seg > 0.0 {
            let t = left / seg;
            return (w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t);
        }
        left -= seg;
    }
    pts[pts.len() / 2]
}

/// The colour scale: a bar at the right of the frame as tall as the frame, with a cell for values
/// above the scale at its top and one for values below it at its bottom, a tick and a label at each
/// major level, and a title above. Positions follow what Origin 2016 draws on a page of the same
/// proportions: the bar starts 0.0875 of the page width right of the frame and is 0.04625 of it wide,
/// each end cell is 2 % of the bar's height.
fn draw_scale(out: &mut Vec<Item>, field: &FieldPlot, grid: &FieldGrid, geom: &LayerGeometry, lv: &Levels, env: &Env) {
    let f = geom.frame;
    let bar_x = f.x + f.w + 0.0875 * env.page.width_pt;
    let bar_w = 0.04625 * env.page.width_pt;
    let end = 0.02 * f.h;
    let (top, bottom) = (f.y + end, f.y + f.h - end);
    let n = lv.bands();
    let black = Stroke::solid(Color::BLACK, 0.75);
    // The cells beyond the scale.
    out.push(Item::Rect {
        rect: Rect { x: bar_x, y: f.y, w: bar_w, h: end },
        fill: Some(out_of_range_color(field.palette, true)),
        stroke: None,
    });
    out.push(Item::Rect {
        rect: Rect { x: bar_x, y: bottom, w: bar_w, h: end },
        fill: Some(out_of_range_color(field.palette, false)),
        stroke: None,
    });
    let band_h = (bottom - top) / n as f64;
    for k in 0..n {
        // Band 0 is at the bottom; a little extra height hides hairlines between bands.
        let y = bottom - (k + 1) as f64 * band_h;
        out.push(Item::Rect {
            rect: Rect { x: bar_x, y, w: bar_w, h: band_h + if k + 1 < n { 0.3 } else { 0.0 } },
            fill: Some(band_color(field.palette, k, n)),
            stroke: None,
        });
    }
    out.push(Item::Rect { rect: Rect { x: bar_x, y: f.y, w: bar_w, h: f.h }, fill: None, stroke: Some(black.clone()) });
    let font: &Font = &field.scale.font;
    let size = font.size_pt;
    for j in 0..=lv.major {
        let y = bottom - (bottom - top) * j as f64 / lv.major as f64;
        let mut d = String::new();
        move_to(&mut d, bar_x + bar_w, y);
        line_to(&mut d, bar_x + bar_w + 3.0, y);
        out.push(Item::Path { d, fill: None, stroke: Some(black.clone()) });
        let label = format_level(lv.major_level(j), 4);
        let baseline = y + (text::ASCENT - text::DESCENT) / 2.0 * size;
        text::place(out, env.measurer, &label, font, size, (bar_x + bar_w + 6.0, baseline), Anchor::Start, 0.0);
    }
    let title = field.scale.title.clone().unwrap_or_else(|| grid.z_label.clone());
    if !title.is_empty() {
        let tsize = field.scale.title_font.size_pt;
        text::place(
            out,
            env.measurer,
            &title,
            &field.scale.title_font,
            tsize,
            (bar_x + bar_w / 2.0, f.y - 0.45 * tsize),
            Anchor::Middle,
            0.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_of_the_data_is_rounded_out_to_a_quantum_of_a_two_hundredth_of_the_span() {
        // Measured in Origin 2016 (see docs/spec/matrices.md): data range -> range of the scale.
        let cases = [
            ((-4.961526564226862, 9.9409890610831), (-5.0, 9.95)),
            ((0.1234567, 0.8765432), (0.122, 0.878)),
            ((13.37, 98.76), (13.2, 98.8)),
            ((-0.0456, 0.0789), (-0.046, 0.079)),
            ((1234.5, 9876.5), (1220.0, 9880.0)),
            ((-73.1, 12.9), (-73.2, 13.0)),
            ((0.00123, 0.00987), (0.00122, 0.00988)),
            ((-99123.0, 200456.0), (-100000.0, 201000.0)),
            ((0.0, 7.3), (0.0, 7.3)),
            ((-0.02214, 0.02214), (-0.0222, 0.0222)),
            ((0.1234567, 12.1234567), (0.1, 12.15)),
            ((0.1234567, 23.1234567), (0.1, 23.2)),
            ((0.1234567, 43.1234567), (0.0, 43.2)),
            ((0.1234567, 70.1234567), (0.0, 70.2)),
        ];
        for ((lo, hi), (want_lo, want_hi)) in cases {
            let (a, b) = auto_range(lo, hi);
            assert!((a - want_lo).abs() < 1e-9 * want_lo.abs().max(1e-3), "{lo} -> {a}, Origin {want_lo}");
            assert!((b - want_hi).abs() < 1e-9 * want_hi.abs().max(1e-3), "{hi} -> {b}, Origin {want_hi}");
        }
    }

    #[test]
    fn levels_are_written_with_four_significant_digits() {
        // Labels of Origin's colour scales.
        for (v, s) in [
            (0.122, "0.1220"),
            (98.8, "98.80"),
            (9880.0, "9880"),
            (8.75e-4, "8.750E-04"),
            (0.00122, "0.001220"),
            (0.01650, "0.01650"),
            (2.01e5, "2.010E+05"),
            (12880.0, "1.288E+04"),
            (100.0625, "100.1"),
            (-8.55, "-8.550"),
            (0.0, "0.000"),
            (-4.44089209850063e-16, "-4.441E-16"),
            (7.3, "7.300"),
        ] {
            assert_eq!(format_level(v, 4), s, "{v}");
        }
        assert_eq!(format_level(-3.505, 2), "-3.5");
        assert_eq!(format_level(4.0, 2), "4.0");
        assert_eq!(format_level(-0.5149999999999997, 2), "-0.51");
    }

    #[test]
    fn major_levels_reproduce_the_float_noise_of_origins_labels() {
        // z from -3.3 to 3.3: Origin's middle label reads -4.441E-16, not 0.
        let (lo, hi) = auto_range(-3.3, 3.3);
        let lv = Levels { lo, hi, major: 8, minor: 5 };
        assert_eq!(format_level(lv.major_level(4), 4), "-4.441E-16");
        assert_eq!(format_level(lv.major_level(0), 4), "-3.300");
    }

    #[test]
    fn band_colours_follow_the_palettes_measured_in_origin() {
        // Origin's 48 bands: blue at the bottom, red at the top, green and yellow in the middle.
        assert_eq!(band_color(FieldPalette::Rainbow, 0, 48), Color::rgb(0, 0, 255));
        assert_eq!(band_color(FieldPalette::Rainbow, 47, 48), Color::rgb(255, 0, 0));
        // Band 24 (measured (0, 255, 41) and band 23 (0,255,16)): green, within 6 levels.
        let c = band_color(FieldPalette::Rainbow, 24, 48);
        assert!(c.g > 245 && c.b < 40 && c.r < 40, "{c:?}");
        // The heat map: blue, white in the middle (measured (247, 247, 255) and (255, 247, 247)), red.
        assert_eq!(band_color(FieldPalette::BlueWhiteRed, 0, 30), Color::rgb(0, 0, 255));
        assert_eq!(band_color(FieldPalette::BlueWhiteRed, 29, 30), Color::rgb(255, 0, 0));
        let c = band_color(FieldPalette::BlueWhiteRed, 14, 30);
        assert!(c.r > 240 && c.g > 240 && c.b == 255, "{c:?}");
    }

    #[test]
    fn a_band_polygon_cuts_the_cell_along_the_levels() {
        let c = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        // z grows with x from 0 to 1: the band 0.25..0.75 is the strip x in 0.25..0.75.
        let poly = band_polygon(c, [0.0, 1.0, 1.0, 0.0], 0.25, 0.75);
        assert_eq!(poly.len(), 4);
        assert!(poly.iter().all(|p| (p.0 - 0.25).abs() < 1e-12 || (p.0 - 0.75).abs() < 1e-12));
        // A cell wholly inside the band is the whole square.
        assert_eq!(band_polygon(c, [0.5; 4], 0.0, 1.0).len(), 4);
        // A cell wholly outside gives nothing.
        assert!(band_polygon(c, [0.5; 4], 2.0, 3.0).len() < 3);
    }

    #[test]
    fn level_lines_join_into_one_closed_loop_around_a_peak() {
        let n = 21;
        let mut z = Vec::new();
        for r in 0..n {
            for c in 0..n {
                let (x, y) = (c as f64 - 10.0, r as f64 - 10.0);
                z.push(10.0 - (x * x + y * y).sqrt());
            }
        }
        let grid = FieldGrid {
            rows: n,
            cols: n,
            x_from: -10.0,
            x_to: 10.0,
            y_from: -10.0,
            y_to: 10.0,
            z,
            x_label: String::new(),
            y_label: String::new(),
            z_label: String::new(),
            cells: false,
        };
        let lines = level_lines(&grid, 5.0);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].1, "closed");
        // All points are on the circle of radius 5 (within the linear interpolation).
        for p in &lines[0].0 {
            assert!((p.0.hypot(p.1) - 5.0).abs() < 0.1, "{p:?}");
        }
        // A level outside the data gives no line.
        assert!(level_lines(&grid, 20.0).is_empty());
    }
}
