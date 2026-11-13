//! The Column Properties window, in Origin's layout: Prev and Next buttons at the top, the names
//! (Short Name, Long Name, Units, Comments), a group "Options" with Plot Designation, Format,
//! Display, Column Width and "Apply to all columns to the right", and OK, Cancel, Apply at the
//! bottom right. Nothing reaches the project before OK, Apply, Prev or Next.

use super::frame::{DialogButton, button_row};
use super::graph_props::group_box;
use eframe::egui::{self, DragValue};
use sheaf_core::column::Storage;
use sheaf_core::format::{NameStyle, NumberFormat};
use sheaf_core::{ColumnId, Designation, Document, Project};

pub use sheaf_core::column::MAX_SHORT_NAME;

pub struct ColumnPropsDialog {
    column: ColumnId,
    short_name: String,
    long_name: String,
    units: String,
    comments: String,
    designation: Designation,
    format: NumberFormat,
    /// How the cells are kept (Numeric, Text, Text & Numeric); the data side of Format.
    storage: Storage,
    /// The width in characters the worksheet shows the column at (8 is Origin's default).
    width: u32,
    apply_to_right: bool,
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
        Designation::Group => "Group",
        Designation::Subject => "Subject",
        Designation::Disregard => "Disregard",
    }
}

/// What the Prev and Next buttons did this frame.
enum Step {
    Prev,
    Next,
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
            storage: c.storage(),
            width: u32::from(c.shown_width()),
            apply_to_right: false,
            problem: None,
        })
    }

    /// The column the window shows (for tests).
    #[cfg(test)]
    pub fn column(&self) -> ColumnId {
        self.column
    }

    /// Loads the fields of another column of the same sheet.
    fn load(&mut self, project: &Project, column: ColumnId) {
        let apply_to_right = self.apply_to_right;
        if let Some(fresh) = ColumnPropsDialog::new(project, column) {
            *self = ColumnPropsDialog { apply_to_right, ..fresh };
        }
    }

    /// Whether the fields differ from what the column holds now.
    fn dirty(&self, project: &Project) -> bool {
        let Some(c) = project.column(self.column) else {
            return false;
        };
        self.short_name.trim() != c.short_name
            || self.long_name != c.long_name
            || self.units != c.units
            || self.comments != c.comments
            || self.designation != c.designation
            || self.format != c.format
            || self.storage != c.storage()
            || self.width != u32::from(c.shown_width())
    }

    /// The title: the window and sheet that hold the column.
    fn title(&self, project: &Project) -> String {
        match project.locate(self.column) {
            Some((book, sheet, c)) => format!("Column Properties - [{}]{}!{}", book.name, sheet.name, c.short_name),
            None => "Column Properties".to_string(),
        }
    }

    /// The neighbours of the column in its sheet: (previous, next).
    fn neighbours(&self, project: &Project) -> (Option<ColumnId>, Option<ColumnId>) {
        let Some((_, sheet, _)) = project.locate(self.column) else {
            return (None, None);
        };
        let Some(i) = sheet.column_index(self.column) else {
            return (None, None);
        };
        (i.checked_sub(1).map(|j| sheet.columns[j].id), sheet.columns.get(i + 1).map(|c| c.id))
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if doc.project().column(self.column).is_none() {
            return false;
        }
        let title = self.title(doc.project());
        let (prev, next) = self.neighbours(doc.project());
        let dirty = self.dirty(doc.project());
        let mut open = true;
        let mut pressed: Option<DialogButton> = None;
        let mut step: Option<Step> = None;
        egui::Window::new(title)
            .id(egui::Id::new("column_props"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(prev.is_some(), egui::Button::new("<< Prev")).clicked() {
                        step = Some(Step::Prev);
                    }
                    if ui.add_enabled(next.is_some(), egui::Button::new("Next >>")).clicked() {
                        step = Some(Step::Next);
                    }
                });
                ui.add_space(4.0);
                egui::Grid::new("column_props_names").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Short Name");
                    ui.add(egui::TextEdit::singleline(&mut self.short_name).desired_width(120.0));
                    ui.end_row();
                    ui.label("Long Name");
                    ui.add(egui::TextEdit::singleline(&mut self.long_name).desired_width(290.0));
                    ui.end_row();
                    ui.label("Units");
                    ui.add(egui::TextEdit::singleline(&mut self.units).desired_width(290.0));
                    ui.end_row();
                    ui.label("Comments");
                    ui.add(egui::TextEdit::multiline(&mut self.comments).desired_rows(3).desired_width(290.0));
                    ui.end_row();
                });
                group_box(ui, "Options", |ui| {
                    egui::Grid::new("column_props_options").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                        ui.label("Plot Designation");
                        egui::ComboBox::from_id_salt("column_designation")
                            .selected_text(designation_label(self.designation))
                            .show_ui(ui, |ui| {
                                for d in Designation::ALL {
                                    ui.selectable_value(&mut self.designation, d, designation_label(d));
                                }
                            });
                        ui.end_row();
                        format_rows(ui, &mut self.storage, &mut self.format);
                        ui.label("Column Width");
                        ui.add(DragValue::new(&mut self.width).range(1..=255));
                        ui.end_row();
                    });
                    ui.checkbox(&mut self.apply_to_right, "Apply to all columns to the right")
                        .on_hover_text("Plot Designation and Format go to every column after this one in the sheet");
                });
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::from_rgb(170, 30, 30), p);
                }
                ui.add_space(6.0);
                pressed = button_row(ui, dirty || self.apply_to_right);
            });

        if let Some(s) = step {
            // The changes of this column are kept (as Apply does) before the next one is shown.
            let target = match s {
                Step::Prev => prev,
                Step::Next => next,
            };
            let keep = !self.dirty(doc.project()) || self.apply(doc);
            if let (true, Some(t)) = (keep, target) {
                self.load(doc.project(), t);
            }
        }
        match pressed {
            Some(DialogButton::Ok) => {
                if self.apply(doc) {
                    return false;
                }
            }
            Some(DialogButton::Apply) => {
                self.apply(doc);
            }
            Some(DialogButton::Cancel) => return false,
            None => {}
        }
        open
    }

    /// Writes the fields to the column (and, when asked, the options to the columns to its
    /// right). Returns `false` when the short name is not usable.
    fn apply(&mut self, doc: &mut Document) -> bool {
        // As in Origin, the box keeps letters, digits and dots, drops every other character and cuts
        // the name to 18 characters; the cleaned name is shown again in the box.
        let name = sheaf_core::column::clean_short_name(&self.short_name);
        self.short_name = name.clone();
        if name.is_empty() {
            self.problem = Some("The short name needs at least one letter or digit.".to_string());
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
        let (designation, format, to_right) = (self.designation, self.format, self.apply_to_right);
        let storage = self.storage;
        let width = (self.width as u16 != sheaf_core::column::DEFAULT_WIDTH).then_some(self.width as u16);
        doc.apply("Change column properties", move |p| {
            if let Some(c) = p.column_mut(id) {
                c.short_name = name;
                c.long_name = long_name;
                c.units = units;
                c.comments = comments;
                c.designation = designation;
                c.convert_storage(storage);
                c.format = format;
                c.width = width;
            }
            if to_right {
                let right: Vec<ColumnId> = p
                    .locate(id)
                    .and_then(|(_, sheet, _)| {
                        let i = sheet.column_index(id)?;
                        Some(sheet.columns[i + 1..].iter().map(|c| c.id).collect())
                    })
                    .unwrap_or_default();
                for other in right {
                    if let Some(c) = p.column_mut(other) {
                        c.designation = designation;
                        c.format = format;
                        c.width = width;
                    }
                }
            }
        });
        true
    }
}

