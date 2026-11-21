//! Grouped Histogram (T271): Origin 2026's Plot > Categorical: Grouped Histogram (`plot_ghist`), written
//! from OriginLab's help page on a computer whose Origin (2016) has no such plot; not measured.
//!
//! - Documented: a Y column and at least one group column; one histogram per group; Multiple Data
//!   "Overlay on Same Graph", "Separate Layers on Same Graph" or "Separate Graphs"; "Share X and Bins";
//!   the Hist template. Panels, page splits, distribution curves and data height are options.
//! - Sheaf's choices: one group column; its levels sorted as the grouped box charts sort them
//!   ([`crate::indexed::levels`]); the values of each level go to a column of a new workbook
//!   `GroupedHist<n>` (long name the level, so the legend names the groups); Overlay draws the
//!   histograms in one layer, Separate Layers is the Stacked Histograms graph of those columns; the bins
//!   are shared in both (the usual rule for all the values together).

use crate::column::{ColumnData, Designation};
use crate::graph::HistogramStyle;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::template::Template;

impl Project {
    /// Makes a grouped histogram of `y` split by the levels of `group`: one histogram per level in one
    /// layer (`separate` false) or in stacked layers. Returns the new workbook and the graph.
    pub fn plot_grouped_histogram(
        &mut self,
        y: ColumnId,
        group: ColumnId,
        separate: bool,
    ) -> Result<(WindowId, WindowId), ProjectError> {
        let values = self.numeric(y).ok_or(ProjectError::IndexedNeedsData)?.to_vec();
        let gcol = self.column(group).ok_or(ProjectError::NoSuchColumn(group))?.clone();
        if group == y {
            return Err(ProjectError::IndexedNeedsGroup);
        }
        let levels = crate::indexed::levels(&gcol);
        let mut split: Vec<(String, Vec<f64>)> = levels.into_iter().map(|l| (l, Vec::new())).collect();
        for (row, v) in values.iter().enumerate() {
            if !v.is_finite() {
                continue;
            }
            let Some(level) = crate::indexed::level_text(&gcol, row) else { continue };
            if let Some((_, vals)) = split.iter_mut().find(|(l, _)| *l == level) {
                vals.push(*v);
            }
        }
        split.retain(|(_, v)| !v.is_empty());
        if split.is_empty() {
            return Err(ProjectError::IndexedNeedsGroup);
        }
        let name = self.unique_name("GroupedHist");
        let book = self.add_workbook("");
        if let Some(b) = self.workbook_mut(book) {
            b.name = name.clone();
        }
        let mut cols = Vec::new();
        for (level, vals) in &split {
            let c = self.add_column(book, 0, Designation::Y, ColumnData::numeric(vals.clone()))?;
            if let Some(col) = self.column_mut(c) {
                col.long_name = level.clone();
            }
            cols.push(c);
        }
        let graph = if separate {
            self.plot_stacked_histograms(&cols)?
        } else {
            let graph = self.plot_columns(Template::Histogram, &cols)?;
            // The overlaid histograms share their bins, as the stacked ones do.
            let all: Vec<f64> = split.iter().flat_map(|(_, v)| v.iter().copied()).collect();
            let opts = sheaf_calc::freq::FreqOptions {
                min: None,
                max: None,
                size: sheaf_calc::freq::BinSize::Auto,
                below: false,
                above: false,
            };
            if let Some(freq) = sheaf_calc::freq::freq_counts(&all, &opts) {
                let end = freq.rows.last().map_or(freq.bin_start + freq.bin_size, |r| r.end);
                let style =
                    HistogramStyle { bin_size: Some(freq.bin_size), begin: Some(freq.bin_start), end: Some(end) };
                if let Some(g) = self.graph_mut(graph) {
                    for p in g.layers.iter_mut().flat_map(|l| l.plots.iter_mut()) {
                        p.hist = style;
                    }
                }
            }
            graph
        };
        let source = self.locate(y).map(|(_, _, c)| format!("{} by group", c.display_name()));
        if let (Some(g), Some(long)) = (self.graph_mut(graph), source) {
            g.long_name = long;
        }
        Ok((book, graph))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PlotKind;

    fn project() -> (Project, ColumnId, ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("d");
        let y = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 2.5, 3.0, 4.0, 5.0, f64::NAN, 6.0]))
            .unwrap();
        let g = p
            .add_column(
                book,
                0,
                Designation::X,
                ColumnData::Text(std::sync::Arc::new(
                    ["b", "a", "b", "a", "b", "", "a", "a"].map(String::from).to_vec(),
                )),
            )
            .unwrap();
        (p, y, g)
    }

    #[test]
    fn values_are_split_by_level_and_overlaid_with_shared_bins() {
        let (mut p, y, g) = project();
        let (book, graph) = p.plot_grouped_histogram(y, g, false).unwrap();
        let ws = p.workbook(book).unwrap().sheet();
        assert_eq!(ws.columns.len(), 2, "levels a and b; the empty group and the missing value are left out");
        assert_eq!(ws.columns[0].long_name, "a");
        assert_eq!(ws.columns[0].numeric().unwrap(), [2.0, 3.0, 6.0]);
        assert_eq!(ws.columns[1].numeric().unwrap(), [1.0, 2.5, 4.0]);
        let gr = p.graph(graph).unwrap();
        assert_eq!(gr.layers.len(), 1);
        assert_eq!(gr.layers[0].plots.len(), 2);
        assert!(gr.layers[0].plots.iter().all(|pl| pl.kind == PlotKind::Histogram));
        assert_eq!(gr.layers[0].plots[0].hist, gr.layers[0].plots[1].hist, "shared bins");
        assert!(gr.layers[0].plots[0].hist.bin_size.is_some());
    }

    #[test]
    fn separate_layers_stack_the_histograms() {
        let (mut p, y, g) = project();
        let (_, graph) = p.plot_grouped_histogram(y, g, true).unwrap();
        assert_eq!(p.graph(graph).unwrap().layers.len(), 2);
        assert!(p.plot_grouped_histogram(y, y, false).is_err());
    }
}
