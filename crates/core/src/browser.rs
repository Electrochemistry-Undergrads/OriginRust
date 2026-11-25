//! Graph > Browser Graph (T410; Origin 2020b's Browser Graph, browsing across sheets as in 2024): one plot of a
//! graph steps through a list of similarly structured datasets, one shown at a time, with the previous / next
//! buttons of the bar above the graph. The datasets are either the Y columns of the plot's sheet, or the column
//! in the same place in every sheet of its workbook. Each dataset keeps the X (and Y error) column its sheet
//! gives it; showing another one rescales the layer to it.
//!
//! Sheaf's choices (Origin 2020b not measured here, Origin 2016 has no browser graph): one browsed plot per
//! graph; "the same place" is the column with the same short name, else the one at the same position; a step
//! is an undo step, as other graph edits are.

use crate::column::Designation;
use crate::graph::Graph;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use serde::{Deserialize, Serialize};

/// What the browser steps through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowseBy {
    /// The Y columns of the plot's sheet.
    Columns,
    /// The column in the same place in every sheet of the workbook.
    Sheets,
    /// The column in the same place of the same sheet in every workbook (T456).
    Books,
}

/// One dataset of a browser.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrowserItem {
    pub x: Option<ColumnId>,
    pub y: ColumnId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y_error: Option<ColumnId>,
}

/// A graph's browser (in [`Graph::browser`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphBrowser {
    pub by: BrowseBy,
    /// The browsed plot: its layer and its index in that layer.
    pub layer: usize,
    pub plot: usize,
    pub items: Vec<BrowserItem>,
    /// The dataset shown.
    pub current: usize,
}

impl GraphBrowser {
    /// "Y3 (2 of 5)": what the bar above the graph says.
    pub fn caption(&self, project: &Project) -> String {
        let Some(item) = self.items.get(self.current) else { return String::new() };
        let name = match project.locate(item.y) {
            Some((_, sheet, col)) if self.by == BrowseBy::Sheets => format!("[{}] {}", sheet.name, col.display_name()),
            Some((book, _, col)) if self.by == BrowseBy::Books => format!("[{}] {}", book.name, col.display_name()),
            Some((_, _, col)) => col.display_name().to_string(),
            None => String::new(),
        };
        format!("{name} ({} of {})", self.current + 1, self.items.len())
    }
}

impl Graph {
    /// Every column a browser lists, so that File > Append and other renumbering keep them.
    pub(crate) fn remap_browser(&mut self, f: &dyn Fn(ColumnId) -> ColumnId) {
        if let Some(b) = &mut self.browser {
            for it in &mut b.items {
                it.x = it.x.map(f);
                it.y = f(it.y);
                it.y_error = it.y_error.map(f);
            }
        }
    }
}

impl Project {
    /// The datasets a browser of `by` lists for a plot of column `y`, and which of them is `y`'s.
    pub fn browser_items(&self, y: ColumnId, by: BrowseBy) -> Result<(Vec<BrowserItem>, usize), ProjectError> {
        let (book, sheet, col) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
        let item = |s: &crate::worksheet::Worksheet, i: usize| BrowserItem {
            x: s.x_for(i).map(|c| c.id),
            y: s.columns[i].id,
            y_error: s.y_error_for(i).map(|c| c.id),
        };
        // The column in the same place of a sheet: the same short name, else the same position; numbers only.
        let at = sheet.columns.iter().position(|c| c.id == y).unwrap_or(0);
        let same_place = |s: &crate::worksheet::Worksheet| {
            let i = s
                .columns
                .iter()
                .position(|c| c.short_name.eq_ignore_ascii_case(&col.short_name))
                .or_else(|| (at < s.columns.len()).then_some(at))?;
            (s.columns[i].id == y || s.columns[i].data.is_numeric()).then(|| item(s, i))
        };
        let items: Vec<BrowserItem> = match by {
            BrowseBy::Columns => sheet
                .columns
                .iter()
                .enumerate()
                .filter(|(_, c)| c.id == y || (c.designation == Designation::Y && c.data.is_numeric()))
                .map(|(i, _)| item(sheet, i))
                .collect(),
            BrowseBy::Sheets => book.sheets.iter().filter_map(same_place).collect(),
            // The sheet with the same name in each workbook, else the one at the same position.
            BrowseBy::Books => {
                let sheet_at = book.sheets.iter().position(|s| std::ptr::eq(s, sheet)).unwrap_or(0);
                self.workbooks()
                    .filter_map(|b| {
                        let s = b
                            .sheets
                            .iter()
                            .find(|s| s.name.eq_ignore_ascii_case(&sheet.name))
                            .or_else(|| b.sheets.get(sheet_at))?;
                        same_place(s)
                    })
                    .collect()
            }
        };
        let current = items.iter().position(|it| it.y == y).unwrap_or(0);
        if items.len() < 2 {
            return Err(ProjectError::BrowserNeedsTwo);
        }
        Ok((items, current))
    }

