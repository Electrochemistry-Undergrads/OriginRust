//! The Format bar uses the same change contract for every supported target.

use super::*;
use crate::object_format::{self, Markup, Snapshot as ObjectSnapshot};
use crate::text_format::{Change, Snapshot};

pub(super) fn show(
    ui: &mut Ui,
    c: &MenuContext,
    state: Option<&Snapshot>,
    object: Option<&ObjectSnapshot>,
    actions: &mut Vec<Action>,
) {
    grip(ui);
    for item in items(Bar::Format) {
        match item {
            Item::Gap => gap(ui),
            Item::FontBox => family(ui, state, actions),
            Item::SizeBox => size(ui, state, actions),
            Item::Button(button) if object_icon(button.icon) => {
                if let Some(state) = state.filter(|s| s.worksheet.is_some())
                    && matches!(button.icon, Icon::Underline | Icon::FillColor)
                {
                    worksheet_button(ui, button, state, actions);
                } else {
                    object_button(ui, button, object, actions);
                }
            }
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
                        response.on_disabled_hover_text("Select a graph title, text label, legend or worksheet cells");
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
                let enabled = !matches!(button.icon, Icon::Bigger | Icon::Smaller) || state.scaling_enabled();
                let response = ui.add_enabled(enabled, egui::Button::new("").selected(on).min_size(vec2(CELL, CELL)));
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        enabled,
                        on,
                        format!("Format toolbar: {}", button.tip),
                    )
                });
                paint_icon(
                    ui.painter(),
                    Rect::from_center_size(response.rect.center(), vec2(16.0, 16.0)),
                    button.icon,
                    enabled,
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
    let enabled = state.is_some();
    ui.add_enabled_ui(enabled, |ui| {
        let captured = ui.data_mut(|d| d.get_temp::<Snapshot>(capture_id)).or_else(|| state.cloned());
        let current = state.map_or("Default: Arial", |s| if s.mixed.family { "" } else { s.font.family.as_str() });
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
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, enabled, "Format toolbar: Font"));
        response.on_hover_text("Font");
    });
}

fn size(ui: &mut Ui, state: Option<&Snapshot>, actions: &mut Vec<Action>) {
    if state.is_none() {
        disabled_text_box(
            ui,
            "0",
            44.0,
            "Format toolbar: Font Size",
            "Select a graph title, text label, legend or worksheet cells",
        );
        return;
    }
    let source = state.map_or_else(|| "0".to_string(), Snapshot::size_text);
    let id = ui.id().with(("format_size", state.map(|s| format!("{:?}/{}/{}", s.target, s.generation, s.revision))));
    let mut text = ui.data_mut(|data| data.get_temp::<String>(id)).unwrap_or_else(|| source.clone());
    // A single-line editor may surrender focus and consume Enter while it is drawn.
    // Capture the keys and ownership before constructing it.
    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let had_focus = ui.memory(|m| m.has_focus(id));
    let response = ui.add_enabled(state.is_some(), egui::TextEdit::singleline(&mut text).id(id).desired_width(40.0));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, state.is_some(), "Format toolbar: Font Size")
    });
    if (had_focus || response.has_focus() || response.lost_focus()) && enter {
        if let Some(state) = state
            && let Ok(size) = text.parse::<f64>()
        {
            actions.push(Action::FormatText(state.request(Change::Size(size))));
        }
        response.surrender_focus();
        text = source.clone();
    } else if response.lost_focus() || ((had_focus || response.has_focus()) && escape) {
        text = source;
        response.surrender_focus();
    }
    ui.data_mut(|data| data.insert_temp(id, text));
    response.on_hover_text(state.map_or("Font Size", Snapshot::size_help));
}

// Origin Classic entries 8-17 and 19-24; the other eight remain in the basic menu.
// RGB values measured from Origin 2016 graph/worksheet swatches (graphing 7.2/7.4, GUI 3).
const CLASSIC_EXTRA: [(&str, sheaf_core::Color); 16] = [
    ("Dark Yellow", sheaf_core::Color::rgb(128, 128, 0)),
    ("Navy", sheaf_core::Color::rgb(0, 0, 128)),
    ("Purple", sheaf_core::Color::rgb(128, 0, 128)),
    ("Wine", sheaf_core::Color::rgb(128, 0, 0)),
    ("Olive", sheaf_core::Color::rgb(0, 128, 0)),
    ("Dark Cyan", sheaf_core::Color::rgb(0, 128, 128)),
    ("Royal", sheaf_core::Color::rgb(0, 0, 160)),
    ("Orange", sheaf_core::Color::rgb(255, 128, 0)),
    ("Violet", sheaf_core::Color::rgb(128, 0, 255)),
    ("Pink", sheaf_core::Color::rgb(255, 0, 128)),
    ("LT Gray", sheaf_core::Color::rgb(192, 192, 192)),
    ("Gray", sheaf_core::Color::rgb(128, 128, 128)),
    ("LT Yellow", sheaf_core::Color::rgb(255, 255, 128)),
    ("LT Cyan", sheaf_core::Color::rgb(128, 255, 255)),
    ("LT Magenta", sheaf_core::Color::rgb(255, 128, 255)),
    ("Dark Gray", sheaf_core::Color::rgb(64, 64, 64)),
];