/// The kinds of the Format list, in Origin's names, with Sheaf's "Date and time" last.
const KINDS: [&str; 8] = ["Numeric", "Text", "Text & Numeric", "Time", "Date", "Month", "Day of Week", "Date and time"];

/// Which entry of [`KINDS`] a column with this storage and format shows.
fn kind_of(storage: Storage, format: NumberFormat) -> usize {
    match format {
        NumberFormat::Time(_) => 3,
        NumberFormat::Date(_) => 4,
        NumberFormat::Month(_) => 5,
        NumberFormat::Weekday(_) => 6,
        NumberFormat::DateTime(..) => 7,
        _ => match storage {
            Storage::Numeric => 0,
            Storage::Text => 1,
            Storage::Mixed => 2,
        },
    }
}

/// Switches to kind `kind`: sets the storage and a format that suits it, keeping the numeric
/// display format when the kind stays a plain number.
fn choose_kind(storage: &mut Storage, format: &mut NumberFormat, kind: usize) {
    use sheaf_core::datetime::{DatePattern, TimePattern};
    let plain = matches!(kind_of(*storage, *format), 0..=2) && !matches!(*storage, Storage::Text);
    match kind {
        0..=2 => {
            *storage = [Storage::Numeric, Storage::Text, Storage::Mixed][kind];
            if !plain || kind == 1 {
                *format = NumberFormat::General;
            }
        }
        _ => {
            // Dates, times and names are numbers: a text column becomes a numeric one.
            if *storage == Storage::Text {
                *storage = Storage::Numeric;
            }
            // Moving between Time, Date and Date and time keeps the patterns already chosen.
            let (date, time) = match *format {
                NumberFormat::Date(d) => (d, TimePattern::Hms),
                NumberFormat::Time(t) => (DatePattern::Mdy, t),
                NumberFormat::DateTime(d, t) => (d, t),
                _ => (DatePattern::Mdy, TimePattern::Hms),
            };
            *format = match kind {
                3 => NumberFormat::Time(time),
                4 => NumberFormat::Date(date),
                5 => NumberFormat::Month(NameStyle::Short),
                6 => NumberFormat::Weekday(NameStyle::Short),
                _ => NumberFormat::DateTime(date, time),
            };
        }
    }
}

