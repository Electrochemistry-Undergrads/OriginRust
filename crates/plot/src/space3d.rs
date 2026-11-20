//! 3D XYZ layers (T155, graphing spec "3D Scatter and 3D Trajectory"): the plots read X, Y and the Z
//! column named in `Plot::size_from`, and are drawn as a fixed orthographic view of the XYZ box.
//!
//! The data box is the nice range of each of the three variables, mapped to the cube -1..1. A point
//! of the cube is turned by the layer's azimuth about the vertical Z axis and tilted by its elevation:
//! `u = x cos a - y sin a` across the page, `w = z cos e + (x sin a + y cos a) sin e` up the page. The
//! layer's own X and Y axes are hidden and run over `-SPACE3D_EXTENT..SPACE3D_EXTENT`, so the view always fits.

use crate::LayerGeometry;
use crate::data::PlotData;
use crate::fonts::TextMeasurer;
use crate::scene::{self, Item, Stroke};
use crate::ticks::{self, ResolvedAxis};
use sheaf_core::graph::{Axis, Layer, ScaleKind, Space3dCoords, Space3dScale, Surface3d};
use sheaf_core::{Color, Project};
use std::borrow::Cow;

/// The resolved range, ticks and labels of X, Y and Z.
pub(crate) struct DataBox {
    pub axes: [ResolvedAxis; 3],
}

impl DataBox {
    fn unit(&self, dim: usize, v: f64) -> f64 {
        let a = &self.axes[dim];
        if a.max > a.min { 2.0 * (v - a.min) / (a.max - a.min) - 1.0 } else { 0.0 }
    }
}

/// The end points of a 3D vector plot's arrows in data units (T157); `None` for other plots.
fn vector_ends(
    project: &Project,
    plot: &sheaf_core::graph::Plot,
    x: &[f64],
    y: &[f64],
    z: &[f64],
) -> Option<Vec<[f64; 3]>> {
    let spec = plot.decor.vector.as_ref().filter(|_| plot.kind == sheaf_core::graph::PlotKind::VectorXyxy)?;
    let col = |id| project.numeric(id).unwrap_or_default();
    let (a, b, c) = (col(spec.a), col(spec.b), col(spec.c?));
    let at = |v: &[f64], i: usize| v.get(i).copied().unwrap_or(f64::NAN);
    Some(
        (0..x.len().min(y.len()).min(z.len()))
            .map(|r| {
                let d = [at(a, r), at(b, r), at(c, r)];
                if spec.delta { [x[r] + d[0], y[r] + d[1], z[r] + d[2]] } else { d }
            })
            .collect(),
    )
}

/// Row `r`'s X and Y in the cube (NaN when missing).
fn xyz_unit(project: &Project, layer: &Layer, plot: &sheaf_core::graph::Plot, b: &DataBox, r: usize) -> (f64, f64) {
    match xyz(project, layer, plot) {
        Some((x, y, _)) => (x.get(r).map_or(f64::NAN, |v| b.unit(0, *v)), y.get(r).map_or(f64::NAN, |v| b.unit(1, *v))),
        None => (f64::NAN, f64::NAN),
    }
}

/// Half the width of a 3D ribbon (T161) along the depth axis, in depth steps.
const RIBBON_HALF: f64 = 0.25;

/// True for a point of the cube -1..1: points outside a set axis range (T164) are not drawn.
fn inside(p: [f64; 3]) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() <= 1.0 + 1e-9)
}

/// The page position (in data units of the hidden axes) of a point of the cube -1..1.
pub(crate) fn view(c: &Space3dCoords, [x, y, z]: [f64; 3]) -> (f64, f64) {
    let (sa, ca) = c.azimuth_deg.to_radians().sin_cos();
    let (se, ce) = c.elevation_deg.to_radians().sin_cos();
    (x * ca - y * sa, z * ce + (x * sa + y * ca) * se)
}

type Columns<'a> = (Cow<'a, [f64]>, Cow<'a, [f64]>, Cow<'a, [f64]>);

/// The X, Y and Z values of a plot: X from its X column or the row numbers. In an XYY layer (T158)
/// the plot's own values are Z and its Y is its depth: the first plot farthest back (`n`), the last
/// in front (1).
fn xyz<'a>(project: &'a Project, layer: &Layer, plot: &sheaf_core::graph::Plot) -> Option<Columns<'a>> {
    let data = project.numeric(plot.y)?;
    let x = match plot.x.and_then(|id| project.numeric(id)) {
        Some(x) => Cow::Borrowed(x),
        None => Cow::Owned((1..=data.len()).map(|r| r as f64).collect()),
    };
    if layer.coords.space3d().is_some_and(|c| c.xyy) {
        let k = layer.plots.iter().position(|p| std::ptr::eq(p, plot))?;
        let depth = (layer.plots.len() - k) as f64;
        return Some((x, Cow::Owned(vec![depth; data.len()]), Cow::Borrowed(data)));
    }
    let z = project.numeric(plot.size_from?)?;
    Some((x, Cow::Borrowed(data), Cow::Borrowed(z)))
}

