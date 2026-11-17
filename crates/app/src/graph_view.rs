//! A graph window, as in Origin: the page on a grey ground, layer icons at its top left, and the
//! mouse actions of `docs/spec/gui-look.md` section 4.
//!
//! The figure is rasterised by `sheaf-plot` (the same code path as PNG export) at the resolution
//! of the window and shown as an image. It is rendered again only when the project, the size or
//! the zoom changes. The scene it was drawn from is kept, so that a click can be traced back to
//! the axis, plot, legend or label under the pointer (`sheaf_plot::hit`):
//!
//! - a click selects that object (handles are drawn round it) and makes its layer active;
//! - a double click opens the matching settings (axis: that axis; plot: that plot; legend);
//! - a right click opens Origin's menu for that object;
//! - Delete removes the selected legend, axis title or text label (handled with the app's keys).
//!
//! Each layer has a numbered icon: a click activates the layer, a double click opens its plot
//! list, a right click the layer menu.

use crate::actions::{Action, DataTool, GraphOp, GraphTool};
use crate::dialogs::graph_props::PropsTab;
use crate::gadget::Integrator;
use crate::graph_scale::{self, ScaleState};
use crate::graph_zoom::{self, Zoom};
use crate::menus::{self, MenuContext, WindowKind};
use crate::{data_tools, reader, view_modes};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Vec2, vec2};
use sheaf_core::project::ViewMode;
use sheaf_core::shape::{self, Shape, ShapeKind};
use sheaf_core::{Document, Project, WindowId};
use sheaf_plot::LayerGeometry;
use sheaf_plot::hit::{Hit, hit_test};
use sheaf_plot::scene::{Item, ItemList, ObjectKind, Scene};

/// The grey round the page (Origin shows the part of the window the page does not fill in grey).
const GROUND: Color32 = Color32::from_gray(192);
/// How far from a line, in screen points, a click still picks it.
const PICK_TOLERANCE: f32 = 3.0;
const ICON: f32 = 15.0;

/// The layer a hit is in.
fn layer_of_hit(hit: Hit) -> Option<usize> {
    match hit {
        Hit::Object { layer, .. } | Hit::Layer(layer) => Some(layer),
        Hit::Page => None,
    }
}

/// A drag shorter than this (page points) draws nothing.
const MIN_DRAWN_PT: f64 = 3.0;

