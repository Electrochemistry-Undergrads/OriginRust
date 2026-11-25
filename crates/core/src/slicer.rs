//! Graph > Graph Slicer (T406; Origin 2025's Graph Slicer, base slice, page scope): a graph-level filter on a
//! category column of the plots' source sheet. The values ticked are shown; every plot of the graph whose data
//! come from that sheet draws only the rows whose cell in the slicer column holds one of them. The worksheet
//! itself is not filtered (unlike Column > Filter). As the worksheet filter, the rows are worked out when the
//! slicer is applied and kept until it is applied again.
//!
//! Sheaf's choices (Origin 2025 not measured here): page scope, or one layer's scope (T444; Origin also offers plot
//! scope); one slicer column; values read as the column shows them (numbers without a trailing ".0"). Extract (T444)
//! copies the rows the slicer shows of its sheet to a new workbook.

use crate::graph::Graph;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::rowfilter::RowFilter;
use serde::{Deserialize, Serialize};

/// A graph's slicer (in [`Graph::slicer`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphSlicer {
    /// The category column the slicer reads.
    pub column: ColumnId,
    /// The values shown, as the column shows them.
    pub values: Vec<String>,
    /// For each row of the column's sheet when the slicer was applied: true when the row is shown.
    pub filter: RowFilter,
    /// Layer scope (T444): only the plots of this layer are sliced; `None` for the whole page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<usize>,
}

impl Graph {
    /// The slicer's row filter for a plot of column `y`, when the slicer reads the same sheet.
    pub fn slicer_filter_for(&self, project: &Project, y: ColumnId) -> Option<&RowFilter> {
        let s = self.slicer.as_ref()?;
        project.same_sheet(s.column, y).then_some(&s.filter)
    }

    /// [`Graph::slicer_filter_for`] for a plot of layer `layer`: none outside the slicer's layer scope.
    pub fn slicer_filter_in(&self, project: &Project, layer: usize, y: ColumnId) -> Option<&RowFilter> {
        let s = self.slicer.as_ref()?;
        if s.layer.is_some_and(|l| l != layer) {
            return None;
        }
        self.slicer_filter_for(project, y)
    }
}

impl Project {
    /// True when both columns are in the same worksheet.
    pub fn same_sheet(&self, a: ColumnId, b: ColumnId) -> bool {
        match (self.locate(a), self.locate(b)) {
            (Some((ba, sa, _)), Some((bb, sb, _))) => ba.id == bb.id && std::ptr::eq(sa, sb),
            _ => false,
        }
    }

    /// The distinct values of a slicer column in the order of their first cell (empty cells are no value).
    pub fn slicer_values(&self, column: ColumnId) -> Vec<String> {
        let Some(col) = self.column(column) else { return Vec::new() };
        let mut out: Vec<String> = Vec::new();
        for row in 0..col.data.len() {
            if let Some(t) = crate::indexed::level_text(col, row)
                && !out.contains(&t)
            {
                out.push(t);
            }
        }
        out
    }

    /// Sets graph `graph`'s slicer: the plots from `column`'s sheet show only the rows whose cell there holds
    /// one of `values`. An empty `values` list is refused (nothing would show).
    pub fn set_graph_slicer(
        &mut self,
        graph: WindowId,
        column: ColumnId,
        values: Vec<String>,
    ) -> Result<(), ProjectError> {
        let col = self.column(column).ok_or(ProjectError::NoSuchColumn(column))?;
        if values.is_empty() {
            return Err(ProjectError::SlicerNothingShown);
        }
        let keep: Vec<bool> = (0..col.data.len())
            .map(|r| crate::indexed::level_text(col, r).is_some_and(|t| values.contains(&t)))
            .collect();
        let filter = RowFilter { condition: String::new(), enabled: true, keep };
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let layer = g.slicer.as_ref().and_then(|s| s.layer);
        g.slicer = Some(GraphSlicer { column, values, filter, layer });
        Ok(())
    }

    /// Sets the scope of graph `graph`'s slicer (T444): layer `layer` (0-based) only, or the whole page.
    pub fn set_graph_slicer_layer(&mut self, graph: WindowId, layer: Option<usize>) -> Result<(), ProjectError> {
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        if layer.is_some_and(|l| l >= g.layers.len()) {
            return Err(ProjectError::NoSuchLayer(layer.unwrap_or(0)));
        }
        let s = g.slicer.as_mut().ok_or(ProjectError::NoSlicer)?;
        s.layer = layer;
        Ok(())
    }

