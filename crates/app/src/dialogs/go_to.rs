//! Edit > Go to... (Ctrl+G) in a workbook: a small window that asks for a row and a column and
//! moves the cell cursor there.

use eframe::egui;
use sheaf_core::{WindowId, Worksheet};

/// The window that asks for the cell to go to.
pub struct GoToDialog {
    pub book: WindowId,
    row: String,
    column: String,
    /// What was wrong with the last entry.
    error: Option<String>,
    /// True until the first frame, to put the keyboard in the row field.
    first_frame: bool,
}

/// What the window did in a frame.
pub enum GoToOutcome {
    Open,
    Closed,
    /// Select this cell: column position and row, both counted from zero.
    Go {
        column: usize,
        row: usize,
    },
}

impl GoToDialog {
    pub fn new(book: WindowId) -> Self {
        GoToDialog { book, row: String::new(), column: String::new(), error: None, first_frame: true }
    }

    /// Shows the window; `sheet` is the sheet the cell is looked up in.
    pub fn show(&mut self, ctx: &egui::Context, sheet: &Worksheet) -> GoToOutcome {
        let mut open = true;
        let mut outcome = GoToOutcome::Open;
        egui::Window::new("Go To").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            egui::Grid::new("go_to_grid").num_columns(2).show(ui, |ui| {
                ui.label("Row");
                let row = ui.add(egui::TextEdit::singleline(&mut self.row).desired_width(120.0));
                if self.first_frame {
                    row.request_focus();
                    self.first_frame = false;
                }
                ui.end_row();
                ui.label("Column");
                ui.add(egui::TextEdit::singleline(&mut self.column).desired_width(120.0));
                ui.end_row();
            });
            ui.weak("Column: its number or its name, such as 3 or B. Leave it empty to stay in the column.");
            if let Some(e) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() || enter {
                    match parse_target(sheet, &self.row, &self.column) {
                        Ok((column, row)) => outcome = GoToOutcome::Go { column, row },
                        Err(e) => self.error = Some(e),
                    }
                }
                if ui.button("Cancel").clicked() {
                    outcome = GoToOutcome::Closed;
                }
            });
        });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return GoToOutcome::Closed;
        }
        outcome
    }
}

/// The cell (column position, row; both from zero) that the typed row and column name. An empty
/// row stays in row 1; an empty column stays in the first column.
pub fn parse_target(sheet: &Worksheet, row: &str, column: &str) -> Result<(usize, usize), String> {
    if sheet.columns.is_empty() {
        return Err("The sheet has no columns".to_string());
    }
    let row = row.trim();
    let r = if row.is_empty() {
        0
    } else {
        match row.parse::<usize>() {
            Ok(n) if n >= 1 => n - 1,
            _ => return Err(format!("\"{row}\" is not a row number (1 or more)")),
        }
    };
    let column = column.trim();
    let c = if column.is_empty() {
        0
    } else if let Ok(n) = column.parse::<usize>() {
        if n == 0 || n > sheet.columns.len() {
            return Err(format!("The sheet has columns 1 to {}", sheet.columns.len()));
        }
        n - 1
    } else {
        sheet
            .columns
            .iter()
            .position(|c| c.short_name.eq_ignore_ascii_case(column))
            .ok_or_else(|| format!("There is no column named \"{column}\""))?
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
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0])).unwrap();
        p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0])).unwrap();
        let sheet = &p.workbook(b).unwrap().sheets[0];
        assert_eq!(parse_target(sheet, "5", "2"), Ok((1, 4)));
        assert_eq!(parse_target(sheet, "3", "b"), Ok((1, 2)));
        assert_eq!(parse_target(sheet, "", ""), Ok((0, 0)));
        assert!(parse_target(sheet, "0", "1").is_err());
        assert!(parse_target(sheet, "1", "9").is_err());
        assert!(parse_target(sheet, "1", "Z").is_err());
    }
}
