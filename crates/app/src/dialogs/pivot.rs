//! Worksheet > Pivot Table: the window of Origin's `wpivot`. The sources start from the selected
//! columns (row source, column source, data, in that order) or the sheet's first columns; the
//! table goes to a new sheet of the workbook. Origin's Combine (Others), extra values and the
//! user-parameter label position are not built and are shown disabled.

use super::frame::{DialogButton, button_row};
use super::reshape::ReshapeOutcome;
use eframe::egui;
use sheaf_core::pivot::{InfoPosition, Method, Normalize, PivotOptions, Sort};
use sheaf_core::{ColumnId, Document, WindowId};

pub struct PivotDialog {
    pub book: WindowId,
    sheet: usize,
    /// Row source columns (in sheet order when the table is built).
    rows: Vec<ColumnId>,
    column: Option<ColumnId>,
    data: Option<ColumnId>,
    method: Method,
    row_totals: bool,
    column_totals: bool,
    sort_rows: Sort,
    sort_columns: Sort,
    normalize: Normalize,
    zeros: bool,
    info: InfoPosition,
    problem: Option<String>,
}

impl PivotDialog {
    pub fn new(doc: &Document, book: WindowId, selected: Vec<ColumnId>) -> Option<Self> {
        let b = doc.project().workbook(book)?;
        let sheet_index = b.active_sheet.min(b.sheets.len().checked_sub(1)?);
        let sheet = &b.sheets[sheet_index];
        let mut pick: Vec<ColumnId> = selected.into_iter().filter(|c| sheet.column(*c).is_some()).collect();
        for c in &sheet.columns {
            if pick.len() >= 3 {
                break;
            }
            if !pick.contains(&c.id) {
                pick.push(c.id);
            }
        }
        let defaults = PivotOptions::new(Vec::new(), ColumnId(0), None);
        Some(PivotDialog {
            book,
            sheet: sheet_index,
            rows: pick.first().copied().into_iter().collect(),
            column: pick.get(1).copied(),
            data: pick.get(2).copied().or(pick.get(1).copied()),
            method: defaults.method,
            row_totals: defaults.row_totals,
            column_totals: defaults.column_totals,
            sort_rows: defaults.sort_rows,
            sort_columns: defaults.sort_columns,
            normalize: defaults.normalize,
            zeros: defaults.zeros,
            info: defaults.info,
            problem: None,
        })
    }