/// The outline of an object being drawn, in screen points; `to_screen` takes frame fractions.
fn draw_preview(painter: &egui::Painter, s: &Shape, to_screen: &dyn Fn((f64, f64)) -> Pos2) {
    let stroke = Stroke::new(1.0_f32, Color32::from_rgb(0, 90, 200));
    let (a, b) = (to_screen((s.x1, s.y1)), to_screen((s.x2, s.y2)));
    match s.kind {
        ShapeKind::Line | ShapeKind::Arrow => {
            painter.line_segment([a, b], stroke);
        }
        ShapeKind::Rectangle => {
            painter.rect_stroke(Rect::from_two_pos(a, b), 0.0, stroke, egui::StrokeKind::Middle);
        }
        ShapeKind::Ellipse => {
            let (c, r) = (Rect::from_two_pos(a, b).center(), Rect::from_two_pos(a, b).size() / 2.0);
            let pts = (0..48)
                .map(|k| {
                    let t = std::f32::consts::TAU * k as f32 / 48.0;
                    c + egui::vec2(r.x * t.cos(), r.y * t.sin())
                })
                .collect();
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
    }
}

/// The cached image of one graph.
pub struct GraphTexture {
    revision: u64,
    /// Raster pixels per page point, quantised, so tiny size changes do not re-render.
    scale_key: u32,
    texture: TextureHandle,
    geometry: Vec<LayerGeometry>,
    scene: Scene,
    /// The view mode and the page size the picture was drawn for (Window and Draft View follow the window).
    view_key: (ViewMode, i32, i32),
}

#[derive(Default)]
pub struct GraphUi {
    pub texture: Option<GraphTexture>,
    pub error: Option<String>,
    /// The active layer: menu commands such as Rescale and Plot Setup act on it.
    pub active_layer: usize,
    /// The selected object, with its layer.
    pub selected: Option<(usize, ObjectKind)>,
    /// What the open right-click menu is about.
    menu_on: Option<MenuTarget>,
    /// The page point (in points) that opened the right-click menu: Axis Center needs it.
    menu_page: (f64, f64),
    /// Where the page was drawn in the last frame (screen rectangle) and UI points per page point.
    view: Option<(Rect, f32)>,
    /// The open Integrate gadget, if any.
    pub gadget: Option<Integrator>,
    /// Where the gadget's range was drawn in the last frame.
    pub gadget_rect: Option<Rect>,
    /// The Data Reader tool is on.
    pub reader: bool,
    /// The data point the Data Reader pinned with a click.
    pub reader_point: Option<reader::Point>,
    /// The Text tool is on: the next click on the page places a label.
    pub text_tool: bool,
    /// The Slope tool is on: two clicks on data points of one plot add a slope annotation.
    pub slope_tool: bool,
    /// The first point the Slope tool picked.
    pub slope_first: Option<reader::Point>,
    /// The drawing tool in use (Line, Arrow, Rectangle, Circle): a drag on the page draws one.
    pub shape_tool: Option<ShapeKind>,
    /// The Data menu tool in use (Mask, Move, Remove Bad Data Points).
    pub data_tool: Option<DataTool>,
    /// The Zoom In mode is on: the next click on the page zooms in there, then the mode ends.
    pub zoom_in_tool: bool,
    /// The Pan tool is on: a drag on the page moves the view.
    pub pan_tool: bool,
    /// The Scale In / Scale Out tools and the Scale Out stack (T69).
    pub scale: ScaleState,
    /// The zoom and scroll of this window (view state only; not saved with the project).
    pub zoom: Zoom,
    /// The window rectangle and the page size (points) of the last frame, for the view commands.
    frame: Option<(Rect, Vec2)>,
    /// What the Data menu tool is dragging.
    data: data_tools::State,
    /// An object being drawn.
    drawing: Option<Drawing>,
    /// A drawn object being moved.
    shape_drag: Option<ShapeDrag>,
    /// A label being dragged.
    label_drag: Option<LabelDrag>,
    /// A legend being dragged.
    legend_drag: Option<LegendDrag>,
}

/// An object being drawn: its first point as fractions of the frame of `layer`, and the pointer
/// now (page points).
#[derive(Clone, Copy)]
struct Drawing {
    layer: usize,
    kind: ShapeKind,
    from: (f64, f64),
}

/// A drawn object being moved: how far the pointer has moved since the drag began (page points).
#[derive(Clone, Copy)]
struct ShapeDrag {
    layer: usize,
    shape: usize,
    delta: (f64, f64),
}

/// A legend being moved with the mouse: its centre when the drag began (fractions of the layer
/// frame) and how far the pointer has moved since (page points).
#[derive(Clone, Copy)]
struct LegendDrag {
    layer: usize,
    from: (f64, f64),
    delta: (f64, f64),
}

/// A label being moved with the mouse: its anchor when the drag began (fractions of the layer
/// frame) and how far the pointer has moved since (page points).
#[derive(Clone, Copy)]
struct LabelDrag {
    layer: usize,
    label: usize,
    from: (f64, f64),
    delta: (f64, f64),
}

impl GraphUi {
    /// Whether a Data menu tool has a drag in progress.
    #[cfg(test)]
    pub fn data_dragging(&self) -> bool {
        self.data.drag.is_some()
    }

    /// The tool in use.
    pub fn tool(&self) -> GraphTool {
        if self.reader {
            GraphTool::Reader
        } else if self.slope_tool {
            GraphTool::Slope
        } else if self.text_tool {
            GraphTool::Text
        } else if let Some(kind) = self.shape_tool {
            GraphTool::Draw(kind)
        } else if let Some(tool) = self.data_tool {
            GraphTool::Data(tool)
        } else if self.zoom_in_tool {
            GraphTool::ZoomIn
        } else if self.pan_tool {
            GraphTool::Pan
        } else if let Some(tool) = self.scale.tool {
            match tool {
                graph_scale::ScaleTool::In => GraphTool::ScaleIn,
                graph_scale::ScaleTool::Out => GraphTool::ScaleOut,
            }
        } else {
            GraphTool::Pointer
        }
    }

    /// Zoom Out: half the zoom, down to 25 % (Origin: 200 to 100 to 50 to 25 %, then no change); the
    /// window centre's page point stays centred.
    pub fn zoom_out(&mut self) {
        if let Some((window, page)) = self.frame {
            self.zoom = graph_zoom::zoom_about(window, page, self.zoom, self.zoom.factor / 2.0, window.center());
        } else {
            self.zoom = Zoom::default();
        }
    }

    /// Zoom All: the page and all objects fit the window (T69).
    pub fn zoom_all(&mut self) {
        if let Some((window, page)) = self.frame {
            let bounds = self.texture.as_ref().map(|t| graph_scale::scene_bounds(&t.scene)).unwrap_or_default();
            self.zoom = graph_scale::zoom_all(window, page, &bounds);
        }
    }

    /// Whole Page: zoom 1, scroll 0.
    pub fn whole_page(&mut self) {
        self.zoom = Zoom::default();
    }

    /// Drops the open menu and the drags, which name layers by index, after layers were renumbered.
    pub fn reset_layer_interactions(&mut self) {
        self.menu_on = None;
        self.shape_drag = None;
        self.label_drag = None;
        self.legend_drag = None;
    }

    /// Layers with an index-based shape selection, context menu or drag.
    pub fn shape_interaction_layers(&self) -> Vec<usize> {
        let mut layers = Vec::new();
        if let Some((layer, ObjectKind::Shape(_))) = self.selected {
            layers.push(layer);
        }
        if let Some(MenuTarget::Hit(Hit::Object { layer, kind: ObjectKind::Shape(_) })) = self.menu_on
            && !layers.contains(&layer)
        {
            layers.push(layer);
        }
        if let Some(drag) = self.shape_drag
            && !layers.contains(&drag.layer)
        {
            layers.push(drag.layer);
        }
        layers
    }

    /// Discard index-based shape interactions after a layer's shape list changes.
    pub fn invalidate_shape_interaction(&mut self, layer: usize, keep_selection: bool) {
        if !keep_selection && matches!(self.selected, Some((l, ObjectKind::Shape(_))) if l == layer) {
            self.selected = None;
        }
        if matches!(self.menu_on, Some(MenuTarget::Hit(Hit::Object { layer: l, kind: ObjectKind::Shape(_) })) if l == layer)
        {
            self.menu_on = None;
        }
        if self.shape_drag.is_some_and(|drag| drag.layer == layer) {
            self.shape_drag = None;
        }
    }

    /// What the Delete key does to the selected object: a legend, an axis title or a text label
    /// goes; other objects have no Delete in Origin's menus either.
    pub fn delete_selected(&mut self) -> Option<GraphOp> {
        let (layer, kind) = self.selected?;
        let op = match kind {
            ObjectKind::Legend => GraphOp::HideLegend { layer },
            ObjectKind::XTitle => GraphOp::HideTitle { layer, y: false },
            ObjectKind::YTitle => GraphOp::HideTitle { layer, y: true },
            ObjectKind::Label(label) => GraphOp::DeleteLabel { layer, label },
            ObjectKind::Shape(shape) => GraphOp::DeleteShape { layer, shape },
            ObjectKind::Slope(slope) => GraphOp::DeleteSlope { layer, slope },
            ObjectKind::StraightLine(line) => GraphOp::DeleteStraightLine { layer, line },
            ObjectKind::Scale(scale) => GraphOp::DeleteScale { layer, scale },
            _ => return None,
        };
        self.selected = None;
        Some(op)
    }

    /// A screen point on an object of the graph as drawn in the last frame: a data point in the
    /// middle of a plot, the middle of anything else. Tests click there.
    #[cfg(test)]
    pub fn point_on(&self, layer: usize, kind: ObjectKind) -> Option<Pos2> {
        let (page, fit) = self.view?;
        let scene = &self.texture.as_ref()?.scene;
        let screen = |(x, y): (f64, f64)| page.min + egui::vec2(x as f32 * fit, y as f32 * fit);
        let (l, o) = scene
            .layers
            .iter()
            .flat_map(|l| l.objects.iter().map(move |o| (l, o)))
            .find(|(_, o)| o.layer == layer && o.kind == kind)?;
        if o.list == ItemList::Plots {
            let points: Vec<(f64, f64)> = l.plots[o.items.clone()]
                .iter()
                .filter_map(|i| if let Item::Path { d, .. } = i { Some(sheaf_plot::hit::path_points(d)) } else { None })
                .flatten()
                .collect();
            return points.get(points.len() / 2).map(|p| screen(*p));
        }
        Some(screen((o.bounds.x + o.bounds.w / 2.0, o.bounds.y + o.bounds.h / 2.0)))
    }

    /// The page on screen and the window's content area, as of the last frame.
    #[cfg(test)]
    pub fn page_and_content(&self) -> Option<(Rect, Rect)> {
        Some((self.view?.0, graph_zoom::content(self.frame?.0)))
    }

    /// The middle of the part of the window the page is drawn in, in the last frame.
    #[cfg(test)]
    pub fn view_centre(&self) -> Option<Pos2> {
        self.frame.map(|(window, _)| graph_zoom::content(window).center())
    }

    /// The screen point of a page point (points from the page's top left) in the last frame.
    #[cfg(test)]
    pub fn page_point(&self, x: f64, y: f64) -> Option<Pos2> {
        let (page, fit) = self.view?;
        Some(page.min + egui::vec2(x as f32 * fit, y as f32 * fit))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MenuTarget {
    Hit(Hit),
    LayerIcon(usize),
}

pub fn show(ui: &mut egui::Ui, doc: &Document, graph_id: WindowId, state: &mut GraphUi, actions: &mut Vec<Action>) {
    let Some(graph) = doc.project().graph(graph_id) else {
        ui.label("This graph no longer exists.");
        return;
    };
    let layers = graph.layers.len();
    if state.active_layer >= layers {
        state.active_layer = 0;
    }

    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    ui.painter().rect_filled(rect, 0.0, GROUND);
    let margin = graph_zoom::MARGIN;
    // View mode (T66): Window and Draft View stretch the page over the window.
    let mode = doc.project().view_mode(graph_id);
    let visible = rect;
    let rect = view_modes::view_rect(mode, rect);
    let page_size = view_modes::page_for(mode, vec2(graph.page.width_pt as f32, graph.page.height_pt as f32), rect);
    let (page_w, page_h) = (page_size.x, page_size.y);
    if page_w <= 0.0 || page_h <= 0.0 || rect.width() <= 2.0 * margin || rect.height() <= 2.0 * margin {
        return;
    }
    // Nothing outside the window is drawn or clicked (the page is larger than it when zoomed).
    ui.set_clip_rect(ui.clip_rect().intersect(rect));
    state.frame = Some((rect, page_size));

    // The view input: Zoom In mode, Pan, the wheel and the scroll bars. They change the zoom
    // before the page is placed, from the layout the user saw in the last frame.
    // Zoom In mode and Origin's Zoom-Panning tool take the clicks: a click with the hand changes nothing.
    let zoom_took_input = state.zoom_in_tool || state.pan_tool || state.scale.tool.is_some();
    view_input(ui, &response, rect, page_size, state);

    // UI points per page point (zoom 1: the whole page fits in the window, "fit page to window"),
    // and the page rectangle; every conversion below, and the hit-testing, go through these two.
    let lay = graph_zoom::layout(rect, page_size, state.zoom);
    state.zoom.scroll = lay.scroll;
    let (image_rect, fit) = (lay.image, lay.scale);
    state.view = Some((image_rect, fit));

    // Raster pixels per page point, capped (the texture is then stretched over the page).
    let max_side = ui.input(|i| i.max_texture_side);
    let pixel_scale = graph_zoom::raster_scale(fit, ui.ctx().pixels_per_point(), page_size, max_side);
    let mut key = (pixel_scale * 50.0).round();
    // Rounding up must not push a side over the limit.
    if key / 50.0 * page_w.max(page_h) > graph_zoom::max_raster(max_side) {
        key = (pixel_scale * 50.0).floor();
    }
    let scale_key = key.max(1.0) as u32;
    let view_key = view_modes::view_key(mode, page_size);
    let stale = state
        .texture
        .as_ref()
        .is_none_or(|t| t.revision != doc.revision() || t.scale_key != scale_key || t.view_key != view_key);
    if stale {
        let scale = f64::from(scale_key) / 50.0;
        let stretched = mode.fills_window().then(|| view_modes::resized(doc.project(), graph_id, page_size));
        let project = stretched.as_ref().unwrap_or(doc.project());
        let drawn = sheaf_plot::graph_scene(project, graph_id).and_then(|scene| {
            let raster = view_modes::raster(mode, &scene, scale)?;
            Ok((scene, raster, sheaf_plot::layout_geometry(project, graph_id)?))
        });
        match drawn {
            Ok((scene, raster, geometry)) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [raster.width as usize, raster.height as usize],
                    &raster.rgba,
                );
                let texture = ui.ctx().load_texture(format!("graph-{}", graph_id.0), image, TextureOptions::LINEAR);
                state.texture =
                    Some(GraphTexture { revision: doc.revision(), scale_key, texture, geometry, scene, view_key });
                state.error = None;
            }
            Err(e) => state.error = Some(e.to_string()),
        }
    }

    // A soft shadow and the page itself.
    let painter = ui.painter_at(rect);
    painter.rect_filled(image_rect.translate(egui::vec2(3.0, 3.0)), 0.0, Color32::from_black_alpha(40));
    if let Some(t) = &state.texture {
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(t.texture.id(), image_rect, uv, Color32::WHITE);
    }
    painter.rect_stroke(image_rect, 0.0, Stroke::new(1.0_f32, Color32::from_gray(120)), egui::StrokeKind::Outside);
    paint_scroll_bars(&painter, rect, page_size, state.zoom);
    if let Some(e) = &state.error {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("This graph could not be drawn:\n{e}"),
            egui::FontId::proportional(14.0),
            Color32::DARK_RED,
        );
    }
    let Some(t) = &state.texture else {
        return;
    };
    let to_page = |p: Pos2| (f64::from((p.x - image_rect.left()) / fit), f64::from((p.y - image_rect.top()) / fit));
    let to_screen = |(x, y): (f64, f64)| image_rect.min + egui::vec2(x as f32 * fit, y as f32 * fit);
    // (a straight line that is not Selectable has no hit object, so clicks pass through it)
    let pick = |p: Pos2| hit_test(&t.scene, to_page(p), f64::from(PICK_TOLERANCE / fit));

    // The selection, drawn over the picture.
    if let Some((layer, kind)) = state.selected {
        let moved = state.label_drag.filter(|d| kind == ObjectKind::Label(d.label) && layer == d.layer);
        let moved_legend = state.legend_drag.filter(|d| kind == ObjectKind::Legend && layer == d.layer);
        let moved_shape = state.shape_drag.filter(|d| kind == ObjectKind::Shape(d.shape) && layer == d.layer);
        let shift = moved
            .map(|d| d.delta)
            .or(moved_legend.map(|d| d.delta))
            .or(moved_shape.map(|d| d.delta))
            .unwrap_or((0.0, 0.0));
        draw_selection(&painter, &t.scene, layer, kind, &|(x, y)| to_screen((x + shift.0, y + shift.1)));
    }

    // Data read-out under the pointer, in the layer under it.
    if let Some(pos) = response.hover_pos() {
        let (px, py) = to_page(pos);
        if let Some((_, g)) = t.geometry.iter().enumerate().rev().find(|(i, g)| {
            let f = &g.frame;
            doc.project().graph(graph_id).and_then(|g| g.layers.get(*i)).is_some_and(|l| !l.display.hidden)
                && px >= f.x
                && px <= f.x + f.w
                && py >= f.y
                && py <= f.y + f.h
        }) {
            let (x, y) = g.page_to_data(px, py);
            painter.text(
                visible.left_bottom() + egui::vec2(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                format!("x = {}   y = {}", short(x), short(y)),
                egui::FontId::monospace(12.0),
                Color32::from_gray(40),
            );
        }
    }

    // The Data Reader takes the clicks on the page while it is on.
    if state.reader {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        let hover =
            response.hover_pos().and_then(|p| reader::nearest(doc.project(), graph_id, &t.geometry, &to_screen, p));
        if response.clicked() && hover.is_some() {
            state.reader_point = hover;
        }
        if let Some(p) = state.reader_point
            && response.hovered()
        {
            let step = ui.input(|i| {
                i.key_pressed(egui::Key::ArrowRight) as isize - i.key_pressed(egui::Key::ArrowLeft) as isize
            });
            if step != 0 {
                state.reader_point = Some(reader::step(doc.project(), graph_id, p, step));
            }
        }
        if let Some(p) = state.reader_point {
            reader::draw(&painter, doc.project(), graph_id, &t.geometry, &to_screen, p, true);
        }
        if let Some(p) = hover.filter(|h| Some(*h) != state.reader_point) {
            reader::draw(&painter, doc.project(), graph_id, &t.geometry, &to_screen, p, false);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.reader = false;
            state.reader_point = None;
        }
    }

    // Whether the Slope tool took this frame's click (it switches itself off when the annotation is added).
    let slope_took_input = state.slope_tool;
    // The Slope tool: a click on a data point picks the first point, a click on another point of the
    // same plot adds the annotation.
    if state.slope_tool {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        let only = state.slope_first.map(|p| (p.layer, p.plot));
        let hover = response
            .hover_pos()
            .and_then(|p| reader::nearest_in(doc.project(), graph_id, &t.geometry, &to_screen, p, only));
        let marker = |at: Pos2, color: Color32| {
            let r = Rect::from_center_size(at, egui::vec2(9.0, 9.0));
            painter.rect_stroke(r, 0.0, Stroke::new(1.5_f32, color), egui::StrokeKind::Middle);
        };
        let blue = Color32::from_rgb(0, 90, 200);
        let first_at =
            state.slope_first.and_then(|p| reader::screen_point(doc.project(), graph_id, &t.geometry, &to_screen, p));
        if let Some(a) = first_at {
            marker(a, blue);
        }
        if let Some(h) = hover
            && let Some(b) = reader::screen_point(doc.project(), graph_id, &t.geometry, &to_screen, h)
        {
            marker(b, Color32::from_rgba_unmultiplied(0, 90, 200, 130));
            if let Some(a) = first_at {
                painter.line_segment([a, b], Stroke::new(1.0_f32, blue));
            }
        }
        if response.clicked() {
            // A plot without one point per row, or a masked or missing row, cannot take a slope.
            let refused = hover.and_then(|h| {
                sheaf_plot::slope_check_point(doc.project(), graph_id, h.layer, h.plot, Some(h.row)).err()
            });
            match (state.slope_first, hover) {
                (_, Some(_)) if refused.is_some() => {
                    actions.push(Action::Hint(refused.unwrap_or_default()));
                }
                (None, Some(h)) => {
                    state.slope_first = Some(h);
                    actions.push(Action::Hint("Slope: click a second point of the same plot; Esc cancels".to_string()));
                }
                (Some(first), Some(h)) if h.row != first.row => {
                    let plot = graph.layers.get(first.layer).and_then(|l| l.plots.get(first.plot));
                    if let Some(plot) = plot {
                        let slope = sheaf_core::graph::SlopeAnnotation::for_plot(plot, first.row, h.row);
                        let index = graph.layers[first.layer].slopes.len();
                        state.slope_tool = false;
                        state.slope_first = None;
                        state.selected = Some((first.layer, ObjectKind::Slope(index)));
                        actions.push(Action::ActivateLayer { graph: graph_id, layer: first.layer });
                        actions.push(Action::AddSlope { graph: graph_id, layer: first.layer, slope });
                    }
                }
                (Some(_), _) => actions.push(Action::Hint(
                    "Slope: click a different data point of the same plot; Esc cancels".to_string(),
                )),
                (None, None) => actions.push(Action::Hint("Slope: click near a data point; Esc cancels".to_string())),
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.slope_tool = false;
            state.slope_first = None;
            actions.push(Action::Hint("Slope cancelled".to_string()));
        }
    }

    // The Data menu tools: Mask, Move and Remove Bad Data Points.
    if let Some(tool) = state.data_tool {
        let view = data_tools::View {
            project: doc.project(),
            graph: graph_id,
            geometry: &t.geometry,
            to_screen: &to_screen,
            to_page: &to_page,
        };
        data_tools::show(ui, &painter, &response, &view, tool, &mut state.data, actions);
        // Esc and a right-click (which also opens the context menu) end the mode.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) || response.secondary_clicked() {
            state.data_tool = None;
            state.data = data_tools::State::default();
        }
    }

    // Scale In / Scale Out (T69).
    graph_scale::show(
        ui,
        &painter,
        &response,
        &t.geometry,
        &to_page,
        graph_id,
        state.active_layer,
        &mut state.scale,
        actions,
    );

    // The Text tool: a click places a label there, in the layer under the click.
    if state.text_tool {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.text_tool = false;
        }
    }

    // Dragging a label moves it; the change is made when the button is let go.
    if response.drag_started()
        && !state.reader
        && !state.text_tool
        && state.shape_tool.is_none()
        && !state.slope_tool
        && state.data_tool.is_none()
        && !state.pan_tool
        && !zoom_took_input
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Hit::Object { layer, kind: ObjectKind::Label(label) } = pick(pos)
        && let Some(l) = graph.layers.get(layer).and_then(|l| l.labels.get(label))
    {
        state.selected = Some((layer, ObjectKind::Label(label)));
        state.label_drag = Some(LabelDrag { layer, label, from: (l.x, l.y), delta: (0.0, 0.0) });
        actions.push(Action::ActivateLayer { graph: graph_id, layer });
    }
    if let Some(mut d) = state.label_drag {
        let page = response.drag_delta() / fit;
        d.delta = (d.delta.0 + f64::from(page.x), d.delta.1 + f64::from(page.y));
        state.label_drag = Some(d);
        if response.drag_stopped() {
            state.label_drag = None;
            if let Some(f) = t.geometry.get(d.layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0)
                && (d.delta.0 != 0.0 || d.delta.1 != 0.0)
            {
                actions.push(Action::MoveLabel {
                    graph: graph_id,
                    layer: d.layer,
                    label: d.label,
                    x: d.from.0 + d.delta.0 / f.w,
                    y: d.from.1 + d.delta.1 / f.h,
                });
            }
        }
    }

    // The drawing tools: a drag draws one object, then the tool is the Pointer again.
    if let Some(kind) = state.shape_tool {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.shape_tool = None;
            state.drawing = None;
        }
        if response.drag_started()
            && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        {
            let layer = layer_of_hit(pick(pos)).unwrap_or(state.active_layer);
            if let Some(f) = t.geometry.get(layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0) {
                let (px, py) = to_page(pos);
                state.drawing = Some(Drawing { layer, kind, from: ((px - f.x) / f.w, (py - f.y) / f.h) });
                actions.push(Action::ActivateLayer { graph: graph_id, layer });
            }
        }
        if let Some(d) = state.drawing
            && let Some(f) = t.geometry.get(d.layer).map(|g| g.frame)
            && let Some(pos) = response.interact_pointer_pos()
        {
            let (px, py) = to_page(pos);
            let mut to = ((px - f.x) / f.w, (py - f.y) / f.h);
            if ui.input(|i| i.modifiers.shift) {
                to = shape::constrain(d.kind, d.from, to, (f.w, f.h));
            }
            let mut preview = Shape::new(d.kind, d.from, to);
            preview.color = sheaf_core::Color::BLACK;
            draw_preview(&painter, &preview, &|(x, y)| to_screen((f.x + x * f.w, f.y + y * f.h)));
            if response.drag_stopped() {
                state.drawing = None;
                // A click without a drag draws nothing and leaves the tool on.
                if ((to.0 - d.from.0) * f.w).hypot((to.1 - d.from.1) * f.h) >= MIN_DRAWN_PT {
                    let index = graph.layers.get(d.layer).map_or(0, |l| l.shapes.len());
                    state.shape_tool = None;
                    state.selected = Some((d.layer, ObjectKind::Shape(index)));
                    actions.push(Action::AddShape { graph: graph_id, layer: d.layer, shape: preview });
                }
            }
        }
    }

    // Dragging a drawn object moves it; the change is made when the button is let go.
    if response.drag_started()
        && !state.reader
        && !state.text_tool
        && state.shape_tool.is_none()
        && !state.slope_tool
        && state.data_tool.is_none()
        && !state.pan_tool
        && !zoom_took_input
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Hit::Object { layer, kind: ObjectKind::Shape(shape) } = pick(pos)
    {
        state.selected = Some((layer, ObjectKind::Shape(shape)));
        state.shape_drag = Some(ShapeDrag { layer, shape, delta: (0.0, 0.0) });
        actions.push(Action::ActivateLayer { graph: graph_id, layer });
    }
    if let Some(mut d) = state.shape_drag {
        let page = response.drag_delta() / fit;
        d.delta = (d.delta.0 + f64::from(page.x), d.delta.1 + f64::from(page.y));
        state.shape_drag = Some(d);
        if response.drag_stopped() {
            state.shape_drag = None;
            if let Some(f) = t.geometry.get(d.layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0)
                && (d.delta.0 != 0.0 || d.delta.1 != 0.0)
            {
                actions.push(Action::MoveShape {
                    graph: graph_id,
                    layer: d.layer,
                    shape: d.shape,
                    dx: d.delta.0 / f.w,
                    dy: d.delta.1 / f.h,
                });
            }
        }
    }

    // Dragging the legend moves it too, and makes its position manual.
    if response.drag_started()
        && !state.reader
        && !state.text_tool
        && state.shape_tool.is_none()
        && !state.slope_tool
        && state.data_tool.is_none()
        && !state.pan_tool
        && !zoom_took_input
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Hit::Object { layer, kind: ObjectKind::Legend } = pick(pos)
        && let Some(f) = t.geometry.get(layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0)
        && let Some(o) = t
            .scene
            .layers
            .iter()
            .flat_map(|l| l.objects.iter())
            .find(|o| o.layer == layer && o.kind == ObjectKind::Legend)
    {
        let centre = (o.bounds.x + o.bounds.w / 2.0, o.bounds.y + o.bounds.h / 2.0);
        state.selected = Some((layer, ObjectKind::Legend));
        state.legend_drag =
            Some(LegendDrag { layer, from: ((centre.0 - f.x) / f.w, (centre.1 - f.y) / f.h), delta: (0.0, 0.0) });
        actions.push(Action::ActivateLayer { graph: graph_id, layer });
    }
    if let Some(mut d) = state.legend_drag {
        let page = response.drag_delta() / fit;
        d.delta = (d.delta.0 + f64::from(page.x), d.delta.1 + f64::from(page.y));
        state.legend_drag = Some(d);
        if response.drag_stopped() {
            state.legend_drag = None;
            if let Some(f) = t.geometry.get(d.layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0)
                && (d.delta.0 != 0.0 || d.delta.1 != 0.0)
            {
                actions.push(Action::MoveLegend {
                    graph: graph_id,
                    layer: d.layer,
                    x: d.from.0 + d.delta.0 / f.w,
                    y: d.from.1 + d.delta.1 / f.h,
                });
            }
        }
    }

    // Clicks on the page.
    let layer_of = layer_of_hit;
    if response.clicked()
        && state.text_tool
        && let Some(pos) = response.interact_pointer_pos()
    {
        let (px, py) = to_page(pos);
        let layer = layer_of(pick(pos)).unwrap_or(state.active_layer);
        if let Some(f) = t.geometry.get(layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0) {
            state.text_tool = false;
            actions.push(Action::ActivateLayer { graph: graph_id, layer });
            actions.push(Action::AddText {
                graph: graph_id,
                layer: Some(layer),
                at: Some(((px - f.x) / f.w, (py - f.y) / f.h)),
            });
        }
    } else if response.clicked()
        && !state.reader
        && !slope_took_input
        && !zoom_took_input
        && state.data_tool.is_none()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let hit = pick(pos);
        state.selected = match hit {
            Hit::Object { layer, kind } => Some((layer, kind)),
            _ => None,
        };
        if let Some(layer) = layer_of(hit) {
            actions.push(Action::ActivateLayer { graph: graph_id, layer });
        }
    }
    if response.double_clicked()
        && !state.reader
        && !slope_took_input
        && !zoom_took_input
        && state.data_tool.is_none()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let hit = pick(pos);
        let open = |layer, tab, plot| Action::GraphProperties { graph: graph_id, layer, tab, plot };
        let action = match hit {
            Hit::Object { layer, kind } => match kind {
                ObjectKind::XAxis | ObjectKind::XTitle => Some(open(layer, PropsTab::XAxis, None)),
                ObjectKind::YAxis | ObjectKind::YTitle => Some(open(layer, PropsTab::YAxis, None)),
                ObjectKind::Legend => Some(open(layer, PropsTab::Legend, None)),
                ObjectKind::Plot(p) => Some(open(layer, PropsTab::Plots, Some(p))),
                ObjectKind::Label(label) => Some(Action::EditText { graph: graph_id, layer, label }),
                ObjectKind::Shape(shape) => Some(Action::EditShape { graph: graph_id, layer, shape }),
                ObjectKind::Slope(slope) => Some(Action::EditSlope { graph: graph_id, layer, slope }),
                ObjectKind::StraightLine(line) => Some(Action::EditStraightLine { graph: graph_id, layer, line }),
                ObjectKind::Scale(_) => Some(open(layer, PropsTab::Plots, None)),
            },
            Hit::Layer(layer) => Some(open(layer, PropsTab::Plots, None)),
            Hit::Page => Some(open(state.active_layer, PropsTab::Page, None)),
        };
        actions.extend(action);
    }
    if response.secondary_clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let hit = pick(pos);
        state.menu_on = Some(MenuTarget::Hit(hit));
        state.menu_page = to_page(pos);
        if let Some(layer) = layer_of(hit) {
            state.active_layer = layer;
            actions.push(Action::ActivateLayer { graph: graph_id, layer });
        }
        if let Hit::Object { layer, kind } = hit {
            state.selected = Some((layer, kind));
        }
    }

    // The Integrate gadget, over the picture and above the page's clicks.
    if let Some(mut gadget) = state.gadget
        && let Some(geom) = t.geometry.get(gadget.layer)
    {
        let placement = sheaf_plot::stack_places(doc.project(), graph_id, gadget.layer)
            .ok()
            .and_then(|o| o.get(gadget.plot).copied())
            .unwrap_or_default();
        let c = graph_context(graph_id, state.active_layer, state.tool());
        state.gadget_rect = gadget.show(ui, doc.project(), graph_id, geom, fit, &to_screen, placement, &c, actions);
        state.gadget = Some(gadget);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.gadget = None;
        }
    }

    // Layer icons: after the page, so that they get the clicks on them.
    let mut icon_menu = None;
    // Plot Details > Miscellaneous can hide the icons.
    let shown_icons = if doc.project().graph(graph_id).is_some_and(|g| g.page.style.layer_icons) { layers } else { 0 };
    for i in 0..shown_icons {
        let r = Rect::from_min_size(image_rect.min + egui::vec2(2.0 + i as f32 * (ICON + 2.0), 2.0), Vec2::splat(ICON));
        let id = ui.id().with(("layer_icon", graph_id.0, i));
        let resp = ui.interact(r, id, Sense::click()).on_hover_text(format!("Layer {}", i + 1));
        let active = i == state.active_layer;
        painter.rect_filled(r, 0.0, if active { Color32::from_gray(150) } else { Color32::from_gray(225) });
        painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_gray(90)), egui::StrokeKind::Inside);
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", i + 1),
            egui::FontId::proportional(11.0),
            if active { Color32::BLACK } else { Color32::from_gray(110) },
        );
        if resp.clicked() {
            state.active_layer = i;
            actions.push(Action::ActivateLayer { graph: graph_id, layer: i });
        }
        if resp.double_clicked() {
            actions.push(Action::GraphProperties { graph: graph_id, layer: i, tab: PropsTab::Plots, plot: None });
        }
        if resp.secondary_clicked() {
            state.active_layer = i;
            state.menu_on = Some(MenuTarget::LayerIcon(i));
            actions.push(Action::ActivateLayer { graph: graph_id, layer: i });
        }
        icon_menu = Some(icon_menu.map_or(resp.clone(), |r: egui::Response| r.union(resp)));
    }

    // The right-click menus.
    let c = graph_context(graph_id, state.active_layer, state.tool());
    let entries = |target: MenuTarget| menu_for(doc.project(), graph_id, target, state.selected, state.menu_page);
    let target = state.menu_on;
    let mut shown = response.context_menu(|ui| {
        if let Some(t @ MenuTarget::Hit(_)) = target {
            menus::show_context(ui, &entries(t), &c, actions);
        }
    });
    if let Some(icons) = icon_menu {
        let on_icon = icons.context_menu(|ui| {
            if let Some(t @ MenuTarget::LayerIcon(_)) = target {
                menus::show_context(ui, &entries(t), &c, actions);
            }
        });
        shown = shown.or(on_icon);
    }
    if shown.is_none() && !response.secondary_clicked() {
        state.menu_on = None;
    }
}

