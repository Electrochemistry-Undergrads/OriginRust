//! Pieces every dialog in Origin's layout shares: the button row in Windows order (OK, Cancel,
//! Apply), the "Dialog Theme" row of the X-Function style windows and the tree node with a small
//! plus or minus box that opens and closes a group of options.

use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke, vec2};

/// The three buttons at the bottom right of a dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButton {
    Ok,
    Cancel,
    Apply,
}

/// Draws Apply, Cancel and OK so that they read OK, Cancel, Apply from the left. Call it inside a
/// right-to-left layout (`ui.with_layout(Layout::right_to_left(..))`); anything drawn before it in
/// the closure ends up to the right of Apply. `apply_enabled` greys Apply, as Origin does until
/// something has changed.
pub fn ok_cancel_apply(ui: &mut egui::Ui, apply_enabled: bool) -> Option<DialogButton> {
    let mut pressed = None;
    if ui.add_enabled(apply_enabled, egui::Button::new("Apply")).clicked() {
        pressed = Some(DialogButton::Apply);
    }
    if ui.button("Cancel").clicked() {
        pressed = Some(DialogButton::Cancel);
    }
    if ui.button("OK").clicked() {
        pressed = Some(DialogButton::Ok);
    }
    pressed
}

/// A separate row with the three buttons at the right edge.
pub fn button_row(ui: &mut egui::Ui, apply_enabled: bool) -> Option<DialogButton> {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| ok_cancel_apply(ui, apply_enabled)).inner
}

/// The first row of an X-Function dialog: the label "Dialog Theme", a wide box for the name of a
/// saved theme and the small arrow that opens the menu of theme actions. Sheaf has no saved
/// themes, so the row is shown grey, as the picture of Origin's dialogs shows it with none chosen.
pub fn dialog_theme_row(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Dialog Theme");
        ui.add_enabled_ui(false, |ui| {
            let mut text = String::new();
            let width = (ui.available_width() - 34.0).max(80.0);
            ui.add(egui::TextEdit::singleline(&mut text).desired_width(width));
            ui.add_sized([20.0, 20.0], egui::Button::new("\u{25b6}"));
        });
    });
}

/// A one-line read-only description under the theme row.
pub fn description_row(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.label("Description");
        let mut shown = text.to_string();
        ui.add(egui::TextEdit::singleline(&mut shown).interactive(false).desired_width(f32::INFINITY));
    });
}

/// A group of options under a node with a plus or minus box, as in the option trees of Origin's
/// dialogs. Starts open.
pub fn tree_node<R>(ui: &mut egui::Ui, id: &str, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) {
    egui::CollapsingHeader::new(RichText::new(title).strong())
        .id_salt(id)
        .default_open(true)
        .icon(|ui, openness, response| {
            let rect = egui::Rect::from_center_size(response.rect.center(), vec2(9.0, 9.0));
            let stroke = Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 90));
            ui.painter().rect(rect, 0.0, Color32::WHITE, stroke, egui::StrokeKind::Inside);
            let c = rect.center();
            ui.painter().line_segment([c - vec2(2.0, 0.0), c + vec2(2.0, 0.0)], stroke);
            if openness < 0.5 {
                ui.painter().line_segment([c - vec2(0.0, 2.0), c + vec2(0.0, 2.0)], stroke);
            }
        })
        .show(ui, |ui| {
            add(ui);
        });
}

/// A read-only text box that looks like a field (used for ranges and descriptions).
pub fn read_only_box(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    let mut shown = text.to_string();
    ui.add(egui::TextEdit::singleline(&mut shown).interactive(false).desired_width(width))
}

/// Blue bold text for a hint or state message, as Origin's dialogs draw them.
pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).strong().color(Color32::from_rgb(0, 0, 200)));
}

/// A thin sunken area (the preview area): returns the rect to draw into.
pub fn preview_area(ui: &mut egui::Ui, size: egui::Vec2) -> egui::Rect {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(rect, 0.0, Color32::WHITE, Stroke::new(1.0_f32, crate::theme::BORDER), egui::StrokeKind::Inside);
    rect
}