/// The data box of a 3D layer: the nice ranges of the visible plots' X, Y and Z; `None` without data.
pub(crate) fn data_box(project: &Project, layer: &Layer) -> Option<DataBox> {
    let mut ext = [(f64::INFINITY, f64::NEG_INFINITY); 3];
    for plot in layer.plots.iter().filter(|p| p.visible) {
        let Some((x, y, z)) = xyz(project, layer, plot) else { continue };
        let ends = vector_ends(project, plot, &x, &y, &z);
        for r in 0..x.len().min(y.len()).min(z.len()) {
            let start = [x[r], y[r], z[r]];
            for v in std::iter::once(start).chain(ends.as_ref().and_then(|e| e.get(r).copied())) {
                if v.iter().all(|v| v.is_finite()) {
                    for (e, v) in ext.iter_mut().zip(v) {
                        *e = (e.0.min(v), e.1.max(v));
                    }
                }
            }
        }
    }
    if ext.iter().any(|e| e.0 > e.1) {
        return None;
    }
    let scales = layer.coords.space3d().map(|c| c.scales).unwrap_or_default();
    // The 3D Axis window's From, To and Increment (T164), else automatic.
    let resolve = |a: &Axis, e: (f64, f64), s: &Space3dScale| {
        let a = Axis { min: s.from, max: s.to, major_step: s.step, scale: ScaleKind::Linear, ..a.clone() };
        let r = ticks::resolve_axis(&a, Some(e));
        // About five steps along each edge of the box: its edges are short, so the 2D axes' tick
        // density would crowd the labels.
        if s.step.is_some() || r.major.len() <= 7 || r.max <= r.min {
            return r;
        }
        let step = ticks::nice_step((r.max - r.min) / 5.0);
        let min = s.from.unwrap_or((r.min / step).floor() * step);
        let max = s.to.unwrap_or((r.max / step).ceil() * step);
        let (major, step) = ticks::linear_major(min, max, step, false);
        let labels = ticks::linear_labels(&a, &major, step);
        ResolvedAxis { min, max, step, major, minor: Vec::new(), labels, ..r }
    };
    let mut axes = [
        resolve(&layer.x_axis, ext[0], &scales[0]),
        resolve(&layer.y_axis, ext[1], &scales[1]),
        resolve(&layer.y_axis, ext[2], &scales[2]),
    ];
    // XYY (T158): the depth axis has one tick per plot, labelled with the plot's name.
    if layer.coords.space3d().is_some_and(|c| c.xyy) && !layer.plots.is_empty() {
        let n = layer.plots.len();
        let major: Vec<f64> = (1..=n).map(|d| d as f64).collect();
        let labels = (1..=n).map(|d| project.plot_label(&layer.plots[n - d])).collect();
        let r = axes[1].clone();
        axes[1] = ResolvedAxis { min: 0.5, max: n as f64 + 0.5, step: 1.0, major, minor: Vec::new(), labels, ..r };
    }
    Some(DataBox { axes })
}

/// The From, To and Increment the box of a 3D layer shows on its X, Y and Z axes (T164, for the 3D
/// Axis window); `None` for a layer that is not 3D or has no data.
pub fn space3d_scales(project: &Project, layer: &Layer) -> Option<[(f64, f64, f64); 3]> {
    layer.coords.space3d()?;
    let b = data_box(project, layer)?;
    Some(b.axes.map(|a| (a.min, a.max, a.step)))
}

/// Turns a plot's X, Y and Z rows into their place in the view. Rows without a Z are left out.
pub(crate) fn columns(pd: &mut PlotData, project: &Project, layer: &Layer, c: &Space3dCoords, b: &DataBox) {
    let n = pd.y.len();
    let (mut us, mut ws) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    if let Some((x, y, z)) = xyz(project, layer, pd.plot) {
        for r in 0..n.min(x.len()).min(y.len()).min(z.len()) {
            let p = [b.unit(0, x[r]), b.unit(1, y[r]), b.unit(2, z[r])];
            if inside(p) {
                (us[r], ws[r]) = view(c, p);
            }
        }
    }
    // Z error bars (T156): a change of Z moves a point straight up the page, by `2 cos e / (Z range)`
    // per unit, so the 2D error bars draw them.
    let (_, ce) = c.elevation_deg.to_radians().sin_cos();
    let z_axis = &b.axes[2];
    let per_z = if z_axis.max > z_axis.min { 2.0 * ce / (z_axis.max - z_axis.min) } else { 0.0 };
    pd.err = pd
        .plot
        .y_error
        .and_then(|id| project.numeric(id))
        .map(|e| Cow::Owned((0..n).map(|r| e.get(r).map_or(f64::NAN, |v| v * per_z)).collect()));
    // 3D bars (T156): from the floor of the box under the point to the point, drawn from the back
    // to the front so that nearer bars cover farther ones. The rows are reordered for that, so masks
    // and row filters do not apply to 3D bars.
    if pd.plot.kind == sheaf_core::graph::PlotKind::Column {
        let (sa, ca) = c.azimuth_deg.to_radians().sin_cos();
        let cube: Vec<(f64, f64)> = (0..n).map(|r| xyz_unit(project, layer, pd.plot, b, r)).collect();
        let depth = |r: usize| cube[r].0 * sa + cube[r].1 * ca;
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&p, &q| depth(q).total_cmp(&depth(p)));
        let floor: Vec<f64> = order
            .iter()
            .map(|&r| {
                let (x, y) = cube[r];
                if x.is_finite() && y.is_finite() { view(c, [x, y, -1.0]).1 } else { f64::NAN }
            })
            .collect();
        us = order.iter().map(|&r| us[r]).collect();
        ws = order.iter().map(|&r| ws[r]).collect();
        pd.err =
            pd.err.take().map(|e| Cow::Owned(order.iter().map(|&r| e.get(r).copied().unwrap_or(f64::NAN)).collect()));
        pd.stack = Some((floor, ws.clone()));
        pd.mask = None;
        pd.filters.clear();
    } else if pd.plot.kind == sheaf_core::graph::PlotKind::Area && c.ribbons {
        // 3D Ribbons (T161): the curve at its depth, widened to half a depth step, from a quarter
        // step in front of it to a quarter step behind.
        let (sa, ca) = c.azimuth_deg.to_radians().sin_cos();
        pd.ribbon = xyz(project, layer, pd.plot).map(|(x, y, z)| {
            (0..n)
                .map(|r| {
                    let cube = match (x.get(r), y.get(r), z.get(r)) {
                        (Some(&x), Some(&y), Some(&z)) => [b.unit(0, x), b.unit(1, y), b.unit(2, z)],
                        _ => [f64::NAN; 3],
                    };
                    let at = |dy: f64| {
                        let p = [cube[0], b.unit(1, y.get(r).copied().unwrap_or(f64::NAN) + dy), cube[2]];
                        if inside(p) { view(c, p) } else { (f64::NAN, f64::NAN) }
                    };
                    RibbonRow { edges: [at(-RIBBON_HALF), at(RIBBON_HALF)], depth: cube[0] * sa + cube[1] * ca }
                })
                .collect()
        });
        pd.stack = None;
    } else if pd.plot.kind == sheaf_core::graph::PlotKind::Area {
        // 3D Walls (T158): the area of a curve down to the floor of the box, at the curve's depth.
        let floor = (0..n)
            .map(|r| {
                let (x, y) = xyz_unit(project, layer, pd.plot, b, r);
                if x.is_finite() && y.is_finite() { view(c, [x, y, -1.0]).1 } else { f64::NAN }
            })
            .collect();
        pd.stack = Some((floor, ws.clone()));
    } else {
        pd.stack = None;
    }
    // 3D surfaces (T166): the points joined into triangles over X and Y.
    if let Some(style) = c.surface {
        pd.surface = xyz(project, layer, pd.plot).map(|(x, y, z)| surface(c, b, style, &x, &y, &z));
    }
    // 3D vectors (T157): the arrows end at their end points in the view.
    if let (Some((x, y, z)), Some(v)) = (xyz(project, layer, pd.plot), pd.vector.as_mut())
        && let Some(ends) = vector_ends(project, pd.plot, &x, &y, &z)
    {
        let rows = (0..n)
            .map(|r| match ends.get(r) {
                Some(e) => view(c, [b.unit(0, e[0]), b.unit(1, e[1]), b.unit(2, e[2])]),
                None => (f64::NAN, f64::NAN),
            })
            .collect();
        v.rows = crate::vector_plot::VectorRows::Xy(rows);
    }
    pd.x = Some(Cow::Owned(us));
    pd.y = Cow::Owned(ws);
    pd.x_format = None;
    pd.xerr = None;
    // The Z column places the points; it sizes no symbol.
    pd.size_z = None;
}

