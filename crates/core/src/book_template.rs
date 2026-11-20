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
//! rules are in `docs/spec/workbook-model.md` (13o).
//!
//! An analysis template (File > Save Workbook As Analysis Template..., T163) is the same, plus the
//! book's recalculating analysis runs ([`AnalysisLink`]s with Auto or Manual recalculation) whose
//! input and result columns are all in the book: a new book from it runs them again when data
//! arrives in their input columns.

use crate::analysis::OutputTarget;
use crate::analysis_link::AnalysisLink;
use crate::column::{AnalysisMark, ColumnData, FormulaState};
use crate::ids::{ColumnId, WindowId};
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
    /// The recalculating analysis runs of an analysis template (empty for a plain workbook
    /// template); their column ids are those of `book`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<AnalysisLink>,
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
            // A graph sheet's graph is not in a template (T190).
            sheet.graph_sheet = None;
            sheet.cell_graphs.clear();
            sheet.cell_links.clear();
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
        BookTemplate { name: name.to_string(), book: b, links: Vec::new() }
    }

    /// True for an analysis template: it carries analysis runs.
    pub fn is_analysis(&self) -> bool {
        !self.links.is_empty()
    }

    /// File > Save Workbook As Analysis Template...: the structure of workbook `book` (as
    /// [`BookTemplate::from_workbook`]) with the project's recalculating analysis runs that read and
    /// write only columns of that book. Their result columns keep their Auto or Manual mark (now
    /// current, as they are empty). `None` when there is no such workbook.
    pub fn analysis_from_workbook(name: &str, project: &Project, book: WindowId) -> Option<BookTemplate> {
        let source = project.workbook(book)?;
        let mut t = BookTemplate::from_workbook(name, source);
        let ids: std::collections::HashSet<ColumnId> =
            source.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id)).collect();
        let inside = |c: &ColumnId| ids.contains(c);
        t.links = project
            .analysis_links
            .iter()
            .filter(|l| {
                !l.columns.is_empty()
                    && l.columns.iter().all(inside)
                    && l.outputs.iter().all(inside)
                    && match &l.output {
                        OutputTarget::Existing(cols) => cols.iter().all(inside),
                        OutputTarget::NextToInput | OutputTarget::NewSheet => true,
                    }
            })
            .cloned()
            .collect();
        for link in &t.links {
            let mark = AnalysisMark { mode: link.mode, state: FormulaState::Current };
            for c in t.book.sheets.iter_mut().flat_map(|s| s.columns.iter_mut()) {
                if link.outputs.contains(&c.id) {
                    c.analysis = Some(mark);
                }
            }
        }
        Some(t)
    }
}

/// What [`Project::new_workbook_from_template_with_data`] made (File > Batch Processing..., T165).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchBook {
    pub book: WindowId,
    /// Columns of the file copied into the book.
    pub filled: usize,
    /// Columns of the file beyond the book's data columns, left out.
    pub ignored: usize,
    /// Analysis runs that could not run on the new data.
    pub problems: Vec<String>,
}