/// The width of a scroll bar, in UI points.
const BAR: f32 = 10.0;
/// Sheaf's choice (not measured in Origin): Ctrl+wheel zooms by egui's zoom delta, about 1.2 per
/// wheel notch, about the pointer.
///
/// The view input of a graph window: Esc ends Zoom In and Pan; a click in the Zoom In mode zooms
/// by [`graph_zoom::CLICK_FACTOR`] about the click and ends the mode (Origin's measured behaviour
/// is a mode; ending it after one click is Sheaf's choice); a drag with the Pan tool moves the
/// view; the wheel scrolls (Shift+wheel sideways, a Sheaf choice); the scroll bars drag.
fn view_input(ui: &mut egui::Ui, response: &egui::Response, rect: Rect, page: Vec2, state: &mut GraphUi) {
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if escape {
        state.zoom_in_tool = false;
        state.pan_tool = false;
    }
    if state.zoom_in_tool {
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ZoomIn);
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            // Origin re-centres the view on the clicked point.
            let factor = state.zoom.factor * graph_zoom::CLICK_FACTOR;
            state.zoom = graph_zoom::centre_on(rect, page, state.zoom, factor, pos);
            state.zoom_in_tool = false;
        }
    }
    if state.pan_tool {
        if response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            state.zoom = graph_zoom::pan(rect, page, state.zoom, response.drag_delta());
        } else if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        // Origin's Zoom-Panning tool: "+" and "-" zoom by 1.25 and 0.85 about the middle of the view,
        // the arrow keys move it by a small step, the wheel zooms (measured except the wheel).
        let (plus, minus, keys, wheel, pointer) = ui.input(|i| {
            let plus = i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals);
            let minus = i.key_pressed(egui::Key::Minus);
            let step = graph_zoom::ARROW_STEP_PT;
            let mut d = Vec2::ZERO;
            for (key, v) in [
                (egui::Key::ArrowLeft, egui::vec2(-step, 0.0)),
                (egui::Key::ArrowRight, egui::vec2(step, 0.0)),
                (egui::Key::ArrowUp, egui::vec2(0.0, -step)),
                (egui::Key::ArrowDown, egui::vec2(0.0, step)),
            ] {
                if i.key_pressed(key) {
                    d += v;
                }
            }
            (plus, minus, d, i.raw_scroll_delta.y, i.pointer.hover_pos())
        });
        let centre = graph_zoom::content(rect).center();
        if plus {
            let f = state.zoom.factor * graph_zoom::PAN_ZOOM_IN;
            state.zoom = graph_zoom::zoom_about(rect, page, state.zoom, f, centre);
        }
        if minus {
            let f = state.zoom.factor * graph_zoom::PAN_ZOOM_OUT;
            state.zoom = graph_zoom::zoom_about(rect, page, state.zoom, f, centre);
        }
        if keys != Vec2::ZERO {
            // the view moves over the page: the page moves the other way on screen
            let scale = graph_zoom::layout(rect, page, state.zoom).scale;
            state.zoom = graph_zoom::pan(rect, page, state.zoom, -keys * scale);
        }
        if response.hovered() && wheel != 0.0 {
            let f = state.zoom.factor * if wheel > 0.0 { graph_zoom::PAN_ZOOM_IN } else { graph_zoom::PAN_ZOOM_OUT };
            state.zoom = graph_zoom::zoom_about(rect, page, state.zoom, f, pointer.unwrap_or(centre));
            ui.input_mut(|i| {
                i.smooth_scroll_delta = Vec2::ZERO;
                i.raw_scroll_delta = Vec2::ZERO;
            });
        }
    }
    if response.hovered() && !state.pan_tool {
        let (scroll, zoom_delta, pointer) =
            ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
        if (zoom_delta - 1.0).abs() > f32::EPSILON
            && let Some(at) = pointer
        {
            let factor = state.zoom.factor * zoom_delta;
            state.zoom = graph_zoom::zoom_about(rect, page, state.zoom, factor, at);
        }
        if scroll != Vec2::ZERO && state.zoom.factor > 1.0 {
            state.zoom = graph_zoom::pan(rect, page, state.zoom, scroll);
            ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }
    }
    // The scroll bars.
    let lay = graph_zoom::layout(rect, page, state.zoom);
    let content = graph_zoom::content(rect);
    let (show_x, show_y) = bars_shown(rect, &lay);
    for vertical in [false, true] {
        if !(if vertical { show_y } else { show_x }) {
            continue;
        }
        let track = bar_track(rect, vertical, show_x && show_y);
        let id = ui.id().with(("graph_scroll_bar", vertical));
        let resp = ui.interact(track, id, Sense::click_and_drag());
        if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
            let (len, total, view, from) = if vertical {
                (track.height(), lay.image.height(), content.height(), track.top())
            } else {
                (track.width(), lay.image.width(), content.width(), track.left())
            };
            let at = if vertical { pos.y } else { pos.x } - from;
            let scroll = graph_zoom::scroll_for_thumb(len, total, view, at) / lay.scale;
            if vertical {
                state.zoom.scroll.y = scroll;
            } else {
                state.zoom.scroll.x = scroll;
            }
        }
    }
}

