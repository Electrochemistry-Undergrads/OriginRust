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
use sheaf_core::graph::{Axis, Layer, ScaleKind, Space3dCoords};
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

/// The page position (in data units of the hidden axes) of a point of the cube -1..1.
pub(crate) fn view(c: &Space3dCoords, [x, y, z]: [f64; 3]) -> (f64, f64) {
    let (sa, ca) = c.azimuth_deg.to_radians().sin_cos();
    let (se, ce) = c.elevation_deg.to_radians().sin_cos();
    (x * ca - y * sa, z * ce + (x * sa + y * ca) * se)
}

/// The X, Y and Z values of a plot: X from its X column or the row numbers.
fn xyz<'a>(project: &'a Project, plot: &sheaf_core::graph::Plot) -> Option<(Cow<'a, [f64]>, &'a [f64], &'a [f64])> {
    let y = project.numeric(plot.y)?;
    let z = project.numeric(plot.size_from?)?;
    let x = match plot.x.and_then(|id| project.numeric(id)) {
        Some(x) => Cow::Borrowed(x),
        None => Cow::Owned((1..=y.len()).map(|r| r as f64).collect()),
    };
    Some((x, y, z))
}

/// The data box of a 3D layer: the nice ranges of the visible plots' X, Y and Z; `None` without data.
pub(crate) fn data_box(project: &Project, layer: &Layer) -> Option<DataBox> {
    let mut ext = [(f64::INFINITY, f64::NEG_INFINITY); 3];
    for plot in layer.plots.iter().filter(|p| p.visible) {
        let Some((x, y, z)) = xyz(project, plot) else { continue };
        for r in 0..x.len().min(y.len()).min(z.len()) {
            let v = [x[r], y[r], z[r]];
            if v.iter().all(|v| v.is_finite()) {
                for (e, v) in ext.iter_mut().zip(v) {
                    *e = (e.0.min(v), e.1.max(v));
                }
            }
        }
    }
    if ext.iter().any(|e| e.0 > e.1) {
        return None;
    }
    let free = |a: &Axis| Axis { min: None, max: None, major_step: None, scale: ScaleKind::Linear, ..a.clone() };
    // About five steps along each edge of the box: its edges are short, so the 2D axes' tick
    // density would crowd the labels.
    let resolve = |a: &Axis, e: (f64, f64)| {
        let a = free(a);
        let r = ticks::resolve_axis(&a, Some(e));
        if r.major.len() <= 7 || r.max <= r.min {
            return r;
        }
        let step = ticks::nice_step((r.max - r.min) / 5.0);
        let (min, max) = ((r.min / step).floor() * step, (r.max / step).ceil() * step);
        let (major, step) = ticks::linear_major(min, max, step, false);
        let labels = ticks::linear_labels(&a, &major, step);
        ResolvedAxis { min, max, step, major, minor: Vec::new(), labels, ..r }
    };
    let axes = [resolve(&layer.x_axis, ext[0]), resolve(&layer.y_axis, ext[1]), resolve(&layer.y_axis, ext[2])];
    Some(DataBox { axes })
}

/// Turns a plot's X, Y and Z rows into their place in the view. Rows without a Z are left out.
pub(crate) fn columns(pd: &mut PlotData, project: &Project, c: &Space3dCoords, b: &DataBox) {
    let n = pd.y.len();
    let (mut us, mut ws) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    if let Some((x, y, z)) = xyz(project, pd.plot) {
        for r in 0..n.min(x.len()).min(y.len()).min(z.len()) {
            let p = [b.unit(0, x[r]), b.unit(1, y[r]), b.unit(2, z[r])];
            if p.iter().all(|v| v.is_finite()) {
                (us[r], ws[r]) = view(c, p);
            }
        }
    }
    pd.x = Some(Cow::Owned(us));
    pd.y = Cow::Owned(ws);
    pd.x_format = None;
    pd.err = None;
    pd.xerr = None;
    // The Z column places the points; it sizes no symbol.
    pd.size_z = None;
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
    match layer.plots.iter().find(|p| p.visible) {
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
        let front = Space3dCoords { azimuth_deg: 0.0, elevation_deg: 0.0 };
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
}