    /// Makes plot `plot` of layer `layer` of graph `graph` a browser of `by`; returns how many datasets it
    /// steps through. Fewer than two is refused.
    pub fn set_graph_browser(
        &mut self,
        graph: WindowId,
        layer: usize,
        plot: usize,
        by: BrowseBy,
    ) -> Result<usize, ProjectError> {
        let g = self.graph(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let y = g.layers.get(layer).and_then(|l| l.plots.get(plot)).ok_or(ProjectError::NoSuchPlot(plot))?.y;
        let (items, current) = self.browser_items(y, by)?;
        let n = items.len();
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        g.browser = Some(GraphBrowser { by, layer, plot, items, current });
        Ok(n)
    }

    /// Shows dataset `to` of graph `graph`'s browser: the browsed plot reads its columns and the layer
    /// rescales to it.
    pub fn browse_graph(&mut self, graph: WindowId, to: usize) -> Result<(), ProjectError> {
        // Everything is checked before anything changes.
        let g = self.graph(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let b = g.browser.as_ref().ok_or(ProjectError::NoBrowser)?;
        let item = *b.items.get(to).ok_or(ProjectError::NoBrowser)?;
        let (layer, plot) = (b.layer, b.plot);
        g.layers.get(layer).and_then(|l| l.plots.get(plot)).ok_or(ProjectError::NoSuchPlot(plot))?;
        self.column(item.y).ok_or(ProjectError::NoSuchColumn(item.y))?;
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        if let Some(b) = g.browser.as_mut() {
            b.current = to;
        }
        let l = &mut g.layers[layer];
        let p = &mut l.plots[plot];
        p.x = item.x;
        p.y = item.y;
        p.y_error = item.y_error;
        for a in [&mut l.x_axis, &mut l.y_axis] {
            a.min = None;
            a.max = None;
        }
        Ok(())
    }

    /// Removes graph `graph`'s browser; the plot keeps the dataset it shows. False when it had none.
    pub fn clear_graph_browser(&mut self, graph: WindowId) -> bool {
        self.graph_mut(graph).is_some_and(|g| g.browser.take().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::ColumnData;
    use crate::template::Template;

    #[test]
    fn a_browser_steps_through_the_y_columns_of_the_sheet() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let y1 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let err = p.add_column(b, 0, Designation::YError, ColumnData::numeric(vec![0.1, 0.1])).unwrap();
        let y2 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0])).unwrap();
        let _text = p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["a".into(), "b".into()])).unwrap();
        let y3 = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![5.0, 6.0])).unwrap();
        let g = p.plot_columns(Template::Line, &[y2]).unwrap();
        p.graph_mut(g).unwrap().layers[0].y_axis.max = Some(30.0);
        assert_eq!(p.set_graph_browser(g, 0, 0, BrowseBy::Columns).unwrap(), 3, "the text column is no dataset");
        let br = p.graph(g).unwrap().browser.clone().unwrap();
        assert_eq!(br.items.iter().map(|i| i.y).collect::<Vec<_>>(), [y1, y2, y3]);
        assert_eq!(br.current, 1);
        assert_eq!(br.items[0].y_error, Some(err));
        p.browse_graph(g, 0).unwrap();
        let graph = p.graph(g).unwrap();
        let plot = &graph.layers[0].plots[0];
        assert_eq!((plot.x, plot.y, plot.y_error), (Some(x), y1, Some(err)));
        assert_eq!(graph.layers[0].y_axis.max, None, "rescaled to the dataset");
        assert!(graph.browser.as_ref().unwrap().caption(&p).ends_with("(1 of 3)"));
        assert!(p.browse_graph(g, 3).is_err());
        // It saves with the project.
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.graph(g).unwrap().browser.as_ref().unwrap().current, 0);
        assert!(p.clear_graph_browser(g));
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].y, y1, "the plot keeps the dataset it shows");
        assert!(!p.clear_graph_browser(g));
    }

    #[test]
    fn a_browser_by_sheet_takes_the_same_column_of_every_sheet() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let mut ys = Vec::new();
        for s in 0..3 {
            if s > 0 {
                p.workbook_mut(b).unwrap().sheets.push(crate::worksheet::Worksheet::new(format!("Sheet{}", s + 1)));
            }
            p.add_column(b, s, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
            ys.push(p.add_column(b, s, Designation::Y, ColumnData::numeric(vec![s as f64, 1.0])).unwrap());
        }
        let g = p.plot_columns(Template::Scatter, &[ys[1]]).unwrap();
        p.set_graph_browser(g, 0, 0, BrowseBy::Sheets).unwrap();
        let br = p.graph(g).unwrap().browser.clone().unwrap();
        assert_eq!(br.items.iter().map(|i| i.y).collect::<Vec<_>>(), ys);
        assert_eq!(br.current, 1);
        assert!(br.caption(&p).starts_with("[Sheet2] "), "{}", br.caption(&p));
        // Across workbooks (T456): the same column of Sheet1 in another book with the same layout.
        let other = p.add_workbook("");
        p.add_column(other, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let y_other = p.add_column(other, 0, Designation::Y, ColumnData::numeric(vec![7.0, 8.0])).unwrap();
        let g3 = p.plot_columns(Template::Scatter, &[ys[0]]).unwrap();
        p.set_graph_browser(g3, 0, 0, BrowseBy::Books).unwrap();
        let br = p.graph(g3).unwrap().browser.clone().unwrap();
        assert_eq!(br.items.iter().map(|i| i.y).collect::<Vec<_>>(), [ys[0], y_other]);
        let other_name = p.workbook(other).unwrap().name.clone();
        p.browse_graph(g3, 1).unwrap();
        let br = p.graph(g3).unwrap().browser.clone().unwrap();
        assert!(br.caption(&p).starts_with(&format!("[{other_name}] ")), "{}", br.caption(&p));
        // A lone column is nothing to browse.
        let other = p.add_workbook("");
        let lone = p.add_column(other, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        let g2 = p.plot_columns(Template::Line, &[lone]).unwrap();
        assert!(p.set_graph_browser(g2, 0, 0, BrowseBy::Columns).is_err());
    }
}