/// The triangles of a 3D surface (T166), farthest first, and the Z range its colour map spans.
#[derive(Clone, Debug)]
pub(crate) struct SurfaceData {
    pub style: Surface3d,
    /// Each triangle's corners in the units of the hidden axes, and its mean Z in data units.
    pub triangles: Vec<([(f64, f64); 3], f64)>,
    /// The same triangles flat on the floor of the box, for a projection (T168); empty otherwise.
    pub floor: Vec<([(f64, f64); 3], f64)>,
    /// The walls of X or Y Constant with Base (T170), farthest first: each a curve of Z and the
    /// floor under it, as one polygon; empty for the other styles.
    pub walls: Vec<Vec<(f64, f64)>>,
    pub z_range: (f64, f64),
}

/// Joins the points of a plot inside the box into a Delaunay triangulation over X and Y (in the
/// box's units, so that both axes weigh alike). Of points with the same X and Y the first is kept.
/// Fewer than three points, or points all on a line, make no triangle.
fn surface(c: &Space3dCoords, b: &DataBox, style: Surface3d, x: &[f64], y: &[f64], z: &[f64]) -> SurfaceData {
    let mut seen = std::collections::HashSet::new();
    let mut points: Vec<([f64; 3], f64)> = Vec::new();
    for r in 0..x.len().min(y.len()).min(z.len()) {
        let p = [b.unit(0, x[r]), b.unit(1, y[r]), b.unit(2, z[r])];
        if inside(p) && seen.insert((x[r].to_bits(), y[r].to_bits())) {
            points.push((p, z[r]));
        }
    }
    let z_range = points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
    let (sa, ca) = c.azimuth_deg.to_radians().sin_cos();
    if let Surface3d::XConstant | Surface3d::YConstant = style {
        let walls = constant_walls(c, &points, style == Surface3d::XConstant);
        return SurfaceData { style, triangles: Vec::new(), floor: Vec::new(), walls, z_range };
    }
    let flat: Vec<(f64, f64)> = points.iter().map(|(p, _)| (p[0], p[1])).collect();
    let mut triangles: Vec<(f64, ([(f64, f64); 3], f64))> = sheaf_calc::xyzarea::delaunay(&flat)
        .unwrap_or_default()
        .into_iter()
        .map(|t| {
            let depth = t.iter().map(|&i| points[i].0[0] * sa + points[i].0[1] * ca).sum::<f64>();
            let mean_z = t.iter().map(|&i| points[i].1).sum::<f64>() / 3.0;
            (depth, (t.map(|i| view(c, points[i].0)), mean_z))
        })
        .collect();
    triangles.sort_by(|p, q| q.0.total_cmp(&p.0));
    // Colormap Surface with Projection (T168): each triangle again at Z = the floor of the box.
    let floor = match style {
        Surface3d::ColorMapProjection => sheaf_calc::xyzarea::delaunay(&flat)
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                let mean_z = t.iter().map(|&i| points[i].1).sum::<f64>() / 3.0;
                (t.map(|i| view(c, [points[i].0[0], points[i].0[1], -1.0])), mean_z)
            })
            .collect(),
        _ => Vec::new(),
    };
    SurfaceData { style, triangles: triangles.into_iter().map(|t| t.1).collect(), floor, walls: Vec::new(), z_range }
}

/// X or Y Constant with Base (T170): the points (in the box's units) of each X value (`by_x`) or Y
/// value joined in order of the other coordinate into a curve of Z, closed down along the floor of
/// the box. Groups of one point draw nothing. Farthest first, by the mean depth of the curve.
fn constant_walls(c: &Space3dCoords, points: &[([f64; 3], f64)], by_x: bool) -> Vec<Vec<(f64, f64)>> {
    let (key, along) = if by_x { (0, 1) } else { (1, 0) };
    let mut groups: std::collections::BTreeMap<u64, Vec<[f64; 3]>> = std::collections::BTreeMap::new();
    for (p, _) in points {
        // The box units of equal data values are equal, so their bits group them.
        groups.entry((p[key] + 0.0).to_bits()).or_default().push(*p);
    }
    let (sa, ca) = c.azimuth_deg.to_radians().sin_cos();
    let mut walls: Vec<(f64, Vec<(f64, f64)>)> = groups
        .into_values()
        .filter(|g| g.len() >= 2)
        .map(|mut g| {
            g.sort_by(|a, b| a[along].total_cmp(&b[along]));
            let depth = g.iter().map(|p| p[0] * sa + p[1] * ca).sum::<f64>() / g.len() as f64;
            let mut poly: Vec<(f64, f64)> = g.iter().map(|p| view(c, *p)).collect();
            poly.extend(g.iter().rev().map(|p| view(c, [p[0], p[1], -1.0])));
            (depth, poly)
        })
        .collect();
    walls.sort_by(|p, q| q.0.total_cmp(&p.0));
    walls.into_iter().map(|w| w.1).collect()
}

