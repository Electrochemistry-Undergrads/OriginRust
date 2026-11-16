//! The view modes of a graph window (View > Print View, Page View, Window View, Draft View), the
//! Show > Layer Icons switch and Full Screen (T66; clean-room: behaviour only, measured in Origin
//! 2016, graphing spec section 22).
//!
//! Print and Page View draw the page at its own size on the grey ground (the existing picture).
//! Window and Draft View stretch the page over the whole window: the page takes the window's
//! aspect (the layers grow with the window, the text keeps its size) and no grey is left. Draft
//! View also draws the text without smoothing. Full Screen shows only the page over the whole
//! screen until Esc.

use crate::graph_zoom::MARGIN;
use eframe::egui::{self, Color32, Key, Rect, TextureHandle, TextureOptions, Vec2, vec2};
use sheaf_core::project::ViewMode;
use sheaf_core::{Project, WindowId};

/// The rectangle the zoom layout works in. In Window and Draft View it is the window grown by the
/// layout's margin, so that the page fills the window with no grey border.
pub fn view_rect(mode: ViewMode, window: Rect) -> Rect {
    if mode.fills_window() { window.expand(MARGIN) } else { window }
}

/// The page size (points) a mode shows: the page's own size, except in Window and Draft View where
/// the page keeps its width and takes the aspect of `window` (the layout rectangle of
/// [`view_rect`]).
pub fn page_for(mode: ViewMode, page: Vec2, window: Rect) -> Vec2 {
    if !mode.fills_window() {
        return page;
    }
    let (w, h) = (window.width() - 2.0 * MARGIN, window.height() - 2.0 * MARGIN);
    if w <= 0.0 || h <= 0.0 { page } else { vec2(page.x, (page.x * h / w).round().max(1.0)) }
}

/// What a drawn picture depends on besides the project's revision: the mode and the page size shown.
pub fn view_key(mode: ViewMode, page: Vec2) -> (ViewMode, i32, i32) {
    (mode, page.x.round() as i32, page.y.round() as i32)
}

/// A copy of `project` in which graph `graph` has the page size `page`, to draw the stretched page.
pub fn resized(project: &Project, graph: WindowId, page: Vec2) -> Project {
    let mut p = project.clone();
    if let Some(g) = p.graph_mut(graph) {
        g.page.width_pt = f64::from(page.x);
        g.page.height_pt = f64::from(page.y);
    }
    p
}

/// Draws a scene as the mode shows it (Draft View: unsmoothed text).
pub fn raster(
    mode: ViewMode,
    scene: &sheaf_plot::scene::Scene,
    scale: f64,
) -> Result<sheaf_plot::Raster, sheaf_plot::PlotError> {
    if mode == ViewMode::Draft {
        sheaf_plot::render_scene_raster_draft(scene, scale)
    } else {
        sheaf_plot::render_scene_raster(scene, scale)
    }
}

/// The text of the Reminder Message that Full Screen shows first (the lines Origin shows, measured).
pub const REMINDER: [&str; 3] =
    ["Hit ESC to exit", "Arrow Keys to shift to different graphs.", "Home and End key to first and last graph."];

/// What the Full Screen overlay wants after a frame.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    None,
    /// Esc: the window comes back as it was.
    End,
}

/// Full Screen: one graph's page over the whole screen.
pub struct FullScreen {
    pub graph: WindowId,
    /// The Reminder Message is still shown.
    pub reminder: bool,
    /// The "Do not show this message in the future" tick as it is now.
    pub never_remind: bool,
    texture: Option<(u64, WindowId, u32, TextureHandle)>,
}

/// The graph that Left, Right, Home and End move to: the graphs of the project in window order.
pub fn neighbour(project: &Project, current: WindowId, key: Key) -> WindowId {
    let graphs: Vec<WindowId> =
        project.windows.iter().map(|w| w.id()).filter(|id| project.graph(*id).is_some()).collect();
    let at = graphs.iter().position(|g| *g == current).unwrap_or(0);
    let last = graphs.len().saturating_sub(1);
    let to = match key {
        Key::ArrowLeft => at.saturating_sub(1),
        Key::ArrowRight => (at + 1).min(last),
        Key::Home => 0,
        Key::End => last,
        _ => at,
    };
    graphs.get(to).copied().unwrap_or(current)
}

impl FullScreen {
    pub fn new(graph: WindowId, show_reminder: bool) -> Self {
        FullScreen { graph, reminder: show_reminder, never_remind: false, texture: None }
    }

    /// The page rectangle: the page fitted into the screen, centred.
    pub fn page_rect(screen: Rect, page: Vec2) -> Rect {
        let scale = (screen.width() / page.x).min(screen.height() / page.y);
        Rect::from_center_size(screen.center(), page * scale)
    }

