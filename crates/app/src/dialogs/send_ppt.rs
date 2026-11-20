//! Send Graphs to PowerPoint... (T232): which graphs go into the presentation. The window only asks;
//! the app picks the file and writes it (`sheaf_io::pptx`). Rules: `docs/spec/graphing.md` 17.11.

use eframe::egui;
use sheaf_core::WindowId;

/// What the window asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendPptOutcome {
    Open,
    Closed,
    /// Write these graphs, one slide (or page) each, in this order; `word` for a .docx (T242).
    Send {
        graphs: Vec<WindowId>,
        word: bool,
    },
}

/// The window of the graph page's Send Graphs to PowerPoint....
pub struct SendPptDialog {
    /// The graph whose menu opened the window, and its name.
    active: (WindowId, String),
    /// Every graph of the project, in project order.
    all: Vec<WindowId>,
    /// All graphs of the project rather than the active one.
    pub all_graphs: bool,
    /// Write a Word document (.docx, a graph per page) instead of a presentation (T242).
    pub word: bool,
}

impl SendPptDialog {
    pub fn new(active: (WindowId, String), all: Vec<WindowId>) -> Self {
        SendPptDialog { active, all, all_graphs: false, word: false }
    }

    /// The graphs that OK sends.
    pub fn chosen(&self) -> Vec<WindowId> {
        if self.all_graphs { self.all.clone() } else { vec![self.active.0] }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> SendPptOutcome {
        let mut outcome = SendPptOutcome::Open;
        let mut open = true;
        egui::Window::new("Send Graphs to PowerPoint")
            .id(egui::Id::new("send_graphs_to_powerpoint"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.radio_value(&mut self.all_graphs, false, format!("Active graph ({})", self.active.1));
                ui.radio_value(&mut self.all_graphs, true, format!("All graphs in the project ({})", self.all.len()));
                ui.separator();
                ui.radio_value(&mut self.word, false, "PowerPoint (.pptx)");
                ui.radio_value(&mut self.word, true, "Word (.docx)");
                ui.weak(if self.word {
                    "Sheaf writes a Word document (.docx): one graph per page, the graph as a picture made \
                     to fit the text width, with its name under it."
                } else {
                    "Sheaf writes a PowerPoint file (.pptx): one slide per graph, the graph as a picture \
                     made to fit the slide."
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = SendPptOutcome::Send { graphs: self.chosen(), word: self.word };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = SendPptOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = SendPptOutcome::Closed;
                }
            });
        if !open {
            outcome = SendPptOutcome::Closed;
        }
        outcome
    }
}
