//! Graph > Add Function Graph... (T119): y = f(x) in Sheaf's expression language with the variable `x`,
//! the X range and the number of points; OK adds the curve to the active layer
//! (`Project::add_function_graph`). Rules in `docs/spec/graphing.md`, "Add Function Graph".

use eframe::egui;
use sheaf_core::WindowId;

pub enum FunctionOutcome {
    Open,
    Closed,
    Add { graph: WindowId, layer: usize, expr: String, from: f64, to: f64, points: usize },
}

pub struct FunctionGraphDialog {
    pub graph: WindowId,
    pub layer: usize,
    pub expr: String,
    pub from: f64,
    pub to: f64,
    pub points: usize,
}

impl FunctionGraphDialog {
    pub fn new(graph: WindowId, layer: usize) -> Self {
        FunctionGraphDialog { graph, layer, expr: "sin(x)".into(), from: 0.0, to: 10.0, points: 100 }
    }

    pub fn outcome(&self) -> FunctionOutcome {
        FunctionOutcome::Add {
            graph: self.graph,
            layer: self.layer,
            expr: self.expr.trim().to_string(),
            from: self.from,
            to: self.to,
            points: self.points,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FunctionOutcome {
        let mut open = true;
        let mut outcome = FunctionOutcome::Open;
        egui::Window::new("Add Function Graph")
            .id(egui::Id::new("add_function_graph"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("add_function_graph_grid").num_columns(2).show(ui, |ui| {
                    ui.label("y(x) =");
                    ui.text_edit_singleline(&mut self.expr);
                    ui.end_row();
                    ui.label("From");
                    ui.add(egui::DragValue::new(&mut self.from).speed(0.1));
                    ui.end_row();
                    ui.label("To");
                    ui.add(egui::DragValue::new(&mut self.to).speed(0.1));
                    ui.end_row();
                    ui.label("Points");
                    ui.add(egui::DragValue::new(&mut self.points).range(2..=100_000));
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = self.outcome();
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = FunctionOutcome::Closed;
                    }
                });
            });
        if !open { FunctionOutcome::Closed } else { outcome }
    }
}
