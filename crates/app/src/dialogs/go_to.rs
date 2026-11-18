//! Edit > Go to... (Ctrl+G) in a workbook: a small window that asks for a row and a column and
//! navigates to rows, columns or a cell without changing worksheet data.

use eframe::egui;
use sheaf_core::{Column, Project, WindowId, Worksheet};

mod browser;
use browser::{BrowserOutcome, ColumnBrowser};

/// The window that asks for the cell to go to.
pub struct GoToDialog {
    pub book: WindowId,
    row: String,
    column: String,
    extend: bool,
    can_extend: bool,
    row_capacity: usize,
    current: Option<(usize, usize)>,
    /// What was wrong with the last entry.
    error: Option<String>,
    /// True until the first frame, to put the keyboard in the column field.
    first_frame: bool,
    browser: Option<ColumnBrowser>,
    /// The browser returns a qualified reference, but Go To applies its ordinal locally.
    browser_ordinal: Option<usize>,
}

/// What the window did in a frame.
pub enum GoToOutcome {
    Open,
    Closed,
    /// Zero-based optional row/column, with an optional extension of the current region.
    Go {
        column: Option<usize>,
        row: Option<usize>,
        extend: bool,
    },
}

impl GoToDialog {
    pub fn new(book: WindowId, can_extend: bool, row_capacity: usize, current: Option<(usize, usize)>) -> Self {
        GoToDialog {
            book,
            row: String::new(),
            column: String::new(),
            extend: false,
            can_extend,
            row_capacity: row_capacity.max(1),
            current,
            error: None,
            first_frame: true,
            browser: None,
            browser_ordinal: None,
        }
    }

    /// Shows Go To or its browser. Navigation always uses the original workbook's active sheet.
    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> GoToOutcome {
        let Some(workbook) = project.workbook(self.book) else { return GoToOutcome::Closed };
        let Some(sheet) = workbook.sheets.get(workbook.active_sheet) else { return GoToOutcome::Closed };
        if let Some(browser) = &mut self.browser {
            match browser.show(ctx, project) {
                BrowserOutcome::Open => {}
                BrowserOutcome::Closed => {
                    self.browser = None;
                    self.first_frame = true;
                }
                BrowserOutcome::Selected { reference, ordinal } => {
                    self.column = reference;
                    self.browser_ordinal = Some(ordinal);
                    self.browser = None;
                    self.error = None;
                    self.first_frame = true;
                }
            }
            return GoToOutcome::Open;
        }
        let mut open = true;
        let mut outcome = GoToOutcome::Open;
        let picker_was_open = egui::Popup::is_any_open(ctx);
        egui::Window::new("Go To").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            let current = self
                .current
                .and_then(|(c, r)| sheet.columns.get(c).map(|column| format!("{}[{}]", column.short_name, r + 1)))
                .unwrap_or_else(|| "none".into());
            ui.label(format!("Size = {} × {}, Current = {current}", sheet.columns.len(), self.row_capacity));
            ui.weak("Column name or number. 0 selects the last column or last data row.");
            egui::Grid::new("go_to_grid").num_columns(2).show(ui, |ui| {
                let label = ui.label("Go to Column");
                ui.horizontal(|ui| {
                    let column =
                        ui.add(egui::TextEdit::singleline(&mut self.column).desired_width(190.0)).labelled_by(label.id);
                    if column.changed() {
                        self.browser_ordinal = None;
                    }
                    if self.first_frame {
                        column.request_focus();
                        self.first_frame = false;
                    }
                    ui.menu_button("▶", |ui| {
                        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                            for column in &sheet.columns {
                                let label = format!(
                                    "{}({}): {}",
                                    column.short_name,
                                    column.designation.tag(),
                                    column.long_name
                                );
                                if ui.button(label).clicked() {
                                    self.column = column_reference(column);
                                    self.browser_ordinal = None;
                                    self.error = None;
                                    ui.close();
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("Reset").clicked() {
                            self.column.clear();
                            self.browser_ordinal = None;
                            self.error = None;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Select Columns...").clicked() {
                            self.browser = Some(ColumnBrowser::new(self.book, workbook.active_sheet));
                            egui::Popup::close_all(ctx);
                            ui.close();
                        }
                    })
                    .response
                    .on_hover_text("Choose a column");
                });
                ui.end_row();
                let label = ui.label("Go to Row");
                ui.add(egui::TextEdit::singleline(&mut self.row).desired_width(218.0)).labelled_by(label.id);
                ui.end_row();
            });
            ui.add_enabled(self.can_extend, egui::Checkbox::new(&mut self.extend, "Extend Selection"));
            if let Some(e) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            let enter = !picker_was_open && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        outcome = GoToOutcome::Closed;
                    }
                    if ui.button("OK").clicked() || enter {
                        let browser_column = self.browser_ordinal.map(|ordinal| ordinal.to_string());
                        match parse_target(
                            sheet,
                            &self.row,
                            browser_column.as_deref().unwrap_or(&self.column),
                            self.row_capacity,
                            self.current.map_or(0, |p| p.0),
                        ) {
                            Ok((column, row)) => {
                                outcome = GoToOutcome::Go { column, row, extend: self.can_extend && self.extend }
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                });
            });
        });
        if !open || (!picker_was_open && ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
            return GoToOutcome::Closed;
        }
        outcome
    }
}

