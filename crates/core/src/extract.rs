//! Taking one window out of a project, for File > Save Window As. The result is a small project
//! that holds the window (and, for a graph, the workbooks it reads) and nothing that points outside
//! it. Opening the file appends that project ([`Project::append_windows`]).

use crate::column::{FormulaState, RecalcMode};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError, Window};
use sheaf_expr::{ColumnRef, Expr};
use std::cell::RefCell;
use std::collections::HashSet;

/// One window taken out of a project.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowExtract {
    /// The window and, for a graph, the workbooks it reads, in the order of the source project, with
    /// their ids unchanged. No folders, window places or analysis links.
    pub project: Project,
    /// The position of the asked-for window in `project.windows`.
    pub primary: usize,
    /// What was dropped because it pointed outside the window; shown to the owner as notes.
    pub notes: Vec<String>,
}

impl Project {
    /// The window `id` as a small project of its own. A workbook is taken alone; a graph is taken
    /// with every workbook that holds a column it reads (each once), so that the file plots by
    /// itself. Then, so that nothing dangles:
    ///
    /// - a formula that names another workbook (`[Book2]Sheet1!col(A)`) that is not in the result is
    ///   removed from its column, the values stay;
    /// - analysis links are not kept (as File > Append does not keep them): the marks on the result
    ///   columns are cleared, the values stay, and the number of links that touched these columns is
    ///   noted.
    ///
    /// Folders are flattened (the result has none) and the window places are not kept.
    pub fn extract_window(&self, id: WindowId) -> Result<WindowExtract, ProjectError> {
        let window = self.window(id).ok_or(ProjectError::NoSuchWindow(id))?;
        let mut wanted: HashSet<WindowId> = HashSet::from([id]);
        if let Window::Graph(graph) = window {
            let used = RefCell::new(Vec::new());
            graph.clone().remap_columns(&|c| {
                used.borrow_mut().push(c);
                c
            });
            for column in used.into_inner() {
                if let Some((book, _, _)) = self.locate(column) {
                    wanted.insert(book.id);
                }
            }
        }
        let mut out = Project::new();
        let mut primary = 0;
        for w in self.windows.iter().filter(|w| wanted.contains(&w.id())) {
            if w.id() == id {
                primary = out.windows.len();
            }
            out.windows.push(w.clone());
        }
        let mut notes = Vec::new();
        drop_outside_formulas(&mut out, &mut notes);
        let kept: HashSet<ColumnId> =
            out.workbooks().flat_map(|b| b.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id))).collect();
        let links =
            self.analysis_links.iter().filter(|l| l.columns.iter().chain(&l.outputs).any(|c| kept.contains(c))).count();
        for col in out.windows.iter_mut().filter_map(|w| match w {
            Window::Workbook(b) => Some(b),
            Window::Graph(_) => None,
        }) {
            for c in col.sheets.iter_mut().flat_map(|s| &mut s.columns) {
                c.analysis = None;
            }
        }
        if links > 0 {
            notes
                .push(format!("{links} analysis result(s) no longer recalculate in this file; their values are kept."));
        }
        out.repair_ids();
        Ok(WindowExtract { project: out, primary, notes })
    }
}

