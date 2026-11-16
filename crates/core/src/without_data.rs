//! A copy of a project with the data taken out (File > Save Project without Data... and
//! File > Duplicate Project without Data).
//!
//! The windows, names, column labels, units, designations, number formats, formulas and every
//! graph stay; the values go. What Origin's own items clear is undocumented for 2016, so this
//! follows the documented "clear all data, keep windows and operations" mode of Origin's
//! Clone Project (UNMEASURED Sheaf choice, see `docs/spec/project-file-format.md`).

use crate::column::{ColumnData, FormulaState};
use crate::project::{Project, Window};

impl Project {
    /// A copy without any cell value. The open project is not touched.
    ///
    /// Kept: windows and their places, names, column labels and designations, formulas, graphs
    /// (they plot the same, now empty, columns). Removed: every value (a column keeps its
    /// storage kind), masks, cell notes, cell formats, conditional formats, the row filter,
    /// analysis marks and links (they would recalculate data back), and the comparison source
    /// of a report sheet.
    pub fn without_data(&self) -> Project {
        let mut copy = self.clone();
        for w in &mut copy.windows {
            let Window::Workbook(book) = w else { continue };
            for sheet in &mut book.sheets {
                sheet.filter = None;
                sheet.cond_formats.clear();
                sheet.notes.clear();
                sheet.cell_formats.clear();
                sheet.comparison_source = None;
                for c in &mut sheet.columns {
                    c.data = match &c.data {
                        ColumnData::Numeric(_) => ColumnData::numeric(Vec::new()),
                        ColumnData::Text(_) => ColumnData::text(Vec::new()),
                        ColumnData::Mixed { .. } => ColumnData::mixed_numbers(Vec::new()),
                    };
                    c.mask.clear();
                    c.analysis = None;
                    c.formula_rows = None;
                    if c.formula_state == FormulaState::Pending {
                        c.formula_state = FormulaState::Current;
                    }
                }
            }
        }
        copy.analysis_links.clear();
        copy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;
    use crate::template::Template;

    fn sample() -> (Project, crate::ids::WindowId, crate::ids::ColumnId) {
        let mut p = Project::new();
        let book = p.add_workbook("Data");
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![4.0, 5.0, 6.0])).unwrap();
        p.column_mut(y).unwrap().long_name = "Signal".into();
        p.column_mut(y).unwrap().units = "mV".into();
        p.column_mut(y).unwrap().mask.insert(1);
        p.column_mut(y).unwrap().formula = Some("col(A)*2".into());
        let t = p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into(), "b".into()])).unwrap();
        let _ = (x, t);
        p.add_graph(Template::Line);
        (p, book, y)
    }

    #[test]
    fn values_go_and_structure_stays() {
        let (p, book, y) = sample();
        let copy = p.without_data();
        let b = copy.workbook(book).unwrap();
        assert_eq!(b.sheets[0].columns.len(), 3);
        assert!(b.sheets[0].columns.iter().all(|c| c.is_empty()));
        let c = copy.column(y).unwrap();
        assert_eq!((c.long_name.as_str(), c.units.as_str()), ("Signal", "mV"));
        assert_eq!(c.formula.as_deref(), Some("col(A)*2"));
        assert!(c.mask.is_empty());
        assert!(matches!(b.sheets[0].columns[2].data, ColumnData::Text(_)));
        assert_eq!(copy.windows.len(), p.windows.len());
    }

    #[test]
    fn the_original_is_untouched() {
        let (p, _, y) = sample();
        let before = p.clone();
        let _ = p.without_data();
        assert_eq!(p, before);
        assert_eq!(p.column(y).unwrap().len(), 3);
    }
}
