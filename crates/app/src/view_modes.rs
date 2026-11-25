//! The view modes of a graph window (View > Print View, Page View, Window View, Draft View), the
//! Show > Layer Icons and Axis Layer Icons switches and Full Screen (T66; behaviour measured in Origin
//! 2016, graphing spec section 22).
//!
//! Print and Page View draw the page at its own size on the grey ground (the existing picture).
//! Window and Draft View stretch the page over the whole window: the page takes the window's
//! aspect (the layers grow with the window, the text keeps its size) and no grey is left. Draft
//! View also draws the text without smoothing. Full Screen shows only the page over the whole
//! screen until Esc.

use crate::graph_zoom::MARGIN;
use eframe::egui::{self, Color32, Key, Rect, TextureHandle, TextureOptions, Vec2, vec2};
use sheaf_core::graph::PageGrid;
use sheaf_core::project::ViewMode;
use sheaf_core::{Color, Project, WindowId};
use sheaf_plot::scene::{Item, LayerScene, ObjectKind, Scene, Stroke, line_to, move_to};
use sheaf_plot::ticks::ResolvedAxis;

fn axis_lines(scene: &Scene) -> impl Iterator<Item = (usize, sheaf_plot::Rect)> + '_ {
    let objects = scene.layers.iter().flat_map(|drawn| &drawn.objects);
    objects.filter(|object| matches!(object.kind, ObjectKind::XAxis | ObjectKind::YAxis)).flat_map(|object| {
        object
            .boxes
            .iter()
            .copied()
            .filter(|bounds| match object.kind {
                // The layout stores the primary Cartesian axis strokes as zero-height and
                // zero-width picking boxes at their rendered positions.
                ObjectKind::XAxis => bounds.h == 0.0 && bounds.w > 0.0,
                ObjectKind::YAxis => bounds.w == 0.0 && bounds.h > 0.0,
                _ => false,
            })
            .map(|bounds| (object.layer, bounds))
    })
}

fn primary_axis_lines(scene: &Scene, layer: usize) -> impl Iterator<Item = sheaf_plot::Rect> + '_ {
    axis_lines(scene).filter_map(move |(owner, bounds)| (owner == layer).then_some(bounds))
}

fn axis_segments(line: sheaf_plot::Rect, axis: &ResolvedAxis) -> Vec<[(f64, f64); 2]> {
    let mut gaps: Vec<_> = axis.gaps.iter().map(|gap| gap.span(axis.reversed)).collect();
    gaps.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    // Like the resolved Y scale, the vertical line runs from the bottom to the top.
    let at = |u| (line.x + u * line.w, line.y + (1.0 - u) * line.h);
    let starts = std::iter::once(0.0).chain(gaps.iter().map(|&(_, hi)| hi));
    let ends = gaps.iter().map(|&(lo, _)| lo).chain(std::iter::once(1.0));
    starts.zip(ends).map(|(lo, hi)| [at(lo), at(hi)]).collect()
}

/// Draw the indicator as a graph-window overlay, using the cached scene's axis positions.
pub(crate) fn draw_active_axes(
    painter: &egui::Painter,
    scene: &Scene,
    layer: usize,
    geometry: &sheaf_plot::LayerGeometry,
    to_screen: &impl Fn((f64, f64)) -> egui::Pos2,
    colour: Color32,
) {
    let stroke = egui::Stroke::new(3.0_f32, colour.linear_multiply(0.75));
    for line in primary_axis_lines(scene, layer) {
        let axis = if line.h == 0.0 { &geometry.x } else { &geometry.y };
        for segment in axis_segments(line, axis) {
            painter.line_segment(segment.map(to_screen), stroke);
        }
    }
}

/// Show a layer selection button on each visible Cartesian axis.
pub(crate) fn axis_layer_icons(
    ui: &egui::Ui,
    painter: &egui::Painter,
    scene: &Scene,
    graph: WindowId,
    to_screen: &impl Fn((f64, f64)) -> egui::Pos2,
) -> Option<usize> {
    let mut selected = None;
    for (index, (layer, line)) in axis_lines(scene).enumerate() {
        let center = to_screen((line.x + line.w / 2.0, line.y + line.h / 2.0));
        let text =
            painter.layout_no_wrap((layer + 1).to_string(), egui::FontId::proportional(11.0), Color32::PLACEHOLDER);
        let badge = Rect::from_center_size(center, text.size() + vec2(7.0, 5.0));
        let hit_area = badge.intersect(painter.clip_rect());
        if !hit_area.is_positive() {
            continue;
        }
        let id = ui.id().with(("axis_layer_icon", graph.0, index));
        let response = ui.interact(hit_area, id, egui::Sense::click());
        response.widget_info(|| {
            let axis = if line.h == 0.0 { "X" } else { "Y" };
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                response.enabled(),
                format!("Select layer {} from {axis} axis", layer + 1),
            )
        });
        let style = ui.style().interact(&response);
        painter.rect_filled(badge, 3.0, style.bg_fill);
        painter.galley(center - text.size() / 2.0, text, style.fg_stroke.color);
        if response.clicked() {
            selected = Some(layer);
        }
    }
    selected
}

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

