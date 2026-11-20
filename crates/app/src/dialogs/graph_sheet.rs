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

/// Where the picked graph goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickTarget {
    /// Add Graph as Sheet... of this workbook (T190).
    Sheet(WindowId),
    /// A data cell's Insert Graph... (T194).
    Cell { column: sheaf_core::ColumnId, row: usize },
}

/// The window that picks a graph for a graph sheet or a data cell.
pub struct GraphSheetPicker {
    pub target: PickTarget,
    /// The project's graphs: id and the title the list shows.
    graphs: Vec<(WindowId, String)>,
    selected: Option<usize>,
}

impl GraphSheetPicker {
    pub fn new(target: PickTarget, graphs: Vec<(WindowId, String)>) -> Self {
        let selected = (!graphs.is_empty()).then_some(0);
        GraphSheetPicker { target, graphs, selected }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> GraphSheetOutcome {
        let mut outcome = GraphSheetOutcome::Open;
        let mut open = true;
        let title = match self.target {
            PickTarget::Sheet(_) => "Add Graph as Sheet",
            PickTarget::Cell { .. } => "Insert Graph",
        };
        egui::Window::new(title)
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
                ui.weak(match self.target {
                    PickTarget::Sheet(_) => {
                        "The sheet shows the graph as its window does; the graph window stays in the project."
                    }
                    PickTarget::Cell { .. } => "The cell shows a picture of the graph; a double click opens it.",
                });
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