    fn options(&self) -> Result<PivotOptions, String> {
        if self.rows.is_empty() {
            return Err("Choose at least one row source column".into());
        }
        let column = self.column.ok_or("Choose the column source")?;
        Ok(PivotOptions {
            method: self.method,
            row_totals: self.row_totals,
            column_totals: self.column_totals,
            sort_rows: self.sort_rows,
            sort_columns: self.sort_columns,
            normalize: self.normalize,
            zeros: self.zeros,
            info: self.info,
            ..PivotOptions::new(self.rows.clone(), column, self.data)
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> ReshapeOutcome {
        let Some(sheet) = doc.project().workbook(self.book).and_then(|b| b.sheets.get(self.sheet)) else {
            return ReshapeOutcome::Closed;
        };
        let names: Vec<(ColumnId, String)> = sheet
            .columns
            .iter()
            .map(|c| {
                let n = if c.long_name.is_empty() {
                    c.short_name.clone()
                } else {
                    format!("{} ({})", c.short_name, c.long_name)
                };
                (c.id, n)
            })
            .collect();
        let name_of = |id: Option<ColumnId>| {
            id.and_then(|id| names.iter().find(|(c, _)| *c == id)).map_or("(none)".to_string(), |(_, n)| n.clone())
        };
        self.rows.retain(|r| names.iter().any(|(c, _)| c == r));
        let mut open = true;
        let mut pressed = None;
        egui::Window::new("Pivot Table")
            .id(egui::Id::new("pivot_table"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("pivot_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Row Source");
                    ui.vertical(|ui| {
                        for (id, n) in &names {
                            let mut on = self.rows.contains(id);
                            if ui.checkbox(&mut on, n).changed() {
                                if on {
                                    self.rows.push(*id);
                                } else {
                                    self.rows.retain(|r| r != id);
                                }
                            }
                        }
                    });
                    ui.end_row();
                    ui.label("Column Source");
                    egui::ComboBox::from_id_salt("pivot_col").selected_text(name_of(self.column)).show_ui(ui, |ui| {
                        for (id, n) in &names {
                            ui.selectable_value(&mut self.column, Some(*id), n);
                        }
                    });
                    ui.end_row();
                    ui.label("Summarize by");
                    egui::ComboBox::from_id_salt("pivot_method").selected_text(self.method.label()).show_ui(ui, |ui| {
                        for m in Method::ALL {
                            ui.selectable_value(&mut self.method, m, m.label());
                        }
                    });
                    ui.end_row();
                    ui.label("Data Source");
                    ui.add_enabled_ui(self.method != Method::Count, |ui| {
                        egui::ComboBox::from_id_salt("pivot_data").selected_text(name_of(self.data)).show_ui(
                            ui,
                            |ui| {
                                for (id, n) in &names {
                                    ui.selectable_value(&mut self.data, Some(*id), n);
                                }
                            },
                        );
                    });
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(&mut self.row_totals, "Row Totals");
                    ui.end_row();
                    ui.label("Sort Rows");
                    egui::ComboBox::from_id_salt("pivot_sort_rows").selected_text(self.sort_rows.label(true)).show_ui(
                        ui,
                        |ui| {
                            for s in Sort::ROWS {
                                ui.selectable_value(&mut self.sort_rows, s, s.label(true));
                            }
                        },
                    );
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(&mut self.column_totals, "Column Totals");
                    ui.end_row();
                    ui.label("Sort Columns");
                    egui::ComboBox::from_id_salt("pivot_sort_cols")
                        .selected_text(self.sort_columns.label(false))
                        .show_ui(ui, |ui| {
                            for s in Sort::COLUMNS {
                                ui.selectable_value(&mut self.sort_columns, s, s.label(false));
                            }
                        });
                    ui.end_row();
                    ui.label("Normalize");
                    let norm = |n: Normalize| match n {
                        Normalize::None => "None",
                        Normalize::Fraction => "Fraction",
                        Normalize::Percent => "Percent",
                    };
                    egui::ComboBox::from_id_salt("pivot_norm").selected_text(norm(self.normalize)).show_ui(ui, |ui| {
                        for n in [Normalize::None, Normalize::Fraction, Normalize::Percent] {
                            ui.selectable_value(&mut self.normalize, n, norm(n));
                        }
                    });
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(&mut self.zeros, "Show Missing as 0");
                    ui.end_row();
                    ui.label("Column Info Position");
                    let pos = |p: InfoPosition| match p {
                        InfoPosition::LongName => "Long Name",
                        InfoPosition::Comments => "Comments",
                        InfoPosition::AppendToLongName => "Append to Long Name",
                    };
                    egui::ComboBox::from_id_salt("pivot_pos").selected_text(pos(self.info)).show_ui(ui, |ui| {
                        for p in [InfoPosition::LongName, InfoPosition::Comments, InfoPosition::AppendToLongName] {
                            ui.selectable_value(&mut self.info, p, pos(p));
                        }
                        ui.add_enabled(false, egui::Button::new("User Defined Parameters"));
                    });
                    ui.end_row();
                    ui.label("Combine Direction");
                    ui.add_enabled_ui(false, |ui| {
                        egui::ComboBox::from_id_salt("pivot_combine").selected_text("None").show_ui(ui, |_| {});
                    });
                    ui.end_row();
                });
                ui.weak("The table goes to a new sheet of this workbook; the data stays as it is.");
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => {
                // Row sources in sheet order.
                let order: Vec<ColumnId> = names.iter().map(|(c, _)| *c).filter(|c| self.rows.contains(c)).collect();
                self.rows = order;
                match self.options() {
                    Err(e) => self.problem = Some(e),
                    Ok(o) => {
                        let (book, sheet) = (self.book, self.sheet);
                        match doc.edit("Pivot table", move |p| p.pivot_table(book, sheet, &o)) {
                            Ok(index) => return ReshapeOutcome::Done(index),
                            Err(e) => self.problem = Some(e.to_string()),
                        }
                    }
                }
            }
            Some(DialogButton::Cancel) => return ReshapeOutcome::Closed,
            _ => {}
        }
        if open { ReshapeOutcome::Open } else { ReshapeOutcome::Closed }
    }
}
