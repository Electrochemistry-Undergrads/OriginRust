//! Edits made from a graph's Data menu (Mask Data Points, Remove Bad Data Points, Move Data
//! Points): they change cells of the worksheet a plot comes from, each as one undo step.
//!
//! The rules (graphing spec, "Data menu tools"): a mask is not a typed value, so it is allowed on
//! any column, also one that a formula keeps up to date. Setting a cell missing or to a new number
//! follows the worksheet's rule for typing: a column calculated by a formula that keeps running
//! ([`Column::is_locked`]) is left alone and counted in [`DataEditReport::locked`]. Rows are never
//! deleted. Recalculating columns and analysis links follow inside the same undo step, as for any
//! other edit.

use crate::column::{Column, ColumnData};
use crate::history::Document;
use crate::ids::ColumnId;
use crate::project::Project;
use std::collections::BTreeSet;

/// One cell of a worksheet column: the point a plot draws at `row` of its Y (or X) column.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DataCell {
    pub column: ColumnId,
    /// 0-based row.
    pub row: usize,
}

impl DataCell {
    pub fn new(column: ColumnId, row: usize) -> Self {
        DataCell { column, row }
    }
}

/// What a data edit did.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DataEditReport {
    /// The number of cells whose mask or value changed. 0 means nothing was done and no undo step
    /// was made.
    pub changed: usize,
    /// The number of columns left alone because a formula keeps them up to date.
    pub locked: usize,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DataEditError {
    #[error("there is no column with id {0:?}")]
    NoSuchColumn(ColumnId),
    #[error("column {column} has no row {row}", row = .row + 1)]
    NoSuchCell { column: String, row: usize },
    #[error("column {0} holds text, not numbers")]
    NotNumeric(String),
    #[error("a point cannot be moved to a value that is not finite")]
    NotFinite,
}

impl Project {
    /// Masks (`masked`) or unmasks the cells. A row beyond the data of its column is skipped, and a
    /// column that does not exist is an error (nothing is changed). Returns how many cells changed.
    pub fn mask_data_cells(&mut self, cells: &[DataCell], masked: bool) -> Result<usize, DataEditError> {
        let cells: BTreeSet<DataCell> = cells.iter().copied().collect();
        for c in &cells {
            self.column(c.column).ok_or(DataEditError::NoSuchColumn(c.column))?;
        }
        let mut changed = 0;
        for cell in cells {
            let Some(c) = self.column_mut(cell.column) else { continue };
            if cell.row >= c.len() || c.mask.contains(&cell.row) == masked {
                continue;
            }
            if masked {
                c.mask.insert(cell.row);
            } else {
                c.mask.remove(&cell.row);
            }
            changed += 1;
        }
        Ok(changed)
    }

    /// Sets the cells to missing values (the rows stay). Columns that a formula keeps up to date
    /// are left alone; a row beyond the data is already missing and is skipped. Returns how many
    /// cells changed and how many columns were left alone.
    pub fn set_cells_missing(&mut self, cells: &[DataCell]) -> Result<DataEditReport, DataEditError> {
        let cells: BTreeSet<DataCell> = cells.iter().copied().collect();
        for c in &cells {
            self.column(c.column).ok_or(DataEditError::NoSuchColumn(c.column))?;
        }
        let mut report = DataEditReport::default();
        let mut locked = BTreeSet::new();
        for cell in cells {
            let Some(c) = self.column_mut(cell.column) else { continue };
            if c.is_locked() {
                locked.insert(cell.column);
            } else if !is_blank(c, cell.row) {
                c.set_cell_text(cell.row, "");
                report.changed += 1;
            }
        }
        report.locked = locked.len();
        Ok(report)
    }

