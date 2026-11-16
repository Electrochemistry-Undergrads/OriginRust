//! Extra Classic choices use the same captured graph-object request as the basic palette.
use super::*;

pub(super) fn show(ui: &mut Ui, captured: &ObjectSnapshot, fill: bool, actions: &mut Vec<Action>) {
    let prefix = if fill { "Fill Color" } else { "Line Color" };
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
        let change = if fill { object_format::Change::Fill(Some(color)) } else { object_format::Change::Line(color) };
        actions.push(Action::FormatObject(captured.request(change)));
        ui.close();
    }
}
