//! The view transform of a graph window: zoom and scroll over the page (clean-room: behaviour only).
//!
//! Zoom 1 is Origin's 100 %: the whole page fits in the window. At a larger zoom the page is larger
//! than the window and `scroll` (the page point at the top left of the window's content area) says
//! which part shows. All of it is screen-only view state; the model and the export are untouched.

use eframe::egui::{Pos2, Rect, Vec2, vec2};

/// The grey border kept round the page at zoom 1, in UI points.
pub const MARGIN: f32 = 12.0;
/// The largest zoom: 3200 %.
pub const MAX_ZOOM: f32 = 32.0;
/// The zoom factor of one Zoom In click (Origin: 100 % to 200 %).
pub const CLICK_FACTOR: f32 = 2.0;
/// The largest side of the page raster, in pixels.
pub const MAX_RASTER_PX: f32 = 4096.0;

/// The zoom and scroll of one graph window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    /// 1.0 = the whole page fits in the window.
    pub factor: f32,
    /// The page point (points from the page's top left) at the top left of the content area.
    pub scroll: Vec2,
}

impl Default for Zoom {
    fn default() -> Self {
        Zoom { factor: 1.0, scroll: Vec2::ZERO }
    }
}

/// Where the page is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// The page on screen.
    pub image: Rect,
    /// UI points per page point (fit scale times zoom).
    pub scale: f32,
    /// The scroll after clamping (zero on an axis where the page is smaller than the window).
    pub scroll: Vec2,
}

/// UI points per page point at zoom 1.
pub fn fit_scale(window: Rect, page: Vec2) -> f32 {
    ((window.width() - 2.0 * MARGIN) / page.x).min((window.height() - 2.0 * MARGIN) / page.y).max(0.05)
}

/// The part of the window the page is drawn in.
pub fn content(window: Rect) -> Rect {
    window.shrink(MARGIN)
}

/// The page rectangle for `zoom` in `window`. The scroll is clamped so that the page cannot be
/// scrolled away; an axis on which the page is smaller than the window is centred.
pub fn layout(window: Rect, page: Vec2, zoom: Zoom) -> Layout {
    let scale = fit_scale(window, page) * zoom.factor.clamp(1.0, MAX_ZOOM);
    let content = content(window);
    let axis = |avail: f32, size: f32, scroll: f32| -> (f32, f32) {
        if size <= avail + 0.01 {
            ((avail - size) / 2.0, 0.0)
        } else {
            let scroll = scroll.clamp(0.0, (size - avail) / scale);
            (-scroll * scale, scroll)
        }
    };
    let (ox, sx) = axis(content.width(), page.x * scale, zoom.scroll.x);
    let (oy, sy) = axis(content.height(), page.y * scale, zoom.scroll.y);
    let min = content.min + vec2(ox, oy);
    Layout { image: Rect::from_min_size(min, page * scale), scale, scroll: vec2(sx, sy) }
}

/// The zoom with `factor` after the change, keeping the page point under `anchor` (a screen
/// position) where it is, as far as the scroll range allows.
pub fn zoom_about(window: Rect, page: Vec2, zoom: Zoom, factor: f32, anchor: Pos2) -> Zoom {
    let old = layout(window, page, zoom);
    let factor = factor.clamp(1.0, MAX_ZOOM);
    let scale = fit_scale(window, page) * factor;
    let under = (anchor - old.image.min) / old.scale;
    let min = anchor - under * scale;
    let scroll = (content(window).min - min) / scale;
    let new = Zoom { factor, scroll };
    Zoom { factor, scroll: layout(window, page, new).scroll }
}

/// The zoom after moving the view by `delta` UI points (a pan drag moves the page with the pointer).
pub fn pan(window: Rect, page: Vec2, zoom: Zoom, delta: Vec2) -> Zoom {
    let l = layout(window, page, zoom);
    let scroll = zoom.scroll - delta / l.scale;
    Zoom { factor: zoom.factor, scroll: layout(window, page, Zoom { scroll, ..zoom }).scroll }
}

/// The largest raster side: [`MAX_RASTER_PX`], or less where the graphics card takes smaller textures.
pub fn max_raster(max_texture_side: usize) -> f32 {
    MAX_RASTER_PX.min(max_texture_side as f32)
}