    /// Sets numeric cells to the given numbers, all or nothing: when any of the columns is kept up
    /// to date by a formula nothing changes and `locked` says how many such columns there are.
    /// A column that does not exist, a row beyond its data, a text column and a number that is
    /// not finite are errors (nothing is changed). Returns how many cells changed.
    pub fn set_cell_numbers(&mut self, values: &[(DataCell, f64)]) -> Result<DataEditReport, DataEditError> {
        let mut locked = BTreeSet::new();
        for (cell, value) in values {
            let c = self.column(cell.column).ok_or(DataEditError::NoSuchColumn(cell.column))?;
            if cell.row >= c.len() {
                return Err(DataEditError::NoSuchCell { column: c.short_name.clone(), row: cell.row });
            }
            if matches!(c.data, ColumnData::Text(_)) {
                return Err(DataEditError::NotNumeric(c.short_name.clone()));
            }
            if !value.is_finite() {
                return Err(DataEditError::NotFinite);
            }
            if c.is_locked() {
                locked.insert(cell.column);
            }
        }
        let mut report = DataEditReport { changed: 0, locked: locked.len() };
        if report.locked > 0 {
            return Ok(report);
        }
        for (cell, value) in values {
            if let Some(c) = self.column_mut(cell.column)
                && !holds_number(c, cell.row, *value)
            {
                // The shortest text that reads back as the same number.
                c.set_cell_text(cell.row, &value.to_string());
                report.changed += 1;
            }
        }
        Ok(report)
    }
}

/// True when the cell is a missing value (no number, no text).
fn is_blank(c: &Column, row: usize) -> bool {
    match &c.data {
        ColumnData::Numeric(v) => v.get(row).is_none_or(|x| x.is_nan()),
        ColumnData::Text(v) => v.get(row).is_none_or(String::is_empty),
        ColumnData::Mixed { numbers, .. } => {
            numbers.get(row).is_none_or(|x| x.is_nan()) && c.data.text_cell(row).is_none()
        }
    }
}

fn holds_number(c: &Column, row: usize, value: f64) -> bool {
    c.numeric().and_then(|v| v.get(row)).is_some_and(|x| x.to_bits() == value.to_bits())
        && c.data.text_cell(row).is_none()
}

/// `B(Y)` for one column, `3 columns` for several.
fn columns_label(project: &Project, cells: &[DataCell]) -> String {
    let ids: BTreeSet<ColumnId> = cells.iter().map(|c| c.column).collect();
    match (ids.len(), ids.first().and_then(|id| project.column(*id))) {
        (1, Some(c)) => format!("{}({})", c.short_name, c.designation.tag()),
        (n, _) => format!("{n} columns"),
    }
}

impl Document {
    /// Data > Mask Data Points: masks (`masked`) or unmasks the cells, one undo step labelled
    /// "Set Mask Data" (Origin's text). Nothing is recorded when no cell
    /// would change. Masks are allowed on columns that a formula keeps up to date.
    pub fn mask_points(&mut self, cells: &[DataCell], masked: bool) -> Result<DataEditReport, DataEditError> {
        let mut probe = self.project().clone();
        let would = probe.mask_data_cells(cells, masked)?;
        if would == 0 {
            return Ok(DataEditReport::default());
        }
        // Origin 2016 names the step "Set Mask Data" (unmask: not measured, same text).
        let label = "Set Mask Data".to_string();
        let changed = self.edit(&label, |p| p.mask_data_cells(cells, masked))?;
        Ok(DataEditReport { changed, locked: 0 })
    }

    /// Data > Remove Bad Data Points: sets the cells to missing values and keeps their rows, one
    /// undo step labelled "Remove Data Point" (Origin's text). Columns calculated by a formula are left alone (see
    /// [`DataEditReport::locked`]); nothing is recorded when no cell would change.
    pub fn remove_bad_points(&mut self, cells: &[DataCell]) -> Result<DataEditReport, DataEditError> {
        let mut probe = self.project().clone();
        let would = probe.set_cells_missing(cells)?;
        if would.changed == 0 {
            return Ok(would);
        }
        // Origin 2016 names the step "Remove Data Point".
        let label = "Remove Data Point";
        self.edit(label, |p| p.set_cells_missing(cells))
    }

