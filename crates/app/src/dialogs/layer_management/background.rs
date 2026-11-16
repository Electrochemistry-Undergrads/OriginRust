//! The measured Background Color control of Layer Management > Display.
use eframe::egui;
use sheaf_core::Color;
use sheaf_core::graph::Graph;

use crate::object_format::PALETTE;

#[derive(Default)]
pub(super) struct BackgroundDraft {
    selection: Option<Vec<usize>>,
    value: Option<Color>,
    mixed: bool,
    dirty: bool,
}

impl BackgroundDraft {
    pub fn invalidate(&mut self) {
        self.selection = None;
    }

    /// Return an effective Apply, including `Some(None)` to clear the background.
    pub fn show(&mut self, ui: &mut egui::Ui, graph: &Graph, selection: &[usize]) -> Option<Option<Color>> {
        if self.selection.as_deref() != Some(selection) {
            self.selection = Some(selection.to_vec());
            self.value = selection.first().and_then(|&i| graph.layers[i].background);
            self.mixed = selection.iter().any(|&i| graph.layers[i].background != self.value);
            self.dirty = false;
        }
        ui.horizontal(|ui| {
            ui.label("Drawing Order");
            ui.add_enabled(false, egui::Button::new("Draw Composite"));
        });
        ui.group(|ui| {
            ui.strong("Option");
            let _ = ui.radio(true, "Color");
            ui.add_enabled(false, egui::RadioButton::new(false, "Border Dimensions"));
            ui.add_enabled(false, egui::RadioButton::new(false, "Scale Elements"));
        });
        ui.group(|ui| {
            ui.strong("Color");
            ui.add_enabled_ui(!selection.is_empty(), |ui| {
                egui::Grid::new("lm_background_color").num_columns(2).show(ui, |ui| {
                    let label = ui.label("Background Color");
                    let text = if self.mixed && !self.dirty {
                        "Mixed".into()
                    } else {
                        self.value.map_or_else(
                            || "None".into(),
                            |c| {
                                PALETTE
                                    .iter()
                                    .find(|(_, color)| *color == c)
                                    .map_or_else(|| c.to_string(), |(n, _)| (*n).into())
                            },
                        )
                    };
                    egui::ComboBox::from_id_salt("lm_background_choice")
                        .selected_text(text)
                        .show_ui(ui, |ui| {
                            for (name, color) in
                                std::iter::once(("None", None)).chain(PALETTE.map(|(n, c)| (n, Some(c))))
                            {
                                if ui.selectable_label(self.value == color && !self.mixed, name).clicked() {
                                    self.value = color;
                                    self.mixed = false;
                                    self.dirty = true;
                                    ui.close();
                                }
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    if let Some(color) = &mut self.value {
                        for (name, channel) in [
                            ("Background Red", &mut color.r),
                            ("Background Green", &mut color.g),
                            ("Background Blue", &mut color.b),
                        ] {
                            let label = ui.label(name);
                            if ui.add(egui::DragValue::new(channel).range(0..=255)).labelled_by(label.id).changed() {
                                self.dirty = true;
                                self.mixed = false;
                                color.a = 255;
                            }
                            ui.end_row();
                        }
                        ui.label("Preview");
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(90.0, 18.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(color.r, color.g, color.b));
                        ui.end_row();
                    }
                    for name in ["Border Fill Color", "Border Color"] {
                        let label = ui.label(name);
                        ui.add_enabled(false, egui::Button::new("None")).labelled_by(label.id);
                        ui.end_row();
                    }
                });
            });
        });
        let changed = self.dirty && selection.iter().any(|&i| graph.layers[i].background != self.value);
        if ui.add_enabled(changed, egui::Button::new("Apply")).clicked() {
            self.dirty = false;
            return Some(self.value);
        }
        None
    }
}
