//! Column > Filter > Add or Remove Filter: a window that sets the condition of the worksheet filter
//! (rows that fail it are hidden, not deleted), with Origin's numeric presets for the column the
//! window was opened from.

use super::frame::{DialogButton, button_row};
use eframe::egui::{self, RichText};
use sheaf_core::{Document, WindowId};

pub struct RowFilterDialog {
    pub book: WindowId,
    sheet: usize,
    /// Short name of the column the presets are written for.
    column: String,
    condition: String,
    had_filter: bool,
    problem: Option<String>,
    focus: bool,
}

/// The presets: a label and the condition for a column.
pub fn presets(column: &str) -> [(&'static str, String); 4] {
    let c = format!("col({column})");
    [
        ("Above average", format!("{c} > mean({c})")),
        ("Below average", format!("{c} < mean({c})")),
        ("Within 3 sigma of the mean", format!("abs({c} - mean({c})) <= 3 * sd({c})")),
        ("Not missing", format!("{c} == {c}")),
    ]
}

impl RowFilterDialog {
    pub fn new(doc: &Document, book: WindowId, column: Option<sheaf_core::ColumnId>) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet_index = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let sheet = &b.sheets[sheet_index];
        let short = column
            .and_then(|id| sheet.column(id))
            .or_else(|| sheet.columns.iter().find(|c| c.data.is_numeric()))
            .map(|c| c.short_name.clone())?;
        let existing = sheet.filter.as_ref();
        Some(RowFilterDialog {
            book,
            sheet: sheet_index,
            column: short,
            condition: existing.map(|f| f.condition.clone()).unwrap_or_default(),
            had_filter: existing.is_some(),
            problem: None,
            focus: true,
        })
    }

    /// Draws the window; false when it closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)).is_none() {
            return false;
        }
        let mut open = true;
        let mut pressed = None;
        let mut remove = false;
        egui::Window::new("Worksheet Filter")
            .id(egui::Id::new("row_filter"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Show only the rows for which this condition holds:");
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.condition)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(420.0)
                        .hint_text("for example  col(B) > 0.5"),
                );
                if self.focus {
                    edit.request_focus();
                    self.focus = false;
                }
                ui.horizontal(|ui| {
                    ui.label("Preset");
                    egui::ComboBox::from_id_salt("row_filter_preset").selected_text("Choose...").show_ui(ui, |ui| {
                        for (label, text) in presets(&self.column) {
                            if ui.selectable_label(false, label).clicked() {
                                self.condition = text;
                            }
                        }
                    });
                    ui.weak(format!("(for column {})", self.column));
                });
                ui.weak("The hidden rows stay in the sheet; analysis and fits leave them out.");
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), RichText::new(p));
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.had_filter, egui::Button::new("Remove Filter")).clicked() {
                        remove = true;
                    }
                    pressed = button_row(ui, false);
                });
            });
        let (book, sheet) = (self.book, self.sheet);
        if remove {
            doc.apply("Remove filter", |p| p.remove_row_filter(book, sheet));
            return false;
        }
        match pressed {
            Some(DialogButton::Ok) => {
                let condition = self.condition.clone();
                match doc.edit("Filter rows", |p| p.set_row_filter(book, sheet, &condition)) {
                    Ok(_) => return false,
                    Err(e) => self.problem = Some(e.to_string()),
                }
            }
            Some(DialogButton::Cancel) => return false,
            _ => {}
        }
        open
    }
}