/// Whether the horizontal and the vertical scroll bar show: the page is larger than the content
/// area on that axis.
fn bars_shown(rect: Rect, lay: &graph_zoom::Layout) -> (bool, bool) {
    let content = graph_zoom::content(rect);
    (lay.image.width() > content.width() + 0.01, lay.image.height() > content.height() + 0.01)
}

/// The track of a scroll bar along the right (vertical) or bottom edge of the window.
fn bar_track(rect: Rect, vertical: bool, both: bool) -> Rect {
    let corner = if both { BAR } else { 0.0 };
    if vertical {
        Rect::from_min_max(egui::pos2(rect.right() - BAR, rect.top()), egui::pos2(rect.right(), rect.bottom() - corner))
    } else {
        Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - BAR),
            egui::pos2(rect.right() - corner, rect.bottom()),
        )
    }
}

/// The scroll bars, drawn over the window's right and bottom edges while the page is larger than
/// the window.
fn paint_scroll_bars(painter: &egui::Painter, rect: Rect, page: Vec2, zoom: Zoom) {
    let lay = graph_zoom::layout(rect, page, zoom);
    let content = graph_zoom::content(rect);
    let (show_x, show_y) = bars_shown(rect, &lay);
    for vertical in [false, true] {
        if !(if vertical { show_y } else { show_x }) {
            continue;
        }
        let track = bar_track(rect, vertical, show_x && show_y);
        painter.rect_filled(track, 0.0, Color32::from_gray(225));
        let (len, total, view, scroll) = if vertical {
            (track.height(), lay.image.height(), content.height(), lay.scroll.y * lay.scale)
        } else {
            (track.width(), lay.image.width(), content.width(), lay.scroll.x * lay.scale)
        };
        let (start, size) = graph_zoom::thumb(len, total, view, scroll);
        let thumb = if vertical {
            Rect::from_min_size(egui::pos2(track.left(), track.top() + start), egui::vec2(BAR, size))
        } else {
            Rect::from_min_size(egui::pos2(track.left() + start, track.top()), egui::vec2(size, BAR))
        };
        painter.rect_filled(thumb.shrink(1.5), 2.0, Color32::from_gray(130));
    }
}

