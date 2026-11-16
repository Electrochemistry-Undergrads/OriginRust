//! The Format bar uses the same change contract for every supported target.

use super::*;
use crate::text_format::{Change, Snapshot};
use sheaf_core::Color;

pub(super) fn show(ui: &mut Ui, c: &MenuContext, state: Option<&Snapshot>, actions: &mut Vec<Action>) {
    grip(ui);
    for item in items(Bar::Format) {
        match item {
            Item::Gap => gap(ui),
            Item::FontBox => family(ui, state, actions),
            Item::SizeBox => size(ui, state, actions),
            Item::Button(button) => {
                let Some(state) = state else {
                    if matches!(button.icon, Icon::Bold | Icon::Italic | Icon::Bigger | Icon::Smaller | Icon::FontColor)
                    {
                        let response = ui.add_enabled(false, egui::Button::new("").min_size(vec2(CELL, CELL)));
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                false,
                                format!("Format toolbar: {}", button.tip),
                            )
                        });
                        paint_icon(
                            ui.painter(),
                            Rect::from_center_size(response.rect.center(), vec2(16.0, 16.0)),
                            button.icon,
                            false,
                        );
                        response.on_disabled_hover_text("Select a graph title, text label or legend");
                    } else {
                        draw_button(ui, Bar::Format, button, c, actions);
                    }
                    continue;
                };
                let (change, on) = match button.icon {
                    Icon::Bold => (Change::Bold(!state.font.bold), state.font.bold),
                    Icon::Italic => (Change::Italic(!state.font.italic), state.font.italic),
                    Icon::Bigger => (Change::Scale(1.2), false),
                    Icon::Smaller => (Change::Scale(1.0 / 1.2), false),
                    Icon::FontColor => {
                        colors(ui, state, actions);
                        continue;
                    }
                    _ => {
                        draw_button(ui, Bar::Format, button, c, actions);
                        continue;
                    }
                };
                let response = ui.add(egui::Button::new("").selected(on).min_size(vec2(CELL, CELL)));
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        on,
                        format!("Format toolbar: {}", button.tip),
                    )
                });
                paint_icon(
                    ui.painter(),
                    Rect::from_center_size(response.rect.center(), vec2(16.0, 16.0)),
                    button.icon,
                    true,
                );
                if response.clicked() {
                    actions.push(Action::FormatText(state.request(change)));
                }
                response.on_hover_text(button.tip);
            }
        }
    }
}

fn families() -> &'static [String] {
    static FAMILIES: OnceLock<Vec<String>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut names: Vec<_> = sheaf_plot::SystemFonts::shared()
            .database()
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
            .collect();
        names.sort();
        names.dedup();
        names
    })
}

fn family(ui: &mut Ui, state: Option<&Snapshot>, actions: &mut Vec<Action>) {
    let capture_id = ui.id().with("format_family_capture");
    ui.add_enabled_ui(state.is_some(), |ui| {
        let captured = ui.data_mut(|d| d.get_temp::<Snapshot>(capture_id)).or_else(|| state.cloned());
        let current = state.map_or("Default: Arial", |s| s.font.family.as_str());
        let response = egui::ComboBox::from_id_salt("format_family")
            .width(118.0)
            .selected_text(current)
            .show_ui(ui, |ui| {
                for name in families() {
                    if ui.selectable_label(name == current, name).clicked()
                        && let Some(state) = captured.as_ref()
                    {
                        actions.push(Action::FormatText(state.request(Change::Family(name.clone()))));
                    }
                }
            })
            .response;
        if response.clicked()
            && let Some(state) = state
        {
            ui.data_mut(|d| d.insert_temp(capture_id, state.clone()));
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, state.is_some(), "Format toolbar: Font")
        });
        response.on_hover_text("Font");
    });
}

fn size(ui: &mut Ui, state: Option<&Snapshot>, actions: &mut Vec<Action>) {
    if state.is_none() {
        disabled_text_box(ui, "0", 44.0, "Format toolbar: Font Size", "Select a graph title, text label or legend");
        return;
    }
    let source = state.map_or_else(|| "0".to_string(), |s| s.font.size_pt.to_string());
    let id = ui.id().with(("format_size", state.map(|s| format!("{:?}/{}/{}", s.target, s.generation, s.revision))));
    let mut text = ui.data_mut(|data| data.get_temp::<String>(id)).unwrap_or_else(|| source.clone());
    let response = ui.add_enabled(state.is_some(), egui::TextEdit::singleline(&mut text).id(id).desired_width(40.0));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, state.is_some(), "Format toolbar: Font Size")
    });
    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        if let Some(state) = state
            && let Ok(size) = text.parse::<f64>()
        {
            actions.push(Action::FormatText(state.request(Change::Size(size))));
        }
        response.surrender_focus();
        text = source.clone();
    } else if response.lost_focus() || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape))) {
        text = source;
        response.surrender_focus();
    }
    ui.data_mut(|data| data.insert_temp(id, text));
    response.on_hover_text("Font Size (1–1000 pt); Enter applies, Escape cancels");
}

fn colors(ui: &mut Ui, state: &Snapshot, actions: &mut Vec<Action>) {
    let capture_id = ui.id().with("format_color_capture");
    let captured = ui.data_mut(|d| d.get_temp::<Snapshot>(capture_id)).unwrap_or_else(|| state.clone());
    let color = &state.font.color;
    let label = egui::RichText::new("A").strong().color(Color32::from_rgb(color.r, color.g, color.b));
    let response = ui
        .menu_button(label, |ui| {
            // A compact named palette; custom/persistent Origin palettes remain a separate feature.
            for (name, color) in [
                ("Black", Color::rgb(0, 0, 0)),
                ("Red", Color::rgb(255, 0, 0)),
                ("Green", Color::rgb(0, 255, 0)),
                ("Blue", Color::rgb(0, 0, 255)),
                ("Cyan", Color::rgb(0, 255, 255)),
                ("Magenta", Color::rgb(255, 0, 255)),
                ("Yellow", Color::rgb(255, 255, 0)),
                ("White", Color::rgb(255, 255, 255)),
            ] {
                if ui.button(format!("Font Color: {name}")).clicked() {
                    actions.push(Action::FormatText(captured.request(Change::Color(color))));
                    ui.close();
                }
            }
        })
        .response;
    if response.clicked() {
        ui.data_mut(|d| d.insert_temp(capture_id, state.clone()));
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Format toolbar: Font Color"));
    response.on_hover_text("Font Color");
}