/// A page-wide alignment guide without pickable objects.
fn grid_underlay(scene: &Scene, scale: f64, intervals: u32) -> LayerScene {
    let (width, height) = (scene.width, scene.height);
    let mut path = String::new();
    for i in 1..intervals {
        let fraction = f64::from(i) / f64::from(intervals);
        let (x, y) = (width * fraction, height * fraction);
        move_to(&mut path, x, 0.0);
        line_to(&mut path, x, height);
        move_to(&mut path, 0.0, y);
        line_to(&mut path, width, y);
    }
    LayerScene {
        clip: sheaf_plot::Rect { x: 0.0, y: 0.0, w: width, h: height },
        grid: vec![Item::Path {
            d: path,
            fill: None,
            stroke: Some(Stroke::solid(Color::rgb(192, 192, 192), 1.0 / scale)),
        }],
        plots: Vec::new(),
        overlay: Vec::new(),
        underlay_len: 0,
        data_on_top: false,
        objects: Vec::new(),
    }
}

/// Draws the graph-window picture, including the optional underlay and Draft View's unsmoothed text.
pub fn raster(
    mode: ViewMode,
    scene: &mut Scene,
    scale: f64,
    grid: PageGrid,
) -> Result<sheaf_plot::Raster, sheaf_plot::PlotError> {
    // The default Origin spacings measured in a fitted page (graphing 22.12 and 22.13).
    let intervals = match grid {
        PageGrid::Off => 0,
        PageGrid::Object => 36,
        PageGrid::Layer => 25,
    };
    if intervals > 0 {
        scene.layers.insert(0, grid_underlay(scene, scale, intervals));
    }
    let raster = if mode == ViewMode::Draft {
        sheaf_plot::render_scene_raster_draft(scene, scale)
    } else {
        sheaf_plot::render_scene_raster(scene, scale)
    };
    // Keep the original scene and its layer indices for picking and editing.
    if intervals > 0 {
        scene.layers.remove(0);
    }
    raster
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
    /// Slide Show of Dependent Graphs (T129): the graphs shown, in order; `None` for View > Full Screen,
    /// which steps through every graph of the project.
    pub slides: Option<Vec<WindowId>>,
    texture: Option<(u64, WindowId, u32, TextureHandle)>,
}

