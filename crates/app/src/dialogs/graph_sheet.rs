//! Add Graph as Sheet... (T190): pick one of the project's graphs to show as a sheet of the workbook.
//! The window only asks; the app adds the sheet. Rules: `docs/spec/workbook-model.md`, "Graph sheets".

use eframe::egui;
use sheaf_core::WindowId;

/// What the window asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphSheetOutcome {
    Open,
    Closed,
    /// Add a sheet that shows this graph.
    Pick(WindowId),
}

/// The Add Graph as Sheet window for workbook `book`.
pub struct GraphSheetPicker {
    pub book: WindowId,
    /// The project's graphs: id and the title the list shows.
    graphs: Vec<(WindowId, String)>,
    selected: Option<usize>,
}

impl GraphSheetPicker {
    pub fn new(book: WindowId, graphs: Vec<(WindowId, String)>) -> Self {
        let selected = (!graphs.is_empty()).then_some(0);
        GraphSheetPicker { book, graphs, selected }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> GraphSheetOutcome {
        let mut outcome = GraphSheetOutcome::Open;
        let mut open = true;
        egui::Window::new("Add Graph as Sheet")
            .id(egui::Id::new("add_graph_as_sheet"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.graphs.is_empty() {
                    ui.weak("The project has no graph.");
                }
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (i, (_, title)) in self.graphs.iter().enumerate() {
                        if ui.selectable_label(self.selected == Some(i), title).clicked() {
                            self.selected = Some(i);
                        }
                    }
                });
                ui.weak("The sheet shows the graph as its window does; the graph window stays in the project.");
                ui.horizontal(|ui| {
                    let chosen = self.selected.and_then(|i| self.graphs.get(i)).map(|(id, _)| *id);
                    if ui.add_enabled(chosen.is_some(), egui::Button::new("OK")).clicked()
                        && let Some(id) = chosen
                    {
                        outcome = GraphSheetOutcome::Pick(id);
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = GraphSheetOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = GraphSheetOutcome::Closed;
                }
            });
        if !open { GraphSheetOutcome::Closed } else { outcome }
    }
}
