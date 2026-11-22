//! Graph > New Layer(Axes) > Open Dialog: the existing layer presets, captured at open.
use eframe::egui;
use sheaf_core::layers::NewLayerKind;
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Stale,
    Add { graph: WindowId, layer: usize, kind: NewLayerKind, activate: bool, auto_offset: bool },
}

pub struct AddLayerDialog {
    graph: WindowId,
    layer: usize,
    name: String,
    generation: u64,
    revision: u64,
    kind: NewLayerKind,
    activate: bool,
    auto_offset: bool,
}

impl AddLayerDialog {
    pub fn new(doc: &Document, graph: WindowId, layer: usize) -> Option<Self> {
        let source = doc.project().graph(graph)?;
        source.layers.get(layer)?;
        Some(Self {
            graph,
            layer,
            name: source.name.clone(),
            generation: doc.generation(),
            revision: doc.revision(),
            kind: NewLayerKind::Normal,
            activate: true,
            auto_offset: false,
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> Outcome {
        if doc.generation() != self.generation
            || doc.revision() != self.revision
            || doc.project().graph(self.graph).and_then(|g| g.layers.get(self.layer)).is_none()
        {
            return Outcome::Stale;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut outcome = Outcome::Open;
        egui::Window::new("New Layer (Axes)")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(430.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Input Graph");
                    ui.label(&self.name);
                });
                ui.label(format!("Source: Layer {}", self.layer + 1));
                ui.label("Layer Type");
                let response = egui::ComboBox::from_id_salt("add_layer_type")
                    .selected_text(self.kind.menu_label())
                    .width(410.0)
                    .show_ui(ui, |ui| {
                        for kind in NewLayerKind::ALL {
                            ui.selectable_value(&mut self.kind, kind, kind.menu_label());
                        }
                    });
                response
                    .response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Layer Type"));
                let mut user_defined = false;
                ui.add_enabled(false, egui::Checkbox::new(&mut user_defined, "User Defined"))
                    .on_disabled_hover_text("Custom axis definitions and scale formulas are not available yet.");
                let supports_auto_offset = self.kind.supports_auto_offset();
                ui.add_enabled(
                    supports_auto_offset,
                    egui::Checkbox::new(&mut self.auto_offset, "Auto Offset Overlapped Axes"),
                )
                .on_disabled_hover_text("Available for linked presets that use the full parent frame.");
                ui.checkbox(&mut self.activate, "Activate New Layer");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = Outcome::Add {
                            graph: self.graph,
                            layer: self.layer,
                            kind: self.kind,
                            activate: self.activate,
                            auto_offset: supports_auto_offset && self.auto_offset,
                        };
                    }
                    if ui.button("Cancel").clicked() {
                        outcome = Outcome::Closed;
                    }
                });
            });
        if open { outcome } else { Outcome::Closed }
    }
}
