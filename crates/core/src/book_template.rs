//! User workbook templates (File > Save Template As... on a workbook, File > New > From Template):
//! the structure and look of a workbook without its data (T160).
//!
//! A [`BookTemplate`] is a workbook whose columns hold no values. It keeps the sheets (names, long
//! names, comments, tab label, view switches, frozen panes, category fonts), and per column the
//! short and long names, units, comments, user parameter rows, designation, number format, width,
//! hidden and categorical flags, sampling interval and Set Column Values formula with its recalc
//! mode and rows; per sheet the row filter, conditional formats and cell formats stay. The values,
//! masks, cell notes, Set As Begin/End ranges, analysis marks and the import source go.
//! [`Project::new_workbook_from_template`] adds a copy with fresh ids and a free `Book<n>` name. The
//! rules are in `docs/spec/workbook.md` (section "User workbook templates").

use crate::column::{ColumnData, FormulaState};
use crate::ids::WindowId;
use crate::project::Project;
use crate::worksheet::Workbook;
use serde::{Deserialize, Serialize};

/// A named workbook structure without data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BookTemplate {
    /// The name the template is listed under.
    pub name: String,
    /// The workbook with its values removed (see the module docs). Its id and short name mean nothing.
    pub book: Workbook,
}

impl BookTemplate {
    /// Takes the structure of `book`: everything but the data (see the module docs). The book's
    /// long name stays (a new book from the template shows it as its title).
    pub fn from_workbook(name: &str, book: &Workbook) -> BookTemplate {
        let mut b = book.clone();
        b.id = WindowId(0);
        b.name = name.to_string();
        b.source = None;
        for sheet in &mut b.sheets {
            sheet.notes.clear();
            sheet.comparison_source = None;
            for c in &mut sheet.columns {
                c.data = match &c.data {
                    ColumnData::Numeric(_) => ColumnData::numeric(Vec::new()),
                    ColumnData::Text(_) => ColumnData::text(Vec::new()),
                    ColumnData::Mixed { .. } => ColumnData::mixed_numbers(Vec::new()),
                };
                c.mask.clear();
                c.mask_disabled = false;
                c.analysis = None;
                c.fit_result = false;
                c.data_range = None;
                if c.formula_state == FormulaState::Pending {
                    c.formula_state = FormulaState::Current;
                }
            }
        }
        BookTemplate { name: name.to_string(), book: b }
    }
}

impl Project {
    /// File > New > From Template with a workbook template: adds an empty workbook built like the
    /// template, with fresh window and column ids and a free `Book<n>` name. Returns its id.
    pub fn new_workbook_from_template(&mut self, template: &BookTemplate) -> WindowId {
        let mut book = template.book.clone();
        book.active_sheet = book.active_sheet.min(book.sheets.len().saturating_sub(1));
        self.adopt_workbook(book)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Designation;
    use crate::worksheet::Worksheet;

    fn sample() -> (Project, WindowId) {
        let mut p = Project::new();
        let book = p.add_workbook("Run log");
        let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![4.0, 5.0, 6.0])).unwrap();
        p.add_column(book, 0, Designation::Label, ColumnData::text(vec!["a".into()])).unwrap();
        let c = p.column_mut(y).unwrap();
        c.long_name = "Signal".into();
        c.units = "mV".into();
        c.comments = "raw".into();
        c.width = Some(14);
        c.mask.insert(1);
        c.formula = Some("col(A)*2".into());
        c.params.insert("Gain".into(), "10".into());
        p.column_mut(x).unwrap().data_range = Some(crate::data_range::DataRange { begin: 1, end: None });
        let b = p.workbook_mut(book).unwrap();
        b.sheets[0].name = "Raw".into();
        b.sheets[0].frozen = crate::worksheet::Frozen { rows: 1, columns: 0 };
        b.sheets.push(Worksheet::new("Summary"));
        b.source = Some(crate::worksheet::ImportSource { path: "x.csv".into(), importer: "ascii".into() });
        (p, book)
    }

    #[test]
    fn the_template_keeps_structure_and_drops_values() {
        let (p, book) = sample();
        let t = BookTemplate::from_workbook("Log", p.workbook(book).unwrap());
        assert_eq!(t.name, "Log");
        assert_eq!(t.book.long_name, "Run log");
        assert!(t.book.source.is_none());
        let names: Vec<_> = t.book.sheets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Raw", "Summary"]);
        assert_eq!(t.book.sheets[0].frozen.rows, 1);
        let cols = &t.book.sheets[0].columns;
        assert_eq!(cols.len(), 3);
        assert!(cols.iter().all(|c| c.is_empty()), "no values");
        let y = &cols[1];
        assert_eq!((y.long_name.as_str(), y.units.as_str(), y.comments.as_str()), ("Signal", "mV", "raw"));
        assert_eq!(y.formula.as_deref(), Some("col(A)*2"));
        assert_eq!(y.width, Some(14));
        assert_eq!(y.params.get("Gain").map(String::as_str), Some("10"));
        assert!(y.mask.is_empty());
        assert!(cols[0].data_range.is_none());
        assert!(matches!(cols[2].data, ColumnData::Text(_)), "a text column stays text");
        assert_eq!(
            cols.iter().map(|c| c.designation).collect::<Vec<_>>(),
            [Designation::X, Designation::Y, Designation::Label]
        );
    }

    #[test]
    fn a_new_book_from_the_template_gets_fresh_ids_and_a_free_name() {
        let (mut p, book) = sample();
        let t = BookTemplate::from_workbook("Log", p.workbook(book).unwrap());
        let made = p.new_workbook_from_template(&t);
        let made_again = p.new_workbook_from_template(&t);
        assert_ne!(made, made_again);
        let (a, b) = (p.workbook(made).unwrap(), p.workbook(made_again).unwrap());
        assert_eq!((a.name.as_str(), b.name.as_str()), ("Book2", "Book3"));
        let mut ids: Vec<_> =
            p.workbooks().flat_map(|b| b.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id))).collect();
        let n = ids.len();
        ids.sort_by_key(|c| c.0);
        ids.dedup();
        assert_eq!(ids.len(), n, "column ids stay unique");
        assert_eq!(a.sheets.len(), 2);
        assert!(a.sheets[0].columns.iter().all(|c| c.is_empty()));
        // The source book still has its values.
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns[0].len(), 3);
    }

    #[test]
    fn the_template_survives_serde() {
        let (p, book) = sample();
        let t = BookTemplate::from_workbook("Log", p.workbook(book).unwrap());
        let json = serde_json::to_string(&t).unwrap();
        let back: BookTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.book.sheets.len(), 2);
        assert_eq!(back.book.sheets[0].columns[1].long_name, "Signal");
    }
}
