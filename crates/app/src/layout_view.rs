//! A layout page window (T237): the page on a grey ground, scaled to fit the window, with a live
//! picture of each graph item. A graph that no longer exists leaves an outlined empty box. The
//! pictures are rendered by `sheaf-plot` and kept until the project or the size changes.

use crate::actions::Action;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, TextureHandle, TextureOptions, vec2};
use sheaf_core::layout_page::LayoutContent;
use sheaf_core::{Document, WindowId};
use std::collections::HashMap;

const GROUND: Color32 = Color32::from_gray(192);

/// What a layout window keeps between frames.
#[derive(Default)]
pub struct LayoutUi {
    /// Graph pictures: (document revision, pixel width) they were drawn for, and the texture.
    pictures: HashMap<WindowId, ((u64, u32), TextureHandle)>,
}

/// Where the page lies in `area`: the largest rectangle of the page's shape, centred.
pub fn page_rect(area: Rect, width_pt: f64, height_pt: f64) -> Rect {
    let aspect = (width_pt / height_pt.max(1.0)) as f32;
    let margin = 8.0;
    let (aw, ah) = ((area.width() - 2.0 * margin).max(1.0), (area.height() - 2.0 * margin).max(1.0));
    let (w, h) = if aw / ah > aspect { (ah * aspect, ah) } else { (aw, aw / aspect) };
    Rect::from_center_size(area.center(), vec2(w, h))
}

pub fn show(ui: &mut egui::Ui, doc: &Document, id: WindowId, state: &mut LayoutUi, _actions: &mut Vec<Action>) {
    let Some(page) = doc.project().layout(id) else { return };
    let area = ui.available_rect_before_wrap();
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, GROUND);
    let rect = page_rect(area, page.width_pt, page.height_pt);
    painter.rect_filled(rect, 0.0, Color32::WHITE);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, Color32::from_gray(96)), egui::StrokeKind::Outside);
    let ppp = ui.ctx().pixels_per_point();
    for item in &page.items {
        let r = Rect::from_min_size(
            Pos2::new(rect.left() + item.x as f32 * rect.width(), rect.top() + item.y as f32 * rect.height()),
            vec2(item.w as f32 * rect.width(), item.h as f32 * rect.height()),
        );
        let LayoutContent::Graph { graph } = item.content;
        let Some(g) = doc.project().graph(graph) else {
            painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::GRAY), egui::StrokeKind::Inside);
            continue;
        };
        let px = (r.width() * ppp).round().max(1.0) as u32;
        let key = (doc.revision(), px);
        let fresh = state.pictures.get(&graph).is_some_and(|(k, _)| *k == key);
        if !fresh {
            let scale = f64::from(px) / g.page.width_pt.max(1.0);
            if let Ok(raster) = sheaf_plot::render_raster(doc.project(), graph, scale) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [raster.width as usize, raster.height as usize],
                    &raster.rgba,
                );
                let tex = ui.ctx().load_texture(format!("layout-{}-{}", id.0, graph.0), image, TextureOptions::LINEAR);
                state.pictures.insert(graph, (key, tex));
            }
        }
        if let Some((_, tex)) = state.pictures.get(&graph) {
            painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
    }
    // The window takes the space (and the clicks) of its whole area.
    ui.allocate_rect(area, egui::Sense::hover());
}

#[cfg(test)]
mod tests {
    use super::page_rect;
    use eframe::egui::{Pos2, Rect};

    #[test]
    fn the_page_keeps_its_shape_and_is_centred() {
        let area = Rect::from_min_max(Pos2::ZERO, Pos2::new(1000.0, 400.0));
        let r = page_rect(area, 792.0, 612.0);
        assert!((r.width() / r.height() - 792.0 / 612.0).abs() < 1e-4);
        assert!((r.center().x - 500.0).abs() < 1e-3 && (r.height() - 384.0).abs() < 1e-3);
    }
}