/// The menu context of a graph window's own menus.
fn graph_context(graph_id: WindowId, layer: usize, tool: GraphTool) -> MenuContext<'static> {
    MenuContext {
        kind: WindowKind::Graph,
        book: None,
        graph: Some(graph_id),
        active: Some(graph_id),
        selected: &[],
        sorting_columns: Vec::new(),
        cells: false,
        whole_columns: false,
        imported: false,
        undo: None,
        redo: None,
        undo_ai: false,
        windows: Vec::new(),
        folders: Vec::new(),
        folder: Vec::new(),
        view_windows: Default::default(),
        open: None,
        recent_projects: &[],
        recent_books: &[],
        recent_graphs: &[],
        recent_commands: &crate::menus::NO_COMMANDS,
        recent_exports: &[],
        user_templates: &[],
        templates: false,
        recent_imports: &[],
        panels: [true; 4],
        ai_link: false,
        layer,
        tool,
        curve: None,
        layer_curves: Vec::new(),
        stack: None,
        can_cumulate: false,
        can_color_scale: false,
        selected_shape: None,
        view_mode: Default::default(),
        layer_icons: true,
    }
}

/// How the layer icon's menu names a plot: `g1 [Book1]Sheet1! "Conc"(X), "Signal"(Y) [1*:10*]`.
pub(crate) fn plot_source_line(project: &Project, plot: &sheaf_core::Plot) -> String {
    let Some((book, sheet, y)) = project.locate(plot.y) else {
        return project.plot_label(plot);
    };
    let x = match plot.x.and_then(|x| sheet.column(x)) {
        Some(x) => format!("\"{}\"(X), ", x.display_name()),
        None => String::new(),
    };
    let range = match plot.rows {
        Some((a, b)) => format!("{}:{}", a + 1, b),
        None => format!("1*:{}*", y.len()),
    };
    format!("g1 [{}]{}! {x}\"{}\"(Y) [{range}]", book.name, sheet.name, y.display_name())
}

