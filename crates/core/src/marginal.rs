//! Plot > Statistics > Marginal Histograms and Marginal Box Charts (T110), as Origin 2016's, measured on
//! 2026-11-16 (`docs/spec/graphing.md`, "Marginal plots"):
//!
//! - Three layers. Layer 1 is the scatter of Y against X at left 15 %, top 25.01 %, 60 % x 60 % of the
//!   page. Layer 2, above it, is linked to it at left 0, top -33.01 %, width 100 %, height 33.01 % (of
//!   layer 1) and shares its X scale; layer 3, to its right, at left 100 %, top 0, width 33.01 %, height
//!   100 %, shares its Y scale.
//! - Marginal Histograms: the X values binned along the top and the Y values along the right (the bars
//!   lie on their side there), light grey (192, 192, 192) with black edges; the scatter's symbols are
//!   black. The histogram layers show only the base line, without tick labels.
//! - Marginal Box Charts: a red box of the X values lying above the scatter and one of the Y values
//!   standing to its right, each about 0.4 of the box layer's unit span (0.5 to 1.5) wide, with the mean
//!   as a small hollow square; blue scatter symbols, the scatter's frame closed. The box layers draw no
//!   axes.

use crate::Color;
use crate::graph::{PlotKind, SymbolShape};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::template::Template;
use serde::{Deserialize, Serialize};

/// Which margin a marginal plot stands in (in [`crate::graph::PlotDecor::marginal`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginalSide {
    /// Above the scatter: the values run along X, the counts (or the box) up the Y axis.
    Top,
    /// Right of the scatter: the values run up Y, the counts (or the box) along X.
    Right,
}

/// The fill of a marginal histogram's bars (measured light grey).
pub const HISTOGRAM_FILL: Color = Color::rgb(192, 192, 192);

impl Project {
    /// Plots a marginal graph from the Y column `ys[0]` and its X column: the scatter in layer 0, the
    /// X values' histogram or box in layer 1 (top) and the Y values' in layer 2 (right).
    pub fn plot_marginal(&mut self, template: Template, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        if ys.len() != 1 {
            return Err(ProjectError::MarginalColumns("a marginal plot needs one Y column with an X column"));
        }
        self.plot_marginal_groups(template, ys)
    }

