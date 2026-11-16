//! The worksheet filter (Column > Filter): a condition in the column-formula language that hides the
//! rows for which it does not hold, without deleting them. Hidden rows stay in the data; the grid
//! does not show them and analysis and fits leave them out. As in Origin the result is kept until
//! the filter is applied again (Reapply Filter), so editing a cell does not change the rows shown.

use crate::analysis::AnalysisError;
use crate::column::Column;
use crate::graph::Plot;
use crate::ids::{ColumnId, WindowId};
use crate::project::Project;
use crate::worksheet::Worksheet;
use serde::{Deserialize, Serialize};
use sheaf_expr::Expr;
use std::borrow::Cow;

/// A filter on the rows of a worksheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RowFilter {
    pub condition: String,
    /// False while the filter is switched off (Enable/Disable Filter): all rows show.
    pub enabled: bool,
    /// For each row of the sheet when the filter was applied: true when it passed. Rows beyond
    /// the end pass.
    pub keep: Vec<bool>,
}

impl RowFilter {
    pub fn passes(&self, row: usize) -> bool {
        !self.enabled || self.keep.get(row).copied().unwrap_or(true)
    }
}

impl Worksheet {
    /// Whether the filter lets this row show (true without a filter).
    pub fn row_shown(&self, row: usize) -> bool {
        self.filter.as_ref().is_none_or(|f| f.passes(row))
    }

    /// The numbers of a column as analysis reads them: masked cells and rows hidden by the filter
    /// are missing values. Borrowed when nothing is left out.
    pub fn values_of<'a>(&self, c: &'a Column) -> Option<Cow<'a, [f64]>> {
        let values = c.numeric_unmasked()?;
        let Some(f) = self.filter.as_ref().filter(|f| f.enabled && f.keep.iter().any(|k| !k)) else {
            return Some(values);
        };
        let mut owned = values.into_owned();
        for (i, v) in owned.iter_mut().enumerate() {
            if !f.passes(i) {
                *v = f64::NAN;
            }
        }
        Some(Cow::Owned(owned))
    }

    /// The rows the grid shows among the first `rows`: all of them without an active filter.
    pub fn shown_rows(&self, rows: usize) -> Option<Vec<usize>> {
        let f = self.filter.as_ref().filter(|f| f.enabled && f.keep.iter().any(|k| !k))?;
        Some((0..rows).filter(|r| f.passes(*r)).collect())
    }
}

impl Project {
    /// Active worksheet filters on a plot's source columns. A row must pass all of them;
    /// repeated references to the same sheet share one filter. Masks are deliberately separate:
    /// the graph still draws a visible masked point in red.
    pub fn plot_row_filters(&self, plot: &Plot) -> Vec<&RowFilter> {
        let mut filters: Vec<&RowFilter> = Vec::new();
        for id in [Some(plot.y), plot.x, plot.y_error, plot.x_error, plot.y_base, plot.size_from, plot.color_from]
            .into_iter()
            .flatten()
        {
            if let Some(filter) = self.locate(id).and_then(|(_, s, _)| s.filter.as_ref())
                && filter.enabled
                && filter.keep.iter().any(|keep| !keep)
                && !filters.iter().any(|f| std::ptr::eq(*f, filter))
            {
                filters.push(filter);
            }
        }
        filters
    }

    /// The numbers of a Y column that a plot of it shows (before any row range of the plot): rows passing the
    /// sheet's active filter, and not masked while Hide Masked Points is on. Empty for a column without numbers.
    pub fn shown_values(&self, y: ColumnId) -> Vec<f64> {
        let Some(values) = self.numeric(y) else { return Vec::new() };
        let filter = self
            .locate(y)
            .and_then(|(_, s, _)| s.filter.as_ref())
            .filter(|f| f.enabled && f.keep.iter().any(|keep| !keep));
        let mask = self.column(y).and_then(|c| c.active_mask()).filter(|_| self.hide_masked);
        values
            .iter()
            .enumerate()
            .filter(|(r, _)| filter.is_none_or(|f| f.passes(*r)) && !mask.is_some_and(|m| m.contains(r)))
            .map(|(_, v)| *v)
            .collect()
    }