fn classic_colors(ui: &mut Ui, captured: &Snapshot, fill: bool, actions: &mut Vec<Action>) {
    let prefix = if fill { "Fill Color" } else { "Font Color" };
    let response = ui.menu_button(format!("{prefix}: More Classic colors"), |ui| {
        for (name, color) in CLASSIC_EXTRA {
            if ui.button(format!("{prefix}: {name}")).clicked() {
                ui.close();
                return Some(color);
            }
        }
        None
    });
    response.response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{prefix}: More Classic colors"))
    });
    if let Some(color) = response.inner.flatten() {
        let change = if fill { Change::Fill(Some(color)) } else { Change::Color(color) };
        actions.push(Action::FormatText(captured.request(change)));
        // Also close the root palette, so the next toolbar/editor interaction owns focus.
        ui.close();
    }
}

fn colors(ui: &mut Ui, state: &Snapshot, actions: &mut Vec<Action>) {
    let capture_id = ui.id().with("format_color_capture");
    let captured = ui.data_mut(|d| d.get_temp::<Snapshot>(capture_id)).unwrap_or_else(|| state.clone());
    let color = &state.font.color;
    let label = egui::RichText::new("A").strong().color(Color32::from_rgb(color.r, color.g, color.b));
    let response = ui
        .menu_button(label, |ui| {
            // Shared named presets; the custom chooser below is worksheet-only.
            for (name, color) in object_format::PALETTE {
                if ui.button(format!("Font Color: {name}")).clicked() {
                    actions.push(Action::FormatText(captured.request(Change::Color(color))));
                    ui.close();
                }
            }
            classic_colors(ui, &captured, false, actions);
            if captured.worksheet.is_some() {
                ui.separator();
                if ui.button("Font Color: Custom...").clicked() {
                    use crate::dialogs::worksheet_color::{Kind, WorksheetColorDialog};
                    if let Some(dialog) = WorksheetColorDialog::new(captured.clone(), Kind::Text) {
                        actions.push(Action::WorksheetColorDialog(Box::new(dialog)));
                    }
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

/// Worksheet controls use range requests; graph-object formatting keeps its own dispatcher.
fn worksheet_button(ui: &mut Ui, button: &Button, state: &Snapshot, actions: &mut Vec<Action>) {
    let style = state.worksheet.expect("worksheet control target");
    if button.icon == Icon::FillColor {
        worksheet_fill(ui, state, actions);
        return;
    }
    let response = ui.add(egui::Button::new("").selected(style.underline).min_size(vec2(CELL, CELL)));
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            style.underline,
            "Format toolbar: Underline",
        )
    });
    paint(ui, &response, button.icon, true);
    if response.clicked() {
        actions.push(Action::FormatText(state.request(Change::Underline(!style.underline))));
    }
    response.on_hover_text("Underline");
}

fn worksheet_fill(ui: &mut Ui, state: &Snapshot, actions: &mut Vec<Action>) {
    let capture_id = ui.id().with("worksheet_fill_capture");
    let captured = ui.data_mut(|d| d.get_temp::<Snapshot>(capture_id)).unwrap_or_else(|| state.clone());
    let mut label = egui::RichText::new("F").strong();
    if !state.mixed.fill
        && let Some(color) = state.worksheet.and_then(|s| s.fill)
    {
        let dark = u32::from(color.r) * 299 + u32::from(color.g) * 587 + u32::from(color.b) * 114 < 128_000;
        label = label.background_color(Color32::from_rgb(color.r, color.g, color.b)).color(if dark {
            Color32::WHITE
        } else {
            Color32::BLACK
        });
    }
    let response = ui
        .menu_button(label, |ui| {
            if ui.button("Fill Color: None").clicked() {
                actions.push(Action::FormatText(captured.request(Change::Fill(None))));
                ui.close();
            }
            for (name, color) in object_format::PALETTE {
                if ui.button(format!("Fill Color: {name}")).clicked() {
                    actions.push(Action::FormatText(captured.request(Change::Fill(Some(color)))));
                    ui.close();
                }
            }
            classic_colors(ui, &captured, true, actions);
            ui.separator();
            if ui.button("Fill Color: Custom...").clicked() {
                use crate::dialogs::worksheet_color::{Kind, WorksheetColorDialog};
                if let Some(dialog) = WorksheetColorDialog::new(captured.clone(), Kind::Fill) {
                    actions.push(Action::WorksheetColorDialog(Box::new(dialog)));
                }
                ui.close();
            }
        })
        .response;
    if response.clicked() {
        ui.data_mut(|d| d.insert_temp(capture_id, state.clone()));
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Format toolbar: Fill Color"));
    response.on_hover_text(if state.mixed.fill { "Fill Color (mixed selection)" } else { "Fill Color" });
}

fn object_icon(icon: Icon) -> bool {
    matches!(
        icon,
        Icon::Underline | Icon::Superscript | Icon::Subscript | Icon::Greek | Icon::FillColor | Icon::LineColor
    )
}

/// The reason a whole-object control is grey.
fn reason(icon: Icon) -> &'static str {
    match icon {
        Icon::FillColor => "Select a rectangle, circle or legend",
        Icon::LineColor => "Select a drawn object or straight line",
        _ => "Select a text label or an axis title with its own text",
    }
}

fn paint(ui: &Ui, response: &egui::Response, icon: Icon, enabled: bool) {
    paint_icon(ui.painter(), Rect::from_center_size(response.rect.center(), vec2(16.0, 16.0)), icon, enabled);
}

/// Underline, Superscript, Subscript, Greek, Fill Color and Line Color.
fn object_button(ui: &mut Ui, button: &Button, object: Option<&ObjectSnapshot>, actions: &mut Vec<Action>) {
    let label = format!("Format toolbar: {}", button.tip);
    let markup = match button.icon {
        Icon::Underline => Some(Markup::Underline),
        Icon::Superscript => Some(Markup::Superscript),
        Icon::Subscript => Some(Markup::Subscript),
        Icon::Greek => Some(Markup::Greek),
        _ => None,
    };
    let live = object.filter(|o| match button.icon {
        Icon::FillColor => o.fill.is_some(),
        Icon::LineColor => o.line.is_some(),
        _ => o.markup.is_some(),
    });
    let Some(state) = live else {
        let response = ui.add_enabled(false, egui::Button::new("").min_size(vec2(CELL, CELL)));
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, false, label.clone()));
        paint(ui, &response, button.icon, false);
        response.on_disabled_hover_text(reason(button.icon));
        return;
    };
    if let Some(markup) = markup {
        let on = state.wrapped(markup);
        let response = ui.add(egui::Button::new("").selected(on).min_size(vec2(CELL, CELL)));
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, label.clone()));
        paint(ui, &response, button.icon, true);
        if response.clicked() {
            actions.push(Action::FormatObject(state.request(object_format::Change::Markup(markup))));
        }
        response.on_hover_text(button.tip);
    } else {
        colour_menu(ui, button, state, &label, actions);
    }
}

