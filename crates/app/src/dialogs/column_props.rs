//! The column properties window: names, units, comments, plotting role and
//! number format of one column.

use eframe::egui::{self, DragValue};
use sheaf_core::format::NumberFormat;
use sheaf_core::{ColumnId, Designation, Document, Project};

pub struct ColumnPropsDialog {
    column: ColumnId,
    short_name: String,
    long_name: String,
    units: String,
    comments: String,
    designation: Designation,
    format: NumberFormat,
    problem: Option<String>,
}

pub fn designation_label(d: Designation) -> &'static str {
    match d {
        Designation::X => "X",
        Designation::Y => "Y",
        Designation::Z => "Z",
        Designation::YError => "Y error",
        Designation::XError => "X error",
        Designation::Label => "Label",
        Designation::Disregard => "Disregard",
    }
}

impl ColumnPropsDialog {
    pub fn new(project: &Project, column: ColumnId) -> Option<Self> {
        let c = project.column(column)?;
        Some(ColumnPropsDialog {
            column,
            short_name: c.short_name.clone(),
            long_name: c.long_name.clone(),
            units: c.units.clone(),
            comments: c.comments.clone(),
            designation: c.designation,
            format: c.format,
            problem: None,
        })
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().column(self.column).is_none() {
            return false;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new("Column properties")
            .id(egui::Id::new("column_props"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("column_props_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Short name");
                    ui.add(egui::TextEdit::singleline(&mut self.short_name).desired_width(90.0));
                    ui.end_row();
                    ui.label("Long name");
                    ui.add(egui::TextEdit::singleline(&mut self.long_name).desired_width(240.0));
                    ui.end_row();
                    ui.label("Units");
                    ui.add(egui::TextEdit::singleline(&mut self.units).desired_width(120.0));
                    ui.end_row();
                    ui.label("Comments");
                    ui.add(egui::TextEdit::multiline(&mut self.comments).desired_rows(2).desired_width(240.0));
                    ui.end_row();
                    ui.label("Plot as");
                    egui::ComboBox::from_id_salt("column_designation")
                        .selected_text(designation_label(self.designation))
                        .show_ui(ui, |ui| {
                            for d in Designation::ALL {
                                ui.selectable_value(&mut self.designation, d, designation_label(d));
                            }
                        });
                    ui.end_row();
                    ui.label("Number format");
                    ui.horizontal(|ui| format_ui(ui, &mut self.format));
                    ui.end_row();
                });
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() && self.apply(doc) {
                        close = true;
                    }
                    if ui.button("Apply").clicked() {
                        self.apply(doc);
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        open && !close
    }

    /// Writes the fields to the column. Returns `false` when the short name is not usable.
    fn apply(&mut self, doc: &mut Document) -> bool {
        let name = self.short_name.trim().to_string();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.problem = Some("The short name may only have letters, digits and underscores.".to_string());
            return false;
        }
        let taken = doc.project().locate(self.column).is_some_and(|(_, sheet, _)| {
            sheet.columns.iter().any(|c| c.id != self.column && c.short_name.eq_ignore_ascii_case(&name))
        });
        if taken {
            self.problem = Some(format!("Another column of this sheet is already named '{name}'."));
            return false;
        }
        self.problem = None;
        let id = self.column;
        let (long_name, units, comments) = (self.long_name.clone(), self.units.clone(), self.comments.clone());
        let (designation, format) = (self.designation, self.format);
        doc.apply("Change column properties", move |p| {
            if let Some(c) = p.column_mut(id) {
                c.short_name = name;
                c.long_name = long_name;
                c.units = units;
                c.comments = comments;
                c.designation = designation;
                c.format = format;
            }
        });
        true
    }
}

fn format_ui(ui: &mut egui::Ui, format: &mut NumberFormat) {
    use sheaf_core::datetime::{DatePattern, TimePattern};
    let (mut kind, digits) = match *format {
        NumberFormat::General => (0, 3),
        NumberFormat::Decimal(d) => (1, d),
        NumberFormat::Scientific(d) => (2, d),
        NumberFormat::Significant(d) => (3, d),
        NumberFormat::Date(_) => (4, 0),
        NumberFormat::Time(_) => (5, 0),
        NumberFormat::DateTime(..) => (6, 0),
    };
    let names = ["Automatic", "Decimal places", "Scientific", "Significant digits", "Date", "Time", "Date and time"];
    egui::ComboBox::from_id_salt("column_format").selected_text(names[kind]).show_ui(ui, |ui| {
        for (i, n) in names.iter().enumerate() {
            ui.selectable_value(&mut kind, i, *n);
        }
    });
    if kind >= 4 {
        // The patterns of the current format, or the defaults when the kind has just been chosen.
        let (mut date, mut time) = match *format {
            NumberFormat::Date(d) => (d, TimePattern::Hms),
            NumberFormat::Time(t) => (DatePattern::Iso, t),
            NumberFormat::DateTime(d, t) => (d, t),
            _ => (DatePattern::Iso, TimePattern::Hms),
        };
        if kind != 5 {
            egui::ComboBox::from_id_salt("column_date_pattern").selected_text(date.label()).show_ui(ui, |ui| {
                for p in DatePattern::ALL {
                    ui.selectable_value(&mut date, p, p.label());
                }
            });
        }
        if kind != 4 {
            egui::ComboBox::from_id_salt("column_time_pattern").selected_text(time.label()).show_ui(ui, |ui| {
                for p in TimePattern::ALL {
                    ui.selectable_value(&mut time, p, p.label());
                }
            });
        }
        *format = match kind {
            4 => NumberFormat::Date(date),
            5 => NumberFormat::Time(time),
            _ => NumberFormat::DateTime(date, time),
        };
        return;
    }
    let mut d = digits;
    if kind != 0 {
        ui.add(DragValue::new(&mut d).range(0..=15));
    }
    *format = match kind {
        1 => NumberFormat::Decimal(d),
        2 => NumberFormat::Scientific(d),
        3 => NumberFormat::Significant(d.max(1)),
        _ => NumberFormat::General,
    };
}
