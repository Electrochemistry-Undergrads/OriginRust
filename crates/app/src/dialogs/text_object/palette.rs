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
    DarkYellow,
    Navy,
    Purple,
    Wine,
    Olive,
    DarkCyan,
    Royal,
    Orange,
    Violet,
    Pink,
    LtGray,
    Gray,
    LtYellow,
    LtCyan,
    LtMagenta,
    DarkGray,
    Rgb([u8; 3]),
}

impl TextColor {
    pub const BASIC: [Self; 8] =
        [Self::Black, Self::Red, Self::Green, Self::Blue, Self::Cyan, Self::Magenta, Self::Yellow, Self::White];

    pub const EXTRA: [Self; 16] = [
        Self::DarkYellow,
        Self::Navy,
        Self::Purple,
        Self::Wine,
        Self::Olive,
        Self::DarkCyan,
        Self::Royal,
        Self::Orange,
        Self::Violet,
        Self::Pink,
        Self::LtGray,
        Self::Gray,
        Self::LtYellow,
        Self::LtCyan,
        Self::LtMagenta,
        Self::DarkGray,
    ];

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
            Self::DarkYellow => "Dark Yellow",
            Self::Navy => "Navy",
            Self::Purple => "Purple",
            Self::Wine => "Wine",
            Self::Olive => "Olive",
            Self::DarkCyan => "Dark Cyan",
            Self::Royal => "Royal",
            Self::Orange => "Orange",
            Self::Violet => "Violet",
            Self::Pink => "Pink",
            Self::LtGray => "LT Gray",
            Self::Gray => "Gray",
            Self::LtYellow => "LT Yellow",
            Self::LtCyan => "LT Cyan",
            Self::LtMagenta => "LT Magenta",
            Self::DarkGray => "Dark Gray",
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
            Self::DarkYellow => r"\c8(",
            Self::Navy => r"\c9(",
            Self::Purple => r"\c10(",
            Self::Wine => r"\c11(",
            Self::Olive => r"\c12(",
            Self::DarkCyan => r"\c13(",
            Self::Royal => r"\c14(",
            Self::Orange => r"\c15(",
            Self::Violet => r"\c16(",
            Self::Pink => r"\c17(",
            Self::LtGray => r"\c19(",
            Self::Gray => r"\c20(",
            Self::LtYellow => r"\c21(",
            Self::LtCyan => r"\c22(",
            Self::LtMagenta => r"\c23(",
            Self::DarkGray => r"\c24(",
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
            Self::DarkYellow => [128, 128, 0],
            Self::Navy => [0, 0, 128],
            Self::Purple => [128, 0, 128],
            Self::Wine => [128, 0, 0],
            Self::Olive => [0, 128, 0],
            Self::DarkCyan => [0, 128, 128],
            Self::Royal => [0, 0, 160],
            Self::Orange => [255, 128, 0],
            Self::Violet => [128, 0, 255],
            Self::Pink => [255, 0, 128],
            Self::LtGray => [192, 192, 192],
            Self::Gray => [128, 128, 128],
            Self::LtYellow => [255, 255, 128],
            Self::LtCyan => [128, 255, 255],
            Self::LtMagenta => [255, 128, 255],
            Self::DarkGray => [64, 64, 64],
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
        for color in TextColor::BASIC {
            if ui.button(color.label()).clicked() {
                self.channels = color.channels().map(|c| c.to_string());
                selected = Some(color);
            }
        }
        egui::containers::menu::SubMenuButton::new("More Classic colors").ui(ui, |ui| {
            for color in TextColor::EXTRA {
                if ui.button(color.label()).clicked() {
                    self.channels = color.channels().map(|c| c.to_string());
                    selected = Some(color);
                    ui.close();
                }
            }
        });
        ui.separator();
        if let Some(color) = crate::custom_colors::menu(ui, "Text color") {
            let rgb = [color.r, color.g, color.b];
            self.channels = rgb.map(|c| c.to_string());
            selected = Some(TextColor::Rgb(rgb));
        }
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
                crate::custom_colors::add_button(ui, rgb.map(|[r, g, b]| Color::rgb(r, g, b)), "Text color");
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
