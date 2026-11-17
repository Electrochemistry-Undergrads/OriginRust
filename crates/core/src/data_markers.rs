//! The graph's Data menu: Mark Data Range, Clear Data Markers, Set Display Range and Reset to Full Range
//! (T118). Built from Origin's documented behaviour, **not measured** (Origin's COM automation was
//! unregistered on the test machine on 2026-11-16; `docs/spec/graphing.md`, "Data markers and display range"):
//!
//! - Mark Data Range puts two data markers on the active plot, at its first and last plotted rows; the user
//!   then moves them to other points.
//! - Set Display Range restricts every plot that has markers to the rows between them (both included) and
//!   removes the markers; Reset to Full Range plots every row again.
//! - Clear Data Markers removes the markers of every plot of the graph.

use crate::graph::Graph;
use crate::ids::WindowId;
use crate::project::Project;

impl Project {
    /// Mark Data Range: markers at the first and last plotted rows of plot `plot` of layer `layer`.
    /// False when there is no such plot or it has no rows.
    pub fn mark_data_range(&mut self, graph: WindowId, layer: usize, plot: usize) -> bool {
        let Some(p) = self.graph(graph).and_then(|g| g.layers.get(layer)).and_then(|l| l.plots.get(plot)) else {
            return false;
        };
        let len = self.column(p.y).map_or(0, |c| c.data.len());
        let (lo, hi) = p.rows.map_or((0, len), |(a, b)| (a, b.min(len)));
        if hi <= lo {
            return false;
        }
        self.graph_mut(graph).expect("the graph exists").layers[layer].plots[plot].decor.markers = Some((lo, hi - 1));
        true
    }
}

impl Graph {
    /// Moves the markers of a plot (rows, in either order).
    pub fn set_data_markers(&mut self, layer: usize, plot: usize, a: usize, b: usize) {
        if let Some(p) = self.layers.get_mut(layer).and_then(|l| l.plots.get_mut(plot)) {
            p.decor.markers = Some((a.min(b), a.max(b)));
        }
    }

    /// Clear Data Markers. True when there were any.
    pub fn clear_data_markers(&mut self) -> bool {
        let mut any = false;
        for p in self.layers.iter_mut().flat_map(|l| l.plots.iter_mut()) {
            any |= p.decor.markers.take().is_some();
        }
        any
    }

    /// Set Display Range: every plot with markers plots only the rows between them; the markers go. True
    /// when a plot changed.
    pub fn set_display_range(&mut self) -> bool {
        let mut any = false;
        for p in self.layers.iter_mut().flat_map(|l| l.plots.iter_mut()) {
            if let Some((lo, hi)) = p.decor.markers.take() {
                p.rows = Some((lo, hi + 1));
                any = true;
            }
        }
        any
    }

    /// Reset to Full Range: every plot plots all its rows. True when a plot changed.
    pub fn reset_full_range(&mut self) -> bool {
        let mut any = false;
        for p in self.layers.iter_mut().flat_map(|l| l.plots.iter_mut()) {
            any |= p.rows.take().is_some();
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    #[test]
    fn markers_become_the_display_range_and_reset_restores_all_rows() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric((0..10).map(f64::from).collect())).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric((0..10).map(f64::from).collect())).unwrap();
        let g = p.plot_columns(Template::Line, &[y]).unwrap();
        assert!(p.mark_data_range(g, 0, 0));
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].decor.markers, Some((0, 9)));
        let graph = p.graph_mut(g).unwrap();
        graph.set_data_markers(0, 0, 7, 2);
        assert_eq!(graph.layers[0].plots[0].decor.markers, Some((2, 7)));
        assert!(graph.set_display_range());
        let plot = &graph.layers[0].plots[0];
        assert_eq!((plot.rows, plot.decor.markers), (Some((2, 8)), None), "rows 2..=7 as a half-open range");
        assert!(!graph.clear_data_markers());
        assert!(graph.reset_full_range());
        assert_eq!(graph.layers[0].plots[0].rows, None);
        // Marking a plot with a display range marks its ends.
        p.graph_mut(g).unwrap().layers[0].plots[0].rows = Some((3, 6));
        assert!(p.mark_data_range(g, 0, 0));
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].decor.markers, Some((3, 5)));
        assert!(!p.mark_data_range(g, 0, 5), "no such plot");
    }
}
