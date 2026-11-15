//! Add Trendline / Edit Trendline: the small window opened from a plot's right-click menu
//! (graphing spec 18.11). OK commits one undo step; Cancel changes nothing.
//!
//! A plot can hold several trendlines. Add appends one; Edit and Remove act on the last one.

use super::graph_props::{color_button, dash_ui};
use eframe::egui;
use sheaf_core::graph::{TrendKind, Trendline};
use sheaf_core::{Color, Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Type {
    Linear,
    Polynomial,
    Power,
    Logarithmic,
    Exponential,
    MovingAverage,
}

const TYPES: [(Type, &str); 6] = [
    (Type::Linear, "Linear"),
    (Type::Polynomial, "Polynomial"),
    (Type::Power, "Power"),
    (Type::Logarithmic, "Logarithmic"),
    (Type::Exponential, "Exponential"),
    (Type::MovingAverage, "Moving Average"),
];

pub struct TrendlineDialog {
    graph: WindowId,
    layer: usize,
    plot: usize,
    /// True when editing the plot's last trendline, false when adding one.
    edit: bool,
    ty: Type,
    order: u8,
    window: u8,
    show_equation: bool,
    show_r2: bool,
    auto_color: bool,
    color: Color,
    width_pt: f64,
    dash: sheaf_core::graph::Dash,
    label_offset_pt: (f64, f64),
}

impl TrendlineDialog {
    /// `None` when the plot does not exist. With `edit`, the plot's last trendline is loaded
    /// (`None` when it has none).
    pub fn new(doc: &Document, graph: WindowId, layer: usize, plot: usize, edit: bool) -> Option<Self> {
        let p = doc.project().graph(graph)?.layers.get(layer)?.plots.get(plot)?;
        let t = if edit { p.decor.trendlines.last()?.clone() } else { Trendline::new(TrendKind::Linear) };
        let (ty, order, window) = match t.kind {
            TrendKind::Linear => (Type::Linear, 2, 2),
            TrendKind::Polynomial { order } => (Type::Polynomial, order, 2),
            TrendKind::Power => (Type::Power, 2, 2),
            TrendKind::Logarithmic => (Type::Logarithmic, 2, 2),
            TrendKind::Exponential => (Type::Exponential, 2, 2),
            TrendKind::MovingAverage { window } => (Type::MovingAverage, 2, window),
        };
        Some(TrendlineDialog {
            graph,
            layer,
            plot,
            edit,
            ty,
            order,
            window,
            show_equation: t.show_equation,
            show_r2: t.show_r2,
            auto_color: t.color.is_none(),
            color: t.color.unwrap_or(Color::BLACK),
            width_pt: t.width_pt,
            dash: t.dash,
            label_offset_pt: t.label_offset_pt,
        })
    }

    fn kind(&self) -> TrendKind {
        match self.ty {
            Type::Linear => TrendKind::Linear,
            Type::Polynomial => TrendKind::Polynomial { order: self.order },
            Type::Power => TrendKind::Power,
            Type::Logarithmic => TrendKind::Logarithmic,
            Type::Exponential => TrendKind::Exponential,
            Type::MovingAverage => TrendKind::MovingAverage { window: self.window },
        }
    }

    fn trendline(&self) -> Trendline {
        Trendline {
            kind: self.kind(),
            show_equation: self.show_equation,
            show_r2: self.show_r2,
            color: (!self.auto_color).then_some(self.color),
            width_pt: self.width_pt,
            dash: self.dash,
            label_offset_pt: self.label_offset_pt,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        let exists = doc
            .project()
            .graph(self.graph)
            .and_then(|g| g.layers.get(self.layer))
            .and_then(|l| l.plots.get(self.plot))
            .is_some_and(|p| !self.edit || !p.decor.trendlines.is_empty());
        if !exists || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut outcome = Outcome::Open;
        let title = if self.edit { "Edit Trendline" } else { "Add Trendline" };
        egui::Window::new(title)
            .id(egui::Id::new("trendline_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = ui.label("Type");
                    let name = TYPES.iter().find(|(t, _)| *t == self.ty).map_or("", |(_, n)| n);
                    egui::ComboBox::from_id_salt("trendline_type")
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            for (t, n) in TYPES {
                                ui.selectable_value(&mut self.ty, t, n);
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
                match self.ty {
                    Type::Polynomial => {
                        ui.horizontal(|ui| {
                            let label = ui.label("Order");
                            ui.add(egui::DragValue::new(&mut self.order).range(2..=10)).labelled_by(label.id);
                        });
                    }
                    Type::MovingAverage => {
                        ui.horizontal(|ui| {
                            let label = ui.label("Points of Window");
                            ui.add(egui::DragValue::new(&mut self.window).range(2..=10)).labelled_by(label.id);
                        });
                    }
                    _ => {}
                }
                ui.add_enabled_ui(self.ty != Type::MovingAverage, |ui| {
                    ui.checkbox(&mut self.show_equation, "Show Equation");
                    ui.checkbox(&mut self.show_r2, "Show R\u{b2}");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Color");
                    ui.checkbox(&mut self.auto_color, "Auto");
                    if !self.auto_color {
                        color_button(ui, &mut self.color);
                    }
                });
                ui.horizontal(|ui| {
                    let label = ui.label("Width");
                    ui.add(egui::DragValue::new(&mut self.width_pt).range(0.0..=20.0).speed(0.1).suffix(" pt"))
                        .labelled_by(label.id);
                });
                ui.horizontal(|ui| {
                    ui.label("Dash");
                    dash_ui(ui, "trendline_dash", &mut self.dash);
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        self.apply(doc);
                        outcome = Outcome::Done;
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Outcome::Closed;
                    }
                });
            });
        if !open {
            return Outcome::Closed;
        }
        outcome
    }

    fn apply(&self, doc: &mut Document) {
        let new = self.trendline();
        let current = doc
            .project()
            .graph(self.graph)
            .and_then(|g| g.layers.get(self.layer))
            .and_then(|l| l.plots.get(self.plot))
            .map(|p| p.decor.trendlines.clone());
        let Some(mut lines) = current else { return };
        if self.edit {
            match lines.last_mut() {
                Some(last) => *last = new,
                None => return,
            }
        } else {
            lines.push(new);
        }
        let label = if self.edit { "Edit trendline" } else { "Add trendline" };
        doc.apply(label, |project| {
            project.graph_mut(self.graph).unwrap().layers[self.layer].plots[self.plot].decor.trendlines = lines;
        });
    }
}
