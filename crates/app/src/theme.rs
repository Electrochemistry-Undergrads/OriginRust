//! The look of the whole program, set once at start-up (ADR 0007, point 7): light only,
//! about 12 px text, square corners, one-pixel borders, flat buttons and small spacing,
//! after the Windows controls Origin uses. Colours are approximate measurements.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use std::sync::Arc;

/// The grey of the area in which the child windows float.
pub const WORKSPACE_GREY: Color32 = Color32::from_rgb(171, 171, 171);
/// The very light grey of menu bar, tool bars and panels.
pub const PANEL: Color32 = Color32::from_rgb(250, 250, 252);
/// Border of controls and panels.
pub const BORDER: Color32 = Color32::from_rgb(173, 173, 173);
/// Windows' blue for hover borders and selections.
pub const ACCENT: Color32 = Color32::from_rgb(0, 120, 215);

/// Applies fonts, text sizes, colours and spacing to the context.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    ctx.set_fonts(fonts());
    ctx.all_styles_mut(style);
}

/// Segoe UI from the Windows font folder if it is there. The font is never shipped with
/// the program; without it egui's own font is used.
fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let folder = std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(|| "C:/Windows".into());
    if let Ok(bytes) = std::fs::read(folder.join("Fonts").join("segoeui.ttf")) {
        fonts.font_data.insert("segoe-ui".to_string(), Arc::new(FontData::from_owned(bytes)));
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "segoe-ui".to_string());
    }
    fonts
}

fn style(s: &mut egui::Style) {
    s.text_styles = [
        (TextStyle::Small, FontId::new(10.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();
    s.spacing.item_spacing = egui::vec2(6.0, 4.0);
    s.spacing.button_padding = egui::vec2(8.0, 2.0);
    s.spacing.interact_size = egui::vec2(24.0, 20.0);
    s.spacing.menu_margin = egui::Margin::same(2);
    s.spacing.window_margin = egui::Margin::same(6);
    s.spacing.indent = 14.0;
    s.spacing.icon_width = 13.0;
    s.spacing.icon_width_inner = 7.0;
    s.spacing.icon_spacing = 4.0;
    s.interaction.selectable_labels = false;

    let v = &mut s.visuals;
    let square = CornerRadius::ZERO;
    v.window_corner_radius = square;
    v.menu_corner_radius = square;
    v.window_shadow = egui::Shadow { offset: [2, 2], blur: 6, spread: 0, color: Color32::from_black_alpha(50) };
    v.popup_shadow = v.window_shadow;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_fill = Color32::WHITE;
    v.panel_fill = PANEL;
    v.extreme_bg_color = Color32::WHITE;
    v.faint_bg_color = Color32::from_rgb(244, 244, 246);
    v.selection.bg_fill = Color32::from_rgb(204, 228, 247);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = ACCENT;
    v.striped = false;
    v.button_frame = true;

    let w = &mut v.widgets;
    for widget in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        widget.corner_radius = square;
        widget.expansion = 0.0;
    }
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(218, 218, 220));
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    w.inactive.weak_bg_fill = Color32::from_rgb(225, 225, 225);
    w.inactive.bg_fill = Color32::from_rgb(225, 225, 225);
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    w.hovered.weak_bg_fill = Color32::from_rgb(229, 241, 251);
    w.hovered.bg_fill = Color32::from_rgb(229, 241, 251);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    w.active.weak_bg_fill = Color32::from_rgb(204, 228, 247);
    w.active.bg_fill = Color32::from_rgb(204, 228, 247);
    w.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0, 84, 153));
    w.active.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    w.open.weak_bg_fill = Color32::from_rgb(204, 228, 247);
    w.open.bg_fill = Color32::from_rgb(204, 228, 247);
    w.open.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0, 84, 153));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_theme_is_light_square_and_twelve_pixels() {
        let ctx = egui::Context::default();
        apply(&ctx);
        let s = ctx.style();
        assert!(!s.visuals.dark_mode);
        assert_eq!(s.text_styles[&TextStyle::Body].size, 12.0);
        assert_eq!(s.visuals.window_corner_radius, CornerRadius::ZERO);
        assert_eq!(s.visuals.widgets.inactive.corner_radius, CornerRadius::ZERO);
        assert_eq!(s.visuals.widgets.inactive.bg_stroke.width, 1.0);
    }
}