/// Draws a 3D surface (T166): its triangles from the farthest to the nearest, filled as its style
/// says and edged with the plot's line.
pub(crate) fn draw_surface(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(s) = &pd.surface else { return };
    let plot = pd.plot;
    let stroke = (plot.line.width_pt > 0.0).then(|| crate::plots::line_stroke(&plot.line));
    let path = |corners: &[(f64, f64); 3]| {
        let mut d = String::new();
        for (k, &(x, y)) in corners.iter().enumerate() {
            let (px, py) = geom.data_to_page(x, y);
            if k == 0 { scene::move_to(&mut d, px, py) } else { scene::line_to(&mut d, px, py) }
        }
        d.push('Z');
        d
    };
    let mapped = |z: f64| crate::plots::map_color(crate::plots::z_fraction(s.z_range, z));
    // The projection (T168) first, under the surface: the colour map without mesh lines, each
    // triangle edged in its own colour so that no seams show between them.
    for (corners, mean_z) in &s.floor {
        let c = mapped(*mean_z);
        out.push(Item::Path { d: path(corners), fill: Some(c), stroke: Some(Stroke::solid(c, 0.25)) });
    }
    for wall in &s.walls {
        let mut d = String::new();
        for (k, &(x, y)) in wall.iter().enumerate() {
            let (px, py) = geom.data_to_page(x, y);
            if k == 0 { scene::move_to(&mut d, px, py) } else { scene::line_to(&mut d, px, py) }
        }
        d.push('Z');
        out.push(Item::Path { d, fill: Some(plot.symbol.color), stroke: stroke.clone() });
    }
    for (corners, mean_z) in &s.triangles {
        let fill = match s.style {
            Surface3d::ColorMap | Surface3d::ColorMapProjection => Some(mapped(*mean_z)),
            Surface3d::ColorFill | Surface3d::XConstant | Surface3d::YConstant => Some(plot.symbol.color),
            Surface3d::WireFrame => None,
            Surface3d::WireSurface => Some(Color::WHITE),
        };
        if fill.is_none() && stroke.is_none() {
            continue;
        }
        out.push(Item::Path { d: path(corners), fill, stroke: stroke.clone() });
    }
}

/// One row of a 3D ribbon (T161): its front and back edge points in the units of the hidden axes,
/// and its depth in the view (larger is farther).
#[derive(Clone, Copy, Debug)]
pub(crate) struct RibbonRow {
    pub edges: [(f64, f64); 2],
    pub depth: f64,
}

/// Draws a 3D ribbon (T161): one quadrilateral per pair of consecutive rows, filled with the plot's
/// fill colour and edged with its line, the farthest first.
pub(crate) fn draw_ribbon(out: &mut Vec<Item>, pd: &PlotData, geom: &LayerGeometry) {
    let Some(rows) = &pd.ribbon else { return };
    let plot = pd.plot;
    let stroke = (plot.line.width_pt > 0.0).then(|| crate::plots::line_stroke(&plot.line));
    let shown: Vec<usize> = pd.rows().filter(|&r| r < rows.len()).collect();
    let ok = |r: &RibbonRow| r.edges.iter().all(|p| p.0.is_finite() && p.1.is_finite());
    let mut quads: Vec<(f64, [(f64, f64); 4])> = shown
        .windows(2)
        .filter(|w| w[1] == w[0] + 1 && ok(&rows[w[0]]) && ok(&rows[w[1]]))
        .map(|w| {
            let (p, q) = (rows[w[0]], rows[w[1]]);
            (p.depth + q.depth, [p.edges[0], q.edges[0], q.edges[1], p.edges[1]])
        })
        .collect();
    quads.sort_by(|p, q| q.0.total_cmp(&p.0));
    for (_, quad) in quads {
        let mut d = String::new();
        for (k, &(x, y)) in quad.iter().enumerate() {
            let (px, py) = geom.data_to_page(x, y);
            if k == 0 { scene::move_to(&mut d, px, py) } else { scene::line_to(&mut d, px, py) }
        }
        d.push('Z');
        out.push(Item::Path { d, fill: Some(plot.symbol.color), stroke: stroke.clone() });
    }
}

fn page(geom: &LayerGeometry, (x, y): (f64, f64)) -> (f64, f64) {
    geom.data_to_page(x, y)
}

fn line(out: &mut Vec<Item>, geom: &LayerGeometry, c: &Space3dCoords, a: [f64; 3], b: [f64; 3], stroke: &Stroke) {
    let mut d = String::new();
    let (p, q) = (page(geom, view(c, a)), page(geom, view(c, b)));
    scene::move_to(&mut d, p.0, p.1);
    scene::line_to(&mut d, q.0, q.1);
    out.push(Item::Path { d, fill: None, stroke: Some(stroke.clone()) });
}

/// The column names that title the three axes, from the first visible plot.
fn titles(project: &Project, layer: &Layer) -> [String; 3] {
    let name = |id: Option<sheaf_core::ColumnId>| {
        id.and_then(|id| project.column(id)).map(|c| c.axis_title()).unwrap_or_default()
    };
    let xyy = layer.coords.space3d().is_some_and(|c| c.xyy);
    match layer.plots.iter().find(|p| p.visible) {
        // XYY (T158): the depth axis names its plots at the ticks; Z is the first plot's column.
        Some(p) if xyy => [name(p.x), String::new(), name(Some(p.y))],
        Some(p) => [name(p.x), name(Some(p.y)), name(p.size_from)],
        None => Default::default(),
    }
}

