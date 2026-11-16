//! Column > Fill Column With > A set of Numbers..., A set of Date/Time values... and Arbitrary set of
//! Text&Numeric values... (T74): one window with the fields of Origin's `patternN`, `patternD` and
//! `patternT`. The fill itself is `Project::fill_pattern`; rules in `docs/spec/workbook-model.md`,
//! "Fill Column With patterns".

use eframe::egui::{self, DragValue, RichText};
use sheaf_core::ColumnId;
use sheaf_core::fill_pattern::{DateUnit, FillMode, FillPattern};

/// Which of the three items opened the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillKind {
    Numbers,
    Dates,
    Text,
}

impl FillKind {
    pub fn title(self) -> &'static str {
        match self {
            FillKind::Numbers => "Fill Worksheet with a set of Numbers",
            FillKind::Dates => "Fill Worksheet with a set of Date/Time values",
            FillKind::Text => "Fill Worksheet with Arbitrary set of Text&Numeric values",
        }
    }
}

pub enum FillOutcome {
    Open,
    Closed,
    Fill(Vec<ColumnId>, FillPattern),
}

/// Today's date as a date number (days, Origin's system), from the system clock (UTC).
fn today() -> f64 {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    (secs / 86_400) as f64 + sheaf_core::datetime::UNIX_EPOCH_VALUE
}

pub struct FillPatternDialog {
    pub kind: FillKind,
    columns: Vec<ColumnId>,
    from: f64,
    to: f64,
    inc: f64,
    random: bool,
    each: usize,
    sequence: usize,
    size: usize,
    // Dates and times.
    time: bool,
    date_from: String,
    date_to: String,
    unit: DateUnit,
    // Text.
    text: String,
    problem: Option<String>,
}

impl FillPatternDialog {
    /// A window for `columns` with Origin's defaults: 1 to 10 by 1, each value and the run once, a
    /// random set of 10; dates from today for ten days by 1 day; times 00:00 to 12:00 by 1 hour.
    pub fn new(kind: FillKind, columns: Vec<ColumnId>) -> Self {
        let d = today();
        let fmt = |v: f64| sheaf_core::datetime::format_date(v, sheaf_core::datetime::DatePattern::Iso);
        FillPatternDialog {
            kind,
            columns,
            from: 1.0,
            to: 10.0,
            inc: 1.0,
            random: false,
            each: 1,
            sequence: 1,
            size: 10,
            time: false,
            date_from: fmt(d),
            date_to: fmt(d + 9.0),
            unit: DateUnit::Day,
            text: String::new(),
            problem: None,
        }
    }

    /// The fill the window asks for, or why it cannot be made.
    pub fn pattern(&self) -> Result<FillPattern, String> {
        let mode = if self.random { FillMode::Random } else { FillMode::Repeat };
        Ok(match self.kind {
            FillKind::Numbers => FillPattern::Numbers {
                from: self.from,
                to: self.to,
                inc: self.inc,
                mode,
                each: self.each,
                sequence: self.sequence,
                size: self.size,
            },
            FillKind::Dates => {
                let read = |t: &str| -> Result<f64, String> {
                    let p = sheaf_core::datetime::parse(t.trim(), Default::default())
                        .ok_or_else(|| format!("'{t}' is not a date or time"))?;
                    Ok(p.value)
                };
                let (from, to) = (read(&self.date_from)?, read(&self.date_to)?);
                FillPattern::Dates { time: self.time, from, to, inc: self.inc, unit: self.unit, mode }
            }
            FillKind::Text => FillPattern::Text {
                text: self.text.clone(),
                mode,
                each: self.each,
                sequence: self.sequence,
                size: self.size,
            },
        })
    }

    pub fn set_problem(&mut self, p: String) {
        self.problem = Some(p);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FillOutcome {
        let mut outcome = FillOutcome::Open;
        let mut open = true;
        egui::Window::new(self.kind.title())
            .id(egui::Id::new("fill_pattern"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("fill_pattern_grid").num_columns(2).show(ui, |ui| {
                    match self.kind {
                        FillKind::Numbers => {
                            ui.label("From");
                            ui.add(DragValue::new(&mut self.from).speed(0.1));
                            ui.end_row();
                            ui.label("To");
                            ui.add(DragValue::new(&mut self.to).speed(0.1));
                            ui.end_row();
                            ui.label("Increment");
                            ui.add(DragValue::new(&mut self.inc).speed(0.1));
                            ui.end_row();
                        }
                        FillKind::Dates => {
                            ui.label("Format");
                            ui.horizontal(|ui| {
                                if ui.radio(!self.time, "Date").clicked() && self.time {
                                    self.time = false;
                                    let d = today();
                                    let fmt = |v| {
                                        sheaf_core::datetime::format_date(v, sheaf_core::datetime::DatePattern::Iso)
                                    };
                                    (self.date_from, self.date_to, self.unit) = (fmt(d), fmt(d + 9.0), DateUnit::Day);
                                }
                                if ui.radio(self.time, "Time").clicked() && !self.time {
                                    self.time = true;
                                    (self.date_from, self.date_to, self.unit) =
                                        ("00:00".to_string(), "12:00".to_string(), DateUnit::Hour);
                                }
                            });
                            ui.end_row();
                            ui.label("From");
                            ui.add(egui::TextEdit::singleline(&mut self.date_from).desired_width(140.0));
                            ui.end_row();
                            ui.label("To");
                            ui.add(egui::TextEdit::singleline(&mut self.date_to).desired_width(140.0));
                            ui.end_row();
                            ui.label("Increment");
                            ui.horizontal(|ui| {
                                ui.add(DragValue::new(&mut self.inc).speed(0.1));
                                egui::ComboBox::from_id_salt("fill_unit").selected_text(self.unit.name()).show_ui(
                                    ui,
                                    |ui| {
                                        for u in DateUnit::ALL {
                                            ui.selectable_value(&mut self.unit, u, u.name());
                                        }
                                    },
                                );
                            });
                            ui.end_row();
                        }
                        FillKind::Text => {
                            ui.label("Text to Repeat");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.text)
                                    .desired_width(260.0)
                                    .hint_text(r#"Boston "New York" Chicago"#),
                            );
                            ui.end_row();
                        }
                    }
                    ui.label("Mode");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.random, false, "Repeat");
                        ui.radio_value(&mut self.random, true, "Random");
                    });
                    ui.end_row();
                    if self.kind != FillKind::Dates {
                        if self.random {
                            ui.label("Total Number of Whole Set");
                            ui.add(DragValue::new(&mut self.size).range(1..=10_000_000));
                            ui.end_row();
                        } else {
                            ui.label("Repeat Times for Each Value");
                            ui.add(DragValue::new(&mut self.each).range(1..=1_000_000));
                            ui.end_row();
                            ui.label("Repeat Times for The Sequence");
                            ui.add(DragValue::new(&mut self.sequence).range(1..=1_000_000));
                            ui.end_row();
                        }
                    } else if self.random {
                        ui.label("");
                        ui.weak("Random dates fill as many rows as the sheet has.");
                        ui.end_row();
                    }
                });
                if let Some(p) = &self.problem {
                    ui.label(RichText::new(p).color(egui::Color32::from_rgb(190, 30, 30)));
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        match self.pattern() {
                            Ok(p) => outcome = FillOutcome::Fill(self.columns.clone(), p),
                            Err(e) => self.problem = Some(e),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = FillOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = FillOutcome::Closed;
                }
            });
        if !open { FillOutcome::Closed } else { outcome }
    }
}
