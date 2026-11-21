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
    /// Graph and worksheet pictures: (document revision, pixel width) they were drawn for, and the
    /// texture.
    pictures: HashMap<(usize, LayoutContent), ((u64, u32, u32), TextureHandle)>,
    /// The selected item (T237 slice 2).
    pub selected: Option<usize>,
    /// An item being moved or resized: (index, resize, how far the pointer went in page fractions).
    drag: Option<(usize, bool, (f64, f64))>,
    /// Where the page was drawn in the last frame (tests aim at it).
    pub page_on_screen: Option<Rect>,
}

/// The size of the corner handle that resizes, in screen points.
const HANDLE: f32 = 7.0;

/// An item's rectangle in fractions of the page after a drag of `delta` (page fractions): moved, or
/// resized from its bottom right corner (at least 2 % of the page each way).
pub fn dragged(rect: (f64, f64, f64, f64), resize: bool, delta: (f64, f64)) -> (f64, f64, f64, f64) {
    let (x, y, w, h) = rect;
    if resize { (x, y, (w + delta.0).max(0.02), (h + delta.1).max(0.02)) } else { (x + delta.0, y + delta.1, w, h) }
}

/// Layout > Export Page as Image (T237 slices 3 and 7): writes `layout` to `path` in the format of
/// its extension (`.png` when it has none; pictures at 300 dpi, or SVG/PDF drawings) and returns the
/// path written.
pub fn write_layout_image(
    project: &sheaf_core::Project,
    layout: WindowId,
    path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let mut path = path.to_path_buf();
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_ascii_lowercase(),
        None => {
            path.set_extension("png");
            "png".to_string()
        }
    };
    let bytes = sheaf_plot::render_layout_image(project, layout, &ext, 300.0).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Where the page lies in `area`: the largest rectangle of the page's shape, centred.
pub fn page_rect(area: Rect, width_pt: f64, height_pt: f64) -> Rect {
    let aspect = (width_pt / height_pt.max(1.0)) as f32;
    let margin = 8.0;
    let (aw, ah) = ((area.width() - 2.0 * margin).max(1.0), (area.height() - 2.0 * margin).max(1.0));
    let (w, h) = if aw / ah > aspect { (ah * aspect, ah) } else { (aw, aw / aspect) };
    Rect::from_center_size(area.center(), vec2(w, h))
}