impl Project {
    /// File > Batch Processing... (T165): a new workbook from `template` (as
    /// [`Project::new_workbook_from_template`]) titled `title`, holding the first sheet of `data` (a
    /// data file as read by an importer). The file's columns fill, in order, the data columns of the
    /// book's first sheet: its columns that are neither the result of an analysis run nor computed
    /// by a formula. Each takes the file's values and the long name, units and comments the file
    /// gives (the template's designation and format stay); file columns beyond them are left out.
    /// Then the book's analysis runs (Auto and Manual) run on the new data; formula columns follow
    /// when the edit ends (as for any new column).
    pub fn new_workbook_from_template_with_data(
        &mut self,
        template: &BookTemplate,
        data: &Workbook,
        title: &str,
    ) -> BatchBook {
        let book = self.new_workbook_from_template(template);
        let outputs: std::collections::HashSet<ColumnId> =
            self.analysis_links.iter().flat_map(|l| l.outputs.iter().copied()).collect();
        let mut report = BatchBook { book, filled: 0, ignored: 0, problems: Vec::new() };
        let source: &[crate::column::Column] = data.sheets.first().map(|s| s.columns.as_slice()).unwrap_or(&[]);
        if let Some(b) = self.workbook_mut(book) {
            b.long_name = title.to_string();
            let targets = b.sheets.first_mut().map(|s| {
                s.columns.iter_mut().filter(|c| !outputs.contains(&c.id) && c.formula.is_none()).collect::<Vec<_>>()
            });
            let targets = targets.unwrap_or_default();
            report.ignored = source.len().saturating_sub(targets.len());
            for (col, src) in targets.into_iter().zip(source) {
                col.data = src.data.clone();
                for (mine, theirs) in [
                    (&mut col.long_name, &src.long_name),
                    (&mut col.units, &src.units),
                    (&mut col.comments, &src.comments),
                ] {
                    if !theirs.is_empty() {
                        mine.clone_from(theirs);
                    }
                }
                report.filled += 1;
            }
        }
        let ids: std::collections::HashSet<ColumnId> = self
            .workbook(book)
            .map(|b| b.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id)).collect())
            .unwrap_or_default();
        let runs: Vec<usize> = (0..self.analysis_links.len())
            .filter(|&i| self.analysis_links[i].outputs.first().is_some_and(|c| ids.contains(c)))
            .collect();
        for i in runs {
            if let Err(e) = self.recalculate_analysis(i) {
                report.problems.push(e.to_string());
            }
        }
        report
    }

    /// File > New > From Template with a workbook template: adds an empty workbook built like the
    /// template, with fresh window and column ids and a free `Book<n>` name. Returns its id.
    ///
    /// The analysis runs of an analysis template join the project with the new column ids, so data
    /// put into their input columns runs them (Auto) or marks them pending (Manual). A run that
    /// names a column the template does not hold is left out.
    pub fn new_workbook_from_template(&mut self, template: &BookTemplate) -> WindowId {
        let mut book = template.book.clone();
        book.active_sheet = book.active_sheet.min(book.sheets.len().saturating_sub(1));
        let (id, map) = self.adopt_workbook_mapped(book);
        let remap = |cols: &[ColumnId]| cols.iter().map(|c| map.get(c).copied()).collect::<Option<Vec<_>>>();
        for link in &template.links {
            let (Some(columns), Some(outputs)) = (remap(&link.columns), remap(&link.outputs)) else { continue };
            let output = match &link.output {
                OutputTarget::Existing(cols) => match remap(cols) {
                    Some(cols) => OutputTarget::Existing(cols),
                    None => continue,
                },
                other => other.clone(),
            };
            self.analysis_links.push(AnalysisLink { columns, outputs, output, ..link.clone() });
        }
        id
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

    fn linked_book() -> (Project, WindowId, ColumnId) {
        use crate::analysis::{Request, RowRange, Tool};
        use crate::column::RecalcMode;
        use sheaf_calc::{Boundary, SmoothMethod, SmoothOptions};
        let mut p = Project::new();
        let book = p.add_workbook("Assay");
        let x = (0..10).map(f64::from).collect();
        let y = (0..10).map(|i| f64::from(i % 3)).collect();
        p.add_column(book, 0, Designation::X, ColumnData::numeric(x)).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(y)).unwrap();
        let o = SmoothOptions {
            method: SmoothMethod::AdjacentAveraging { weighted: false },
            npts: 3,
            boundary: Boundary::None,
        };
        let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
        p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
        (p, book, y)
    }

    #[test]
    fn an_analysis_template_keeps_the_books_linked_runs_and_a_new_book_runs_them_on_new_data() {
        let (mut p, book, _) = linked_book();
        assert_eq!(p.analysis_links.len(), 1);
        let t = BookTemplate::analysis_from_workbook("Assay", &p, book).unwrap();
        assert!(t.is_analysis());
        let cols = &t.book.sheets[0].columns;
        assert_eq!(cols.len(), 3, "X, Y and the smoothed result");
        assert!(cols.iter().all(|c| c.is_empty()));
        assert!(cols[2].analysis.is_some(), "the result column keeps its mark");
        assert!(BookTemplate::from_workbook("Plain", p.workbook(book).unwrap()).links.is_empty());

        let made = p.new_workbook_from_template(&t);
        assert_eq!(p.analysis_links.len(), 2);
        let (nx, ny, out) = {
            let c = &p.workbook(made).unwrap().sheets[0].columns;
            (c[0].id, c[1].id, c[2].id)
        };
        assert_eq!(p.analysis_links[1].columns, [ny]);
        assert_eq!(p.analysis_links[1].outputs, [out]);

        // New data in the input: the Auto run fills the result column in place.
        let before = p.clone();
        p.column_mut(nx).unwrap().data = ColumnData::numeric(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        p.column_mut(ny).unwrap().data = ColumnData::numeric(vec![0.0, 3.0, 0.0, 3.0, 0.0]);
        let problems = p.update_analysis_links(&before);
        assert!(problems.is_empty(), "{problems:?}");
        let result = p.column(out).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(p.workbook(made).unwrap().sheets[0].columns.len(), 3, "written in place, no new column");
        // The first book's run is untouched.
        assert_eq!(p.workbook(book).unwrap().sheets[0].columns[2].len(), 10);
    }

    #[test]
    fn runs_that_reach_outside_the_book_are_left_out() {
        let (mut p, book, y) = linked_book();
        let other = p.add_workbook("Elsewhere");
        let foreign = p.add_column(other, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        p.analysis_links[0].outputs.push(foreign);
        let t = BookTemplate::analysis_from_workbook("Assay", &p, book).unwrap();
        assert!(t.links.is_empty());
        assert!(!t.is_analysis());
        let _ = y;
        assert!(BookTemplate::analysis_from_workbook("x", &p, WindowId(999)).is_none());
    }
    #[test]
    fn batch_data_fills_the_data_columns_and_runs_the_analysis() {
        let (mut p, book, _) = linked_book();
        let t = BookTemplate::analysis_from_workbook("Assay", &p, book).unwrap();
        // A data file with three columns: two fill X and Y; the third has no data column to go to.
        let mut file = Workbook::new(WindowId(0), "f");
        let mut s = Worksheet::new("f");
        for (k, v) in [vec![1.0, 2.0, 3.0, 4.0], vec![0.0, 3.0, 0.0, 3.0], vec![9.0; 4]].into_iter().enumerate() {
            let mut c = crate::column::Column::new(
                ColumnId(k as u64 + 1),
                crate::column::default_short_name(k),
                Designation::Y,
                ColumnData::numeric(v),
            );
            c.long_name = format!("file {k}");
            s.columns.push(c);
        }
        file.sheets = vec![s];
        let r = p.new_workbook_from_template_with_data(&t, &file, "run1.csv");
        assert_eq!((r.filled, r.ignored), (2, 1));
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        let b = p.workbook(r.book).unwrap();
        assert_eq!(b.long_name, "run1.csv");
        let cols = &b.sheets[0].columns;
        assert_eq!(cols.len(), 3, "no column added for the file's third");
        assert_eq!(cols[0].designation, Designation::X, "the template's designation stays");
        assert_eq!(cols[1].long_name, "file 1");
        assert_eq!(cols[1].numeric().unwrap(), [0.0, 3.0, 0.0, 3.0]);
        assert_eq!(cols[2].len(), 4, "the smoothing ran on the new data");
    }
}