/// Whether the requested layer is the only shown layer.
fn other_layers_hidden(graph: &sheaf_core::graph::Graph, layer: usize) -> bool {
    graph.layers.len() > 1
        && graph.layers.get(layer).is_some_and(|l| !l.display.hidden)
        && graph.layers.iter().enumerate().all(|(i, l)| i == layer || l.display.hidden)
}

/// The entries of the right-click menu for what was clicked.
fn menu_for(
    project: &Project,
    graph_id: WindowId,
    target: MenuTarget,
    selected: Option<(usize, ObjectKind)>,
    page: (f64, f64),
) -> Vec<menus::Entry> {
    let Some(graph) = project.graph(graph_id) else {
        return Vec::new();
    };
    match target {
        MenuTarget::LayerIcon(layer) => {
            let plots: Vec<String> = graph
                .layers
                .get(layer)
                .map(|l| l.plots.iter().map(|p| plot_source_line(project, p)).collect())
                .unwrap_or_default();
            let active = match selected {
                Some((l, ObjectKind::Plot(p))) if l == layer => Some(p),
                _ => None,
            };
            let mut entries = menus::layer_icon_menu(layer, graph.layers.len(), &plots, active);
            if graph.layers.get(layer).is_some_and(|l| l.display.hidden)
                && let Some(menus::Entry::Item(item)) = entries.first_mut()
            {
                item.label = "\u{2713} Hide Layer".into();
            }
            if other_layers_hidden(graph, layer)
                && let Some(menus::Entry::Item(item)) = entries.get_mut(1)
            {
                item.label = "\u{2713} Hide Other Layers".into();
            }
            entries
        }
        MenuTarget::Hit(Hit::Object { layer, kind }) => match kind {
            ObjectKind::Plot(p) => {
                let Some(plot) = graph.layers.get(layer).and_then(|l| l.plots.get(p)) else {
                    return Vec::new();
                };
                let book = project.locate(plot.y).map(|(b, _, _)| (b.id, b.name.clone()));
                let on_right = graph.right_y_member(layer) == Some(layer);
                let cols = plot_columns(project, plot);
                menus::graph_plot_menu(
                    layer,
                    p,
                    plot.kind,
                    on_right,
                    book,
                    plot.decor.trendlines.len(),
                    graph.layers.len(),
                    &cols,
                )
            }
            ObjectKind::XAxis | ObjectKind::YAxis => {
                let y = kind == ObjectKind::YAxis;
                let grids = graph.layers.get(layer).is_some_and(|l| {
                    let a = if y { &l.y_axis } else { &l.x_axis };
                    a.major_grid.is_some() || a.minor_grid.is_some()
                });
                menus::graph_axis_menu(layer, y, grids, page)
            }
            ObjectKind::XTitle => menus::graph_title_menu(layer, false),
            ObjectKind::YTitle => menus::graph_title_menu(layer, true),
            ObjectKind::Legend => {
                let l = graph.layers.get(layer);
                let applies = |k| l.is_some_and(|l| sheaf_core::layer_scale::LayerScale::applies(l, k));
                menus::graph_legend_menu(
                    layer,
                    applies(sheaf_core::layer_scale::LayerScaleKind::Color),
                    applies(sheaf_core::layer_scale::LayerScaleKind::Bubble),
                )
            }
            ObjectKind::Label(k) => menus::graph_label_menu(layer, k),
            ObjectKind::Shape(k) => menus::graph_shape_menu(layer, k),
            ObjectKind::Slope(k) => menus::graph_slope_menu(layer, k),
            ObjectKind::StraightLine(k) => menus::graph_straight_line_menu(layer, k),
            ObjectKind::Scale(k) => menus::graph_scale_menu(layer, k),
        },
        MenuTarget::Hit(Hit::Layer(layer)) => menus::graph_page_menu(layer),
        MenuTarget::Hit(Hit::Page) => menus::graph_page_menu(0),
    }
}

