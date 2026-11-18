//! A one-off opaque RGB chooser for a captured worksheet Format-toolbar target.

use crate::text_format::{Change, Request, Snapshot};
use eframe::egui;
use sheaf_core::Color;

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Text,
    Fill,
}

#[derive(Clone, Debug)]
pub struct WorksheetColorDialog {
    snapshot: Snapshot,
    kind: Kind,
    channels: [String; 3],
    focus_first: bool,
}

impl WorksheetColorDialog {
    pub fn new(snapshot: Snapshot, kind: Kind) -> Option<Self> {
        let worksheet = snapshot.worksheet?;
        let color = match kind {
            Kind::Text => snapshot.font.color,
            Kind::Fill => worksheet.fill.unwrap_or(Color::rgb(255, 255, 255)),
        };
        Some(Self {
            snapshot,
            kind,
            channels: [color.r.to_string(), color.g.to_string(), color.b.to_string()],
            focus_first: true,
        })
    }

    pub fn matches(&self, current: &Snapshot) -> bool {
        current.worksheet.is_some()
            && current.target == self.snapshot.target
            && current.generation == self.snapshot.generation
            && current.revision == self.snapshot.revision
    }

    fn color(&self) -> Option<Color> {
        Some(Color::rgb(
            self.channels[0].trim().parse().ok()?,
            self.channels[1].trim().parse().ok()?,
            self.channels[2].trim().parse().ok()?,
        ))
    }

    /// A request is returned only on OK; the caller closes this dialog before dispatching it.
    pub fn show(&mut self, ctx: &egui::Context) -> (bool, Option<Request>) {
        let ids = ["worksheet_color_red", "worksheet_color_green", "worksheet_color_blue"].map(egui::Id::new);
        let owns_keys = ctx.memory(|m| ids.iter().any(|id| m.has_focus(*id)));
        let enter = owns_keys && ctx.input(|i| i.modifiers == egui::Modifiers::NONE && i.key_pressed(egui::Key::Enter));
        let mut cancel = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut accept = false;
        let mut open = true;
        let title = match self.kind {
            Kind::Text => "Worksheet Text Color",
            Kind::Fill => "Worksheet Fill Color",
        };
        egui::Window::new(title)
            .id(egui::Id::new("worksheet_custom_color"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .min_width(220.0)
            .show(ctx, |ui| {
                if let Some(color) = crate::custom_colors::menu(ui, "Worksheet color") {
                    self.channels = [color.r, color.g, color.b].map(|c| c.to_string());
                }
                egui::Grid::new("worksheet_rgb_fields").num_columns(2).show(ui, |ui| {
                    for (index, label) in ["Red", "Green", "Blue"].into_iter().enumerate() {
                        ui.label(label);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.channels[index]).id(ids[index]).desired_width(70.0),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::TextEdit,
                                true,
                                format!("Worksheet color: {label}"),
                            )
                        });
                        if self.focus_first && index == 0 {
                            response.request_focus();
                        }
                        ui.end_row();
                    }
                });
                let chosen = self.color();
                if let Some(color) = chosen {
                    ui.horizontal(|ui| {
                        ui.label("Preview");
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(90.0, 24.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(color.r, color.g, color.b));
                    });
                } else {
                    ui.label("Enter whole numbers from 0 to 255.");
                }
                crate::custom_colors::add_button(ui, chosen, "Worksheet color");
                ui.horizontal(|ui| {
                    accept = ui.add_enabled(chosen.is_some(), egui::Button::new("OK")).clicked();
                    cancel |= ui.button("Cancel").clicked();
                });
            });
        self.focus_first = false;
        if !open || cancel {
            return (false, None);
        }
        if (accept || enter)
            && let Some(color) = self.color()
        {
            let change = match self.kind {
                Kind::Text => Change::Color(color),
                Kind::Fill => Change::Fill(Some(color)),
            };
            return (false, Some(self.snapshot.request(change)));
        }
        (true, None)
    }
}
