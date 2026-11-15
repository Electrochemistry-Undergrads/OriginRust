//! The properties window of a slope annotation (double click, right-click Properties): Show Legs,
//! colour (Auto is the plot's) and width. OK and Apply are one undo step each.

use super::graph_props::color_button;
use eframe::egui::{self, DragValue};
use sheaf_core::graph::SlopeAnnotation;
use sheaf_core::{Color, ColumnId, Document, WindowId};

pub struct SlopeDialog {
    graph: WindowId,
    layer: usize,
    slope: usize,
    /// What the annotation was when the window opened, to tell when Delete, Undo or Redo put
    /// another one at its index.
    plot: ColumnId,
    rows: (usize, usize),
    show_legs: bool,
    auto_color: bool,
    color: Color,
    width: f64,
}

impl SlopeDialog {
    /// For annotation number `slope` of `layer`.
    pub fn edit(doc: &Document, graph: WindowId, layer: usize, slope: usize) -> Option<Self> {
        let s = doc.project().graph(graph)?.layers.get(layer)?.slopes.get(slope)?;
        Some(SlopeDialog {
            graph,
            layer,
            slope,
            plot: s.plot,
            rows: (s.row_a, s.row_b),
            show_legs: s.show_legs,
            auto_color: s.color.is_none(),
            color: s.color.unwrap_or(Color::BLACK),
            width: s.width_pt,
        })
    }

    fn current<'a>(&self, doc: &'a Document) -> Option<&'a SlopeAnnotation> {
        let s = doc.project().graph(self.graph)?.layers.get(self.layer)?.slopes.get(self.slope)?;
        (s.plot == self.plot && (s.row_a, s.row_b) == self.rows).then_some(s)
    }

    /// Draws the window. Returns `false` when it was closed or when its annotation is gone.
    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> bool {
        if self.current(doc).is_none() {
            return false;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new("Slope Annotation Properties")
            .id(egui::Id::new("slope_object"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("slope_object_grid").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Rows");
                    ui.label(format!("{} and {}", self.rows.0 + 1, self.rows.1 + 1));
                    ui.end_row();
                    ui.label("Legs");
                    ui.checkbox(&mut self.show_legs, "Show Legs");
                    ui.end_row();
                    ui.label("Color");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.auto_color, "Auto");
                        if !self.auto_color {
                            color_button(ui, &mut self.color);
                        }
                    });
                    ui.end_row();
                    ui.label("Width");
                    ui.add(DragValue::new(&mut self.width).range(0.0..=40.0).speed(0.1).suffix(" pt"));
                    ui.end_row();
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
        let (graph, layer, k) = (self.graph, self.layer, self.slope);
        doc.apply("Edit slope annotation", |p| {
            if let Some(s) = p.graph_mut(graph).and_then(|g| g.layers.get_mut(layer)).and_then(|l| l.slopes.get_mut(k))
            {
                s.show_legs = self.show_legs;
                s.color = (!self.auto_color).then_some(self.color);
                s.width_pt = self.width;
            }
        });
    }
}
