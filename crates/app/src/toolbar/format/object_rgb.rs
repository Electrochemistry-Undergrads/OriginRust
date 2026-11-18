//! Opaque fill/line drafts retain the object snapshot captured by the root palette.
use super::*;
use sheaf_core::Color;

#[derive(Clone)]
pub(super) struct Picker {
    channels: [String; 3],
}

enum Choice {
    Apply(Color),
    Cancel,
}

impl Picker {
    fn new(color: Color) -> Self {
        Self { channels: [color.r.to_string(), color.g.to_string(), color.b.to_string()] }
    }

    fn color(&self) -> Option<Color> {
        Some(Color::rgb(
            self.channels[0].trim().parse().ok()?,
            self.channels[1].trim().parse().ok()?,
            self.channels[2].trim().parse().ok()?,
        ))
    }

    fn show(&mut self, ui: &mut Ui, prefix: &str) -> Option<Choice> {
        let ids = ["red", "green", "blue"].map(|name| ui.make_persistent_id(("object_rgb", prefix, name)));
        let enter = ui.ctx().memory(|m| ids.iter().any(|id| m.has_focus(*id)))
            && ui.input(|i| i.modifiers == egui::Modifiers::NONE && i.key_pressed(egui::Key::Enter));
        ui.label(prefix);
        egui::Grid::new(("object_rgb_fields", prefix)).show(ui, |ui| {
            for (index, name) in ["Red", "Green", "Blue"].into_iter().enumerate() {
                ui.label(name);
                let response =
                    ui.add(egui::TextEdit::singleline(&mut self.channels[index]).id(ids[index]).desired_width(70.0));
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, format!("{prefix}: {name}"))
                });
                ui.end_row();
            }
        });
        let color = self.color();
        if let Some(color) = color {
            ui.horizontal(|ui| {
                ui.label("Preview");
                let (rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 24.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(color.r, color.g, color.b));
            });
        } else {
            ui.label("Enter whole numbers from 0 to 255.");
        }
        crate::custom_colors::add_button(ui, color, prefix);
        let accept = ui.add_enabled(color.is_some(), egui::Button::new(format!("Apply {prefix}"))).clicked();
        if ui.button(format!("Cancel {prefix}")).clicked() {
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

pub(super) fn show(ui: &mut Ui, captured: &ObjectSnapshot, fill: bool, id: egui::Id, actions: &mut Vec<Action>) {
    let prefix = if fill { "Object fill color" } else { "Object line color" };
    let color =
        if fill { captured.fill.flatten().unwrap_or(Color::WHITE) } else { captured.line.unwrap_or(Color::BLACK) };
    let mut picker = ui.data_mut(|d| d.get_temp::<Picker>(id)).unwrap_or_else(|| Picker::new(color));
    let label = if fill { "Fill Color: Custom RGB..." } else { "Line Color: Custom RGB..." };
    let mut chosen = None;
    ui.separator();
    let response = egui::containers::menu::SubMenuButton::new(label)
        .config(egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
        .ui(ui, |ui| {
            if let Some(choice) = picker.show(ui, prefix) {
                chosen = Some(choice);
                ui.close();
            }
        })
        .0;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    ui.data_mut(|d| d.insert_temp(id, picker));
    if let Some(choice) = chosen {
        if let Choice::Apply(color) = choice {
            let change =
                if fill { object_format::Change::Fill(Some(color)) } else { object_format::Change::Line(color) };
            actions.push(Action::FormatObject(captured.request(change)));
        }
        ui.close();
    }
}
