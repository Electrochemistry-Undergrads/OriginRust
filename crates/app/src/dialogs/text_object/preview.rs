//! A disposable one-label graph uses the same public rendering path as export.
use eframe::egui;
use sheaf_core::graph::{AxisSide, Font, TextLabel};
use sheaf_core::template::Template;
use sheaf_core::{Color, Document, Project, WindowId};
use sheaf_plot::scene::{ItemList, LayerScene, ObjectKind, Scene};
use sheaf_plot::{ExportLayout, ExportSize, Margin, Raster};

#[derive(Clone, Copy)]
pub(super) struct Context<'a> {
    pub doc: &'a Document,
    pub graph: WindowId,
    pub layer: usize,
    pub object: ObjectKind,
}

#[derive(Clone, PartialEq)]
struct Key {
    text: String,
    font: Font,
    rotation: f64,
    pixels_per_point: f32,
    context: Option<(u64, u64, WindowId, usize, ObjectKind)>,
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
        let key = Key {
            text: text.to_string(),
            font,
            rotation,
            pixels_per_point: ui.ctx().pixels_per_point(),
            context: None,
        };
        self.show_key(ui, key, render);
    }

    pub fn show_object(&mut self, ui: &mut egui::Ui, text: &str, font: Font, context: Context<'_>) {
        let key = Key {
            text: text.to_string(),
            font,
            rotation: 0.0,
            pixels_per_point: ui.ctx().pixels_per_point(),
            context: Some((
                context.doc.generation(),
                context.doc.revision(),
                context.graph,
                context.layer,
                context.object,
            )),
        };
        self.show_key(ui, key, |key| render_object(key, context));
    }

    fn show_key(&mut self, ui: &mut egui::Ui, key: Key, render: impl FnOnce(&Key) -> Result<Raster, String>) {
        if self.key.as_ref() != Some(&key) {
            self.texture = None;
            self.error = None;
            if !sheaf_plot::markup::plain(&key.text).is_empty() {
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

fn render_object(key: &Key, context: Context<'_>) -> Result<Raster, String> {
    if key.text.chars().count() > 8192 {
        return Err("Text is too long for a preview.".into());
    }
    let mut project = context.doc.project().clone();
    let graph = project.graph_mut(context.graph).ok_or("The graph no longer exists.")?;
    let layer_index = if matches!(context.object, ObjectKind::XTitle | ObjectKind::YTitle) {
        // A single layer avoids shared-title suppression; its data and metadata stay intact.
        let mut layer = graph.layers.get(context.layer).ok_or("The layer no longer exists.")?.clone();
        layer.link = None;
        graph.layers = vec![layer];
        0
    } else {
        context.layer
    };
    let layer = graph.layers.get_mut(layer_index).ok_or("The layer no longer exists.")?;
    layer.display = Default::default();
    match context.object {
        ObjectKind::XTitle | ObjectKind::YTitle => {
            let axis = if context.object == ObjectKind::YTitle { &mut layer.y_axis } else { &mut layer.x_axis };
            axis.title = Some(key.text.clone());
            axis.title_font = key.font.clone();
            axis.show_title = true;
            if axis.side == AxisSide::Hidden {
                axis.side = AxisSide::Normal;
            }
        }
        ObjectKind::Legend => {
            layer.legend.text = Some(key.text.clone());
            layer.legend.font = key.font.clone();
            layer.legend.visible = true;
        }
        _ => return Err("This object has no text preview.".into()),
    }
    let source = sheaf_plot::graph_scene(&project, context.graph).map_err(|e| e.to_string())?;
    let (items, bounds) = source
        .layers
        .iter()
        .find_map(|layer| {
            let object = layer.objects.iter().find(|o| o.layer == layer_index && o.kind == context.object)?;
            let items = match object.list {
                ItemList::Overlay => &layer.overlay,
                ItemList::Plots => &layer.plots,
            };
            Some((items.get(object.items.clone())?.to_vec(), object.bounds))
        })
        .ok_or("There is no text to preview.")?;
    let region = sheaf_plot::Rect { x: bounds.x - 8.0, y: bounds.y - 8.0, w: bounds.w + 16.0, h: bounds.h + 16.0 };
    let scale = (360.0 / region.w).min(130.0 / region.h).min(1.0);
    if !scale.is_finite() || scale < 0.01 {
        return Err("Text is too large for a preview.".into());
    }
    let scene = Scene {
        width: source.width,
        height: source.height,
        background: Color::WHITE,
        layers: vec![LayerScene {
            clip: region,
            grid: vec![],
            plots: vec![],
            overlay: items,
            underlay_len: 0,
            objects: vec![],
        }],
    };
    sheaf_plot::render_scene_raster_region(&scene, region, f64::from(key.pixels_per_point) * scale)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cropped_scene_preview_includes_objects_outside_the_graph_page() {
        let region = sheaf_plot::Rect { x: -45.0, y: -35.0, w: 30.0, h: 20.0 };
        let scene = Scene {
            width: 80.0,
            height: 60.0,
            background: Color::WHITE,
            layers: vec![LayerScene {
                clip: region,
                grid: vec![],
                plots: vec![],
                underlay_len: 0,
                objects: vec![],
                overlay: vec![sheaf_plot::scene::Item::Rect {
                    rect: sheaf_plot::Rect { x: -40.0, y: -30.0, w: 20.0, h: 10.0 },
                    fill: Some(Color::RED),
                    stroke: None,
                }],
            }],
        };
        let image = sheaf_plot::render_scene_raster_region(&scene, region, 2.0).unwrap();
        assert_eq!((image.width, image.height), (60, 40));
        assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[0] > 200 && p[1] < 40 && p[2] < 40));
    }

    #[test]
    fn preview_uses_published_colour_and_decoration_rendering_and_bounds_its_image() {
        let key = Key {
            text: r"\c4(\u(cm\+(-2)))".into(),
            font: Font::new("Arial", 30.0),
            rotation: 25.0,
            pixels_per_point: 2.0,
            context: None,
        };
        let image = render(&key).unwrap();
        assert!(image.width <= 722 && image.height <= 262);
        assert!(
            image.rgba.as_chunks::<4>().0.iter().any(|p| p[2] > 150 && p[0] < 100 && p[1] < 100),
            "blue native-renderer strokes"
        );
    }
}
