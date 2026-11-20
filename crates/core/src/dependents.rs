//! Which windows depend on a window and which a window reads (Window Properties' Dependents, T220).
//!
//! A workbook's dependents are the graphs that plot any of its columns, the other workbooks whose
//! column formulas name it (`[Book1]...`), and the other workbooks that hold the results of an analysis
//! run reading it (T03 links) or cells linked to it (T211). A graph depends on the workbooks it plots.

use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, Window};
use std::collections::HashSet;

impl Project {
    /// The windows that depend on window `id`, in project order, without `id` itself.
    pub fn dependents(&self, id: WindowId) -> Vec<WindowId> {
        let Some(book) = self.workbook(id) else { return Vec::new() };
        let own: HashSet<ColumnId> = book.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id)).collect();
        let tag = format!("[{}]", book.name.to_lowercase());
        let mut out: HashSet<WindowId> = HashSet::new();
        for w in &self.windows {
            match w {
                Window::Graph(g) => {
                    if g.referenced_columns().iter().any(|c| own.contains(c)) {
                        out.insert(g.id);
                    }
                }
                Window::Workbook(b) if b.id != id => {
                    let names_it = b
                        .sheets
                        .iter()
                        .flat_map(|s| &s.columns)
                        .any(|c| c.formula.as_deref().is_some_and(|f| f.to_lowercase().contains(&tag)));
                    let links_it = b.sheets.iter().flat_map(|s| &s.cell_links).any(|l| own.contains(&l.source));
                    if names_it || links_it {
                        out.insert(b.id);
                    }
                }
                _ => {}
            }
        }
        for link in &self.analysis_links {
            if link.columns.iter().any(|c| own.contains(c)) {
                for o in &link.outputs {
                    if let Some((b, _, _)) = self.locate(*o)
                        && b.id != id
                    {
                        out.insert(b.id);
                    }
                }
            }
        }
        self.windows.iter().map(|w| w.id()).filter(|w| out.contains(w)).collect()
    }

    /// The workbooks a graph plots data of, in project order; nothing for other windows.
    pub fn plotted_books(&self, id: WindowId) -> Vec<WindowId> {
        let Some(g) = self.graph(id) else { return Vec::new() };
        let columns: HashSet<ColumnId> = g.referenced_columns().into_iter().collect();
        self.workbooks()
            .filter(|b| b.sheets.iter().flat_map(|s| &s.columns).any(|c| columns.contains(&c.id)))
            .map(|b| b.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    #[test]
    fn graphs_formulas_and_links_make_dependents() {
        let mut p = Project::new();
        let b1 = p.add_workbook("Data");
        let x = p.add_column(b1, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let y = p.add_column(b1, 0, Designation::Y, ColumnData::numeric(vec![3.0, 4.0])).unwrap();
        let _ = x;
        let g = p.plot_columns(Template::Line, &[y]).unwrap();
        let b2 = p.add_workbook("Other");
        let f = p.add_column(b2, 0, Designation::Y, ColumnData::numeric(vec![])).unwrap();
        let b3 = p.add_workbook("Linked");
        let l = p.add_column(b3, 0, Designation::Y, ColumnData::numeric(vec![])).unwrap();
        let b4 = p.add_workbook("Alone");
        let name = p.workbook(b1).unwrap().name.clone();
        p.column_mut(f).unwrap().formula = Some(format!("[{name}]Sheet1!col(B)*2"));
        p.workbook_mut(b3).unwrap().sheets[0].set_cell_link(l, 0, Some((y, 0)));

        assert_eq!(p.dependents(b1), vec![g, b2, b3]);
        assert!(p.dependents(b4).is_empty());
        assert!(p.dependents(g).is_empty(), "a graph has no dependents here");
        assert_eq!(p.plotted_books(g), vec![b1]);
        assert!(p.plotted_books(b1).is_empty());
    }
}
