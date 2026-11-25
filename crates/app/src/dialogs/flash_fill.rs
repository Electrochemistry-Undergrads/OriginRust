//! Column > Flash Fill... (T430): the rule Flash Fill found for the column from its typed examples, a preview of
//! the first rows, and Fill Values (static texts) or Keep Formula (a column formula that follows its sources).
//! Rules in `sheaf_core::flash_fill`.

use eframe::egui;
use sheaf_core::ColumnId;
use sheaf_core::flash_fill::FlashFill;

/// Rows the preview shows.
const PREVIEW_ROWS: usize = 8;

pub enum FlashOutcome {
    Open,
    Closed,
    Fill { column: ColumnId, fill: Box<FlashFill> },
    Formula { column: ColumnId, formula: String },
}

pub struct FlashFillDialog {
    pub column: ColumnId,
    pub fill: FlashFill,
    /// The first rows: the source texts and the result.
    pub preview: Vec<(String, String)>,
}

impl FlashFillDialog {
    pub fn new(project: &sheaf_core::Project, column: ColumnId, fill: FlashFill) -> Self {
        let text = |id: ColumnId, r: usize| {
            project.column(id).and_then(|c| sheaf_core::indexed::level_text(c, r)).unwrap_or_default()
        };
        let preview = fill
            .values
            .iter()
            .enumerate()
            .take(PREVIEW_ROWS)
            .map(|(r, v)| {
                let from: Vec<String> = fill.sources.iter().map(|&s| text(s, r)).filter(|t| !t.is_empty()).collect();
                (from.join(" | "), v.clone())
            })
            .collect();
        FlashFillDialog { column, fill, preview }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FlashOutcome {
        let mut open = true;
        let mut outcome = FlashOutcome::Open;
        egui::Window::new("Flash Fill")
            .id(egui::Id::new("flash_fill"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(match &self.fill.formula {
                    Some(f) => format!("Rule: {f}"),
                    None => "The rule cannot be written as a formula; Fill Values only.".to_string(),
                });
                ui.separator();
                egui::Grid::new("flash_fill_preview").striped(true).show(ui, |ui| {
                    ui.strong("From");
                    ui.strong("Gives");
                    ui.end_row();
                    for (r, (from, to)) in self.preview.iter().enumerate() {
                        ui.label(from);
                        let example = self.fill.examples.contains(&r);
                        ui.label(if example { format!("{to}  (example)") } else { to.clone() });
                        ui.end_row();
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Fill Values").clicked() {
                        outcome = FlashOutcome::Fill { column: self.column, fill: Box::new(self.fill.clone()) };
                    }
                    if let Some(f) = &self.fill.formula
                        && ui.button("Keep Formula").clicked()
                    {
                        outcome = FlashOutcome::Formula { column: self.column, formula: f.clone() };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = FlashOutcome::Closed;
                    }
                });
            });
        if !open { FlashOutcome::Closed } else { outcome }
    }
}
