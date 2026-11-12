//! The "Set Column Values" window: fill a column from a formula over the
//! other columns of its sheet.

use eframe::egui::{self, DragValue, RichText};
use sheaf_core::{ColumnId, Document, Project, RecalcMode};

pub struct SetValuesDialog {
    column: ColumnId,
    formula: String,
    /// Fill all rows of the sheet, or only `from..=to` (1-based, as shown in the grid).
    all_rows: bool,
    from: usize,
    to: usize,
    /// Whether and when the formula runs again, as in Origin's Recalculate choice.
    mode: RecalcMode,
    problem: Option<String>,
    focus: bool,
}

impl SetValuesDialog {
    pub fn new(project: &Project, column: ColumnId) -> Option<Self> {
        let (_, sheet, c) = project.locate(column)?;
        Some(SetValuesDialog {
            column,
            formula: c.formula.clone().unwrap_or_default(),
            all_rows: c.formula_rows.is_none(),
            from: c.formula_rows.as_ref().map_or(1, |r| r.start + 1),
            to: c.formula_rows.as_ref().map_or(sheet.row_count().max(1), |r| r.end),
            // Origin opens the dialog on Manual for a column that has no recalculating formula.
            mode: if c.is_locked() { c.recalc } else { RecalcMode::Manual },
            problem: None,
            focus: true,
        })
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        let Some((_, sheet, column)) = doc.project().locate(self.column) else {
            return false;
        };
        let target = column.short_name.clone();
        let others: Vec<(String, String)> = sheet
            .columns
            .iter()
            .map(|c| (c.short_name.clone(), if c.units.is_empty() { c.long_name.clone() } else { c.axis_title() }))
            .collect();

        let mut open = true;
        let mut close = false;
        let mut apply = false;
        egui::Window::new(format!("Set column values: {target}"))
            .id(egui::Id::new("set_values"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("col({target}) =")).monospace());
                    let edit = ui.add(
                        egui::TextEdit::multiline(&mut self.formula)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(3)
                            .desired_width(380.0)
                            .hint_text("for example  col(B)*1000"),
                    );
                    if self.focus {
                        edit.request_focus();
                        self.focus = false;
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.weak("Insert:");
                    for (short, label) in &others {
                        let button = ui.small_button(short);
                        let button = if label.is_empty() { button } else { button.on_hover_text(label) };
                        if button.clicked() {
                            self.formula.push_str(&format!("col({short})"));
                        }
                    }
                });
                ui.weak("i is the row number.  Functions: abs sqrt exp ln log sin cos tan min max mean sum ...");
                ui.weak("Operators: + - * / ^   comparisons   cond ? a : b");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Rows");
                    ui.radio_value(&mut self.all_rows, true, "All");
                    ui.radio_value(&mut self.all_rows, false, "From");
                    ui.add_enabled_ui(!self.all_rows, |ui| {
                        ui.add(DragValue::new(&mut self.from).range(1..=sheaf_core::formula::MAX_FORMULA_ROWS));
                        ui.label("to");
                        ui.add(DragValue::new(&mut self.to).range(1..=sheaf_core::formula::MAX_FORMULA_ROWS));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Recalculate");
                    for m in RecalcMode::ALL {
                        let hint = match m {
                            RecalcMode::None => "Run the formula once; the values are plain data afterwards",
                            RecalcMode::Auto => "Recalculate whenever a column the formula reads changes",
                            RecalcMode::Manual => "Mark the column when an input changes; recalculate by hand",
                        };
                        ui.radio_value(&mut self.mode, m, m.label()).on_hover_text(hint);
                    }
                });
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        apply = true;
                        close = true;
                    }
                    if ui.button("Apply").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });

        if apply {
            let rows = (!self.all_rows).then(|| (self.from - 1)..self.to.max(self.from));
            let (id, formula, mode) = (self.column, self.formula.clone(), self.mode);
            match doc.edit("Set column values", move |p| p.set_column_values_with(id, &formula, rows, mode)) {
                Ok(()) => self.problem = None,
                Err(e) => {
                    self.problem = Some(e.to_string());
                    close = false;
                }
            }
        }
        open && !close
    }
}
