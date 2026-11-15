//! Column > Mask Cells by Condition...: a window that masks the cells of the selected columns
//! that meet a condition (outliers by standard deviation, or a comparison with a value).

use super::frame::{DialogButton, button_row};
use eframe::egui::{self, DragValue};
use sheaf_core::sheetops::MaskCondition;
use sheaf_core::{ColumnId, Document, WindowId};

/// The conditions in the order of the Condition list.
pub const CONDITIONS: [&str; 6] = ["Outliers by Std. Deviation", ">", ">=", "<", "<=", "="];

/// Builds the condition for a list entry (`kind` indexes `CONDITIONS`).
pub fn condition(kind: usize, multiple: u8, value: f64) -> MaskCondition {
    match kind {
        0 => MaskCondition::OutliersBySd(multiple),
        1 => MaskCondition::Greater(value),
        2 => MaskCondition::GreaterOrEqual(value),
        3 => MaskCondition::Less(value),
        4 => MaskCondition::LessOrEqual(value),
        _ => MaskCondition::Equal(value),
    }
}

pub struct MaskConditionDialog {
    pub book: WindowId,
    columns: Vec<ColumnId>,
    input: String,
    kind: usize,
    multiple: u8,
    pub(crate) value: f64,
    absolute: bool,
    keep: bool,
}

impl MaskConditionDialog {
    pub fn new(doc: &Document, book: WindowId, columns: Vec<ColumnId>) -> Option<Self> {
        let sheet = doc.project().workbook(book)?.sheet();
        let names: Vec<String> =
            columns.iter().filter_map(|id| sheet.column(*id)).map(|c| c.short_name.clone()).collect();
        if names.is_empty() {
            return None;
        }
        Some(MaskConditionDialog {
            book,
            columns,
            input: names.join(", "),
            kind: 0,
            multiple: 1,
            value: 0.0,
            absolute: false,
            keep: false,
        })
    }

    /// Draws the window; false when it closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().workbook(self.book).is_none() {
            return false;
        }
        let mut open = true;
        let mut pressed = None;
        egui::Window::new("Mask Cells by Condition")
            .id(egui::Id::new("mask_condition"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("mask_condition_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Input");
                    let mut shown = self.input.clone();
                    ui.add(egui::TextEdit::singleline(&mut shown).interactive(false).desired_width(220.0));
                    ui.end_row();
                    ui.label("Condition");
                    egui::ComboBox::from_id_salt("mask_condition_kind")
                        .selected_text(CONDITIONS[self.kind])
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            for (i, name) in CONDITIONS.iter().enumerate() {
                                ui.selectable_value(&mut self.kind, i, *name);
                            }
                        });
                    ui.end_row();
                    if self.kind == 0 {
                        ui.label("Multiple of SD");
                        egui::ComboBox::from_id_salt("mask_condition_multiple")
                            .selected_text(self.multiple.to_string())
                            .show_ui(ui, |ui| {
                                for m in 1..=6u8 {
                                    ui.selectable_value(&mut self.multiple, m, m.to_string());
                                }
                            });
                        ui.end_row();
                    } else {
                        ui.label("Value");
                        ui.add(DragValue::new(&mut self.value).speed(0.1));
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut self.absolute, "Use Absolute Values to Test");
                        ui.end_row();
                    }
                    ui.label("");
                    ui.checkbox(&mut self.keep, "Keep Existing Mask");
                    ui.end_row();
                });
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => {
                let cond = condition(self.kind, self.multiple, self.value);
                let (book, columns, absolute, keep) = (self.book, self.columns.clone(), self.absolute, self.keep);
                doc.apply("Mask cells by condition", |p| {
                    let Some(ws) = p.workbook_mut(book).map(|b| b.sheet_mut()) else { return };
                    let positions: Vec<usize> =
                        columns.iter().filter_map(|id| ws.columns.iter().position(|c| c.id == *id)).collect();
                    ws.mask_by_condition(&positions, cond, absolute, keep);
                });
                false
            }
            Some(DialogButton::Cancel) => false,
            _ => open,
        }
    }
}
