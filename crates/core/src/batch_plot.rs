//! Window > Duplicate (Batch Plotting) (T99), as Origin 2016's, measured on 2026-11-16
//! (`docs/spec/graphing.md`, "Batch plotting"):
//!
//! - **Duplicate with New Sheets/Books...** opens "Select Worksheet" (the other sheets, Match Plot Columns
//!   by Column Index, Graph Short Name and Long Name `<auto>`). Picking `[Book1]Sheet2` for a graph of
//!   Sheet1's B made Graph2, long name "Graph1 - Copy", plotting Sheet2's B: the plots take the columns at
//!   the same positions in the chosen sheet.
//! - **Duplicate with New Columns...** opens "Select Column - [Book1]Sheet1" (the sheet's other Y columns,
//!   Use Associated X ticked). Picking C made a copy plotting C against its X column A, titles and legend
//!   following.
//! - Origin then asks whether to rescale the axes to the new data (Yes by default); the copies rescale.

use crate::graph::Graph;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError, Window};

/// How the columns of another sheet are matched to the graph's (Origin's "Match Plot Columns by").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MatchBy {
    #[default]
    ColumnIndex,
    ShortName,
    LongName,
}

impl MatchBy {
    pub const ALL: [MatchBy; 3] = [MatchBy::ColumnIndex, MatchBy::ShortName, MatchBy::LongName];

    pub fn name(self) -> &'static str {
        match self {
            MatchBy::ColumnIndex => "Column Index",
            MatchBy::ShortName => "Short Name",
            MatchBy::LongName => "Long Name",
        }
    }
}

impl Project {
    /// The sheet a graph plots from (its first plot's), as (book, sheet index).
    pub fn graph_source_sheet(&self, graph: WindowId) -> Option<(WindowId, usize)> {
        let g = self.graph(graph)?;
        let y = g.layers.iter().flat_map(|l| &l.plots).map(|p| p.y).next()?;
        let (b, s, _) = self.locate(y)?;
        Some((b.id, b.sheets.iter().position(|x| std::ptr::eq(x, s))?))
    }

    /// A copy of `graph` named as Origin's ("<long name or name> - Copy") with every column mapped by
    /// `map` (columns it gives `None` for are kept), its axes rescaled. Returns the new graph.
    fn duplicate_graph_with(
        &mut self,
        graph: WindowId,
        map: &dyn Fn(ColumnId) -> Option<ColumnId>,
    ) -> Option<WindowId> {
        let Window::Graph(mut g) = self.window(graph)?.clone() else { return None };
        let title = if g.long_name.is_empty() { g.name.clone() } else { g.long_name.clone() };
        remap_graph(&mut g, map);
        for l in &mut g.layers {
            for a in [&mut l.x_axis, &mut l.y_axis] {
                a.min = None;
                a.max = None;
            }
        }
        g.long_name = format!("{title} - Copy");
        Some(self.adopt_graph(g))
    }

