//! Graph > Add Function Graph... (T119): y = f(x) in Sheaf's expression language with the variable `x`,
//! the X range and the number of points; OK adds the curve to the active layer
//! (`Project::add_function_graph`). Rules in `docs/spec/graphing.md`, "Add Function Graph". As Origin 2016's
//! (measured by T322): 100 points, the range of the layer's X axis to start with (Origin's Auto), and a Plot
//! Type of Line, Scatter, Line + Symbol or Column / Bar.

use eframe::egui;
use sheaf_core::WindowId;
use sheaf_core::graph::PlotKind;

/// Origin's Plot Type choices of a function plot (measured).
pub const PLOT_TYPES: [(PlotKind, &str); 4] = [
    (PlotKind::Line, "Line"),
    (PlotKind::Scatter, "Scatter"),
    (PlotKind::LineSymbol, "Line + Symbol"),
    (PlotKind::Column, "Column / Bar"),
];

pub enum FunctionOutcome {
    Open,
    Closed,
    Add { graph: WindowId, layer: usize, expr: String, from: f64, to: f64, points: usize, kind: PlotKind },
}

pub struct FunctionGraphDialog {
    pub graph: WindowId,
    pub layer: usize,
    pub expr: String,
    pub from: f64,
    pub to: f64,
    pub points: usize,
    pub kind: PlotKind,
}

impl FunctionGraphDialog {
    pub fn new(graph: WindowId, layer: usize) -> Self {
        FunctionGraphDialog {
            graph,
            layer,
            expr: "sin(x)".into(),
            from: 0.0,
            to: 10.0,
            points: 100,
            kind: PlotKind::Line,
        }
    }

    /// The window for an existing layer whose X axis shows `range`: the function starts over that range, as
    /// Origin's Auto From/To does.
    pub fn over(graph: WindowId, layer: usize, range: Option<(f64, f64)>) -> Self {
        let mut d = Self::new(graph, layer);
        if let Some((from, to)) = range.filter(|(a, b)| a.is_finite() && b.is_finite() && a != b) {
            (d.from, d.to) = (from, to);
        }
        d
    }

    pub fn outcome(&self) -> FunctionOutcome {
        FunctionOutcome::Add {
            graph: self.graph,
            layer: self.layer,
            expr: self.expr.trim().to_string(),
            from: self.from,
            to: self.to,
            points: self.points,
            kind: self.kind,
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
                    ui.label("Plot Type");
                    let shown = PLOT_TYPES.iter().find(|t| t.0 == self.kind).map_or("Line", |t| t.1);
                    egui::ComboBox::from_id_salt("add_function_graph_kind").selected_text(shown).show_ui(ui, |ui| {
                        for (k, name) in PLOT_TYPES {
                            ui.selectable_value(&mut self.kind, k, name);
                        }
                    });
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