/// The box of a 3D layer: the three back planes with grid lines at the major ticks in `grid`, and the
/// tick labels and axis titles on the front edges in `overlay`.
pub(crate) fn draw_box(
    grid: &mut Vec<Item>,
    overlay: &mut Vec<Item>,
    project: &Project,
    layer: &Layer,
    geom: &LayerGeometry,
    c: &Space3dCoords,
    m: &dyn TextMeasurer,
) {
    let Some(b) = data_box(project, layer) else { return };
    let (sa, ca) = c.azimuth_deg.to_radians().sin_cos();
    // The far side of each horizontal axis (larger depth `x sin a + y cos a` is farther away).
    let far = [if sa >= 0.0 { 1.0 } else { -1.0 }, if ca >= 0.0 { 1.0 } else { -1.0 }];
    let edge = Stroke::solid(Color::BLACK, 0.75);
    let gridline = Stroke::solid(Color::rgb(192, 192, 192), 0.5);
    let ticks_of = |dim: usize| -> Vec<(f64, String)> {
        let a = &b.axes[dim];
        a.major.iter().zip(&a.labels).map(|(v, l)| (b.unit(dim, *v), l.clone())).collect()
    };
    // Back planes: x = far x, y = far y, and the floor z = -1.
    let [fx, fy] = far;
    for (u, _) in ticks_of(0) {
        line(grid, geom, c, [u, fy, -1.0], [u, fy, 1.0], &gridline);
        line(grid, geom, c, [u, -fy, -1.0], [u, fy, -1.0], &gridline);
    }
    for (v, _) in ticks_of(1) {
        line(grid, geom, c, [fx, v, -1.0], [fx, v, 1.0], &gridline);
        line(grid, geom, c, [-fx, v, -1.0], [fx, v, -1.0], &gridline);
    }
    for (w, _) in ticks_of(2) {
        line(grid, geom, c, [-fx, fy, w], [fx, fy, w], &gridline);
        line(grid, geom, c, [fx, -fy, w], [fx, fy, w], &gridline);
    }
    // The outline of the three planes.
    let corners = [
        ([-fx, fy, -1.0], [fx, fy, -1.0]),
        ([fx, -fy, -1.0], [fx, fy, -1.0]),
        ([-fx, -fy, -1.0], [-fx, fy, -1.0]),
        ([-fx, -fy, -1.0], [fx, -fy, -1.0]),
        ([fx, fy, -1.0], [fx, fy, 1.0]),
        ([-fx, fy, -1.0], [-fx, fy, 1.0]),
        ([fx, -fy, -1.0], [fx, -fy, 1.0]),
        ([-fx, fy, 1.0], [fx, fy, 1.0]),
        ([fx, -fy, 1.0], [fx, fy, 1.0]),
    ];
    for (a, z) in corners {
        line(grid, geom, c, a, z, &edge);
    }
    // Tick labels and titles on the front edges: X along y = near, Y along x = near (both on the
    // floor), Z up the near-x, far-y edge.
    let font = &layer.x_axis.tick_font;
    let title_font = &layer.x_axis.title_font;
    let centre = page(geom, (0.0, 0.0));
    let place = |out: &mut Vec<Item>, at: [f64; 3], text: &str, font: &sheaf_core::graph::Font, gap: f64| {
        let p = page(geom, view(c, at));
        let (dx, dy) = (p.0 - centre.0, p.1 - centre.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-9);
        let w = crate::text::width(m, text, font, font.size_pt);
        let (x, y) = (p.0 + dx / len * gap - w / 2.0, p.1 + dy / len * gap);
        crate::text::place_label(out, m, text, font, (x, y), 0.0, &|_| None);
    };
    let names = titles(project, layer);
    let edges: [(usize, Box<dyn Fn(f64) -> [f64; 3]>); 3] = [
        (0, Box::new(move |t| [t, -fy, -1.0])),
        (1, Box::new(move |t| [-fx, t, -1.0])),
        (2, Box::new(move |t| [-fx, fy, t])),
    ];
    // Where two labelled edges meet, one label is enough: X leaves out its end at the Y edge, Z its
    // bottom at the floor.
    let shared = |dim: usize, t: f64| match dim {
        0 => (t + fx).abs() < 1e-9,
        2 => t <= -1.0 + 1e-9,
        _ => false,
    };
    for (dim, at) in &edges {
        for (t, label) in ticks_of(*dim).into_iter().filter(|(t, _)| !shared(*dim, *t)) {
            place(overlay, at(t), &label, font, font.size_pt);
        }
        if !names[*dim].is_empty() {
            place(overlay, at(0.0), &names[*dim], title_font, font.size_pt + 2.5 * title_font.size_pt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_turns_and_tilts_the_cube() {
        let front = Space3dCoords { azimuth_deg: 0.0, elevation_deg: 0.0, ..Default::default() };
        assert_eq!(view(&front, [1.0, 1.0, 1.0]), (1.0, 1.0), "seen from the front, depth vanishes");
        let c = Space3dCoords::default();
        let corners = [-1.0, 1.0];
        for x in corners {
            for y in corners {
                for z in corners {
                    let (u, w) = view(&c, [x, y, z]);
                    let e = sheaf_core::template::SPACE3D_EXTENT;
                    assert!(u.abs() <= e && w.abs() <= e, "({x}, {y}, {z}) fits");
                }
            }
        }
        // Higher Z is higher on the page.
        assert!(view(&c, [0.0, 0.0, 1.0]).1 > view(&c, [0.0, 0.0, -1.0]).1);
    }

    #[test]
    fn a_3d_scatter_places_the_points_in_the_view_and_draws_the_box() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 5.0, 10.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 10.0, 20.0])).unwrap();
        let z = p.add_column(b, 0, Designation::Z, ColumnData::numeric(vec![0.0, 50.0, 100.0])).unwrap();
        p.column_mut(z).unwrap().long_name = "Height".into();
        let g = p.plot_columns(Template::Scatter3d, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        let layer = &graph.layers[0];
        assert!(layer.coords.space3d().is_some());
        assert_eq!((layer.plots[0].x, layer.plots[0].size_from), (Some(x), Some(z)));
        let c = *layer.coords.space3d().unwrap();
        let layers = crate::data::collect_graph(&p, graph);
        let pd = &layers[0].plots[0];
        // Each row is its X, Y and Z mapped into the data box's cube, then turned and tilted.
        let b = data_box(&p, layer).unwrap();
        let px = pd.x.as_deref().unwrap();
        for (r, v) in [[0.0, 0.0, 0.0], [5.0, 10.0, 50.0], [10.0, 20.0, 100.0]].iter().enumerate() {
            let want = view(&c, [b.unit(0, v[0]), b.unit(1, v[1]), b.unit(2, v[2])]);
            assert!((px[r] - want.0).abs() < 1e-9 && (pd.y[r] - want.1).abs() < 1e-9, "row {r}");
        }
        assert!(b.axes[2].min <= 0.0 && b.axes[2].max >= 100.0, "the Z range holds the data");
        assert!(pd.y[2] > pd.y[0], "higher Z is higher on the page");
        assert!(pd.size_z.is_none(), "Z sizes no symbol");
        let svg = crate::tests::svg_of(&p, g);
        let t = crate::tests::texts(&crate::tests::doc(&svg));
        for want in ["Height", "100", "20", "10"] {
            assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
        }
        // The trajectory joins the points.
        let g = p.plot_columns(Template::Trajectory3d, &[y]).unwrap();
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].kind, sheaf_core::graph::PlotKind::LineSymbol);
    }

    #[test]
    fn z_error_bars_and_bars_from_the_floor() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 0.0, 1.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 0.0, 1.0, 1.0])).unwrap();
        let z = p.add_column(b, 0, Designation::Z, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        let e = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.5; 4])).unwrap();
        let g = p.plot_columns(Template::Scatter3dError, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!((graph.layers[0].plots[0].size_from, graph.layers[0].plots[0].y_error), (Some(z), Some(e)));
        let layers = crate::data::collect_graph(&p, graph);
        let pd = &layers[0].plots[0];
        let bx = data_box(&p, &graph.layers[0]).unwrap();
        let c = *graph.layers[0].coords.space3d().unwrap();
        let per_z = 2.0 * c.elevation_deg.to_radians().cos() / (bx.axes[2].max - bx.axes[2].min);
        let err = pd.err.as_deref().unwrap();
        assert!(err.iter().all(|v| (v - 0.5 * per_z).abs() < 1e-12), "{err:?}");

        let g = p.plot_columns(Template::Bars3d, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        let layers = crate::data::collect_graph(&p, graph);
        let pd = &layers[0].plots[0];
        let (floor, top) = pd.stack.as_ref().expect("bars stand on the floor");
        assert!(floor.iter().zip(top).all(|(f, t)| f < t));
        // Back to front: the depth falls along the drawn rows. A bar's floor is at
        // w = -cos e + depth sin e, so the floor itself falls too.
        let depth: Vec<f64> = floor
            .iter()
            .map(|w| (w + c.elevation_deg.to_radians().cos()) / c.elevation_deg.to_radians().sin())
            .collect();
        assert!(depth.windows(2).all(|w| w[0] >= w[1] - 1e-12), "{depth:?}");
    }

    #[test]
    fn vectors_3d_end_at_their_end_points_or_deltas() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        let col = |p: &mut Project, d, v: Vec<f64>| p.add_column(b, 0, d, ColumnData::numeric(v)).unwrap();
        col(&mut p, Designation::X, vec![0.0, 1.0]);
        let y = col(&mut p, Designation::Y, vec![0.0, 1.0]);
        col(&mut p, Designation::Z, vec![0.0, 1.0]);
        col(&mut p, Designation::Y, vec![0.5, 0.5]);
        col(&mut p, Designation::Y, vec![0.5, 0.5]);
        col(&mut p, Designation::Y, vec![2.0, 2.0]);
        for (template, end) in [(Template::Vector3dXyz, [0.5, 0.5, 2.0]), (Template::Vector3dDelta, [0.5, 0.5, 2.0])] {
            let g = p.plot_columns(template, &[y]).unwrap();
            let graph = p.graph(g).unwrap();
            let layer = &graph.layers[0];
            let c = *layer.coords.space3d().unwrap();
            let bx = data_box(&p, layer).unwrap();
            assert!(bx.axes[2].max >= 2.0, "the box holds the arrow ends");
            let layers = crate::data::collect_graph(&p, graph);
            let v = layers[0].plots[0].vector.as_ref().expect("vector data");
            let crate::vector_plot::VectorRows::Xy(rows) = &v.rows else { panic!("end points") };
            // Row 0 starts at (0, 0, 0): its end is the end columns, or the start plus the deltas.
            let want = view(&c, [bx.unit(0, end[0]), bx.unit(1, end[1]), bx.unit(2, end[2])]);
            assert!((rows[0].0 - want.0).abs() < 1e-9 && (rows[0].1 - want.1).abs() < 1e-9, "{template:?}");
            if template == Template::Vector3dDelta {
                let w1 = view(&c, [bx.unit(0, 1.5), bx.unit(1, 1.5), bx.unit(2, 3.0)]);
                assert!((rows[1].0 - w1.0).abs() < 1e-9 && (rows[1].1 - w1.1).abs() < 1e-9, "deltas add to row 1");
            }
        }
        // Too few columns after Y.
        let short = p.add_workbook("Short");
        p.add_column(short, 0, Designation::X, ColumnData::numeric(vec![1.0])).unwrap();
        let y2 = p.add_column(short, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        assert!(p.plot_columns(Template::Vector3dXyz, &[y2]).is_err());
    }

    #[test]
    fn xyy_puts_each_column_at_its_own_depth_and_walls_stand_on_the_floor() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        let ys: Vec<_> = (0..3)
            .map(|k| p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0 + k as f64; 3])).unwrap())
            .collect();
        for (k, y) in ys.iter().enumerate() {
            p.column_mut(*y).unwrap().long_name = format!("S{k}");
        }
        let g = p.plot_columns(Template::Walls3d, &ys).unwrap();
        let graph = p.graph(g).unwrap();
        let layer = &graph.layers[0];
        assert!(layer.coords.space3d().is_some_and(|c| c.xyy));
        assert_eq!(layer.plots.len(), 3);
        let bx = data_box(&p, layer).unwrap();
        // The depth axis: one tick per plot, the first plot farthest back (depth 3).
        assert_eq!(bx.axes[1].major, [1.0, 2.0, 3.0]);
        assert_eq!(bx.axes[1].labels, ["S2", "S1", "S0"]);
        let c = *layer.coords.space3d().unwrap();
        let layers = crate::data::collect_graph(&p, graph);
        for (k, pd) in layers[0].plots.iter().enumerate() {
            let depth = (3 - k) as f64;
            let want = view(&c, [bx.unit(0, 0.0), bx.unit(1, depth), bx.unit(2, 1.0 + k as f64)]);
            let px = pd.x.as_deref().unwrap();
            assert!((px[0] - want.0).abs() < 1e-9 && (pd.y[0] - want.1).abs() < 1e-9, "plot {k}");
            let (floor, top) = pd.stack.as_ref().expect("a wall stands on the floor");
            assert!(floor.iter().zip(top).all(|(f, t)| f < t));
        }
        let titles = titles(&p, layer);
        assert_eq!((titles[1].as_str(), titles[2].as_str()), ("", "S0"));
    }

    #[test]
    fn ribbons_are_strips_at_each_depth_drawn_back_to_front() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0, 3.0])).unwrap();
        let ys: Vec<_> = (0..2)
            .map(|k| {
                p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 3.0 + k as f64, 2.0, 4.0])).unwrap()
            })
            .collect();
        let g = p.plot_columns(Template::Ribbons3d, &ys).unwrap();
        let graph = p.graph(g).unwrap();
        let layer = &graph.layers[0];
        let c = *layer.coords.space3d().unwrap();
        assert!(c.xyy && c.ribbons);
        let bx = data_box(&p, layer).unwrap();
        let layers = crate::data::collect_graph(&p, graph);
        let geom = crate::layout::graph_geometry(&p, graph, &layers, &crate::FixedMetrics).remove(0);
        for (k, pd) in layers[0].plots.iter().enumerate() {
            assert!(pd.stack.is_none(), "a ribbon does not stand on the floor");
            let rows = pd.ribbon.as_ref().expect("a ribbon");
            let depth = (2 - k) as f64;
            // Row 1: a quarter step in front of and behind the plot's depth.
            let z = bx.unit(2, 3.0 + k as f64);
            let front = view(&c, [bx.unit(0, 1.0), bx.unit(1, depth - RIBBON_HALF), z]);
            let back = view(&c, [bx.unit(0, 1.0), bx.unit(1, depth + RIBBON_HALF), z]);
            let close = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
            assert!(close(rows[1].edges[0], front) && close(rows[1].edges[1], back), "plot {k}");
            // Three quads, farthest (largest X, as sin a > 0) first.
            let mut out = Vec::new();
            draw_ribbon(&mut out, pd, &geom);
            assert_eq!(out.len(), 3);
            let first_x = |item: &Item| match item {
                Item::Path { d, fill: Some(_), stroke: Some(_) } => {
                    d.trim_start_matches('M').split([' ', 'L']).next().unwrap().parse::<f64>().unwrap()
                }
                other => panic!("a filled and edged quad, not {other:?}"),
            };
            let xs: Vec<f64> = out.iter().map(first_x).collect();
            assert!(xs[0] > xs[1] && xs[1] > xs[2], "back to front: {xs:?}");
        }
    }

    #[test]
    fn set_scales_fix_the_box_and_points_outside_are_not_drawn() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::graph::Coords;
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        p.add_column(b, 0, Designation::Z, ColumnData::numeric(vec![1.0, 5.0, 9.0])).unwrap();
        let g = p.plot_columns(Template::Scatter3d, &[y]).unwrap();
        let auto = space3d_scales(&p, &p.graph(g).unwrap().layers[0]).unwrap();
        assert!(auto[2].0 <= 1.0 && auto[2].1 >= 9.0, "automatic Z: {auto:?}");
        if let Coords::Space3d(c) = &mut p.graph_mut(g).unwrap().layers[0].coords {
            c.scales[2] = Space3dScale { from: Some(0.0), to: Some(6.0), step: Some(2.0) };
        }
        let graph = p.graph(g).unwrap();
        let layer = &graph.layers[0];
        let bx = data_box(&p, layer).unwrap();
        assert_eq!((bx.axes[2].min, bx.axes[2].max), (0.0, 6.0));
        assert_eq!(bx.axes[2].major, [0.0, 2.0, 4.0, 6.0]);
        assert_eq!(space3d_scales(&p, layer).unwrap()[2], (0.0, 6.0, 2.0));
        let layers = crate::data::collect_graph(&p, graph);
        let pd = &layers[0].plots[0];
        // Z = 9 is above To = 6: that point is left out.
        assert!(pd.y[0].is_finite() && pd.y[1].is_finite() && pd.y[2].is_nan());
    }

    #[test]
    fn surfaces_join_the_points_into_triangles_drawn_back_to_front() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::graph::Surface3d;
        use sheaf_core::template::Template;
        // A 3 x 3 grid, plus a repeat of the first point (left out) and a point with no Z.
        let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..3 {
            for j in 0..3 {
                xs.push(i as f64);
                ys.push(j as f64);
                zs.push((i * j) as f64);
            }
        }
        xs.extend([0.0, 5.0]);
        ys.extend([0.0, 5.0]);
        zs.extend([7.0, f64::NAN]);
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(xs)).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(ys)).unwrap();
        p.add_column(b, 0, Designation::Z, ColumnData::numeric(zs)).unwrap();
        for (t, style) in [
            (Template::ColorMapSurface3d, Surface3d::ColorMap),
            (Template::ColorFillSurface3d, Surface3d::ColorFill),
            (Template::WireFrame3d, Surface3d::WireFrame),
            (Template::WireSurface3d, Surface3d::WireSurface),
            (Template::ColorMapProjection3d, Surface3d::ColorMapProjection),
        ] {
            let g = p.plot_columns(t, &[y]).unwrap();
            let graph = p.graph(g).unwrap();
            let layer = &graph.layers[0];
            assert_eq!(layer.coords.space3d().unwrap().surface, Some(style));
            // The mesh keeps the template's black line; the plot colour fills (red first).
            let plot = &layer.plots[0];
            assert_eq!(plot.line.color, sheaf_core::Color::BLACK);
            assert_eq!(plot.symbol.color, sheaf_core::Color::rgb(255, 0, 0));
            let layers = crate::data::collect_graph(&p, graph);
            let pd = &layers[0].plots[0];
            let s = pd.surface.as_ref().expect("a surface");
            // Nine distinct points on a grid make eight triangles; the Z range is 0..4.
            assert_eq!(s.triangles.len(), 8, "{t:?}");
            assert_eq!(s.z_range, (0.0, 4.0));
            let geom = crate::layout::graph_geometry(&p, graph, &layers, &crate::FixedMetrics).remove(0);
            let mut out = Vec::new();
            draw_surface(&mut out, pd, &geom);
            // A projection (T168) draws the triangles again on the floor first, with no mesh.
            if style == Surface3d::ColorMapProjection {
                assert_eq!(s.floor.len(), 8);
                // Every corner is a grid point seen at the floor of the box (Z unit -1).
                let c = *layer.coords.space3d().unwrap();
                let bx = data_box(&p, layer).unwrap();
                let grid: Vec<(f64, f64)> = (0..3)
                    .flat_map(|i| (0..3).map(move |j| (i as f64, j as f64)))
                    .map(|(x, y)| view(&c, [bx.unit(0, x), bx.unit(1, y), -1.0]))
                    .collect();
                let on_floor =
                    |q: &(f64, f64)| grid.iter().any(|g| (g.0 - q.0).abs() < 1e-9 && (g.1 - q.1).abs() < 1e-9);
                assert!(s.floor.iter().all(|(t, _)| t.iter().all(on_floor)));
                assert_eq!(out.len(), 16);
                assert!(
                    out[..8]
                        .iter()
                        .all(|i| matches!(i, Item::Path { fill: Some(f), stroke: Some(st), .. } if st.color == *f))
                );
                out.drain(..8);
            } else {
                assert!(s.floor.is_empty());
            }
            assert_eq!(out.len(), 8);
            let fills: Vec<_> =
                out.iter().map(|i| if let Item::Path { fill, .. } = i { *fill } else { None }).collect();
            match style {
                Surface3d::WireFrame => assert!(fills.iter().all(Option::is_none)),
                Surface3d::WireSurface => assert!(fills.iter().all(|f| *f == Some(sheaf_core::Color::WHITE))),
                Surface3d::ColorFill => assert!(fills.iter().all(|f| *f == Some(plot.symbol.color))),
                // Mean Z 0 (blue) to 4 (red): the colours differ.
                Surface3d::ColorMap | Surface3d::ColorMapProjection => {
                    assert!(fills.windows(2).any(|w| w[0] != w[1]))
                }
                // Walls, not triangles: their own test.
                Surface3d::XConstant | Surface3d::YConstant => unreachable!(),
            }
        }
        // Farthest first: at azimuth 40 the far corner is at the largest X and Y.
        let g = p.plot_columns(Template::ColorFillSurface3d, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        let layers = crate::data::collect_graph(&p, graph);
        let tris = &layers[0].plots[0].surface.as_ref().unwrap().triangles;
        // Z = X * Y: the far corner (2, 2) has the largest Z, the near corner (0, 0) the smallest.
        assert!(tris.first().unwrap().1 > tris.last().unwrap().1);
    }

    #[test]
    fn constant_with_base_draws_one_wall_per_x_or_y_value_back_to_front() {
        use sheaf_core::column::{ColumnData, Designation};
        use sheaf_core::graph::Surface3d;
        use sheaf_core::template::Template;
        // Three X values by four Y values, in a shuffled row order, and a point alone in X and Y (no wall).
        let mut rows = Vec::new();
        for j in [2, 0, 3, 1] {
            for i in 0..3 {
                rows.push((i as f64, j as f64, (i + j) as f64));
            }
        }
        rows.push((9.0, 9.0, 1.0));
        let mut p = Project::new();
        let b = p.add_workbook("Book1");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(rows.iter().map(|r| r.0).collect())).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(rows.iter().map(|r| r.1).collect())).unwrap();
        p.add_column(b, 0, Designation::Z, ColumnData::numeric(rows.iter().map(|r| r.2).collect())).unwrap();
        for (t, style, walls, points) in
            [(Template::XConstant3d, Surface3d::XConstant, 3, 4), (Template::YConstant3d, Surface3d::YConstant, 4, 3)]
        {
            let g = p.plot_columns(t, &[y]).unwrap();
            let graph = p.graph(g).unwrap();
            let layer = &graph.layers[0];
            let c = *layer.coords.space3d().unwrap();
            assert_eq!(c.surface, Some(style));
            let bx = data_box(&p, layer).unwrap();
            let layers = crate::data::collect_graph(&p, graph);
            let pd = &layers[0].plots[0];
            let s = pd.surface.as_ref().unwrap();
            assert!(s.triangles.is_empty());
            assert_eq!(s.walls.len(), walls, "{t:?}");
            // Each wall: the curve in order of the other coordinate, then the floor back.
            assert!(s.walls.iter().all(|w| w.len() == 2 * points));
            let at = |x: f64, y: f64, z: f64| view(&c, [bx.unit(0, x), bx.unit(1, y), bx.unit(2, z)]);
            let floor = |x: f64, y: f64| view(&c, [bx.unit(0, x), bx.unit(1, y), -1.0]);
            let close = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
            // The last wall is the nearest: X = 0 (or Y = 0) at azimuth 40.
            let near = s.walls.last().unwrap();
            if style == Surface3d::XConstant {
                assert!(close(near[0], at(0.0, 0.0, 0.0)) && close(near[3], at(0.0, 3.0, 3.0)));
                assert!(close(near[4], floor(0.0, 3.0)) && close(near[7], floor(0.0, 0.0)));
            } else {
                assert!(close(near[0], at(0.0, 0.0, 0.0)) && close(near[2], at(2.0, 0.0, 2.0)));
            }
            let geom = crate::layout::graph_geometry(&p, graph, &layers, &crate::FixedMetrics).remove(0);
            let mut out = Vec::new();
            draw_surface(&mut out, pd, &geom);
            assert_eq!(out.len(), walls);
            assert!(
                out.iter().all(
                    |i| matches!(i, Item::Path { fill: Some(f), stroke: Some(_), .. } if *f == pd.plot.symbol.color)
                )
            );
        }
    }
}