    /// Draws the page and the reminder and reads the keys of Full Screen. Called instead of the
    /// whole window.
    pub fn show(&mut self, ctx: &egui::Context, project: &Project, revision: u64) -> Event {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show(ctx, |ui| {
            let screen = ui.max_rect();
            let Some(graph) = project.graph(self.graph) else {
                return;
            };
            let page = vec2(graph.page.width_pt as f32, graph.page.height_pt as f32);
            if page.x <= 0.0 || page.y <= 0.0 {
                return;
            }
            let target = Self::page_rect(screen, page);
            let pixels = target.width() * ui.ctx().pixels_per_point();
            let max_side = ui.input(|i| i.max_texture_side) as f32;
            let scale = (pixels / page.x).clamp(0.05, max_side / page.x.max(page.y));
            let key = (scale * 50.0).round() as u32;
            let stale =
                self.texture.as_ref().is_none_or(|(rev, g, k, _)| *rev != revision || *g != self.graph || *k != key);
            if stale {
                let scene = sheaf_plot::graph_scene(project, self.graph);
                let raster = scene.and_then(|s| sheaf_plot::render_scene_raster(&s, f64::from(key) / 50.0));
                if let Ok(r) = raster {
                    let size = [r.width as usize, r.height as usize];
                    let image = egui::ColorImage::from_rgba_unmultiplied(size, &r.rgba);
                    let name = format!("fullscreen-{}", self.graph.0);
                    let texture = ui.ctx().load_texture(name, image, TextureOptions::LINEAR);
                    self.texture = Some((revision, self.graph, key, texture));
                }
            }
            if let Some((_, _, _, t)) = &self.texture {
                let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter().image(t.id(), target, uv, Color32::WHITE);
            }
        });
        if self.reminder {
            egui::Window::new("Reminder Message")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    for line in REMINDER {
                        ui.label(line);
                    }
                    ui.add_space(6.0);
                    ui.checkbox(&mut self.never_remind, "Do not show this message in the future");
                    ui.add_space(6.0);
                    if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.reminder = false;
                    }
                });
        }
        let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));
        if pressed(Key::Escape) {
            return Event::End;
        }
        if !self.reminder {
            for key in [Key::ArrowLeft, Key::ArrowRight, Key::Home, Key::End] {
                if pressed(key) {
                    self.graph = neighbour(project, self.graph, key);
                }
            }
        }
        Event::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn window() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(824.0, 424.0))
    }

    #[test]
    fn page_and_print_view_keep_the_page_and_the_zoom_rectangle() {
        let page = vec2(720.0, 540.0);
        for mode in [ViewMode::Page, ViewMode::Print] {
            assert_eq!(page_for(mode, page, window()), page);
            assert_eq!(view_rect(mode, window()), window());
        }
    }

    #[test]
    fn window_and_draft_view_give_the_page_the_window_aspect_and_no_border() {
        let page = vec2(720.0, 540.0);
        for mode in [ViewMode::Window, ViewMode::Draft] {
            let rect = view_rect(mode, window());
            let shown = page_for(mode, page, rect);
            assert_eq!(shown.x, 720.0);
            // The layout's content rectangle is the window and the page has its aspect: 824 x 424.
            assert_eq!(shown.y, (720.0_f32 * 424.0 / 824.0).round());
            assert_eq!(crate::graph_zoom::content(rect), window());
        }
    }

    #[test]
    fn the_page_of_full_screen_is_fitted_and_centred() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1920.0, 1080.0));
        let r = FullScreen::page_rect(screen, vec2(720.0, 540.0));
        assert_eq!(r.height(), 1080.0);
        assert_eq!(r.width(), 1440.0);
        assert_eq!(r.center(), screen.center());
    }

    #[test]
    fn the_arrow_keys_and_home_and_end_move_between_the_graphs() {
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let (a, b, c) = (p.add_graph(Template::Line), p.add_graph(Template::Line), p.add_graph(Template::Line));
        assert_eq!(neighbour(&p, a, Key::ArrowRight), b);
        assert_eq!(neighbour(&p, c, Key::ArrowRight), c);
        assert_eq!(neighbour(&p, b, Key::ArrowLeft), a);
        assert_eq!(neighbour(&p, a, Key::ArrowLeft), a);
        assert_eq!(neighbour(&p, b, Key::End), c);
        assert_eq!(neighbour(&p, b, Key::Home), a);
    }

    #[test]
    fn view_modes_are_kept_in_the_project_and_page_view_is_not_stored() {
        use sheaf_core::template::Template;
        let mut p = Project::new();
        let g = p.add_graph(Template::Line);
        assert_eq!(p.view_mode(g), ViewMode::Page);
        p.set_view_mode(g, ViewMode::Window);
        assert_eq!(p.view_mode(g), ViewMode::Window);
        assert_eq!(p.view_modes.len(), 1);
        p.set_view_mode(g, ViewMode::Page);
        assert!(p.view_modes.is_empty());
    }
}