/// The native picker includes the short name to distinguish repeated long names.
fn column_reference(column: &Column) -> String {
    if column.long_name.is_empty() {
        column.short_name.clone()
    } else {
        format!("{}\"{}\"", column.short_name, column.long_name)
    }
}

/// Parses optional one-based row/column inputs, returning zero-based positions. Column lookup
/// accepts short names or picker references, then the first matching long name, ignoring ASCII case.
pub fn parse_target(
    sheet: &Worksheet,
    row: &str,
    column: &str,
    row_capacity: usize,
    current_column: usize,
) -> Result<(Option<usize>, Option<usize>), String> {
    if sheet.columns.is_empty() {
        return Err("The sheet has no columns".to_string());
    }
    let column = column.trim();
    let c = if column.is_empty() {
        None
    } else if let Ok(n) = column.parse::<usize>() {
        if n > sheet.columns.len() {
            return Err(format!("The sheet has columns 1 to {} (0 selects the last)", sheet.columns.len()));
        }
        Some(if n == 0 { sheet.columns.len() - 1 } else { n - 1 })
    } else {
        Some(
            sheet
                .columns
                .iter()
                .position(|c| {
                    c.short_name.eq_ignore_ascii_case(column) || column_reference(c).eq_ignore_ascii_case(column)
                })
                .or_else(|| sheet.columns.iter().position(|c| c.long_name.eq_ignore_ascii_case(column)))
                .ok_or_else(|| format!("There is no column named \"{column}\""))?,
        )
    };
    let row = row.trim();
    let r = if row.is_empty() {
        None
    } else {
        match row.parse::<i32>() {
            Ok(0) => {
                let end_column = c.unwrap_or(current_column).min(sheet.columns.len() - 1);
                Some(sheet.columns[end_column].len().saturating_sub(1))
            }
            Ok(n) if n >= 1 => Some((n as usize - 1).min(row_capacity.saturating_sub(1))),
            _ => return Err(format!("\"{row}\" is not a row number (0 to {})", i32::MAX)),
        }
    };
    Ok((c, r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn a_row_and_a_column_number_or_name_pick_a_cell() {
        let mut p = Project::default();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0])).unwrap();
        p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 4.0, 6.0, 8.0])).unwrap();
        let sheet = &p.workbook(b).unwrap().sheets[0];
        let parse = |r, c| parse_target(sheet, r, c, 100, 0);
        assert_eq!(parse("5", "2"), Ok((Some(1), Some(4))));
        assert_eq!(parse("3", "b"), Ok((Some(1), Some(2))));
        assert_eq!(parse("", ""), Ok((None, None)));
        assert_eq!(parse("5", ""), Ok((None, Some(4))));
        assert_eq!(parse("", "b"), Ok((Some(1), None)));
        assert_eq!(parse("0", "0"), Ok((Some(1), Some(3))));
        assert_eq!(parse("1000", "2"), Ok((Some(1), Some(99))));
        assert!(parse("1", "9").is_err());
        assert!(parse("-1", "1").is_err());
        assert!(parse("1", "Z").is_err());
        assert!(parse("2147483648", "1").is_err());
    }

    #[test]
    fn zero_uses_the_destination_column_end_or_the_first_column_of_the_current_region() {
        let mut project = Project::default();
        let book = project.add_workbook("");
        for count in [4, 2, 6, 3] {
            project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0; count])).unwrap();
        }
        let sheet = project.workbook(book).unwrap().sheet();
        // Origin 2016 unequal-end/range-end probes: B has two values, C six, D three;
        // row capacity is 100. Omitting the column keeps B or B:C and uses B's end.
        assert_eq!(parse_target(sheet, "0", "2", 100, 1), Ok((Some(1), Some(1))));
        assert_eq!(parse_target(sheet, "0", "0", 100, 1), Ok((Some(3), Some(2))));
        assert_eq!(parse_target(sheet, "0", "", 100, 1), Ok((None, Some(1))));
    }

    #[test]
    fn names_ignore_ascii_case_and_duplicate_long_names_use_the_first_column() {
        let mut project = Project::default();
        let book = project.add_workbook("");
        for label in ["Time", "Shared current", "Potential", "Shared current"] {
            let id = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
            project.workbook_mut(book).unwrap().sheets[0].column_mut(id).unwrap().long_name = label.into();
        }
        let sheet = project.workbook(book).unwrap().sheet();
        for (name, index) in
            [("b", 1), ("Potential", 2), ("potential", 2), ("Shared current", 1), ("D\"Shared current\"", 3), ("4", 3)]
        {
            assert_eq!(parse_target(sheet, "10", name, 100, 0), Ok((Some(index), Some(9))), "{name}");
        }
        assert!(parse_target(sheet, "10", "missing name", 100, 0).is_err());
    }
}
