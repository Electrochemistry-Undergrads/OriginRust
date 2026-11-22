//! The data points inside a drawn rectangle or ellipse of a graph (T355; Origin 2024b's "extract data from regions
//! on a graph", not in Origin 2016, so the rules are Sheaf's own; graphing spec 17.12).
//!
//! A drawn object's corners are fractions of its layer's frame. A plot's point is inside when its place on the page
//! (through the layer's axis scales, so log and reciprocal axes and reversed limits count) falls inside the
//! rectangle, or inside the ellipse inscribed in it. Every plot of the layer is searched over the rows it shows;
//! rows with a missing X or Y are left out. Offsets of stacked plots are not applied.

use crate::{PlotError, layout_geometry};
use sheaf_core::shape::ShapeKind;
use sheaf_core::{Project, WindowId};

/// The points of one plot inside the region.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionPoints {
    /// The plot's index in the layer.
    pub plot: usize,
    /// The source rows (0-based) of the points, in row order.
    pub rows: Vec<usize>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// Whether the frame fraction `(u, v)` is inside a box with corners `(x1, y1)` and `(x2, y2)`, or the ellipse
/// inscribed in it.
fn inside(kind: ShapeKind, (x1, y1, x2, y2): (f64, f64, f64, f64), (u, v): (f64, f64)) -> bool {
    let (lo_x, hi_x, lo_y, hi_y) = (x1.min(x2), x1.max(x2), y1.min(y2), y1.max(y2));
    match kind {
        ShapeKind::Ellipse => {
            let (rx, ry) = ((hi_x - lo_x) / 2.0, (hi_y - lo_y) / 2.0);
            if rx <= 0.0 || ry <= 0.0 {
                return false;
            }
            let (dx, dy) = ((u - (lo_x + rx)) / rx, (v - (lo_y + ry)) / ry);
            dx * dx + dy * dy <= 1.0
        }
        _ => u >= lo_x && u <= hi_x && v >= lo_y && v <= hi_y,
    }
}

/// The data points of every plot of layer `layer` inside drawn object `shape` of that layer (a rectangle or an
/// ellipse); plots without a point inside are left out. `None` for another kind of object or one that is missing.
pub fn points_in_shape(
    project: &Project,
    graph: WindowId,
    layer: usize,
    shape: usize,
) -> Result<Option<Vec<RegionPoints>>, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let l = g.layers.get(layer).ok_or(PlotError::NoSuchLayer(layer))?;
    let Some(s) = l.shapes.get(shape).filter(|s| matches!(s.kind, ShapeKind::Rectangle | ShapeKind::Ellipse)) else {
        return Ok(None);
    };
    let geometry = layout_geometry(project, graph)?;
    let geom = geometry.get(layer).ok_or(PlotError::NoSuchLayer(layer))?;
    let f = geom.frame;
    if f.w <= 0.0 || f.h <= 0.0 {
        return Ok(Some(Vec::new()));
    }
    let corners = (s.x1, s.y1, s.x2, s.y2);
    let mut out = Vec::new();
    for (k, p) in l.plots.iter().enumerate() {
        let Some(ys) = project.column(p.y).and_then(|c| c.numeric()) else { continue };
        let xs: Vec<f64> = match p.x.and_then(|x| project.column(x)).and_then(|c| c.numeric()) {
            Some(x) => x.to_vec(),
            None => (1..=ys.len()).map(|i| i as f64).collect(),
        };
        let (start, end) = sheaf_core::stack::row_range(p, xs.len().min(ys.len()));
        let mut found = RegionPoints { plot: k, rows: Vec::new(), x: Vec::new(), y: Vec::new() };
        for r in start..end {
            let (x, y) = (xs[r], ys[r]);
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            let (px, py) = geom.data_to_page(x, y);
            if !(px.is_finite() && py.is_finite()) {
                continue;
            }
            if inside(s.kind, corners, ((px - f.x) / f.w, (py - f.y) / f.h)) {
                found.rows.push(r);
                found.x.push(x);
                found.y.push(y);
            }
        }
        if !found.rows.is_empty() {
            out.push(found);
        }
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::graph::ScaleKind;
    use sheaf_core::shape::Shape;
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation};

    /// y = x for x = 0..10, axes 0..10 on both sides.
    fn project() -> (Project, WindowId) {
        let mut p = Project::new();
        let b = p.add_workbook("d");
        let v: Vec<f64> = (0..=10).map(f64::from).collect();
        p.add_column(b, 0, Designation::X, ColumnData::numeric(v.clone())).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
        let g = p.add_graph(Template::Scatter);
        p.add_plot(g, 0, y).unwrap();
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        (l.x_axis.min, l.x_axis.max, l.y_axis.min, l.y_axis.max) = (Some(0.0), Some(10.0), Some(0.0), Some(10.0));
        (p, g)
    }

    fn add(p: &mut Project, g: WindowId, kind: ShapeKind, c: (f64, f64, f64, f64)) -> usize {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.shapes.push(Shape::new(kind, (c.0, c.1), (c.2, c.3)));
        l.shapes.len() - 1
    }

    #[test]
    fn a_rectangle_takes_the_points_inside_it_through_the_axes() {
        let (mut p, g) = project();
        // Fractions: x 0.25..0.65 is data 2.5..6.5; y from the top 0.3..0.8 is data 7..2.
        let r = add(&mut p, g, ShapeKind::Rectangle, (0.65, 0.8, 0.25, 0.3));
        let found = points_in_shape(&p, g, 0, r).unwrap().unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rows, [3, 4, 5, 6]);
        assert_eq!(found[0].y, [3.0, 4.0, 5.0, 6.0]);
        // On a log X axis the same box covers other data.
        p.graph_mut(g).unwrap().layers[0].x_axis.scale = ScaleKind::Log10;
        p.graph_mut(g).unwrap().layers[0].x_axis.min = Some(1.0);
        let found = points_in_shape(&p, g, 0, r).unwrap().unwrap();
        assert!(found[0].rows != [3, 4, 5, 6], "{:?}", found[0].rows);
    }

    #[test]
    fn an_ellipse_leaves_out_the_corners_and_lines_are_not_regions() {
        let (mut p, g) = project();
        // The box of data 0..10 x 0..10 (the whole frame): its ellipse leaves out the ends of y = x.
        let e = add(&mut p, g, ShapeKind::Ellipse, (0.0, 0.0, 1.0, 1.0));
        let found = points_in_shape(&p, g, 0, e).unwrap().unwrap();
        // On the diagonal the ellipse reaches |k - 5| <= 5 / sqrt 2.
        assert_eq!(found[0].rows, [2, 3, 4, 5, 6, 7, 8]);
        let line = add(&mut p, g, ShapeKind::Line, (0.0, 0.0, 1.0, 1.0));
        assert_eq!(points_in_shape(&p, g, 0, line).unwrap(), None);
        let empty = add(&mut p, g, ShapeKind::Rectangle, (0.9, 0.9, 0.95, 0.95));
        assert_eq!(points_in_shape(&p, g, 0, empty).unwrap(), Some(Vec::new()));
    }
}
