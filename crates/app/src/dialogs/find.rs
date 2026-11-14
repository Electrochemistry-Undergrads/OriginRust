//! Edit > Find in Sheets... (Ctrl+F) in a workbook: finds a text in the cells of the active sheet
//! and moves the cell cursor to the next cell that holds it. Cells are compared by the text the
//! worksheet shows, searched column by column from the cell after the cursor, wrapping around.

use eframe::egui;
use sheaf_core::{WindowId, Worksheet};

/// The window that asks what to find.
pub struct FindDialog {
    pub book: WindowId,
    what: String,
    match_case: bool,
    whole_cell: bool,
    /// What the last search found or not.
    note: Option<String>,
    /// True until the first frame, to put the keyboard in the text field.
    first_frame: bool,
}

/// What the window did in a frame.
pub enum FindOutcome {
    Open,
    Closed,
    /// Find the next cell after the cursor; the caller searches with [`find_next`].
    Next,
}

impl FindDialog {
    pub fn new(book: WindowId) -> Self {
        FindDialog { book, what: String::new(), match_case: false, whole_cell: false, note: None, first_frame: true }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FindOutcome {
        let mut open = true;
        let mut outcome = FindOutcome::Open;
        egui::Window::new("Find in Sheets").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Find what");
                let field = ui.add(egui::TextEdit::singleline(&mut self.what).desired_width(180.0));
                if self.first_frame {
                    field.request_focus();
                    self.first_frame = false;
                }
            });
            ui.checkbox(&mut self.match_case, "Match case");
            ui.checkbox(&mut self.whole_cell, "Match whole cell only");
            if let Some(n) = &self.note {
                ui.weak(n);
            }
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                if ui.button("Find Next").clicked() || enter {
                    outcome = FindOutcome::Next;
                }
                if ui.button("Close").clicked() {
                    outcome = FindOutcome::Closed;
                }
            });
        });
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return FindOutcome::Closed;
        }
        outcome
    }

    /// The next cell after `from` (column position, row) that matches, and a note for the window.
    pub fn search(&mut self, sheet: &Worksheet, from: Option<(usize, usize)>) -> Option<(usize, usize)> {
        if self.what.is_empty() {
            self.note = Some("Type the text to find".to_string());
            return None;
        }
        let hit = find_next(sheet, &self.what, self.match_case, self.whole_cell, from);
        self.note = match hit {
            Some((c, r)) => Some(format!("Found in column {}, row {}", sheet.columns[c].short_name, r + 1)),
            None => Some(format!("\"{}\" was not found", self.what)),
        };
        hit
    }

    #[cfg(test)]
    pub fn set_text(&mut self, what: &str) {
        self.what = what.to_string();
    }
}

/// The first cell after `from` (column position, row; both from zero), going down each column and
/// then on to the next column, wrapping around to the start, whose shown text contains `what` (or
/// equals it with `whole_cell`). `None` when no cell matches.
pub fn find_next(
    sheet: &Worksheet,
    what: &str,
    match_case: bool,
    whole_cell: bool,
    from: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let fold = |s: &str| if match_case { s.to_string() } else { s.to_lowercase() };
    let needle = fold(what);
    let cells: Vec<(usize, usize)> =
        sheet.columns.iter().enumerate().flat_map(|(c, col)| (0..col.len()).map(move |r| (c, r))).collect();
    if cells.is_empty() {
        return None;
    }
    let start = from.and_then(|f| cells.iter().position(|&x| x == f)).map_or(0, |i| i + 1);
    (0..cells.len()).map(|k| cells[(start + k) % cells.len()]).find(|&(c, r)| {
        let text = fold(&sheet.columns[c].shown_text(r));
        if whole_cell { text == needle } else { !text.is_empty() && text.contains(&needle) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation, Project};

    #[test]
    fn find_goes_down_columns_from_the_cursor_and_wraps_around() {
        let mut p = Project::default();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.5, 3.0])).unwrap();
        p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["LMO".into(), "Ni-LMO".into(), "x".into()])).unwrap();
        let sheet = &p.workbook(b).unwrap().sheets[0];
        assert_eq!(find_next(sheet, "lmo", false, false, None), Some((1, 0)));
        assert_eq!(find_next(sheet, "lmo", false, false, Some((1, 0))), Some((1, 1)));
        assert_eq!(find_next(sheet, "lmo", false, false, Some((1, 1))), Some((1, 0)), "wraps around");
        assert_eq!(find_next(sheet, "lmo", true, false, None), None, "case matters");
        assert_eq!(find_next(sheet, "LMO", true, true, None), Some((1, 0)), "whole cell");
        assert_eq!(find_next(sheet, "2.5", false, true, None), Some((0, 1)), "numbers by their shown text");
        assert_eq!(find_next(sheet, "zzz", false, false, None), None);
    }
}
