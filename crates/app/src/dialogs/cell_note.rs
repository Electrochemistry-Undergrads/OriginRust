//! The note editor: a small modal window with a multi-line plain-text box for the note of one
//! cell (Origin edits notes in a separate Notes window, which Sheaf does not have). OK stores the
//! text as one undo step ("Add note", "Edit note", or "Delete note" when an existing note is
//! emptied); empty text on a cell without a note changes nothing.

use super::frame::{DialogButton, button_row};
use eframe::egui;
use sheaf_core::{ColumnId, Document, WindowId};

pub struct CellNoteDialog {
    pub book: WindowId,
    sheet: usize,
    column: ColumnId,
    row: usize,
    existing: bool,
    pub text: String,
    first_frame: bool,
}

impl CellNoteDialog {
    pub fn new(doc: &Document, book: WindowId, column: ColumnId, row: usize) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let current = b.sheets[sheet].note(column, row);
        Some(CellNoteDialog {
            book,
            sheet,
            column,
            row,
            existing: current.is_some(),
            text: current.unwrap_or_default().to_string(),
            first_frame: true,
        })
    }

    /// Draws the window; false when it closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)).is_none() {
            return false;
        }
        let mut open = true;
        let mut pressed = None;
        let title = if self.existing { "Edit Note" } else { "Add Note" };
        egui::Window::new(title)
            .id(egui::Id::new("cell_note"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut self.text)
                        .id(egui::Id::new("cell_note_text"))
                        .desired_rows(6)
                        .desired_width(f32::INFINITY),
                );
                if self.first_frame {
                    edit.request_focus();
                    self.first_frame = false;
                }
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => {
                let (book, sheet, column, row) = (self.book, self.sheet, self.column, self.row);
                let empty = self.text.trim().is_empty();
                let label = match (self.existing, empty) {
                    (true, true) => "Delete note",
                    (true, false) => "Edit note",
                    (false, _) => "Add note",
                };
                let text = self.text.clone();
                if self.existing || !empty {
                    doc.apply(label, |p| {
                        if let Some(s) = p.workbook_mut(book).and_then(|b| b.sheets.get_mut(sheet)) {
                            s.set_note(column, row, &text);
                        }
                    });
                }
                false
            }
            Some(DialogButton::Cancel) => false,
            _ => open,
        }
    }
}
