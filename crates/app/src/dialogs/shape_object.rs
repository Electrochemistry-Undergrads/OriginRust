//! The properties window of a drawn line, arrow, rectangle or ellipse (double click, right-click
//! Properties): line colour, width and dash, the fill of a box, and the arrow heads. OK and Apply
//! are one undo step each.

use super::graph_props::{color_button, dash_ui};
use eframe::egui::{self, DragValue};
use sheaf_core::graph::Dash;
use sheaf_core::shape::{Shape, ShapeKind};
use sheaf_core::{Color, Document, WindowId};

pub struct ShapeDialog {
    graph: WindowId,
    layer: usize,
    shape: usize,
    kind: ShapeKind,
    color: Color,
    width: f64,
    dash: Dash,
    filled: bool,
    fill: Color,
    head: f64,
    head_at_start: bool,
}

impl ShapeDialog {
    /// For object number `shape` of `layer`.
    pub fn edit(doc: &Document, graph: WindowId, layer: usize, shape: usize) -> Option<Self> {
        let s = doc.project().graph(graph)?.layers.get(layer)?.shapes.get(shape)?;
        Some(ShapeDialog {
            graph,
            layer,
            shape,
            kind: s.kind,
            color: s.color,
            width: s.width_pt,
            dash: s.dash,
            filled: s.fill.is_some(),
            fill: s.fill.unwrap_or(Color::rgba(200, 220, 255, 255)),
            head: s.head_pt,
            head_at_start: s.head_at_start,
        })
    }

    fn current<'a>(&self, doc: &'a Document) -> Option<&'a Shape> {
        doc.project().graph(self.graph)?.layers.get(self.layer)?.shapes.get(self.shape)
    }

    /// Draws the window. Returns `false` when it was closed.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if self.current(doc).is_none() {
            return false;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new(format!("{} Properties", self.kind.name()))
            .id(egui::Id::new("shape_object"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("shape_object_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Line color");
                    color_button(ui, &mut self.color);
                    ui.end_row();
                    ui.label("Width");
                    ui.add(DragValue::new(&mut self.width).range(0.0..=40.0).speed(0.1).suffix(" pt"));
                    ui.end_row();
                    ui.label("Style");
                    dash_ui(ui, "shape_dash", &mut self.dash);
                    ui.end_row();
                    if !self.kind.is_linear() {
                        ui.label("Fill");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.filled, "Filled");
                            if self.filled {
                                color_button(ui, &mut self.fill);
                            }
                        });
                        ui.end_row();
                    }
                    if self.kind == ShapeKind::Arrow {
                        ui.label("Head length");
                        ui.add(DragValue::new(&mut self.head).range(1.0..=200.0).speed(0.2).suffix(" pt"));
                        ui.end_row();
                        ui.label("Heads");
                        ui.checkbox(&mut self.head_at_start, "Also at the start");
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        self.apply(doc);
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if ui.button("Apply").clicked() {
                        self.apply(doc);
                    }
                });
            });
        open && !close
    }

    fn apply(&self, doc: &mut Document) {
        let (graph, layer, k) = (self.graph, self.layer, self.shape);
        let new = |s: &mut Shape| {
            s.color = self.color;
            s.width_pt = self.width;
            s.dash = self.dash;
            s.fill = (!self.kind.is_linear() && self.filled).then_some(self.fill);
            s.head_pt = self.head;
            s.head_at_start = self.head_at_start && self.kind == ShapeKind::Arrow;
        };
        doc.apply("Edit drawn object", |p| {
            if let Some(s) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.shapes.get_mut(k))
            {
                new(s);
            }
        });
    }
}
