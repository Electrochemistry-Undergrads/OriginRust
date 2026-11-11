//! The "Set Column Values" window: fill a column from a formula over the
//! other columns of its sheet.

use eframe::egui::{self, DragValue, RichText};
use sheaf_core::{ColumnId, Document, Project};

pub struct SetValuesDialog {
    column: ColumnId,
    formula: String,
    /// Fill all rows of the sheet, or only `from..=to` (1-based, as shown in the grid).
    all_rows: bool,
    from: usize,
    to: usize,
    problem: Option<String>,
    focus: bool,
}

impl SetValuesDialog {
    pub fn new(project: &Project, column: ColumnId) -> Option<Self> {
        let (_, sheet, c) = project.locate(column)?;
        Some(SetValuesDialog {
            column,
            formula: c.formula.clone().unwrap_or_default(),
            all_rows: true,
            from: 1,
            to: sheet.row_count().max(1),
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
                        ui.add(DragValue::new(&mut self.from).range(1..=usize::MAX));
                        ui.label("to");
                        ui.add(DragValue::new(&mut self.to).range(1..=usize::MAX));
                    });
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
            let (id, formula) = (self.column, self.formula.clone());
            match doc.edit("Set column values", move |p| p.set_column_values(id, &formula, rows)) {
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
