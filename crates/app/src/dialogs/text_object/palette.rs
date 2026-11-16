//! Published Classic colours and opaque RGB text escapes (graphing 7.2-7.3).
use eframe::egui;
use sheaf_core::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextColor {
    Black,
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    White,
    Rgb([u8; 3]),
}

impl TextColor {
    pub const ALL: [Self; 8] =
        [Self::Black, Self::Red, Self::Green, Self::Blue, Self::Cyan, Self::Magenta, Self::Yellow, Self::White];

    pub fn label(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Cyan => "Cyan",
            Self::Magenta => "Magenta",
            Self::Yellow => "Yellow",
            Self::White => "White",
            Self::Rgb(_) => "Custom RGB...",
        }
    }

    pub fn prefix(self) -> String {
        let prefix = match self {
            Self::Black => r"\c1(",
            Self::Red => r"\c2(",
            Self::Green => r"\c3(",
            Self::Blue => r"\c4(",
            Self::Cyan => r"\c5(",
            Self::Magenta => r"\c6(",
            Self::Yellow => r"\c7(",
            Self::White => r"\c18(",
            Self::Rgb([r, g, b]) => {
                let code = 0x0100_0001_u32 + u32::from_le_bytes([r, g, b, 0]);
                return format!(r"\c{code}(");
            }
        };
        prefix.to_string()
    }

    fn channels(self) -> [u8; 3] {
        match self {
            Self::Black => [0, 0, 0],
            Self::Red => [255, 0, 0],
            Self::Green => [0, 255, 0],
            Self::Blue => [0, 0, 255],
            Self::Cyan => [0, 255, 255],
            Self::Magenta => [255, 0, 255],
            Self::Yellow => [255, 255, 0],
            Self::White => [255, 255, 255],
            Self::Rgb(rgb) => rgb,
        }
    }
}

pub(super) struct Picker {
    channels: [String; 3],
}

impl Picker {
    pub fn new(color: Color) -> Self {
        Self { channels: [color.r, color.g, color.b].map(|c| c.to_string()) }
    }

    fn rgb(&self) -> Option<[u8; 3]> {
        Some([
            self.channels[0].trim().parse().ok()?,
            self.channels[1].trim().parse().ok()?,
            self.channels[2].trim().parse().ok()?,
        ])
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<TextColor> {
        let mut selected = None;
        for color in TextColor::ALL {
            if ui.button(color.label()).clicked() {
                self.channels = color.channels().map(|c| c.to_string());
                selected = Some(color);
            }
        }
        ui.separator();
        egui::containers::menu::SubMenuButton::new("Custom RGB...")
            .config(
                egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
            )
            .ui(ui, |ui| {
                egui::Grid::new("text_color_rgb").show(ui, |ui| {
                    for (index, name) in ["Red", "Green", "Blue"].into_iter().enumerate() {
                        let label = ui.label(name);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.channels[index])
                                .id(ui.make_persistent_id(("text_color_channel", index)))
                                .desired_width(48.0)
                                .char_limit(3),
                        )
                        .labelled_by(label.id);
                        ui.end_row();
                    }
                });
                let rgb = self.rgb();
                if let Some([r, g, b]) = rgb {
                    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(r, g, b)).show(ui, |ui| {
                        ui.allocate_space(egui::vec2(70.0, 12.0));
                    });
                } else {
                    ui.label("Use whole numbers from 0 to 255.");
                }
                if ui.add_enabled(rgb.is_some(), egui::Button::new("Apply color")).clicked() {
                    selected = rgb.map(TextColor::Rgb);
                    ui.close();
                }
            });
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_rgb_matches_the_renderer_at_channel_carries_and_boundaries() {
        for rgb in [[0, 0, 0], [255, 255, 255], [255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0], [17, 34, 51]] {
            let raw = format!("{}sample)", TextColor::Rgb(rgb).prefix());
            let runs = sheaf_plot::markup::parse(&raw);
            assert_eq!(runs.len(), 1, "{raw}");
            assert_eq!(runs[0].text, "sample");
            assert_eq!(runs[0].color, Some(Color::rgb(rgb[0], rgb[1], rgb[2])), "{raw}");
        }
    }
}