/// The columns of a plot's sheet as Change X and Change Y list them: `[Book1]Sheet1!A(X)`.
fn plot_columns(project: &Project, plot: &sheaf_core::graph::Plot) -> menus::PlotColumns {
    let list = project
        .locate(plot.y)
        .map(|(book, sheet, _)| {
            sheet
                .columns
                .iter()
                .map(|c| (c.id, format!("[{}]{}!{}({})", book.name, sheet.name, c.short_name, c.designation.tag())))
                .collect()
        })
        .unwrap_or_default();
    menus::PlotColumns { list, x: plot.x, y: Some(plot.y), grouped: plot.group.is_some() }
}

/// Handles round the selected object: small squares on the data points of a plot (at most about
/// a hundred), green diamonds on the corners and edges of anything else.
fn draw_selection(
    painter: &egui::Painter,
    scene: &Scene,
    layer: usize,
    kind: ObjectKind,
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
) {
    let found = scene
        .layers
        .iter()
        .flat_map(|l| l.objects.iter().map(move |o| (l, o)))
        .filter(|(_, o)| o.layer == layer && o.kind == kind);
    let mut bounds: Option<Rect> = None;
    let mut points: Vec<(f64, f64)> = Vec::new();
    for (l, o) in found {
        let b = Rect::from_min_max(
            to_screen((o.bounds.x, o.bounds.y)),
            to_screen((o.bounds.x + o.bounds.w, o.bounds.y + o.bounds.h)),
        );
        bounds = Some(bounds.map_or(b, |a| a.union(b)));
        if o.list == ItemList::Plots {
            for item in &l.plots[o.items.clone()] {
                if let Item::Path { d, .. } = item {
                    points.extend(sheaf_plot::hit::path_points(d));
                }
            }
        }
    }
    let Some(bounds) = bounds else { return };
    let edge = Stroke::new(1.0_f32, Color32::BLACK);
    if matches!(kind, ObjectKind::Plot(_)) {
        let step = points.len().div_ceil(100).max(1);
        for p in points.iter().step_by(step) {
            let r = Rect::from_center_size(to_screen(*p), Vec2::splat(5.0));
            painter.rect_filled(r, 0.0, Color32::WHITE);
            painter.rect_stroke(r, 0.0, edge, egui::StrokeKind::Inside);
        }
        return;
    }
    let b = bounds.expand(2.0);
    let green = Color32::from_rgb(0, 170, 0);
    for p in [
        b.left_top(),
        b.center_top(),
        b.right_top(),
        b.right_center(),
        b.right_bottom(),
        b.center_bottom(),
        b.left_bottom(),
        b.left_center(),
    ] {
        let s = 4.0;
        let pts =
            vec![p + egui::vec2(0.0, -s), p + egui::vec2(s, 0.0), p + egui::vec2(0.0, s), p + egui::vec2(-s, 0.0)];
        painter.add(egui::Shape::convex_polygon(pts, green, edge));
    }
}

/// The range an axis gets from Axis Center: the span is kept, in the axis' own scale (decades on a
/// log axis), and the value at unit position `u` (0 at the axis' low end, 1 at its high end, as
/// drawn) becomes the middle. `None` when the position or the result is not a number.
fn centered_range(axis: &sheaf_plot::ticks::ResolvedAxis, u: f64) -> Option<(f64, f64)> {
    let (a, b) = (axis.from_unit(u - 0.5), axis.from_unit(u + 0.5));
    (u.is_finite() && a.is_finite() && b.is_finite() && a != b).then(|| (a.min(b), a.max(b)))
}