/// The raster scale (pixels per page point) for a page drawn at `ui_scale` UI points per page
/// point: capped so that neither side of the raster exceeds [`MAX_RASTER_PX`]. The texture is then
/// stretched over the page.
pub fn raster_scale(ui_scale: f32, pixels_per_point: f32, page: Vec2, max_texture_side: usize) -> f32 {
    let wanted = ui_scale * pixels_per_point;
    let cap = max_raster(max_texture_side) / page.x.max(page.y).max(1.0);
    wanted.min(cap)
}

/// The scroll bar thumb along a track: `(start, length)` in track points, for a view of `view`
/// page points out of `total`, scrolled to `scroll`.
pub fn thumb(track: f32, total: f32, view: f32, scroll: f32) -> (f32, f32) {
    let len = (track * view / total).clamp(20.0_f32.min(track), track);
    let max = (total - view).max(f32::EPSILON);
    (((track - len) * (scroll / max)).clamp(0.0, track - len), len)
}

/// The scroll for a thumb centred at `at` on a track (the inverse of [`thumb`]).
pub fn scroll_for_thumb(track: f32, total: f32, view: f32, at: f32) -> f32 {
    let (_, len) = thumb(track, total, view, 0.0);
    let free = (track - len).max(f32::EPSILON);
    ((at - len / 2.0) / free).clamp(0.0, 1.0) * (total - view).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn window() -> Rect {
        Rect::from_min_size(pos2(10.0, 20.0), vec2(800.0, 600.0))
    }
    fn page() -> Vec2 {
        vec2(504.0, 360.0)
    }

    #[test]
    fn zoom_one_fits_and_centres_the_page() {
        let l = layout(window(), page(), Zoom::default());
        assert!(window().contains_rect(l.image));
        assert!((l.image.center() - window().center()).length() < 0.1);
        assert_eq!(l.scroll, Vec2::ZERO);
    }

    #[test]
    fn the_point_under_the_pointer_stays_there() {
        let anchor = pos2(400.0, 300.0);
        let z0 = Zoom::default();
        let l0 = layout(window(), page(), z0);
        let before = (anchor - l0.image.min) / l0.scale;
        let z1 = zoom_about(window(), page(), z0, 2.0, anchor);
        let l = layout(window(), page(), z1);
        let after = (anchor - l.image.min) / l.scale;
        assert!((before - after).length() < 0.01, "{before:?} {after:?}");
        assert!((l.scale / fit_scale(window(), page()) - 2.0).abs() < 1e-4);
    }

    #[test]
    fn the_scroll_cannot_leave_the_page_and_zoom_is_limited() {
        let z = Zoom { factor: 4.0, scroll: vec2(-500.0, 9e6) };
        let l = layout(window(), page(), z);
        assert_eq!(l.scroll.x, 0.0);
        assert!(l.image.max.y >= content(window()).max.y - 0.01);
        assert_eq!(zoom_about(window(), page(), Zoom::default(), 100.0, window().center()).factor, MAX_ZOOM);
        assert_eq!(zoom_about(window(), page(), Zoom::default(), 0.1, window().center()).factor, 1.0);
    }

    #[test]
    fn zooming_out_to_one_recentres_the_page() {
        let z = zoom_about(window(), page(), Zoom::default(), 2.0, pos2(100.0, 100.0));
        let back = zoom_about(window(), page(), z, 1.0, window().center());
        assert_eq!(back.scroll, Vec2::ZERO);
    }

    #[test]
    fn a_pan_moves_the_page_with_the_pointer() {
        let z = zoom_about(window(), page(), Zoom::default(), 4.0, window().center());
        let before = layout(window(), page(), z).image.min;
        let moved = layout(window(), page(), pan(window(), page(), z, vec2(-30.0, -20.0))).image.min;
        assert!((moved - before - vec2(-30.0, -20.0)).length() < 0.01);
    }

    #[test]
    fn the_raster_is_capped() {
        assert!((raster_scale(1.0, 2.0, page(), 8192) - 2.0).abs() < 1e-6);
        let s = raster_scale(30.0, 2.0, page(), 8192);
        assert!(page().x * s <= MAX_RASTER_PX + 0.01);
        let s = raster_scale(30.0, 2.0, page(), 2048);
        assert!(page().x * s <= 2048.01, "a smaller texture limit caps it further");
    }

    #[test]
    fn the_thumb_and_its_inverse_agree() {
        let (start, len) = thumb(400.0, 2000.0, 500.0, 750.0);
        assert!(len > 90.0 && len < 110.0);
        assert!((start - (400.0 - len) / 2.0).abs() < 0.01);
        let back = scroll_for_thumb(400.0, 2000.0, 500.0, start + len / 2.0);
        assert!((back - 750.0).abs() < 0.01);
    }
}