/// The slide that a key or click moves to in a slide show: Right, Page Down, Space and a click go on, Left
/// and Page Up go back, Home and End go to the ends.
pub fn next_slide(slides: &[WindowId], current: WindowId, key: Key) -> WindowId {
    let at = slides.iter().position(|g| *g == current).unwrap_or(0);
    let last = slides.len().saturating_sub(1);
    let to = match key {
        Key::ArrowLeft | Key::PageUp => at.saturating_sub(1),
        Key::ArrowRight | Key::PageDown | Key::Space => (at + 1).min(last),
        Key::Home => 0,
        Key::End => last,
        _ => at,
    };
    slides.get(to).copied().unwrap_or(current)
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

/// The graphs that plot any of `columns`, in window order (Slide Show of Dependent Graphs, T129).
pub fn dependent_graphs(project: &Project, columns: &[sheaf_core::ColumnId]) -> Vec<WindowId> {
    project.graphs().filter(|g| g.referenced_columns().iter().any(|c| columns.contains(c))).map(|g| g.id).collect()
}

impl FullScreen {
    pub fn new(graph: WindowId, show_reminder: bool) -> Self {
        FullScreen { graph, reminder: show_reminder, never_remind: false, slides: None, texture: None }
    }

    /// A slide show of `graphs` (not empty), from the first, without the reminder.
    pub fn slide_show(graphs: Vec<WindowId>) -> Self {
        FullScreen { graph: graphs[0], reminder: false, never_remind: false, slides: Some(graphs), texture: None }
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
        if let Some(slides) = &self.slides {
            let keys = [Key::ArrowLeft, Key::ArrowRight, Key::PageUp, Key::PageDown, Key::Space, Key::Home, Key::End];
            for key in keys {
                if pressed(key) {
                    self.graph = next_slide(slides, self.graph, key);
                }
            }
            if ctx.input(|i| i.pointer.primary_clicked()) {
                self.graph = next_slide(slides, self.graph, Key::Space);
            }
        } else if !self.reminder {
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
    fn alignment_grids_are_rasterized_under_the_graph_and_leave_the_scene_intact() {
        let page = sheaf_plot::Rect { x: 0.0, y: 0.0, w: 72.0, h: 72.0 };
        let shape = sheaf_plot::Rect { x: 20.0, y: 20.0, w: 20.0, h: 20.0 };
        let mut scene = Scene {
            width: page.w,
            height: page.h,
            background: Color::WHITE,
            layers: vec![LayerScene {
                clip: page,
                grid: Vec::new(),
                plots: Vec::new(),
                overlay: vec![Item::Rect { rect: shape, fill: Some(Color::BLACK), stroke: None }],
                underlay_len: 0,
                data_on_top: false,
                objects: Vec::new(),
            }],
        };
        let original = scene.clone();
        let plain = raster(ViewMode::Page, &mut scene, 1.0, PageGrid::Off).unwrap();
        let object_grid = raster(ViewMode::Page, &mut scene, 1.0, PageGrid::Object).unwrap();
        let layer_grid = raster(ViewMode::Page, &mut scene, 1.0, PageGrid::Layer).unwrap();
        let shape_pixel = 4 * (24 * 72 + 24);
        assert_ne!(object_grid.rgba, layer_grid.rgba);
        for shown in [object_grid, layer_grid] {
            assert_ne!(plain.rgba, shown.rgba);
            assert_eq!(&shown.rgba[shape_pixel..shape_pixel + 4], &[0, 0, 0, 255]);
        }
        assert_eq!(scene, original);
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
    fn a_slide_show_steps_through_its_graphs_and_stops_at_the_ends() {
        let g = [WindowId(3), WindowId(7), WindowId(9)];
        assert_eq!(next_slide(&g, WindowId(3), Key::Space), WindowId(7));
        assert_eq!(next_slide(&g, WindowId(7), Key::PageDown), WindowId(9));
        assert_eq!(next_slide(&g, WindowId(9), Key::ArrowRight), WindowId(9), "the last slide stays");
        assert_eq!(next_slide(&g, WindowId(9), Key::PageUp), WindowId(7));
        assert_eq!(next_slide(&g, WindowId(7), Key::Home), WindowId(3));
        assert_eq!(next_slide(&g, WindowId(3), Key::End), WindowId(9));
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

    #[test]
    fn indicator_uses_rendered_primary_axes_of_the_selected_layer() {
        let mut project = Project::new();
        let id = project.add_graph(sheaf_core::template::Template::Line);
        let graph = project.graph_mut(id).unwrap();
        graph.add_layer(sheaf_core::layers::NewLayerKind::InsetWithData, 0).unwrap();

        let scene = sheaf_plot::graph_scene(&project, id).unwrap();
        let outer: Vec<_> = primary_axis_lines(&scene, 0).collect();
        let inset: Vec<_> = primary_axis_lines(&scene, 1).collect();
        assert_eq!((outer.len(), inset.len()), (2, 2));
        assert_ne!(outer, inset, "the indicator follows the active inset's axes");

        let graph = project.graph_mut(id).unwrap();
        graph.layers[1].display.show_x_axes = false;
        graph.layers[1].y_axis.offset_pct = 15.0;
        let scene = sheaf_plot::graph_scene(&project, id).unwrap();
        let lines: Vec<_> = primary_axis_lines(&scene, 1).collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].w, 0.0);
        let old_y = inset.iter().find(|line| line.w == 0.0).unwrap();
        assert!(lines[0].x < old_y.x, "the indicator follows the displaced Y axis");

        project.graph_mut(id).unwrap().layers[1].display.hidden = true;
        let scene = sheaf_plot::graph_scene(&project, id).unwrap();
        assert_eq!(primary_axis_lines(&scene, 1).count(), 0);
        assert_eq!(primary_axis_lines(&scene, 9).count(), 0);
    }

    #[test]
    fn indicator_leaves_axis_breaks_open() {
        use sheaf_core::graph::AxisBreak;
        use sheaf_core::template::Template;

        let mut project = Project::new();
        let id = project.add_graph(Template::Line);
        let axis = &mut project.graph_mut(id).unwrap().layers[0].y_axis;
        axis.min = Some(10.0);
        axis.max = Some(0.0);
        let mut gap = AxisBreak::new(2.0, 4.0);
        gap.auto_position = false;
        gap.position_pct = 30.0;
        axis.axis_breaks.push(gap);
        let scene = sheaf_plot::graph_scene(&project, id).unwrap();
        let line = primary_axis_lines(&scene, 0).find(|line| line.w == 0.0).unwrap();
        let geometry = sheaf_plot::layout_geometry(&project, id).unwrap();
        let segments = axis_segments(line, &geometry[0].y);
        let [[bottom, before_gap], [after_gap, top]] = segments.as_slice() else {
            panic!("expected two segments around the axis break, got {segments:?}");
        };
        assert_eq!(*bottom, (line.x, line.y + line.h));
        assert!(before_gap.1 < bottom.1 && before_gap.1 > after_gap.1);
        assert!((before_gap.1 + after_gap.1) * 0.5 < line.y + line.h * 0.5);
        assert_eq!(*top, (line.x, line.y));
    }
}