/// The two rows "Format" (what the cells hold: Numeric, Text, Text & Numeric, Time, Date, Month,
/// Day of Week) and "Display" (how they are shown). Numbers have the display choices Automatic,
/// Decimal places, Scientific, Significant digits, Engineering (default digits, decimal places or
/// significant digits) and Decimal:1,000 (thousands separators); dates and times have their patterns, months and weekdays
/// their name styles.
fn format_rows(ui: &mut egui::Ui, storage: &mut Storage, format: &mut NumberFormat) {
    use sheaf_core::datetime::{DatePattern, TimePattern};
    let mut kind = kind_of(*storage, *format);
    ui.label("Format");
    egui::ComboBox::from_id_salt("column_format").selected_text(KINDS[kind]).show_ui(ui, |ui| {
        for (i, n) in KINDS.iter().enumerate() {
            ui.selectable_value(&mut kind, i, *n);
        }
    });
    ui.end_row();
    if kind != kind_of(*storage, *format) {
        choose_kind(storage, format, kind);
    }
    ui.label("Display");
    match *format {
        NumberFormat::Date(_) | NumberFormat::Time(_) | NumberFormat::DateTime(..) => {
            let (mut date, mut time) = match *format {
                NumberFormat::Date(d) => (d, TimePattern::Hms),
                NumberFormat::Time(t) => (DatePattern::Iso, t),
                NumberFormat::DateTime(d, t) => (d, t),
                _ => unreachable!(),
            };
            if kind == 7 && time.is_elapsed() {
                time = TimePattern::Hms;
            }
            let (show_date, show_time) = (kind != 3, kind != 4);
            ui.horizontal(|ui| {
                if show_date {
                    egui::ComboBox::from_id_salt("column_date_pattern").selected_text(date.label()).show_ui(ui, |ui| {
                        for p in DatePattern::ALL {
                            ui.selectable_value(&mut date, p, p.label());
                        }
                    });
                }
                if show_time {
                    egui::ComboBox::from_id_salt("column_time_pattern").selected_text(time.label()).show_ui(ui, |ui| {
                        for p in TimePattern::ALL {
                            if kind == 7 && p.is_elapsed() {
                                continue;
                            }
                            ui.selectable_value(&mut time, p, p.label());
                        }
                    });
                }
            });
            *format = match kind {
                3 => NumberFormat::Time(time),
                4 => NumberFormat::Date(date),
                _ => NumberFormat::DateTime(date, time),
            };
        }
        NumberFormat::Month(style) | NumberFormat::Weekday(style) => {
            let mut style = style;
            // Origin's list shows samples: Jan, January, J (months) and Mon, Monday, M (weekdays).
            let month = kind == 5;
            egui::ComboBox::from_id_salt("column_name_style").selected_text(style.sample(month)).show_ui(ui, |ui| {
                for n in NameStyle::ALL {
                    ui.selectable_value(&mut style, n, n.sample(month));
                }
            });
            *format = if kind == 5 { NumberFormat::Month(style) } else { NumberFormat::Weekday(style) };
        }
        _ if *storage == Storage::Text => {
            ui.label("Text");
        }
        _ => number_display(ui, format),
    }
    ui.end_row();
}

/// The display choices of a number: the style, and the digits where the style has them.
fn number_display(ui: &mut egui::Ui, format: &mut NumberFormat) {
    let (mut style, digits) = match *format {
        NumberFormat::Decimal(d) => (1, d),
        NumberFormat::Scientific(d) => (2, d),
        NumberFormat::Significant(d) => (3, d),
        NumberFormat::EngineeringAuto => (4, 3),
        NumberFormat::Engineering(d) | NumberFormat::EngineeringSi(d) => (5, d),
        NumberFormat::EngineeringSig(n) => (6, n),
        NumberFormat::DecimalGrouped(d) => (7, d),
        _ => (0, 3),
    };
    // Origin's Display list is Decimal:1000, Scientific:1E3, Engineering:1K, Decimal:1,000 and its
    // Digits list Default Decimal Digits, Set Decimal Places=, Significant Digits=; Sheaf folds the
    // two lists into one.
    let styles = [
        "Automatic",
        "Decimal places",
        "Scientific",
        "Significant digits",
        "Engineering:1K (default digits)",
        "Engineering:1K (decimal places)",
        "Engineering:1K (significant digits)",
        "Decimal:1,000",
    ];
    let mut d = digits;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("column_display").selected_text(styles[style]).show_ui(ui, |ui| {
            for (i, n) in styles.iter().enumerate() {
                ui.selectable_value(&mut style, i, *n);
            }
        });
        if style != 0 && style != 4 {
            ui.add(DragValue::new(&mut d).range(0..=15));
        }
    });
    *format = match style {
        1 => NumberFormat::Decimal(d),
        2 => NumberFormat::Scientific(d),
        3 => NumberFormat::Significant(d.max(1)),
        4 => NumberFormat::EngineeringAuto,
        5 => NumberFormat::EngineeringSi(d),
        6 => NumberFormat::EngineeringSig(d.max(1)),
        7 => NumberFormat::DecimalGrouped(d),
        _ => NumberFormat::General,
    };
}