/// Applies a change made from the graph window to the project. `resolved` is the present layout
/// of the graph (its axes' ranges), used by the axis zoom.
pub fn apply_op(project: &mut Project, graph: WindowId, op: GraphOp, resolved: Option<&[LayerGeometry]>) {
    if let GraphOp::UpdateLegend { layer } = op {
        let Ok(auto) = sheaf_plot::legend_automatic_text(project, graph, layer) else { return };
        if let Some(l) = project.graph_mut(graph).and_then(|g| g.layers.get_mut(layer))
            && let Some(text) = &l.legend.text
        {
            let updated = update_legend_text(text, &auto);
            l.legend.text = (updated != auto).then_some(updated);
        }
        return;
    }
    let Some(g) = project.graph_mut(graph) else { return };
    let layers = g.layers.len();
    let layer_index = match op {
        GraphOp::SetPlotKind { layer, .. }
        | GraphOp::HidePlot { layer, .. }
        | GraphOp::SetPlotColumn { layer, .. }
        | GraphOp::Ungroup { layer, .. }
        | GraphOp::RemoveTrendline { layer, .. }
        | GraphOp::MovePlotToAxis { layer, .. }
        | GraphOp::MovePlotToLayer { layer, .. }
        | GraphOp::HideAxis { layer, .. }
        | GraphOp::ToggleGrids { layer, .. }
        | GraphOp::Rescale { layer }
        | GraphOp::ExchangeAxes { layer }
        | GraphOp::SetStack { layer, .. }
        | GraphOp::AxisZoom { layer, .. }
        | GraphOp::AxisCenter { layer, .. }
        | GraphOp::HideLegend { layer }
        | GraphOp::ReconstructLegend { layer }
        | GraphOp::UpdateLegend { layer }
        | GraphOp::HideTitle { layer, .. }
        | GraphOp::DeleteLabel { layer, .. }
        | GraphOp::DeleteShape { layer, .. }
        | GraphOp::DeleteSlope { layer, .. }
        | GraphOp::DeleteStraightLine { layer, .. }
        | GraphOp::AddScale { layer, .. }
        | GraphOp::DeleteScale { layer, .. }
        | GraphOp::OrderShape { layer, .. }
        | GraphOp::HideLayer { layer }
        | GraphOp::HideOtherLayers { layer }
        | GraphOp::AddLayer { layer, .. }
        | GraphOp::DeleteLayer { layer } => layer,
    };
    if layer_index >= layers {
        return;
    }
    if let GraphOp::SetPlotColumn { layer, plot, y, column } = op {
        // Origin keeps the axis as drawn (measured: Y stayed -25..125): an automatic range is fixed
        // at its present values before the column changes.
        let drawn = resolved.and_then(|r| r.get(layer)).map(|r| if y { r.y.clone() } else { r.x.clone() });
        let grouped = g.layers.get(layer).and_then(|l| l.plots.get(plot)).is_some_and(|p| p.group.is_some());
        if let (Some(ra), false, Some(l)) = (drawn, grouped, g.layers.get_mut(layer)) {
            let a = if y { &mut l.y_axis } else { &mut l.x_axis };
            let (lo, hi) = (ra.from_unit(0.0), ra.from_unit(1.0));
            if a.min.is_none() && a.max.is_none() && lo.is_finite() && hi.is_finite() && lo != hi {
                set_limits(a, lo.min(hi), lo.max(hi), ra.reversed);
            }
        }
        g.set_plot_column(layer, plot, y, column);
        return;
    }
    if let GraphOp::Ungroup { layer, plot } = op {
        g.ungroup_plots(layer, plot);
        return;
    }
    if let GraphOp::MovePlotToAxis { layer, plot, right } = op {
        // An empty layer left behind is kept; a request that cannot be met changes nothing.
        let _ = g.set_plot_y_axis(layer, plot, right);
        return;
    }
    if let GraphOp::MovePlotToLayer { layer, plot, to } = op {
        let _ = g.move_plot_to_layer(layer, plot, to);
        return;
    }
    if let GraphOp::AddLayer { layer, kind } = op {
        let _ = g.add_layer(kind, layer);
        return;
    }
    if let GraphOp::DeleteLayer { layer } = op {
        if let Some(r) = resolved {
            let ranges: Vec<_> =
                r.iter().map(|l| ((l.x.min, l.x.max, l.x.step), (l.y.min, l.y.max, l.y.step))).collect();
            g.freeze_linked_ranges(layer, &ranges);
        }
        let _ = g.delete_layer(layer);
        return;
    }
    if let GraphOp::HideOtherLayers { layer } = op {
        if layers < 2 {
            return;
        }
        let hide = !other_layers_hidden(g, layer);
        for (i, l) in g.layers.iter_mut().enumerate() {
            l.display.hidden = hide && i != layer;
        }
        return;
    }
    let l = &mut g.layers[layer_index];
    match op {
        GraphOp::SetPlotKind { plot, kind, .. } => {
            if let Some(p) = l.plots.get_mut(plot) {
                p.kind = kind;
            }
        }
        GraphOp::RemoveTrendline { plot, .. } => {
            if let Some(p) = l.plots.get_mut(plot) {
                p.decor.trendlines.pop();
            }
        }
        GraphOp::HidePlot { plot, .. } => {
            if let Some(p) = l.plots.get_mut(plot) {
                p.visible = false;
            }
        }
        GraphOp::HideAxis { y, .. } => {
            let a = if y { &mut l.y_axis } else { &mut l.x_axis };
            a.side = sheaf_core::graph::AxisSide::Hidden;
        }
        GraphOp::ToggleGrids { y, .. } => {
            let a = if y { &mut l.y_axis } else { &mut l.x_axis };
            if a.major_grid.is_some() || a.minor_grid.is_some() {
                a.major_grid = None;
                a.minor_grid = None;
            } else {
                a.major_grid = Some(sheaf_core::graph::LineStyle::solid(GRID_COLOR, 0.5));
            }
        }
        GraphOp::Rescale { .. } => {
            for a in [&mut l.x_axis, &mut l.y_axis] {
                a.min = None;
                a.max = None;
            }
        }
        GraphOp::SetStack { stack, .. } => {
            if std::mem::discriminant(&l.stack) != std::mem::discriminant(&stack) {
                l.stack = stack;
            }
        }
        GraphOp::ExchangeAxes { .. } => {
            if l.plots.iter().all(|p| p.x.is_some()) {
                std::mem::swap(&mut l.x_axis, &mut l.y_axis);
                for p in &mut l.plots {
                    if let Some(x) = p.x {
                        p.x = Some(p.y);
                        p.y = x;
                    }
                    std::mem::swap(&mut p.y_error, &mut p.x_error);
                }
            }
        }
        GraphOp::AxisZoom { y, zoom_in, .. } => {
            let Some(r) = resolved.and_then(|r| r.get(layer_index)) else { return };
            let ra = if y { &r.y } else { &r.x };
            // Zoom about the middle in the axis' own scale: a log axis zooms in decades.
            let (u0, u1) = if zoom_in { (0.25, 0.75) } else { (-0.5, 1.5) };
            let (lo, hi) = (ra.from_unit(u0), ra.from_unit(u1));
            if lo.is_finite() && hi.is_finite() && lo != hi {
                let by_limits = ra.reversed;
                let a = if y { &mut l.y_axis } else { &mut l.x_axis };
                set_limits(a, lo.min(hi), lo.max(hi), by_limits);
            }
        }
        GraphOp::AxisCenter { y, page, .. } => {
            let Some(r) = resolved.and_then(|r| r.get(layer_index)) else { return };
            let f = r.frame;
            if !(f.w > 0.0 && f.h > 0.0) {
                return;
            }
            let (ra, u) = if y { (&r.y, 1.0 - (page.1 - f.y) / f.h) } else { (&r.x, (page.0 - f.x) / f.w) };
            if let Some((lo, hi)) = centered_range(ra, u) {
                let by_limits = ra.reversed;
                let a = if y { &mut l.y_axis } else { &mut l.x_axis };
                set_limits(a, lo, hi, by_limits);
            }
        }
        GraphOp::HideLegend { .. } => l.legend.visible = false,
        GraphOp::ReconstructLegend { .. } => l.legend.text = None,
        GraphOp::HideTitle { y, .. } => {
            let a = if y { &mut l.y_axis } else { &mut l.x_axis };
            a.show_title = false;
        }
        GraphOp::DeleteLabel { label, .. } => {
            if label < l.labels.len() {
                l.labels.remove(label);
            }
        }
        GraphOp::DeleteShape { shape, .. } => {
            if shape < l.shapes.len() {
                l.shapes.remove(shape);
            }
        }
        GraphOp::DeleteSlope { slope, .. } => {
            if slope < l.slopes.len() {
                l.slopes.remove(slope);
            }
        }
        GraphOp::DeleteStraightLine { line, .. } => {
            if line < l.straight_lines.len() {
                l.straight_lines.remove(line);
            }
        }
        GraphOp::AddScale { kind, .. } => {
            // One scale of each kind per layer.
            if sheaf_core::layer_scale::LayerScale::applies(l, kind) && !l.scales.iter().any(|s| s.kind == kind) {
                l.scales.push(sheaf_core::layer_scale::LayerScale::new(kind));
            }
        }
        GraphOp::DeleteScale { scale, .. } => {
            if scale < l.scales.len() {
                l.scales.remove(scale);
            }
        }
        GraphOp::OrderShape { shape, front, .. } => {
            let len = l.shapes.len();
            if shape < len && ((front && shape + 1 < len) || (!front && shape > 0)) {
                let value = l.shapes.remove(shape);
                if front {
                    l.shapes.push(value);
                } else {
                    l.shapes.insert(0, value);
                }
            }
        }
        GraphOp::HideLayer { .. } => l.display.hidden = !l.display.hidden,
        GraphOp::HideOtherLayers { .. }
        | GraphOp::AddLayer { .. }
        | GraphOp::DeleteLayer { .. }
        | GraphOp::MovePlotToAxis { .. }
        | GraphOp::MovePlotToLayer { .. }
        | GraphOp::SetPlotColumn { .. }
        | GraphOp::Ungroup { .. }
        | GraphOp::UpdateLegend { .. } => {
            unreachable!("handled above")
        }
    }
}

/// The plot ids a legend line refers to, through `\l(id)` and `%(id)`.
fn legend_refs(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find(['\\', '%']) {
        rest = &rest[at..];
        let body = rest.strip_prefix("\\l(").or_else(|| rest.strip_prefix("\\L(")).or_else(|| rest.strip_prefix("%("));
        match body.and_then(|b| b.split_once(')')) {
            Some((id, _)) => {
                out.push(id);
                rest = &rest[1..];
            }
            None => rest = &rest[1..],
        }
    }
    out
}

/// Update Legend for custom legend text: keeps every line that refers to a plot that still
/// exists (or to none), drops the lines that refer only to plots that are gone, and appends the
/// automatic line of each plot that no kept line mentions. `auto` is the automatic text.
fn update_legend_text(custom: &str, auto: &str) -> String {
    let valid: Vec<&str> = auto.lines().filter_map(|l| legend_refs(l).first().copied()).collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut mentioned: Vec<&str> = Vec::new();
    for line in custom.split('\n') {
        let refs = legend_refs(line);
        if !refs.is_empty() && !refs.iter().any(|r| valid.contains(r)) {
            continue;
        }
        mentioned.extend(refs);
        kept.push(line);
    }
    let mut out: Vec<&str> = kept;
    for line in auto.lines() {
        if legend_refs(line).first().is_some_and(|id| !mentioned.contains(id)) {
            out.push(line);
        }
    }
    out.join("\n")
}

/// The colour of grid lines that "Show Grids" adds: Origin's light grey.
const GRID_COLOR: sheaf_core::Color = sheaf_core::Color { r: 204, g: 204, b: 204, a: 255 };

fn short(v: f64) -> String {
    sheaf_core::format::format_general(v, 5)
}

/// Writes a new range (`lo` below `hi`) as fixed limits, keeping the axis' direction: an axis drawn
/// reversed (`drawn_reversed`) without its Reverse switch is reversed by its limits (min above max),
/// so they are written that way round again.
fn set_limits(a: &mut sheaf_core::graph::Axis, lo: f64, hi: f64, drawn_reversed: bool) {
    if drawn_reversed && !a.reverse {
        a.min = Some(hi);
        a.max = Some(lo);
    } else {
        a.min = Some(lo);
        a.max = Some(hi);
    }
}

#[cfg(test)]
mod legend_update_tests {
    use super::update_legend_text;

    #[test]
    fn keeps_custom_lines_drops_gone_plots_and_adds_new_ones() {
        let auto = "\\l(1) %(1)\n\\l(2) %(2)\n\\l(3) %(3)";
        // Plot 4 is gone, plot 3 is new, plot 1 keeps its custom text.
        let custom = "\\l(1) T = 5 K\n\\l(4) %(4)\nnote\n\\l(2) %(2)";
        assert_eq!(update_legend_text(custom, auto), "\\l(1) T = 5 K\nnote\n\\l(2) %(2)\n\\l(3) %(3)");
    }
}
