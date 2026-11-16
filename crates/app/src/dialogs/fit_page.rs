//! Graph > Fit Page to Layers... (T60, Origin's `pfit2l`, graphing spec 15.7d): the page is cut or
//! grown to the layer frames and the page's objects plus a border; the layers keep their size.

use eframe::egui;
use sheaf_core::fit_page::{FitDirection, FitMargin};
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

#[derive(Copy, Clone, PartialEq)]
enum Margin {
    Border,
    Tight,
}

pub struct FitPageDialog {
    graph: WindowId,
    margin: Margin,
    /// Border Width, percent of the old page (Origin's default 5).
    border: f64,
    direction: FitDirection,
    problem: Option<String>,
}

/// Fits the page of `graph` with these settings as one undo step. Used by the dialog and by Format >
/// Fit Page To Layers, which applies Origin's defaults (Border 5, height and width).
pub fn fit_page(doc: &mut Document, graph: WindowId, margin: FitMargin, direction: FitDirection) -> Result<(), String> {
    let content = sheaf_plot::fit_content_bounds(doc.project(), graph)
        .map_err(|e| e.to_string())?
        .ok_or("the graph has no layer to fit the page to")?;
    doc.edit("Fit Page to Layers", |p| match p.graph_mut(graph) {
        Some(g) => g.fit_page_to_content(content, margin, direction).map_err(|e| e.to_string()),
        None => Err("the graph is gone".to_string()),
    })
}

impl FitPageDialog {
    pub fn new(graph: WindowId) -> Self {
        FitPageDialog { graph, margin: Margin::Border, border: 5.0, direction: FitDirection::Both, problem: None }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().graph(self.graph).is_none() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut result = Outcome::Open;
        egui::Window::new("Fit Page to Layers")
            .id(egui::Id::new("fit_page_to_layers"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("fit_page_grid").num_columns(2).show(ui, |ui| {
                    let label = ui.label("Margin Control");
                    egui::ComboBox::from_id_salt("fit_page_margin")
                        .selected_text(if self.margin == Margin::Border { "Border" } else { "Tight" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.margin, Margin::Border, "Border");
                            ui.selectable_value(&mut self.margin, Margin::Tight, "Tight");
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    let label = ui.label("Border Width");
                    ui.add_enabled(
                        self.margin == Margin::Border,
                        egui::DragValue::new(&mut self.border).range(0.0..=50.0).speed(0.5),
                    )
                    .labelled_by(label.id);
                    ui.end_row();
                    let label = ui.label("Adjust Direction");
                    let name = |d: FitDirection| match d {
                        FitDirection::Both => "Height and Width",
                        FitDirection::Height => "Height Only",
                        FitDirection::Width => "Width Only",
                    };
                    egui::ComboBox::from_id_salt("fit_page_direction")
                        .selected_text(name(self.direction))
                        .show_ui(ui, |ui| {
                            for d in [FitDirection::Both, FitDirection::Height, FitDirection::Width] {
                                ui.selectable_value(&mut self.direction, d, name(d));
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                });
                if let Some(e) = &self.problem {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let margin = match self.margin {
                            Margin::Border => FitMargin::Border(self.border),
                            Margin::Tight => FitMargin::Tight,
                        };
                        match fit_page(doc, self.graph, margin, self.direction) {
                            Ok(()) => result = Outcome::Done,
                            Err(e) => self.problem = Some(e),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        result = Outcome::Closed;
                    }
                });
            });
        if !open { Outcome::Closed } else { result }
    }
}