    /// Data > Move Data Points: sets the Y cell of a point, and its X cell when the plot has an X
    /// column, to new numbers; one undo step labelled "Move point B(Y) row 12". All or nothing:
    /// if either column is calculated by a formula nothing changes (see [`DataEditReport::locked`]).
    /// Nothing is recorded when the point is dropped where it was.
    pub fn move_point(
        &mut self,
        y: (DataCell, f64),
        x: Option<(DataCell, f64)>,
    ) -> Result<DataEditReport, DataEditError> {
        let values: Vec<(DataCell, f64)> = x.into_iter().chain(Some(y)).collect();
        let mut probe = self.project().clone();
        let would = probe.set_cell_numbers(&values)?;
        if would.changed == 0 {
            return Ok(would);
        }
        let label = format!("Move point {} row {}", columns_label(self.project(), &[y.0]), y.0.row + 1);
        self.edit(&label, |p| p.set_cell_numbers(&values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{OutputTarget, Request, RowRange, SmoothOptions, Tool};
    use crate::{ColumnData, Designation, RecalcMode, WindowId};

    /// A workbook with X = 1..=5 (A), Y = 10, 20, 30, 40, 50 (B) and an empty column C.
    fn setup() -> (Document, WindowId, [ColumnId; 3]) {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0, 40.0, 50.0])).unwrap();
        let c = p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        (Document::new(p), book, [a, b, c])
    }

    fn mask(d: &Document, c: ColumnId) -> Vec<usize> {
        d.project().column(c).unwrap().mask.iter().copied().collect()
    }

    fn values(d: &Document, c: ColumnId) -> Vec<f64> {
        d.project().numeric(c).unwrap().to_vec()
    }

    #[test]
    fn masking_cells_is_one_undo_step_with_a_label() {
        let (mut d, _, [_, b, _]) = setup();
        let cells = [DataCell::new(b, 1), DataCell::new(b, 3), DataCell::new(b, 1), DataCell::new(b, 99)];
        let report = d.mask_points(&cells, true).unwrap();
        assert_eq!(report, DataEditReport { changed: 2, locked: 0 });
        assert_eq!(mask(&d, b), vec![1, 3], "a repeated cell counts once and a row beyond the data is skipped");
        assert_eq!(d.undo_label(), Some("Set Mask Data"));
        assert_eq!(values(&d, b)[1], 20.0, "the value stays");
        d.undo();
        assert!(mask(&d, b).is_empty());
        assert!(d.undo_label().is_none(), "one step");
    }

    #[test]
    fn unmasking_cells_changes_only_masked_ones_and_names_the_step() {
        let (mut d, _, [_, b, _]) = setup();
        d.mask_points(&[DataCell::new(b, 0), DataCell::new(b, 2)], true).unwrap();
        let report = d.mask_points(&[DataCell::new(b, 2), DataCell::new(b, 4)], false).unwrap();
        assert_eq!(report.changed, 1);
        assert_eq!(mask(&d, b), vec![0]);
        assert_eq!(d.undo_label(), Some("Set Mask Data"));
    }

    #[test]
    fn a_mask_that_changes_nothing_makes_no_undo_step() {
        let (mut d, _, [_, b, _]) = setup();
        d.mask_points(&[DataCell::new(b, 0)], true).unwrap();
        let revision = d.revision();
        assert_eq!(d.mask_points(&[DataCell::new(b, 0)], true).unwrap().changed, 0);
        assert_eq!(d.mask_points(&[DataCell::new(b, 1)], false).unwrap().changed, 0);
        assert_eq!(d.mask_points(&[], true).unwrap().changed, 0);
        assert_eq!(d.revision(), revision);
        assert_eq!(d.undo_label(), Some("Set Mask Data"));
    }