/// Fill Color (with a "None" entry) or Line Color; the popup keeps the snapshot it opened with.
fn colour_menu(ui: &mut Ui, button: &Button, state: &ObjectSnapshot, label: &str, actions: &mut Vec<Action>) {
    let fill = button.icon == Icon::FillColor;
    let capture_id = ui.id().with(if fill { "format_fill_capture" } else { "format_line_capture" });
    let captured = ui.data_mut(|d| d.get_temp::<ObjectSnapshot>(capture_id)).unwrap_or_else(|| state.clone());
    let current = if fill { state.fill.flatten() } else { state.line };
    let mut text = egui::RichText::new(if fill { "F" } else { "L" }).strong();
    if let Some(c) = current {
        text = if fill {
            text.background_color(Color32::from_rgb(c.r, c.g, c.b))
        } else {
            text.color(Color32::from_rgb(c.r, c.g, c.b))
        };
    }
    let response = ui
        .menu_button(text, |ui| {
            if fill && ui.button("Fill Color: None").clicked() {
                actions.push(Action::FormatObject(captured.request(object_format::Change::Fill(None))));
                ui.close();
            }
            for (name, color) in object_format::PALETTE {
                if ui.button(format!("{}: {name}", button.tip)).clicked() {
                    let change = if fill {
                        object_format::Change::Fill(Some(color))
                    } else {
                        object_format::Change::Line(color)
                    };
                    actions.push(Action::FormatObject(captured.request(change)));
                    ui.close();
                }
            }
        })
        .response;
    if response.clicked() {
        ui.data_mut(|d| d.insert_temp(capture_id, state.clone()));
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.to_string()));
    response.on_hover_text(button.tip);
}
