//! Plot > Specialized > Zoom (T112), as Origin 2016's, measured on 2026-11-16 (`docs/spec/graphing.md`,
//! "Zoom graph"): the selected curves in an upper layer with a cyan, blue-edged region rectangle, and the
//! same curves in a lower layer that shows exactly the rectangle's region (`Layer::zoom`). Origin centred
//! the rectangle on the first data point, 0.19167 of the layer's width and 0.30764 of its height in size.

use crate::Color;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::shape::{Shape, ShapeKind};
use crate::template::{Template, ZOOM_FILL};

/// The region rectangle's size as fractions of the upper layer's frame (measured).
pub const REGION_WIDTH: f64 = 0.19167;
pub const REGION_HEIGHT: f64 = 0.30764;

/// Where a value sits between `lo` and `hi`, 0..1 (0.5 when they are equal or not finite).
fn fraction(v: f64, lo: f64, hi: f64) -> f64 {
    if hi > lo && v.is_finite() { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.5 }
}

impl Project {
    /// Plots the Y columns `ys` (with their X columns) as a Zoom graph: every curve in both layers, the
    /// region rectangle around the first point of the first curve.
    pub fn plot_zoom(&mut self, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let first = *ys.first().ok_or(ProjectError::ZoomNeedsColumn)?;
        let id = self.add_graph(Template::Zoom);
        let mut name = None;
        for &y in ys {
            let (book, sheet, _) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
            let x = sheet.column_index(y).and_then(|i| sheet.x_for(i)).map(|c| c.id);
            name.get_or_insert_with(|| book.title_for_graph());
            self.add_plot_xy(id, 0, x, y, None)?;
            self.add_plot_xy(id, 1, x, y, None)?;
        }
        // The first point of the first curve, placed within the data's extent (the axes round outwards a
        // little, so the rectangle sits near, not exactly on, the point).
        let ys0 = self.numeric(first).unwrap_or_default().to_vec();
        let xs0: Vec<f64> = match self.graph(id).and_then(|g| g.layers[0].plots[0].x) {
            Some(x) => self.numeric(x).unwrap_or_default().to_vec(),
            None => (1..=ys0.len()).map(|i| i as f64).collect(),
        };
        let extent = |v: &[f64]| {
            v.iter()
                .filter(|x| x.is_finite())
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)))
        };
        let ((x_lo, x_hi), (y_lo, y_hi)) = (extent(&xs0), extent(&ys0));
        let (fx, fy) = match xs0.iter().zip(&ys0).find(|(x, y)| x.is_finite() && y.is_finite()) {
            Some((x, y)) => (fraction(*x, x_lo, x_hi), 1.0 - fraction(*y, y_lo, y_hi)),
            None => (0.5, 0.5),
        };
        let left = (fx - REGION_WIDTH / 2.0).clamp(0.0, 1.0 - REGION_WIDTH);
        let top = (fy - REGION_HEIGHT / 2.0).clamp(0.0, 1.0 - REGION_HEIGHT);
        let mut region = Shape::new(ShapeKind::Rectangle, (left, top), (left + REGION_WIDTH, top + REGION_HEIGHT));
        region.color = Color::rgb(0, 0, 255);
        region.fill = Some(ZOOM_FILL);
        let g = self.graph_mut(id).expect("the graph was just added");
        g.layers[0].shapes.push(region);
        g.layers[1].zoom = Some(g.layers[0].shapes.len() - 1);
        if let Some(n) = name {
            g.long_name = n;
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn the_region_starts_around_the_first_point_and_the_lower_layer_zooms_on_it() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=50).map(f64::from).collect())).unwrap();
        let y = p
            .add_column(
                b,
                0,
                Designation::Y,
                ColumnData::numeric((1..=50).map(|i| (f64::from(i) / 5.0).sin() * 10.0).collect()),
            )
            .unwrap();
        let g = p.plot_columns(Template::Zoom, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.len(), 2);
        assert_eq!((graph.layers[0].plots[0].y, graph.layers[1].plots[0].y), (y, y));
        let r = &graph.layers[0].shapes[0];
        assert_eq!((r.kind, r.fill), (ShapeKind::Rectangle, Some(ZOOM_FILL)));
        assert!((r.x2 - r.x1 - REGION_WIDTH).abs() < 1e-12 && r.x1 == 0.0, "the first X is the left end");
        assert_eq!(graph.layers[1].zoom, Some(0));
        assert_eq!(graph.layers[1].background, Some(ZOOM_FILL));
        // Origin's frames: the lower layer is 1.06 x 1.52 of the upper one, 27.1 % of its height below its top.
        let (f0, f1) = (graph.page_frame(0), graph.page_frame(1));
        assert!((f1.top - (f0.top + 1.271 * f0.height)).abs() < 1e-9 && (f1.height - 1.52 * f0.height).abs() < 1e-9);
    }
}