/// Removes the formulas of `project` that read a workbook that is not in it.
fn drop_outside_formulas(project: &mut Project, notes: &mut Vec<String>) {
    let names: Vec<(String, String)> = project.workbooks().map(|b| (b.name.clone(), b.long_name.clone())).collect();
    let inside = |name: &str| {
        names.iter().any(|(short, long)| {
            short.eq_ignore_ascii_case(name) || (!long.is_empty() && long.eq_ignore_ascii_case(name))
        })
    };
    for window in &mut project.windows {
        let Window::Workbook(book) = window else { continue };
        let book_name = book.name.clone();
        for col in book.sheets.iter_mut().flat_map(|s| &mut s.columns) {
            let Some(formula) = &col.formula else { continue };
            let Ok(expr) = Expr::parse(formula) else { continue };
            let outside = expr.columns().into_iter().find_map(|r| match r {
                ColumnRef::Scoped { book: Some(name), .. } if !inside(&name) => Some(name),
                _ => None,
            });
            let Some(outside) = outside else { continue };
            notes.push(format!(
                "{book_name}: the formula of column {} reads [{outside}], which is not in this file; the formula was removed and the values kept.",
                col.display_name()
            ));
            col.formula = None;
            col.recalc = RecalcMode::None;
            col.formula_state = FormulaState::Current;
            col.formula_rows = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{OutputTarget, Request, RowRange, Tool};
    use crate::template::Template;
    use crate::{ColumnData, Designation};
    use sheaf_calc::{Boundary, SmoothMethod, SmoothOptions};

    fn book(p: &mut Project, long: &str, values: &[f64]) -> (WindowId, ColumnId, ColumnId) {
        let b = p.add_workbook(long);
        let x = p
            .add_column(b, 0, Designation::X, ColumnData::numeric((0..values.len()).map(|i| i as f64).collect()))
            .unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(values.to_vec())).unwrap();
        (b, x, y)
    }

    #[test]
    fn a_workbook_is_taken_alone() {
        let mut p = Project::new();
        let (b1, _, y1) = book(&mut p, "one", &[1.0, 2.0]);
        let (_b2, _, _) = book(&mut p, "two", &[3.0, 4.0]);
        let e = p.extract_window(b1).unwrap();
        assert_eq!(e.project.windows.len(), 1);
        assert_eq!((e.primary, e.notes.len()), (0, 0));
        assert_eq!(e.project.numeric(y1).unwrap(), [1.0, 2.0]);
        assert!(e.project.root.folders.is_empty() && e.project.root.windows.is_empty());
    }

    #[test]
    fn a_graph_comes_with_each_book_it_reads_once() {
        let mut p = Project::new();
        let (_a, _, ya) = book(&mut p, "a", &[1.0, 2.0]);
        let (_unused, _, _) = book(&mut p, "unused", &[0.0, 0.0]);
        let (_c, _, yc) = book(&mut p, "c", &[5.0, 6.0]);
        let ya2 = p.add_column(_a, 0, Designation::Y, ColumnData::numeric(vec![7.0, 8.0])).unwrap();
        let g = p.plot_columns(Template::Line, &[ya, ya2, yc]).unwrap();
        let e = p.extract_window(g).unwrap();
        let names: Vec<&str> = e.project.windows.iter().map(|w| w.name()).collect();
        assert_eq!(names.len(), 3, "the graph and two books: {names:?}");
        assert!(matches!(e.project.windows[e.primary], Window::Graph(_)));
        assert!(e.project.window_by_name("Book2").is_none(), "the book the graph does not read stays out");
    }

    #[test]
    fn a_formula_that_reads_a_book_outside_is_dropped_with_a_note_and_keeps_its_values() {
        let mut p = Project::new();
        let (a, _, _) = book(&mut p, "", &[1.0, 2.0]);
        let (_b, _, _) = book(&mut p, "", &[10.0, 20.0]);
        let c = p.add_column(a, 0, Designation::Y, ColumnData::numeric(vec![0.0, 0.0])).unwrap();
        p.set_column_values_with(c, "[Book2]Sheet1!col(B) + col(B)", None, RecalcMode::Auto).unwrap();
        let before = p.numeric(c).unwrap().to_vec();
        assert_eq!(before, [11.0, 22.0]);
        let e = p.extract_window(a).unwrap();
        let col = e.project.column(c).unwrap();
        assert_eq!(col.formula, None);
        assert_eq!(
            (col.recalc, col.formula_state, col.formula_rows.clone()),
            (RecalcMode::None, FormulaState::Current, None)
        );
        assert_eq!(e.project.numeric(c).unwrap(), before);
        assert_eq!(e.notes.len(), 1);
        assert!(e.notes[0].contains("Book2") && e.notes[0].contains("Book1"), "{:?}", e.notes);
    }

    #[test]
    fn a_formula_that_reads_a_book_inside_or_only_its_own_sheet_is_kept() {
        let mut p = Project::new();
        let (a, _, ya) = book(&mut p, "", &[1.0, 2.0]);
        let (_b, _, yb) = book(&mut p, "", &[10.0, 20.0]);
        let own = p.add_column(a, 0, Designation::Y, ColumnData::numeric(vec![0.0; 2])).unwrap();
        p.set_column_values(own, "col(B) * 2 + [book1]Sheet1!col(B)", None).unwrap();
        let other = p.add_column(_b, 0, Designation::Y, ColumnData::numeric(vec![0.0; 2])).unwrap();
        p.set_column_values(other, "[Book1]Sheet1!col(B) + col(B)", None).unwrap();
        // A graph that reads both books keeps both formulas.
        let g = p.plot_columns(Template::Line, &[ya, yb]).unwrap();
        let both = p.extract_window(g).unwrap();
        assert!(both.notes.is_empty(), "{:?}", both.notes);
        assert!(both.project.column(own).unwrap().formula.is_some());
        assert!(both.project.column(other).unwrap().formula.is_some());
        // Book 1 alone keeps the formula that only reads itself.
        let alone = p.extract_window(a).unwrap();
        assert!(alone.notes.is_empty(), "{:?}", alone.notes);
        assert!(alone.project.column(own).unwrap().formula.is_some());
    }

    #[test]
    fn analysis_marks_and_links_are_dropped_with_a_note_and_the_values_stay() {
        let mut p = Project::new();
        let (b, _, y) = book(&mut p, "", &(0..20).map(|i| f64::from(i % 4)).collect::<Vec<_>>());
        let o = SmoothOptions {
            method: SmoothMethod::AdjacentAveraging { weighted: false },
            npts: 3,
            boundary: Boundary::None,
        };
        let request = Request { columns: vec![y], rows: RowRange::All, tool: Tool::Smooth(o) };
        let report = p.analyze_linked(&request, &OutputTarget::NextToInput, RecalcMode::Auto).unwrap();
        let out = report.new_columns[0];
        assert!(p.column(out).unwrap().analysis.is_some() && p.analysis_links.len() == 1);
        let e = p.extract_window(b).unwrap();
        assert!(e.project.analysis_links.is_empty());
        assert!(e.project.column(out).unwrap().analysis.is_none());
        assert_eq!(e.project.numeric(out), p.numeric(out));
        assert_eq!(e.notes.len(), 1);
        assert!(e.notes[0].contains("1 analysis result"), "{:?}", e.notes);
    }

    #[test]
    fn a_missing_window_is_an_error() {
        let p = Project::new();
        assert_eq!(p.extract_window(WindowId(9)).unwrap_err(), ProjectError::NoSuchWindow(WindowId(9)));
    }
}