    /// Sets the filter of a sheet to a condition and applies it. Returns the number of rows that
    /// pass. An existing filter keeps its enabled state turned on.
    pub fn set_row_filter(&mut self, book: WindowId, sheet: usize, condition: &str) -> Result<usize, AnalysisError> {
        let expr = Expr::parse(condition).map_err(|e| AnalysisError::Condition(e.to_string()))?;
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        let n = ws.row_count();
        let values =
            crate::formula::eval_on_sheet(&expr, ws, n).map_err(|e| AnalysisError::Condition(e.to_string()))?;
        let keep: Vec<bool> = values.iter().map(|v| *v != 0.0 && !v.is_nan()).collect();
        let kept = keep.iter().filter(|k| **k).count();
        let ws = &mut self.workbook_mut(book).expect("checked above").sheets[sheet];
        ws.filter = Some(RowFilter { condition: condition.trim().to_string(), enabled: true, keep });
        Ok(kept)
    }

    /// Applies the sheet's filter condition again (after the data changed). Without a filter
    /// nothing happens.
    pub fn reapply_row_filter(&mut self, book: WindowId, sheet: usize) -> Result<(), AnalysisError> {
        let Some(f) = self.workbook(book).and_then(|b| b.sheets.get(sheet)).and_then(|s| s.filter.clone()) else {
            return Ok(());
        };
        self.set_row_filter(book, sheet, &f.condition)?;
        if !f.enabled
            && let Some(nf) = &mut self.workbook_mut(book).expect("exists").sheets[sheet].filter
        {
            nf.enabled = false;
        }
        Ok(())
    }

    /// Removes the filter of a sheet; all rows show again.
    pub fn remove_row_filter(&mut self, book: WindowId, sheet: usize) {
        if let Some(ws) = self.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
            ws.filter = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn a_filter_hides_rows_without_deleting_them_and_analysis_skips_them() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 5.0, 2.0, 8.0])).unwrap();
        assert_eq!(p.set_row_filter(book, 0, "col(A) > 1.5").unwrap(), 3);
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert_eq!(ws.shown_rows(6), Some(vec![1, 2, 3, 4, 5]), "rows below the data show");
        assert_eq!(ws.column(a).unwrap().len(), 4, "nothing was deleted");
        let v = ws.values_of(ws.column(a).unwrap()).unwrap();
        assert!(v[0].is_nan() && v[1] == 5.0);
        // Disabled: every row shows again; reapply keeps it disabled.
        p.workbook_mut(book).unwrap().sheets[0].filter.as_mut().unwrap().enabled = false;
        p.reapply_row_filter(book, 0).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[0];
        assert!(ws.shown_rows(4).is_none() && !ws.filter.as_ref().unwrap().enabled);
        p.remove_row_filter(book, 0);
        assert!(p.workbook(book).unwrap().sheets[0].filter.is_none());
        assert!(p.set_row_filter(book, 0, "col(A) >").is_err());
    }

    #[test]
    fn the_above_average_condition_uses_mean() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 10.0])).unwrap();
        assert_eq!(p.set_row_filter(book, 0, "col(A) > mean(col(A))").unwrap(), 1);
    }

    #[test]
    fn plot_filters_intersect_source_sheets_and_deduplicate_shared_sources() {
        let mut p = Project::new();
        let a = p.add_workbook("first");
        let y = p.add_column(a, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let err = p.add_column(a, 0, Designation::YError, ColumnData::numeric(vec![0.1; 3])).unwrap();
        let b = p.add_workbook("second");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let g = p.add_graph(crate::template::Template::Line);
        p.add_plot(g, 0, y).unwrap();
        let plot = &mut p.graph_mut(g).unwrap().layers[0].plots[0];
        plot.x = Some(x);
        plot.y_error = Some(err);
        p.set_row_filter(a, 0, "col(A) > 1").unwrap();
        p.set_row_filter(b, 0, "col(A) < 3").unwrap();
        let plot = p.graph(g).unwrap().layers[0].plots[0].clone();
        let filters = p.plot_row_filters(&plot);
        assert_eq!(filters.len(), 2, "Y and its error share one sheet filter");
        assert_eq!((0..4).map(|r| filters.iter().all(|f| f.passes(r))).collect::<Vec<_>>(), [false, true, false, true]);
        p.workbook_mut(b).unwrap().sheet_mut().filter.as_mut().unwrap().enabled = false;
        assert_eq!(p.plot_row_filters(&plot).len(), 1);
        p.column_mut(y).unwrap().mask.insert(1);
        assert!(p.plot_row_filters(&plot).iter().all(|f| f.passes(1)), "masks stay separate");
        p.remove_row_filter(a, 0);
        assert!(p.plot_row_filters(&plot).is_empty());
    }
}
