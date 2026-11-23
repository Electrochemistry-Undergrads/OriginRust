//! Graph > New Layer(Axes) > Open Dialog, with a captured graph and source layer.
use eframe::egui;
use sheaf_core::graph::{Graph, ScaleLink};
use sheaf_core::layers::{NewLayerKind, UserDefinedLayer};
use sheaf_core::{Document, WindowId};

#[derive(Clone, Copy)]
pub enum LayerChoice {
    Preset(NewLayerKind),
    UserDefined(UserDefinedLayer),
}

pub enum Outcome {
    Open,
    Closed,
    Stale,
    Add { graph: WindowId, layer: usize, choice: LayerChoice, activate: bool, auto_offset: bool },
}

pub struct AddLayerDialog {
    graph: WindowId,
    layer: usize,
    name: String,
    generation: u64,
    revision: u64,
    kind: NewLayerKind,
    user_defined: bool,
    custom: UserDefinedLayer,
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
            user_defined: false,
            custom: UserDefinedLayer::default(),
            activate: true,
            auto_offset: false,
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> Outcome {
        let Some(graph) = doc.project().graph(self.graph) else { return Outcome::Stale };
        if doc.generation() != self.generation
            || doc.revision() != self.revision
            || graph.layers.get(self.layer).is_none()
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
                ui.checkbox(&mut self.user_defined, "User Defined");
                let (choice, supports_auto_offset, auto_offset_hint) = if self.user_defined {
                    self.custom_controls(ui, graph);
                    (
                        LayerChoice::UserDefined(self.custom),
                        self.custom.supports_auto_offset(),
                        "Choose a parent layer and show at least one axis to use auto offset.",
                    )
                } else {
                    ui.label("Layer Type");
                    egui::ComboBox::from_id_salt("add_layer_type")
                        .selected_text(self.kind.menu_label())
                        .width(410.0)
                        .show_ui(ui, |ui| {
                            for kind in NewLayerKind::ALL {
                                ui.selectable_value(&mut self.kind, kind, kind.menu_label());
                            }
                        })
                        .response
                        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Layer Type"));
                    let hint = match self.kind {
                        NewLayerKind::Normal => "Bottom-X Left-Y uses independent frame placement.",
                        _ => "Available for linked presets that use the full parent frame.",
                    };
                    (LayerChoice::Preset(self.kind), self.kind.supports_auto_offset(), hint)
                };
                ui.add_enabled(
                    supports_auto_offset,
                    egui::Checkbox::new(&mut self.auto_offset, "Auto Offset Overlapped Axes"),
                )
                .on_disabled_hover_text(auto_offset_hint);
                ui.checkbox(&mut self.activate, "Activate New Layer");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = Outcome::Add {
                            graph: self.graph,
                            layer: self.layer,
                            choice,
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

    fn custom_controls(&mut self, ui: &mut egui::Ui, graph: &Graph) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.custom.bottom, "Bottom");
            ui.checkbox(&mut self.custom.top, "Top");
            ui.checkbox(&mut self.custom.left, "Left");
            ui.checkbox(&mut self.custom.right, "Right");
        });
        ui.horizontal(|ui| {
            ui.label("Link To");
            egui::ComboBox::from_id_salt("add_layer_parent")
                .selected_text(self.custom.parent.map_or_else(|| "None".into(), |i| format!("Layer {}", i + 1)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.custom.parent, None, "None");
                    for (index, layer) in graph.layers.iter().enumerate() {
                        if layer.link.is_none() {
                            ui.selectable_value(&mut self.custom.parent, Some(index), format!("Layer {}", index + 1));
                        }
                    }
                })
                .response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Link To"));
        });
        if self.custom.parent.is_some() {
            scale_link(ui, "X Scale Link", &mut self.custom.x_link);
            scale_link(ui, "Y Scale Link", &mut self.custom.y_link);
        }
    }
}

fn scale_link(ui: &mut egui::Ui, label: &str, value: &mut ScaleLink) {
    let name = |link| match link {
        ScaleLink::None => "Independent",
        ScaleLink::Straight => "Straight (1 to 1)",
    };
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(label)
            .selected_text(name(*value))
            .show_ui(ui, |ui| {
                for link in [ScaleLink::None, ScaleLink::Straight] {
                    ui.selectable_value(value, link, name(link));
                }
                ui.add_enabled(false, egui::Button::selectable(false, "Custom Formula"));
            })
            .response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, label));
    });
}
