//! The windows of Worksheet > Transpose, Stack Columns, Unstack Columns, Split Columns and Reduce
//! Rows. Each writes a new sheet of the workbook from the selected columns (all columns when none
//! is selected).

use super::frame::{DialogButton, button_row};
use eframe::egui;
use sheaf_core::reshape::Reduce;
use sheaf_core::{ColumnId, Document, WindowId};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReshapeKind {
    Transpose,
    Stack,
    Unstack,
    Split,
    Reduce,
}

impl ReshapeKind {
    pub fn title(&self) -> &'static str {
        match self {
            ReshapeKind::Transpose => "Transpose",
            ReshapeKind::Stack => "Stack Columns",
            ReshapeKind::Unstack => "Unstack Columns",
            ReshapeKind::Split => "Split Columns",
            ReshapeKind::Reduce => "Reduce Rows",
        }
    }
}

pub struct ReshapeDialog {
    pub book: WindowId,
    sheet: usize,
    kind: ReshapeKind,
    /// The columns the operation reads (empty: every column).
    columns: Vec<ColumnId>,
    /// Unstack: the column that names the groups and the column that holds the values.
    key: Option<ColumnId>,
    value: Option<ColumnId>,
    /// Split: rows per piece. Reduce: rows per group.
    rows: u32,
    how: Reduce,
    problem: Option<String>,
}

pub enum ReshapeOutcome {
    Open,
    Closed,
    /// The operation ran and made this sheet.
    Done(usize),
}

impl ReshapeDialog {
    pub fn new(doc: &Document, book: WindowId, kind: ReshapeKind, columns: Vec<ColumnId>) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet_index = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let sheet = &b.sheets[sheet_index];
        let columns: Vec<ColumnId> = columns.into_iter().filter(|c| sheet.column(*c).is_some()).collect();
        let first = sheet.columns.first().map(|c| c.id);
        let second = sheet.columns.get(1).map(|c| c.id).or(first);
        Some(ReshapeDialog {
            book,
            sheet: sheet_index,
            kind,
            key: columns
                .first()
                .copied()
                .or_else(|| sheet.columns.iter().find(|c| c.categorical).map(|c| c.id))
                .or(first),
            value: columns.get(1).copied().or(second),
            columns,
            rows: 2,
            how: Reduce::Mean,
            problem: None,
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> ReshapeOutcome {
        let Some(sheet) = doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)) else {
            return ReshapeOutcome::Closed;
        };
        let choices: Vec<(ColumnId, String)> = sheet
            .columns
            .iter()
            .map(|c| {
                let t = if c.long_name.is_empty() {
                    c.short_name.clone()
                } else {
                    format!("{} ({})", c.short_name, c.long_name)
                };
                (c.id, t)
            })
            .collect();
        let name_of = |id: Option<ColumnId>| {
            id.and_then(|i| choices.iter().find(|c| c.0 == i)).map(|c| c.1.clone()).unwrap_or_default()
        };
        let scope = if self.columns.is_empty() {
            "all columns of the sheet".to_string()
        } else {
            format!("{} selected column(s)", self.columns.len())
        };
        let mut open = true;
        let mut pressed = None;
        egui::Window::new(self.kind.title())
            .id(egui::Id::new("reshape_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("reshape_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| match self.kind {
                    ReshapeKind::Unstack => {
                        for (label, slot, salt) in [
                            ("Key column", &mut self.key, "unstack_key"),
                            ("Value column", &mut self.value, "unstack_value"),
                        ] {
                            ui.label(label);
                            egui::ComboBox::from_id_salt(salt).selected_text(name_of(*slot)).show_ui(ui, |ui| {
                                for (id, text) in &choices {
                                    ui.selectable_value(slot, Some(*id), text);
                                }
                            });
                            ui.end_row();
                        }
                    }
                    ReshapeKind::Split | ReshapeKind::Reduce => {
                        ui.label("Input");
                        ui.label(&scope);
                        ui.end_row();
                        ui.label(if self.kind == ReshapeKind::Split { "Rows per piece" } else { "Rows per group" });
                        ui.add(egui::DragValue::new(&mut self.rows).range(1..=1_000_000));
                        ui.end_row();
                        if self.kind == ReshapeKind::Reduce {
                            ui.label("Method");
                            egui::ComboBox::from_id_salt("reduce_how").selected_text(self.how.label()).show_ui(
                                ui,
                                |ui| {
                                    for h in Reduce::ALL {
                                        ui.selectable_value(&mut self.how, h, h.label());
                                    }
                                },
                            );
                            ui.end_row();
                        }
                    }
                    _ => {
                        ui.label("Input");
                        ui.label(&scope);
                        ui.end_row();
                    }
                });
                ui.weak("The result goes to a new sheet of this workbook; the data stays as it is.");
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => {
                let (book, sheet, columns, rows, how) =
                    (self.book, self.sheet, self.columns.clone(), self.rows as usize, self.how);
                let (key, value) = (self.key, self.value);
                let kind = self.kind;
                let result = doc.edit(kind.title(), move |p| match kind {
                    ReshapeKind::Transpose => p.transpose(book, sheet, &columns),
                    ReshapeKind::Stack => p.stack_columns(book, sheet, &columns),
                    ReshapeKind::Unstack => match (key, value) {
                        (Some(k), Some(v)) => p.unstack_columns(book, sheet, k, v),
                        _ => Err(sheaf_core::analysis::AnalysisError::BadOption(
                            "choose the key and value columns".into(),
                        )),
                    },
                    ReshapeKind::Split => p.split_columns(book, sheet, &columns, rows),
                    ReshapeKind::Reduce => p.reduce_rows(book, sheet, &columns, rows, how),
                });
                match result {
                    Ok(index) => return ReshapeOutcome::Done(index),
                    Err(e) => self.problem = Some(e.to_string()),
                }
            }
            Some(DialogButton::Cancel) => return ReshapeOutcome::Closed,
            _ => {}
        }
        if open { ReshapeOutcome::Open } else { ReshapeOutcome::Closed }
    }
}