    /// Duplicate with New Sheets/Books: one copy of `graph` per sheet of `sheets`, plotting the columns
    /// of that sheet matched by `by`. A plot whose column has no match keeps its column.
    pub fn batch_plot_sheets(
        &mut self,
        graph: WindowId,
        sheets: &[(WindowId, usize)],
        by: MatchBy,
    ) -> Result<Vec<WindowId>, ProjectError> {
        let (sb, ss) = self.graph_source_sheet(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let source = self.workbook(sb).and_then(|b| b.sheets.get(ss)).ok_or(ProjectError::NoSuchWindow(sb))?.clone();
        let mut made = Vec::new();
        for &(book, sheet) in sheets {
            let target =
                self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(ProjectError::NoSuchLayer(sheet))?.clone();
            let map = |c: ColumnId| -> Option<ColumnId> {
                let i = source.column_index(c)?;
                let col = &source.columns[i];
                let found = match by {
                    MatchBy::ColumnIndex => target.columns.get(i),
                    MatchBy::ShortName => {
                        target.columns.iter().find(|t| t.short_name.eq_ignore_ascii_case(&col.short_name))
                    }
                    MatchBy::LongName => {
                        target.columns.iter().find(|t| !col.long_name.is_empty() && t.long_name == col.long_name)
                    }
                };
                found.map(|t| t.id)
            };
            made.extend(self.duplicate_graph_with(graph, &map));
        }
        Ok(made)
    }

    /// Duplicate with New Columns: one copy of `graph` per column of `columns`, its plots of the graph's
    /// first Y column taking that column (and, with `associated_x`, that column's X column).
    pub fn batch_plot_columns(
        &mut self,
        graph: WindowId,
        columns: &[ColumnId],
        associated_x: bool,
    ) -> Result<Vec<WindowId>, ProjectError> {
        let g = self.graph(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let first = g.layers.iter().flat_map(|l| &l.plots).next().ok_or(ProjectError::NoSuchWindow(graph))?;
        let (old_y, old_x) = (first.y, first.x);
        let mut made = Vec::new();
        for &c in columns {
            let x = if associated_x {
                let (_, sheet, _) = self.locate(c).ok_or(ProjectError::NoSuchColumn(c))?;
                sheet.column_index(c).and_then(|i| sheet.x_for(i)).map(|x| x.id)
            } else {
                None
            };
            let map = move |id: ColumnId| -> Option<ColumnId> {
                if id == old_y {
                    Some(c)
                } else if Some(id) == old_x {
                    x
                } else {
                    None
                }
            };
            made.extend(self.duplicate_graph_with(graph, &map));
        }
        Ok(made)
    }
}

/// Points the plots of a graph at other columns.
fn remap_graph(g: &mut Graph, map: &dyn Fn(ColumnId) -> Option<ColumnId>) {
    let swap = |c: &mut ColumnId| {
        if let Some(n) = map(*c) {
            *c = n;
        }
    };
    for l in &mut g.layers {
        for p in &mut l.plots {
            swap(&mut p.y);
            for c in [&mut p.x, &mut p.y_error, &mut p.x_error].into_iter().flatten() {
                swap(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;
    use crate::worksheet::Worksheet;

    /// Origin's probe: Book1 Sheet1 A 1..4, B 10..40, C 5..8; Sheet2 A 1..4, B 100..400, C 50..80;
    /// Graph1 plots Sheet1's B.
    fn project() -> (Project, WindowId, WindowId, Vec<ColumnId>) {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let mut ids = Vec::new();
        for (d, v) in [
            (Designation::X, vec![1.0, 2.0, 3.0, 4.0]),
            (Designation::Y, vec![10.0, 20.0, 30.0, 40.0]),
            (Designation::Y, vec![5.0, 6.0, 7.0, 8.0]),
        ] {
            ids.push(p.add_column(b, 0, d, ColumnData::numeric(v)).unwrap());
        }
        p.workbook_mut(b).unwrap().sheets.push(Worksheet::new("Sheet2"));
        for (d, v) in [
            (Designation::X, vec![1.0, 2.0, 3.0, 4.0]),
            (Designation::Y, vec![100.0, 200.0, 300.0, 400.0]),
            (Designation::Y, vec![50.0, 60.0, 70.0, 80.0]),
        ] {
            ids.push(p.add_column(b, 1, d, ColumnData::numeric(v)).unwrap());
        }
        let g = p.plot_columns(Template::Line, &[ids[1]]).unwrap();
        (p, b, g, ids)
    }

    #[test]
    fn with_new_sheets_plots_the_same_positions_of_the_other_sheet() {
        let (mut p, b, g, ids) = project();
        p.graph_mut(g).unwrap().layers[0].y_axis.max = Some(42.0);
        let made = p.batch_plot_sheets(g, &[(b, 1)], MatchBy::ColumnIndex).unwrap();
        assert_eq!(made.len(), 1);
        let copy = p.graph(made[0]).unwrap();
        let name = p.graph(g).unwrap().name.clone();
        assert_eq!(copy.long_name, format!("{name} - Copy"));
        let plot = &copy.layers[0].plots[0];
        assert_eq!((plot.x, plot.y), (Some(ids[3]), ids[4]), "Sheet2's A and B");
        assert_eq!(copy.layers[0].y_axis.max, None, "rescaled to the new data");
        // The original is untouched.
        assert_eq!(p.graph(g).unwrap().layers[0].plots[0].y, ids[1]);
    }

    #[test]
    fn with_new_columns_plots_each_column_against_its_x() {
        let (mut p, _, g, ids) = project();
        let made = p.batch_plot_columns(g, &[ids[2]], true).unwrap();
        let plot = &p.graph(made[0]).unwrap().layers[0].plots[0];
        assert_eq!((plot.x, plot.y), (Some(ids[0]), ids[2]), "C against A");
        let by_name = p.batch_plot_sheets(g, &[(p.graph_source_sheet(g).unwrap().0, 1)], MatchBy::ShortName).unwrap();
        assert_eq!(p.graph(by_name[0]).unwrap().layers[0].plots[0].y, ids[4]);
    }
}