    /// [`Project::plot_marginal`] for several Y columns: several Y columns, each with its own X column, are the groups of a grouped marginal plot (T373,
    /// Origin 2021b; for example the columns [`Project::group_columns`] makes): a scatter per group in the
    /// palette's colours, and in each margin a histogram per group in its colour (half transparent, so
    /// the bars of the groups overlap visibly) or a box per group side by side (positions 1, 2, ...).
    pub fn plot_marginal_groups(&mut self, template: Template, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let need = "a marginal plot needs one Y column with an X column";
        if ys.is_empty() {
            return Err(ProjectError::MarginalColumns(need));
        }
        let mut pairs = Vec::new();
        for &y in ys {
            let (_, sheet, _) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
            let x = sheet.column_index(y).and_then(|i| sheet.x_for(i)).map(|c| c.id);
            let Some(x) = x else { return Err(ProjectError::MarginalColumns(need)) };
            pairs.push((x, y));
        }
        let (book, _, _) = self.locate(ys[0]).ok_or(ProjectError::NoSuchColumn(ys[0]))?;
        let name = book.title_for_graph();
        let grouped = pairs.len() > 1;
        let hist = template == Template::MarginalHistograms;
        let kind = if hist { PlotKind::MarginalHistogram } else { PlotKind::MarginalBox };
        let id = self.add_graph(template);
        for &(x, y) in &pairs {
            self.add_plot_xy(id, 0, Some(x), y, None)?;
        }
        if !hist && !grouped {
            let scatter = self.graph_mut(id).and_then(|g| g.layers[0].plots.last_mut()).expect("a plot was just added");
            scatter.symbol.color = Color::rgb(0, 0, 255);
            scatter.auto_color = false;
        }
        let colors: Vec<Color> =
            self.graph(id).map(|g| g.layers[0].plots.iter().map(|p| p.symbol.color).collect()).unwrap_or_default();
        for (i, &(x, y)) in pairs.iter().enumerate() {
            for (layer, column, side) in [(1, x, MarginalSide::Top), (2, y, MarginalSide::Right)] {
                self.add_plot_xy(id, layer, None, column, None)?;
                let plot =
                    self.graph_mut(id).and_then(|g| g.layers[layer].plots.last_mut()).expect("a plot was just added");
                plot.kind = kind;
                plot.auto_color = false;
                plot.line.width_pt = 0.5;
                let group = colors.get(i).copied().unwrap_or(Color::BLACK);
                match (hist, grouped) {
                    (true, false) => {
                        plot.line.color = Color::BLACK;
                        plot.symbol.color = HISTOGRAM_FILL;
                    }
                    (true, true) => {
                        plot.line.color = group;
                        plot.symbol.color = Color::rgba(group.r, group.g, group.b, 128);
                    }
                    (false, _) => {
                        let c = if grouped { group } else { Color::RED };
                        plot.line.color = c;
                        plot.symbol.color = c;
                        plot.symbol.shape = SymbolShape::Square;
                        plot.symbol.filled = false;
                    }
                }
                plot.decor.marginal = Some(side);
                if grouped && !hist {
                    plot.decor.marginal_slot = Some(i as u32 + 1);
                }
            }
        }
        // T508: grouped histograms share their bins, the automatic bins of all the groups' values of each margin.
        if grouped && hist {
            for (layer, axis_x) in [(1usize, true), (2, false)] {
                let mut all = Vec::new();
                for &(x, y) in &pairs {
                    all.extend_from_slice(self.numeric(if axis_x { x } else { y }).unwrap_or(&[]));
                }
                let opts = sheaf_calc::freq::FreqOptions {
                    min: None,
                    max: None,
                    size: sheaf_calc::freq::BinSize::Auto,
                    below: false,
                    above: false,
                };
                let Some(f) = sheaf_calc::freq::freq_counts(&all, &opts) else { continue };
                let end = f.rows.iter().map(|r| r.end).filter(|e| e.is_finite()).fold(f64::NEG_INFINITY, f64::max);
                if !(end.is_finite() && f.bin_size > 0.0) {
                    continue;
                }
                let shared = crate::graph::HistogramStyle {
                    bin_size: Some(f.bin_size),
                    begin: Some(f.bin_start),
                    end: Some(end),
                };
                if let Some(l) = self.graph_mut(id).and_then(|g| g.layers.get_mut(layer)) {
                    for p in &mut l.plots {
                        p.hist = shared;
                    }
                }
            }
        }
        if let Some(g) = self.graph_mut(id) {
            g.long_name = name;
            // A box per group, at positions 1 to k: the box margins span 0.5 to k + 0.5.
            if grouped && !hist {
                let end = pairs.len() as f64 + 0.5;
                g.layers[1].y_axis.max = Some(end);
                g.layers[2].x_axis.max = Some(end);
            }
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn a_marginal_graph_has_the_scatter_and_a_plot_of_each_column_in_the_margins() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.2, 2.5, 2.9])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.1, 3.9, 3.2])).unwrap();
        let y2 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        assert!(p.plot_marginal(Template::MarginalHistograms, &[y, y2]).is_err());
        let g = p.plot_marginal(Template::MarginalHistograms, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.len(), 3);
        let scatter = &graph.layers[0].plots[0];
        assert_eq!((scatter.kind, scatter.x, scatter.y), (PlotKind::Scatter, Some(x), y));
        let (top, right) = (&graph.layers[1].plots[0], &graph.layers[2].plots[0]);
        assert_eq!((top.kind, top.y, top.decor.marginal), (PlotKind::MarginalHistogram, x, Some(MarginalSide::Top)));
        assert_eq!((right.y, right.decor.marginal), (y, Some(MarginalSide::Right)));
        assert_eq!(top.symbol.color, HISTOGRAM_FILL);
        // Origin's frames: the margins are a third of the scatter's size, above it and right of it.
        let (f0, f1, f2) = (graph.page_frame(0), graph.page_frame(1), graph.page_frame(2));
        assert!((f0.left - 0.15).abs() < 1e-9 && (f0.top - 0.2501).abs() < 1e-9 && (f0.width - 0.6).abs() < 1e-9);
        assert!((f1.top + f1.height - f0.top).abs() < 1e-9 && (f1.height - 0.6 * 0.3301).abs() < 1e-9);
        assert!((f2.left - (f0.left + f0.width)).abs() < 1e-9 && (f2.width - 0.6 * 0.3301).abs() < 1e-9);
        let g = p.plot_marginal(Template::MarginalBoxCharts, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers[2].plots[0].kind, PlotKind::MarginalBox);
        assert_eq!(graph.layers[0].plots[0].symbol.color, Color::rgb(0, 0, 255));
        assert_eq!(graph.layers[1].plots[0].line.color, Color::RED);
    }

    #[test]
    fn grouped_marginal_plots_have_a_scatter_and_a_histogram_or_box_per_group() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 1.0, 5.0, 3.0, 6.0])).unwrap();
        let key = p
            .add_column(
                book,
                0,
                Designation::Disregard,
                ColumnData::text(["a", "b", "a", "b", "a", "b"].iter().map(|s| s.to_string()).collect()),
            )
            .unwrap();
        let (_, groups) = p.group_columns(book, 0, &[y], key).unwrap();
        let g = p.plot_marginal_groups(Template::MarginalBoxCharts, &groups).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.iter().map(|l| l.plots.len()).collect::<Vec<_>>(), [2, 2, 2]);
        let slots: Vec<_> = graph.layers[1].plots.iter().map(|q| q.decor.marginal_slot).collect();
        assert_eq!(slots, [Some(1), Some(2)]);
        assert_eq!((graph.layers[1].y_axis.max, graph.layers[2].x_axis.max), (Some(2.5), Some(2.5)), "both boxes show");
        // Each group's box has its scatter's colour.
        assert_eq!(graph.layers[2].plots[1].line.color, graph.layers[0].plots[1].symbol.color);
        assert_ne!(graph.layers[0].plots[0].symbol.color, graph.layers[0].plots[1].symbol.color);
        let h = p.plot_marginal_groups(Template::MarginalHistograms, &groups).unwrap();
        let fill = p.graph(h).unwrap().layers[1].plots[0].symbol.color;
        assert_eq!(fill.a, 128, "half transparent so the groups overlap visibly");
        // T508: both groups' histograms of a margin share one set of bins over all the values.
        let graph = p.graph(h).unwrap();
        for layer in [1, 2] {
            let (a, b) = (&graph.layers[layer].plots[0].hist, &graph.layers[layer].plots[1].hist);
            assert_eq!(a, b, "layer {layer}");
            assert!(a.bin_size.is_some() && a.begin.is_some() && a.end.is_some(), "{a:?}");
        }
        let x = &graph.layers[1].plots[0].hist;
        assert!(x.begin.unwrap() <= 1.0 && x.end.unwrap() >= 6.0, "the bins cover every group's X: {x:?}");
    }
}