    /// Extract (T444): a new workbook with the rows graph `graph`'s slicer shows of its sheet, every column copied
    /// with its designation, names, units and comments. Returns the new workbook.
    pub fn extract_slice(&mut self, graph: WindowId) -> Result<WindowId, ProjectError> {
        let g = self.graph(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let s = g.slicer.as_ref().ok_or(ProjectError::NoSlicer)?;
        let (book, sheet, _) = self.locate(s.column).ok_or(ProjectError::NoSuchColumn(s.column))?;
        let rows = sheet.row_count();
        let kept: Vec<usize> = (0..rows).filter(|&r| s.filter.passes(r)).collect();
        let long = format!("{} sliced", book.title_for_graph());
        let columns: Vec<crate::column::Column> = sheet.columns.clone();
        let out = self.add_workbook(&long);
        for c in columns {
            let data = crate::analysis::take_rows(&c.data, &kept, rows);
            let id = self.add_column(out, 0, c.designation, data)?;
            if let Some(n) = self.column_mut(id) {
                n.short_name = c.short_name.clone();
                n.long_name = c.long_name.clone();
                n.units = c.units.clone();
                n.comments = c.comments.clone();
            }
        }
        Ok(out)
    }

    /// Removes graph `graph`'s slicer, so its plots show every row again. False when it had none.
    pub fn clear_graph_slicer(&mut self, graph: WindowId) -> bool {
        self.graph_mut(graph).is_some_and(|g| g.slicer.take().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    #[test]
    fn a_slicer_keeps_the_rows_of_the_ticked_values_for_plots_of_its_sheet() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let groups = ["a", "b", "a", "c", "b"].iter().map(|s| s.to_string()).collect();
        let g_col = p.add_column(b, 0, Designation::Y, ColumnData::text(groups)).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
        let other = p.add_workbook("");
        let z = p.add_column(other, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        assert_eq!(p.slicer_values(g_col), ["a", "b", "c"]);
        let g = p.plot_columns(Template::Line, &[y]).unwrap();
        assert!(p.set_graph_slicer(g, g_col, Vec::new()).is_err(), "nothing would show");
        p.set_graph_slicer(g, g_col, vec!["a".into(), "c".into()]).unwrap();
        let graph = p.graph(g).unwrap();
        let f = graph.slicer_filter_for(&p, y).expect("same sheet");
        assert_eq!((0..5).map(|r| f.passes(r)).collect::<Vec<_>>(), [true, false, true, true, false]);
        assert!(graph.slicer_filter_for(&p, z).is_none(), "another sheet is not sliced");
        // The worksheet itself is not filtered.
        assert!(p.locate(y).unwrap().1.filter.is_none());
        assert!(p.clear_graph_slicer(g));
        assert!(!p.clear_graph_slicer(g));
        // Extract (T444): the shown rows of the sheet go to a new book.
        p.set_graph_slicer(g, g_col, vec!["a".into(), "c".into()]).unwrap();
        let out = p.extract_slice(g).unwrap();
        let sheet = &p.workbook(out).unwrap().sheets[0];
        assert_eq!(sheet.columns.len(), 2);
        assert_eq!(p.numeric(sheet.columns[1].id).unwrap(), [1.0, 3.0, 4.0]);
        assert_eq!(sheet.columns[0].short_name, p.column(g_col).unwrap().short_name);
        // Layer scope: a plot of another layer is not sliced.
        p.set_graph_slicer_layer(g, Some(0)).unwrap();
        let graph = p.graph(g).unwrap();
        assert!(graph.slicer_filter_in(&p, 0, y).is_some() && graph.slicer_filter_in(&p, 1, y).is_none());
        assert!(p.set_graph_slicer_layer(g, Some(5)).is_err());
        // It saves with the project.
        p.set_graph_slicer(g, g_col, vec!["b".into()]).unwrap();
        assert_eq!(p.graph(g).unwrap().slicer.as_ref().unwrap().layer, Some(0), "a new choice keeps the scope");
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.graph(g).unwrap().slicer.as_ref().unwrap().values, ["b"]);
    }
}