    #[test]
    fn masking_cells_of_two_columns_names_the_number_of_columns() {
        let (mut d, _, [a, b, _]) = setup();
        d.mask_points(&[DataCell::new(a, 0), DataCell::new(b, 0)], true).unwrap();
        assert_eq!(d.undo_label(), Some("Set Mask Data"));
    }

    #[test]
    fn a_missing_column_is_an_error_and_changes_nothing() {
        let (mut d, _, [_, b, _]) = setup();
        let ghost = ColumnId(9999);
        let err = d.mask_points(&[DataCell::new(b, 0), DataCell::new(ghost, 0)], true).unwrap_err();
        assert_eq!(err, DataEditError::NoSuchColumn(ghost));
        assert!(mask(&d, b).is_empty());
        assert!(d.remove_bad_points(&[DataCell::new(ghost, 0)]).is_err());
        assert!(d.move_point((DataCell::new(ghost, 0), 1.0), None).is_err());
    }

    #[test]
    fn removing_a_bad_point_sets_the_cell_missing_and_keeps_the_row() {
        let (mut d, _, [a, b, _]) = setup();
        let report = d.remove_bad_points(&[DataCell::new(b, 2)]).unwrap();
        assert_eq!(report, DataEditReport { changed: 1, locked: 0 });
        let y = values(&d, b);
        assert_eq!(y.len(), 5, "the rows are kept");
        assert!(y[2].is_nan() && y[1] == 20.0 && y[3] == 40.0);
        assert_eq!(values(&d, a), vec![1.0, 2.0, 3.0, 4.0, 5.0], "the X cell is not touched");
        assert_eq!(d.undo_label(), Some("Remove Data Point"));
        d.undo();
        assert_eq!(values(&d, b)[2], 30.0);
        assert!(d.undo_label().is_none());
    }

    #[test]
    fn removing_several_bad_points_is_one_step_and_skips_cells_already_missing() {
        let (mut d, _, [_, b, _]) = setup();
        d.remove_bad_points(&[DataCell::new(b, 0)]).unwrap();
        let report = d.remove_bad_points(&[DataCell::new(b, 0), DataCell::new(b, 1), DataCell::new(b, 4)]).unwrap();
        assert_eq!(report.changed, 2);
        assert_eq!(d.undo_label(), Some("Remove Data Point"));
        let revision = d.revision();
        assert_eq!(d.remove_bad_points(&[DataCell::new(b, 0), DataCell::new(b, 80)]).unwrap().changed, 0);
        assert_eq!(d.revision(), revision, "nothing to do, no step");
    }

