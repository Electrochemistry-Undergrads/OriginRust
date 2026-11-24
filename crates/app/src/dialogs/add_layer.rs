//! Graph > New Layer(Axes) > Open Dialog, with a captured graph and source layer.
use eframe::egui;
use sheaf_core::graph::{Graph, ScaleLink};
use sheaf_core::layers::{NewLayerKind, UserDefinedLayer};
use sheaf_core::{Document, WindowId};

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
    formula_error: Option<&'static str>,
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
            formula_error: None,
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
                let (supports_auto_offset, auto_offset_hint) = if self.user_defined {
                    self.custom_controls(ui, graph);
                    (
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
                    (self.kind.supports_auto_offset(), hint)
                };
                ui.add_enabled(
                    supports_auto_offset,
                    egui::Checkbox::new(&mut self.auto_offset, "Auto Offset Overlapped Axes"),
                )
                .on_disabled_hover_text(auto_offset_hint);
                ui.checkbox(&mut self.activate, "Activate New Layer");
                ui.separator();
                if let Some(error) = self.formula_error.filter(|_| self.user_defined && self.custom.parent.is_some()) {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        self.formula_error = if self.user_defined { self.validate_formulas(doc, graph) } else { None };
                        if self.formula_error.is_some() {
                            return;
                        }
                        outcome = Outcome::Add {
                            graph: self.graph,
                            layer: self.layer,
                            choice: if self.user_defined {
                                LayerChoice::UserDefined(self.custom.clone())
                            } else {
                                LayerChoice::Preset(self.kind)
                            },
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

    fn validate_formulas(&self, doc: &Document, graph: &Graph) -> Option<&'static str> {
        let parent = self.custom.parent?;
        let links = [&self.custom.x_link, &self.custom.y_link];
        let custom = links.map(|link| matches!(link, ScaleLink::Custom { .. }));
        if !custom.into_iter().any(|is_custom| is_custom) {
            return None;
        }
        let layer = &graph.layers[parent];
        if layer.coords.ternary().is_some() || (custom[0] && layer.coords.polar().is_some()) {
            return Some("Choose Independent or Straight for axes with fixed coordinate limits.");
        }
        let Ok(geometry) = sheaf_plot::layout_geometry(doc.project(), self.graph) else {
            return Some("The parent layer's scale could not be resolved.");
        };
        let Some(limits) = geometry.get(parent) else {
            return Some("The parent layer's scale could not be resolved.");
        };
        let x = limits.x.endpoints();
        let y = limits.y.endpoints();
        for (link, scale, error) in [
            (links[0], layer.x_axis.scale, "Enter X formulas that give two different limits on the X scale."),
            (links[1], layer.y_axis.scale, "Enter Y formulas that give two different limits on the Y scale."),
        ] {
            if matches!(link, ScaleLink::Custom { .. }) && link.formula_limits(x, y, scale).is_none() {
                return Some(error);
            }
        }
        None
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
        if let Some(parent) = self.custom.parent {
            let coords = graph.layers[parent].coords;
            let can_link_y = coords.ternary().is_none();
            scale_link(ui, "X", &mut self.custom.x_link, can_link_y && coords.polar().is_none());
            scale_link(ui, "Y", &mut self.custom.y_link, can_link_y);
            if [&self.custom.x_link, &self.custom.y_link].iter().any(|link| matches!(link, ScaleLink::Custom { .. })) {
                ui.weak("X1, X2, Y1 and Y2 are the parent's From and To limits.");
            }
        }
    }
}

fn scale_link(ui: &mut egui::Ui, axis: &str, value: &mut ScaleLink, can_use_formula: bool) {
    let label = format!("{axis} Scale Link");
    let custom = matches!(value, ScaleLink::Custom { .. });
    ui.horizontal(|ui| {
        ui.label(&label);
        egui::ComboBox::from_id_salt(&label)
            .selected_text(match value {
                ScaleLink::None => "Independent",
                ScaleLink::Straight => "Straight (1 to 1)",
                ScaleLink::Custom { .. } => "Custom Formula",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(value, ScaleLink::None, "Independent");
                ui.selectable_value(value, ScaleLink::Straight, "Straight (1 to 1)");
                let formula = ui
                    .add_enabled(can_use_formula, egui::Button::selectable(custom, "Custom Formula"))
                    .on_disabled_hover_text("This axis has fixed coordinate limits.");
                if formula.clicked() && !custom {
                    *value = ScaleLink::Custom { from: format!("{axis}1"), to: format!("{axis}2") };
                }
            })
            .response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, label.as_str()));
    });
    if let ScaleLink::Custom { from, to } = value {
        for (end, text) in [("From", from), ("To", to)] {
            ui.horizontal(|ui| {
                let label = ui.label(format!("{axis} {end} Formula"));
                ui.add(egui::TextEdit::singleline(text).desired_width(170.0)).labelled_by(label.id);
            });
        }
    }
}