pub fn show(ui: &mut egui::Ui, doc: &Document, id: WindowId, state: &mut LayoutUi, actions: &mut Vec<Action>) {
    let Some(page) = doc.project().layout(id) else { return };
    if state.selected.is_some_and(|k| k >= page.items.len()) {
        state.selected = None;
    }
    if state.drag.is_some_and(|d| d.0 >= page.items.len()) {
        state.drag = None;
    }
    let area = ui.available_rect_before_wrap();
    // The page takes the clicks and drags of its whole area (T237 slice 2).
    let response = ui.interact(area, ui.id().with(("layout_page", id.0)), egui::Sense::click_and_drag());
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, GROUND);
    let rect = page_rect(area, page.width_pt, page.height_pt);
    state.page_on_screen = Some(rect);
    let bg = page.background;
    painter.rect_filled(rect, 0.0, Color32::from_rgb(bg.r, bg.g, bg.b));
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, Color32::from_gray(96)), egui::StrokeKind::Outside);
    let ppp = ui.ctx().pixels_per_point();
    let screen = |(x, y, w, h): (f64, f64, f64, f64)| {
        Rect::from_min_size(
            Pos2::new(rect.left() + x as f32 * rect.width(), rect.top() + y as f32 * rect.height()),
            vec2(w as f32 * rect.width(), h as f32 * rect.height()),
        )
    };
    let shown = |k: usize, it: &sheaf_core::layout_page::LayoutItem| {
        let base = (it.x, it.y, it.w, it.h);
        match state.drag {
            Some((d, resize, delta)) if d == k => dragged(base, resize, delta),
            _ => base,
        }
    };
    for (k, item) in page.items.iter().enumerate() {
        let r = screen(shown(k, item));
        let content = (k, item.content.clone());
        let px = (r.width() * ppp).round().max(1.0) as u32;
        let py = (r.height() * ppp).round().max(1.0) as u32;
        let key = (doc.revision(), px, py);
        let fresh = state.pictures.get(&content).is_some_and(|(k, _)| *k == key);
        if !fresh {
            let (_, _, sw, sh) = shown(k, item);
            let size_pt = (sw * page.width_pt, sh * page.height_pt);
            match sheaf_plot::render_layout_item(doc.project(), &content.1, size_pt, px) {
                None => {
                    // What the item showed is gone: an empty frame.
                    state.pictures.remove(&content);
                    painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::GRAY), egui::StrokeKind::Inside);
                    continue;
                }
                Some(Ok(raster)) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [raster.width as usize, raster.height as usize],
                        &raster.rgba,
                    );
                    let name = format!("layout-{}-{k}", id.0);
                    let tex = ui.ctx().load_texture(name, image, TextureOptions::LINEAR);
                    state.pictures.insert(content.clone(), (key, tex));
                }
                Some(Err(_)) => {}
            }
        }
        if let Some((_, tex)) = state.pictures.get(&content) {
            painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
    }
    // The selection: a frame and the resize handle at the bottom right corner.
    if let Some(k) = state.selected
        && let Some(item) = page.items.get(k)
    {
        let r = screen(shown(k, item));
        let blue = Color32::from_rgb(0, 90, 200);
        painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, blue), egui::StrokeKind::Outside);
        painter.rect_filled(Rect::from_center_size(r.right_bottom(), vec2(HANDLE, HANDLE)), 0.0, blue);
    }
    let hit = |p: Pos2| {
        page.items.iter().enumerate().rev().find(|(_, it)| screen((it.x, it.y, it.w, it.h)).contains(p)).map(|(k, _)| k)
    };
    if response.clicked()
        && let Some(p) = response.interact_pointer_pos()
    {
        state.selected = hit(p);
    }
    // A double-click on text opens it for editing (T237 slice 8), on a drawn object its
    // properties (slice 11).
    if response.double_clicked()
        && let Some(p) = response.interact_pointer_pos()
        && let Some(k) = hit(p)
    {
        match page.items[k].content {
            LayoutContent::Text { .. } => actions.push(Action::LayoutText { layout: id, index: Some(k) }),
            LayoutContent::Shape { .. } => actions.push(Action::LayoutShape { layout: id, index: k }),
            _ => {}
        }
    }
    if response.drag_started()
        && let Some(p) = ui.input(|i| i.pointer.press_origin())
    {
        let on_handle = state.selected.and_then(|k| page.items.get(k).map(|it| (k, it))).and_then(|(k, it)| {
            let corner = screen((it.x, it.y, it.w, it.h)).right_bottom();
            (corner.distance(p) <= HANDLE).then_some(k)
        });
        match on_handle {
            Some(k) => state.drag = Some((k, true, (0.0, 0.0))),
            None => {
                state.selected = hit(p);
                state.drag = state.selected.map(|k| (k, false, (0.0, 0.0)));
            }
        }
    }
    if let Some((k, resize, delta)) = state.drag {
        let d = response.drag_delta();
        let delta = (delta.0 + f64::from(d.x / rect.width()), delta.1 + f64::from(d.y / rect.height()));
        state.drag = Some((k, resize, delta));
        if response.drag_stopped() {
            state.drag = None;
            if (delta.0 != 0.0 || delta.1 != 0.0)
                && let Some(it) = page.items.get(k)
            {
                let rect = dragged((it.x, it.y, it.w, it.h), resize, delta);
                actions.push(Action::LayoutSetItem { layout: id, index: k, rect, resize });
            }
        }
    }
    // Delete on the selected item is handled with the app's keys (`SheafApp::shortcuts`).
}

#[cfg(test)]
mod tests {
    use super::{dragged, page_rect};

    #[test]
    fn a_drag_moves_or_resizes_from_the_corner() {
        assert_eq!(dragged((0.1, 0.2, 0.3, 0.4), false, (0.05, -0.1)), (0.15000000000000002, 0.1, 0.3, 0.4));
        assert_eq!(dragged((0.1, 0.2, 0.3, 0.4), true, (0.1, 0.1)), (0.1, 0.2, 0.4, 0.5));
        assert_eq!(dragged((0.1, 0.2, 0.3, 0.4), true, (-1.0, -1.0)), (0.1, 0.2, 0.02, 0.02), "a smallest size");
    }
    use eframe::egui::{Pos2, Rect};

    #[test]
    fn the_page_keeps_its_shape_and_is_centred() {
        let area = Rect::from_min_max(Pos2::ZERO, Pos2::new(1000.0, 400.0));
        let r = page_rect(area, 792.0, 612.0);
        assert!((r.width() / r.height() - 792.0 / 612.0).abs() < 1e-4);
        assert!((r.center().x - 500.0).abs() < 1e-3 && (r.height() - 384.0).abs() < 1e-3);
    }
}