    #[test]
    fn removing_a_bad_point_clears_the_text_of_a_text_and_numeric_cell() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let y = p.add_column(book, 0, Designation::Y, ColumnData::mixed_numbers(vec![1.0, 2.0])).unwrap();
        p.column_mut(y).unwrap().set_cell_text(1, "n/a");
        let mut d = Document::new(p);
        assert_eq!(d.remove_bad_points(&[DataCell::new(y, 1)]).unwrap().changed, 1);
        assert_eq!(d.project().column(y).unwrap().data.text_cell(1), None);
        assert_eq!(d.remove_bad_points(&[DataCell::new(y, 1)]).unwrap().changed, 0);
    }

    #[test]
    fn moving_a_point_sets_its_x_and_y_cells_in_one_step() {
        let (mut d, _, [a, b, _]) = setup();
        let report = d.move_point((DataCell::new(b, 1), 25.5), Some((DataCell::new(a, 1), 2.25))).unwrap();
        assert_eq!(report, DataEditReport { changed: 2, locked: 0 });
        assert_eq!(values(&d, a), vec![1.0, 2.25, 3.0, 4.0, 5.0]);
        assert_eq!(values(&d, b), vec![10.0, 25.5, 30.0, 40.0, 50.0]);
        assert_eq!(d.undo_label(), Some("Move point B(Y) row 2"));
        d.undo();
        assert_eq!(values(&d, a)[1], 2.0);
        assert_eq!(values(&d, b)[1], 20.0);
        assert!(d.undo_label().is_none(), "X and Y came back together");
        d.redo();
        assert_eq!(values(&d, b)[1], 25.5);
    }

    #[test]
    fn a_point_of_a_plot_without_an_x_column_moves_in_y_only() {
        let (mut d, _, [_, b, _]) = setup();
        assert_eq!(d.move_point((DataCell::new(b, 4), -0.1), None).unwrap().changed, 1);
        assert_eq!(values(&d, b)[4], -0.1);
    }

    #[test]
    fn a_moved_value_keeps_every_digit() {
        let (mut d, _, [a, b, _]) = setup();
        let (x, y) = (1.0 / 3.0, 1.2345678901234567e-7);
        d.move_point((DataCell::new(b, 0), y), Some((DataCell::new(a, 0), x))).unwrap();
        assert_eq!(values(&d, a)[0].to_bits(), x.to_bits());
        assert_eq!(values(&d, b)[0].to_bits(), y.to_bits());
    }

    #[test]
    fn dropping_a_point_where_it_was_makes_no_step() {
        let (mut d, _, [a, b, _]) = setup();
        let revision = d.revision();
        let report = d.move_point((DataCell::new(b, 1), 20.0), Some((DataCell::new(a, 1), 2.0))).unwrap();
        assert_eq!(report.changed, 0);
        assert_eq!(d.revision(), revision);
        assert!(d.undo_label().is_none());
    }

    #[test]
    fn moving_a_point_refuses_a_row_beyond_the_data_a_text_column_and_a_value_that_is_not_finite() {
        let (mut d, book, [_, b, c]) = setup();
        let err = d.move_point((DataCell::new(b, 7), 1.0), None).unwrap_err();
        assert_eq!(err, DataEditError::NoSuchCell { column: "B".into(), row: 7 });
        assert_eq!(err.to_string(), "column B has no row 8");
        assert!(d.move_point((DataCell::new(c, 0), 1.0), None).is_err(), "C has no data");
        assert_eq!(d.move_point((DataCell::new(b, 0), f64::NAN), None).unwrap_err(), DataEditError::NotFinite);
        assert_eq!(d.move_point((DataCell::new(b, 0), f64::INFINITY), None).unwrap_err(), DataEditError::NotFinite);
        let t = d.apply("text", |p| p.add_column(book, 0, Designation::Y, ColumnData::text(vec!["a".into()])).unwrap());
        assert_eq!(d.move_point((DataCell::new(t, 0), 1.0), None).unwrap_err(), DataEditError::NotNumeric("D".into()));
        assert_eq!(values(&d, b), vec![10.0, 20.0, 30.0, 40.0, 50.0]);
    }

    #[test]
    fn a_failed_move_leaves_the_x_cell_alone() {
        let (mut d, _, [a, b, _]) = setup();
        assert!(d.move_point((DataCell::new(b, 9), 1.0), Some((DataCell::new(a, 0), 99.0))).is_err());
        assert_eq!(values(&d, a)[0], 1.0);
    }

    /// B is kept up to date from A (B = 10 A, Auto) and D from B (D = B + 1, Auto).
    fn recalculating() -> (Document, [ColumnId; 4]) {
        let (mut d, book, [a, b, _]) = setup();
        let dd = d.apply("add", |p| p.add_column(book, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap());
        d.edit("f", |p| p.set_column_values_with(b, "col(A)*10", None, RecalcMode::Auto)).unwrap();
        d.edit("f", |p| p.set_column_values_with(dd, "col(B)+1", None, RecalcMode::Auto)).unwrap();
        let free =
            d.apply("add", |p| p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0; 5])).unwrap());
        (d, [a, b, dd, free])
    }

    #[test]
    fn a_column_calculated_by_a_formula_is_left_alone_by_remove_and_move() {
        let (mut d, [a, b, _, _]) = recalculating();
        let before = values(&d, b);
        let steps = d.undo_label().map(str::to_string);
        let report = d.remove_bad_points(&[DataCell::new(b, 1)]).unwrap();
        assert_eq!(report, DataEditReport { changed: 0, locked: 1 });
        assert_eq!(values(&d, b), before);
        let report = d.move_point((DataCell::new(b, 1), 5.0), Some((DataCell::new(a, 1), 7.0))).unwrap();
        assert_eq!(report, DataEditReport { changed: 0, locked: 1 });
        assert_eq!(values(&d, a)[1], 2.0, "the X cell was not moved either");
        assert_eq!(d.undo_label().map(str::to_string), steps, "no step was made");
    }

    #[test]
    fn a_mask_is_allowed_on_a_column_calculated_by_a_formula() {
        let (mut d, [_, b, _, _]) = recalculating();
        assert_eq!(d.mask_points(&[DataCell::new(b, 2)], true).unwrap().changed, 1);
        assert_eq!(mask(&d, b), vec![2]);
    }

    #[test]
    fn recalculating_columns_follow_a_moved_or_removed_point_in_the_same_step() {
        let (mut d, [a, b, dd, free]) = recalculating();
        // Moving the X cell of a free Y column: B = 10 A and D = B + 1 follow A.
        d.move_point((DataCell::new(free, 0), 3.0), Some((DataCell::new(a, 0), 4.0))).unwrap();
        assert_eq!(values(&d, b)[0], 40.0);
        assert_eq!(values(&d, dd)[0], 41.0);
        d.undo();
        assert_eq!(values(&d, b)[0], 10.0);
        assert_eq!(values(&d, dd)[0], 11.0);
        // Removing the X value: the formulas read a missing value.
        d.remove_bad_points(&[DataCell::new(a, 2)]).unwrap();
        assert!(values(&d, b)[2].is_nan() && values(&d, dd)[2].is_nan());
        d.undo();
        assert_eq!(values(&d, b)[2], 30.0);
    }

    #[test]
    fn an_auto_analysis_link_follows_a_mask_a_removed_point_and_a_move() {
        use sheaf_calc::{Boundary, SmoothMethod};
        let (mut d, _, [_, b, _]) = setup();
        let options = SmoothOptions {
            method: SmoothMethod::AdjacentAveraging { weighted: false },
            npts: 3,
            boundary: Boundary::None,
        };
        let req = Request { columns: vec![b], rows: RowRange::All, tool: Tool::Smooth(options) };
        let out = d
            .edit("smooth", |p| p.analyze_linked(&req, &OutputTarget::NextToInput, RecalcMode::Auto))
            .unwrap()
            .new_columns[0];
        let plain = values(&d, out);
        assert_eq!(plain[2], 30.0);
        // Masked points are left out of the smoothing.
        d.mask_points(&[DataCell::new(b, 3)], true).unwrap();
        let masked = values(&d, out);
        assert_ne!(masked, plain, "the result column was calculated again");
        assert!(masked[3].is_nan(), "the masked point is missing for the analysis: {masked:?}");
        d.undo();
        assert_eq!(values(&d, out), plain);
        d.move_point((DataCell::new(b, 2), 90.0), None).unwrap();
        assert_eq!(values(&d, out)[2], (20.0 + 90.0 + 40.0) / 3.0);
        d.remove_bad_points(&[DataCell::new(b, 2)]).unwrap();
        let gone = values(&d, out);
        assert!(gone[2].is_nan(), "a missing point is missing in the result: {gone:?}");
        assert_ne!(gone[1], plain[1], "its neighbours were calculated again");
        d.undo();
        d.undo();
        assert_eq!(values(&d, out), plain);
    }
}
