//! A disposable one-label graph uses the same public rendering path as export.
use eframe::egui;
use sheaf_core::Project;
use sheaf_core::graph::{Font, TextLabel};
use sheaf_core::template::Template;
use sheaf_plot::{ExportLayout, ExportSize, Margin, Raster};

#[derive(Clone, PartialEq)]
struct Key {
    text: String,
    font: Font,
    rotation: f64,
    pixels_per_point: f32,
}

#[derive(Default)]
pub(super) struct Preview {
    key: Option<Key>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
}

fn render(key: &Key) -> Result<Raster, String> {
    if key.text.chars().count() > 8192 {
        return Err("Text is too long for a preview.".into());
    }
    let mut project = Project::default();
    let graph = project.add_graph(Template::Line);
    let layer = &mut project.graph_mut(graph).unwrap().layers[0];
    layer.display.show_x_axes = false;
    layer.display.show_y_axes = false;
    layer.legend.visible = false;
    let mut label = TextLabel::new(key.text.clone(), 0.5, 0.5, key.font.clone());
    label.rotation_deg = key.rotation;
    layer.labels.push(label);
    let dpi = 72.0 * f64::from(key.pixels_per_point);
    let layout = ExportLayout { margin: Margin::Border, border_px: Some(8.0 * f64::from(key.pixels_per_point)) };
    let bounds = sheaf_plot::export_area(&project, graph, layout, Some(dpi)).map_err(|e| e.to_string())?;
    let scale = (360.0 / bounds.w).min(130.0 / bounds.h).min(1.0);
    if !scale.is_finite() || scale < 0.01 {
        return Err("Text is too large for a preview.".into());
    }
    sheaf_plot::export_raster(&project, graph, dpi, ExportSize { ratio: Some(scale), ..ExportSize::default() }, layout)
        .map_err(|e| e.to_string())
}

impl Preview {
    pub fn show(&mut self, ui: &mut egui::Ui, text: &str, font: Font, rotation: f64) {
        let key = Key { text: text.to_string(), font, rotation, pixels_per_point: ui.ctx().pixels_per_point() };
        if self.key.as_ref() != Some(&key) {
            self.texture = None;
            self.error = None;
            if !sheaf_plot::markup::plain(text).is_empty() {
                match render(&key) {
                    Ok(image) => {
                        let image = egui::ColorImage::from_rgba_unmultiplied(
                            [image.width as usize, image.height as usize],
                            &image.rgba,
                        );
                        self.texture =
                            Some(ui.ctx().load_texture("text-object-preview", image, egui::TextureOptions::LINEAR));
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            self.key = Some(key);
        }
        ui.label("Preview");
        egui::Frame::group(ui.style()).fill(egui::Color32::WHITE).show(ui, |ui| {
            ui.set_min_size(egui::vec2(370.0, 90.0));
            if let Some(texture) = &self.texture {
                let size = texture.size_vec2() / ui.ctx().pixels_per_point();
                ui.add(egui::Image::new((texture.id(), size)).alt_text("Formatted text preview"));
            } else if let Some(error) = &self.error {
                ui.label(error);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_uses_published_colour_and_decoration_rendering_and_bounds_its_image() {
        let key = Key {
            text: r"\c4(\u(cm\+(-2)))".into(),
            font: Font::new("Arial", 30.0),
            rotation: 25.0,
            pixels_per_point: 2.0,
        };
        let image = render(&key).unwrap();
        assert!(image.width <= 722 && image.height <= 262);
        assert!(
            image.rgba.as_chunks::<4>().0.iter().any(|p| p[2] > 150 && p[0] < 100 && p[1] < 100),
            "blue native-renderer strokes"
        );
    }
}
