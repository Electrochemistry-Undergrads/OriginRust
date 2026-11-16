//! Opaque RGB drafts for a captured graph-font request.
use eframe::egui;
use sheaf_core::Color;

#[derive(Clone)]
pub(super) struct Picker {
    channels: [String; 3],
}

pub(super) enum Choice {
    Apply(Color),
    Cancel,
}

impl Picker {
    pub fn new(color: Color) -> Self {
        Self { channels: [color.r.to_string(), color.g.to_string(), color.b.to_string()] }
    }

    fn color(&self) -> Option<Color> {
        Some(Color::rgb(
            self.channels[0].trim().parse().ok()?,
            self.channels[1].trim().parse().ok()?,
            self.channels[2].trim().parse().ok()?,
        ))
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<Choice> {
        let ids = ["red", "green", "blue"].map(|name| ui.make_persistent_id(("graph_font_rgb", name)));
        let enter = ui.ctx().memory(|m| ids.iter().any(|id| m.has_focus(*id)))
            && ui.input(|i| i.modifiers == egui::Modifiers::NONE && i.key_pressed(egui::Key::Enter));
        ui.label("Font color");
        egui::Grid::new("graph_font_rgb_fields").show(ui, |ui| {
            for (index, name) in ["Red", "Green", "Blue"].into_iter().enumerate() {
                ui.label(name);
                let response =
                    ui.add(egui::TextEdit::singleline(&mut self.channels[index]).id(ids[index]).desired_width(70.0));
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, format!("Graph font color: {name}"))
                });
                ui.end_row();
            }
        });
        let color = self.color();
        if let Some(color) = color {
            ui.horizontal(|ui| {
                ui.label("Preview");
                let (rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 24.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(color.r, color.g, color.b));
            });
        } else {
            ui.label("Enter whole numbers from 0 to 255.");
        }
        let accept = ui.add_enabled(color.is_some(), egui::Button::new("Apply font color")).clicked();
        if ui.button("Cancel font color").clicked() {
            return Some(Choice::Cancel);
        }
        if (accept || enter)
            && let Some(color) = color
        {
            return Some(Choice::Apply(color));
        }
        None
    }
}
