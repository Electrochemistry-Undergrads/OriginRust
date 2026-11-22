//! Render a draft merge without allocating IDs or history in the real document.
use super::MergeDraft;
use eframe::egui;
use sheaf_core::{Document, Project};
use sheaf_plot::Raster;

#[derive(Clone, PartialEq)]
struct Key {
    generation: u64,
    revision: u64,
    draft: MergeDraft,
    valid_folder: bool,
    pixels_per_point: f32,
}

#[derive(Default)]
pub(super) struct Preview {
    auto: bool,
    key: Option<Key>,
    texture: Option<egui::TextureHandle>,
    layers: Option<usize>,
    error: Option<String>,
}

fn render(project: &Project, draft: &MergeDraft, pixels_per_point: f32) -> Result<(Raster, usize), String> {
    let mut project = project.clone();
    let id = draft.merge(&mut project).map_err(|e| e.to_string())?;
    let graph = project.graph(id).unwrap();
    // Fit the whole page, including its margins. Bound raster allocation even for a 200-inch page.
    let scale =
        (640.0 / graph.page.width_pt).min(480.0 / graph.page.height_pt) * f64::from(pixels_per_point.clamp(1.0, 2.0));
    let layers = graph.layers.len();
    let raster = sheaf_plot::render_raster(&project, id, scale).map_err(|e| e.to_string())?;
    Ok((raster, layers))
}

impl Preview {
    pub(super) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui, doc: &Document, draft: MergeDraft, valid_folder: bool) {
        let key = Key {
            generation: doc.generation(),
            revision: doc.revision(),
            draft,
            valid_folder,
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        let changed = self.key.as_ref() != Some(&key);
        if changed {
            self.texture = None;
            self.layers = None;
            self.error = None;
        }
        let mut requested = false;
        ui.horizontal(|ui| {
            requested = ui.button("Preview").clicked();
            if ui.checkbox(&mut self.auto, "Auto Preview").changed() && self.auto {
                requested = true;
            }
        });
        if !key.valid_folder {
            self.error = Some("Source folder tree changed. Reopen Merge Graph Windows.".into());
        } else if requested || (changed && self.auto) {
            self.texture = None;
            self.layers = None;
            self.error = None;
            match render(doc.project(), &key.draft, key.pixels_per_point) {
                Ok((raster, layers)) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [raster.width as usize, raster.height as usize],
                        &raster.rgba,
                    );
                    self.texture =
                        Some(ui.ctx().load_texture("merge-graph-preview", image, egui::TextureOptions::LINEAR));
                    self.layers = Some(layers);
                }
                Err(error) => self.error = Some(error),
            }
        }
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_size(egui::vec2((ui.available_width() - 1.0).max(0.0), 240.0));
            if let Some(texture) = &self.texture {
                let size = texture.size_vec2();
                let ratio = (ui.available_width() / size.x).min(240.0 / size.y);
                ui.add(
                    egui::Image::new((texture.id(), size))
                        .fit_to_exact_size(size * ratio)
                        .alt_text("Merged graph preview"),
                );
            } else if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            } else {
                ui.label("Click Preview to see the merged graph.");
            }
        });
        if let Some(layers) = self.layers {
            let count = key.draft.options.sources.len();
            let source = if count == 1 { "source graph" } else { "source graphs" };
            let layer = if layers == 1 { "layer" } else { "layers" };
            ui.label(format!("{count} {source}, {layers} {layer}"));
        }
        self.key = Some(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::graph::{Font, LayerLink, ScaleLink, TextLabel};
    use sheaf_core::merge::MergeOptions;
    use sheaf_core::template::Template;

    #[test]
    fn preview_matches_the_committed_multilayer_page_without_changing_sources_or_ids() {
        let mut source = Project::default();
        let a = source.add_graph(Template::Line);
        let b = source.add_graph(Template::Line);
        let graph = source.graph_mut(a).unwrap();
        graph.layers[0].labels.push(TextLabel::new("Primary", 0.3, 0.4, Font::new("Arial", 18.0)));
        let mut child = graph.layers[0].clone();
        child.link = Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None });
        graph.layers.push(child);
        let before = source.clone();
        let options = MergeOptions {
            sources: vec![b, a],
            keep_sources: false,
            portrait: true,
            rows: Some(2),
            cols: Some(1),
            ..Default::default()
        };
        let draft = MergeDraft { options: options.clone(), ..Default::default() };
        let (raster, layers) = render(&source, &draft, 1.0).unwrap();
        assert_eq!(source, before);
        assert_eq!(layers, 3);
        let mut committed = source.clone();
        let id = committed.merge_graphs(&options).unwrap();
        assert_eq!(committed.graphs().count(), 1);
        let page = &committed.graph(id).unwrap().page;
        let expected =
            sheaf_plot::render_raster(&committed, id, (640.0 / page.width_pt).min(480.0 / page.height_pt)).unwrap();
        assert_eq!(raster, expected);
        assert!(raster.height > raster.width);
        assert!(raster.rgba.as_chunks::<4>().0.iter().any(|p| p[0..3] != [255, 255, 255]));
        // The disposable merge must not consume the source project's next graph identity.
        let mut without_preview = before;
        assert_eq!(source.add_graph(Template::Line), without_preview.add_graph(Template::Line));
    }

    #[test]
    fn failed_merge_and_extreme_page_sizes_do_not_mutate_the_project() {
        let mut source = Project::default();
        let a = source.add_graph(Template::Line);
        let b = source.add_graph(Template::Line);
        let before = source.clone();
        let failed =
            MergeDraft { options: MergeOptions { sources: vec![a], ..Default::default() }, ..Default::default() };
        assert!(render(&source, &failed, 1.0).is_err());
        for page in [(200.0, 0.5), (0.5, 200.0), (200.0, 200.0)] {
            let draft = MergeDraft {
                options: MergeOptions { sources: vec![a, b], page_in: Some(page), ..Default::default() },
                ..Default::default()
            };
            let (raster, _) = render(&source, &draft, 2.0).unwrap();
            assert!(raster.width <= 1280 && raster.height <= 960);
            assert!(raster.width > 0 && raster.height > 0);
        }
        assert_eq!(source, before);
    }
}
