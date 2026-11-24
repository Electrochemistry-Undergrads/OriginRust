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

/// Where the analysis lock of each layer goes (T186): beside the layer's icon, or beside the
/// locked plot's first or last drawn point, as the page's Lock Position says. `None` for a layer
/// with no lock or when the point is not drawn.
pub(crate) fn analysis_lock_places(
    project: &sheaf_core::Project,
    graph: sheaf_core::WindowId,
    geometry: &[sheaf_plot::LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    image_rect: Rect,
) -> Vec<(usize, Pos2, sheaf_core::analysis_link::PlotLock)> {
    use sheaf_core::graph::LockPosition;
    let Some(g) = project.graph(graph) else { return Vec::new() };
    let mut out = Vec::new();
    for (li, layer) in g.layers.iter().enumerate().filter(|(_, l)| !l.display.hidden) {
        let Some(lock) = project.layer_analysis_lock(layer) else { continue };
        let at = match g.page.style.lock_position {
            // Just below the layer's icon, clear of it so that the icon keeps its clicks (top left of
            // the page when icons are hidden).
            LockPosition::LayerIcon => {
                Some(image_rect.min + egui::vec2(2.0 + li as f32 * (ICON + 2.0) + ICON / 2.0, 2.0 + ICON + 9.0))
            }
            position => {
                let rows = layer.plots.get(lock.plot).and_then(|p| project.numeric(p.y)).map_or(0, |v| v.len());
                let point = |row| {
                    reader::screen_point(
                        project,
                        graph,
                        geometry,
                        to_screen,
                        reader::Point { layer: li, plot: lock.plot, row },
                    )
                };
                let found = if position == LockPosition::FirstPoint {
                    (0..rows).find_map(point)
                } else {
                    (0..rows).rev().find_map(point)
                };
                found.map(|p| p + egui::vec2(-8.0, -12.0))
            }
        };
        if let Some(at) = at {
            out.push((li, at, lock));
        }
    }
    out
}

/// Draws the analysis locks (T186): a padlock, green while the results are current, yellow while a
/// Manual result waits for Recalculate; its hover text says which.
fn draw_analysis_locks(
    ui: &egui::Ui,
    actions: &mut Vec<Action>,
    painter: &egui::Painter,
    project: &sheaf_core::Project,
    graph: sheaf_core::WindowId,
    geometry: &[sheaf_plot::LayerGeometry],
    to_screen: &dyn Fn((f64, f64)) -> Pos2,
    image_rect: Rect,
) {
    use sheaf_core::column::RecalcMode;
    for (li, at, lock) in analysis_lock_places(project, graph, geometry, to_screen, image_rect) {
        let fill = if lock.pending { Color32::from_rgb(235, 190, 0) } else { Color32::from_rgb(0, 160, 0) };
        let body = Rect::from_center_size(at + egui::vec2(0.0, 2.0), egui::vec2(10.0, 8.0));
        let shackle: Vec<Pos2> = (0..=12)
            .map(|k| {
                let a = std::f32::consts::PI * k as f32 / 12.0;
                Pos2::new(at.x + 3.0 * a.cos(), body.top() - 3.5 * a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(shackle, Stroke::new(1.5_f32, Color32::from_gray(60))));
        painter.rect_filled(body, 1.0, fill);
        painter.rect_stroke(body, 1.0, Stroke::new(1.0_f32, Color32::from_gray(60)), egui::StrokeKind::Inside);
        let mode = if lock.mode == RecalcMode::Auto { "Auto" } else { "Manual" };
        let state = if lock.pending { "needs Recalculate" } else { "up to date" };
        let id = ui.id().with(("analysis_lock", graph.0, li));
        let resp = ui
            .interact(body.expand(3.0), id, egui::Sense::click())
            .on_hover_text(format!("Analysis of plot {} (Recalculate {mode}): {state}", lock.plot + 1));
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Analysis lock"));
        // A click opens the lock's menu (T195).
        egui::Popup::menu(&resp).show(|ui| lock_menu(ui, project, &lock, actions));
    }
}

/// The menu of an analysis lock (T195): Recalculate runs the link again on its result columns (one
/// undo step, as the column menu's Recalculate); Go to Results shows the sheet of its first result.
fn lock_menu(
    ui: &mut egui::Ui,
    project: &sheaf_core::Project,
    lock: &sheaf_core::analysis_link::PlotLock,
    actions: &mut Vec<Action>,
) {
    let Some(link) = project.analysis_links.get(lock.link) else { return };
    if ui.button("Recalculate").clicked() {
        actions.push(Action::Recalculate(link.outputs.clone()));
        ui.close();
    }
    let place = link.outputs.first().and_then(|id| {
        let (book, sheet, _) = project.locate(*id)?;
        Some((book.id, book.sheets.iter().position(|s| std::ptr::eq(s, sheet))?))
    });
    if ui.add_enabled(place.is_some(), egui::Button::new("Go to Results")).clicked()
        && let Some((book, sheet)) = place
    {
        actions.push(Action::Focus(book));
        actions.push(Action::SelectSheet { book, sheet });
        ui.close();
    }
}

/// The outline of an object being drawn, in screen points; `to_screen` takes frame fractions.
fn draw_preview(painter: &egui::Painter, s: &Shape, to_screen: &dyn Fn((f64, f64)) -> Pos2) {
    let stroke = Stroke::new(1.0_f32, Color32::from_rgb(0, 90, 200));
    let (a, b) = (to_screen((s.x1, s.y1)), to_screen((s.x2, s.y2)));
    match s.kind {
        ShapeKind::Line | ShapeKind::Arrow => {
            painter.line_segment([a, b], stroke);
        }
        ShapeKind::Rectangle | ShapeKind::Image => {
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

/// Seconds between two intervals when a Time Series Explorer plays (T416).
const EXPLORER_STEP_SECS: f64 = 1.0;

#[derive(Default)]
pub struct GraphUi {
    pub texture: Option<GraphTexture>,
    pub error: Option<String>,
    /// The active layer: menu commands such as Rescale and Plot Setup act on it.
    pub active_layer: usize,
    /// The selected object, with its layer. With more than one drawn object or text label selected
    /// (Shift+click, T159) this is the last-selected one: the Object Edit toolbar's reference.
    pub selected: Option<(usize, ObjectKind)>,
    /// The other selected drawn objects and text labels, in the order they were selected (T159).
    /// Only meaningful while `selected` is a drawn object or text label itself.
    pub also: Vec<(usize, ObjectKind)>,
    /// How far (page points) a drag of several selected objects has moved them so far (T162).
    multi_drag: Option<(f64, f64)>,
    /// What the open right-click menu is about.
    menu_on: Option<MenuTarget>,
    /// The page point (in points) that opened the right-click menu: Axis Center needs it.
    menu_page: (f64, f64),
    /// The page point (in points) clicked last, with either button: the Edit menu's Paste puts a
    /// copied object there (T149).
    pub last_click: Option<(f64, f64)>,
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
    /// Time Series Explorer (T416): playing, with the time (seconds) of the last step.
    pub explorer_play: Option<f64>,
    /// The window rectangle and the page size (points) of the last frame, for the view commands.
    frame: Option<(Rect, Vec2)>,
    /// What the Data menu tool is dragging.
    data: data_tools::State,
    /// An object being drawn.
    drawing: Option<Drawing>,
    /// A drawn object being moved.
    shape_drag: Option<ShapeDrag>,
    resize_drag: Option<ResizeDrag>,
    /// Escape consumes the rest of a selected-shape pointer gesture.
    cancelled_shape_gesture: bool,
    /// A label being dragged.
    label_drag: Option<LabelDrag>,
    /// A legend being dragged.
    legend_drag: Option<LegendDrag>,
    /// An axis title being dragged (T335).
    title_drag: Option<TitleDrag>,
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

/// Handle coordinates identify which stored x and y each corner changes.
#[derive(Clone)]
struct ResizeDrag {
    layer: usize,
    shape: usize,
    original: Shape,
    handle: (bool, bool),
    delta: (f64, f64),
}

fn shape_handles(s: &Shape) -> Vec<((bool, bool), (f64, f64))> {
    let mut handles = vec![((false, false), (s.x1, s.y1)), ((true, true), (s.x2, s.y2))];
    if !s.kind.is_linear() {
        handles.extend([((false, true), (s.x1, s.y2)), ((true, false), (s.x2, s.y1))]);
    }
    handles
}

fn handle_hit(s: &Shape, pos: Pos2, to_screen: &dyn Fn((f64, f64)) -> Pos2) -> Option<(bool, bool)> {
    shape_handles(s).into_iter().find(|(_, p)| to_screen(*p).distance(pos) <= 7.0).map(|(handle, _)| handle)
}

fn resized(d: &ResizeDrag, frame: (f64, f64)) -> Shape {
    let mut s = d.original.clone();
    let (dx, dy) = (d.delta.0 / frame.0, d.delta.1 / frame.1);
    if d.handle.0 {
        s.x2 += dx;
    } else {
        s.x1 += dx;
    }
    if d.handle.1 {
        s.y2 += dy;
    } else {
        s.y1 += dy;
    }
    s
}

// Screen coordinates are f32; sub-thousandth-point residue can remain after dragging exactly
// onto the opposite handle. Reject only that visually indistinguishable collapsed geometry.
fn valid_resize(s: &Shape, frame_screen: (f64, f64)) -> bool {
    let (dx, dy) = ((s.x1 - s.x2).abs() * frame_screen.0, (s.y1 - s.y2).abs() * frame_screen.1);
    const EPSILON: f64 = 0.001;
    [s.x1, s.y1, s.x2, s.y2, dx, dy].iter().all(|v| v.is_finite())
        && if s.kind.is_linear() { dx.hypot(dy) > EPSILON } else { dx > EPSILON && dy > EPSILON }
}

/// A legend being moved with the mouse: its centre when the drag began (fractions of the layer
/// frame) and how far the pointer has moved since (page points).
#[derive(Clone, Copy)]
struct LegendDrag {
    layer: usize,
    from: (f64, f64),
    delta: (f64, f64),
}

/// An axis title being moved with the mouse: the layer, whether it is the Y title, and how far the
/// pointer has moved since the drag began (page points).
#[derive(Clone, Copy)]
struct TitleDrag {
    layer: usize,
    y: bool,
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

    /// The toolbar zoom keeps the view centre's page point centred where possible.
    pub fn set_zoom_percent(&mut self, percent: u32) {
        let factor = (percent as f32 / 100.0).clamp(graph_zoom::MIN_ZOOM, graph_zoom::MAX_ZOOM);
        self.zoom = self.frame.map_or(Zoom { factor, ..Zoom::default() }, |(window, page)| {
            graph_zoom::zoom_about(window, page, self.zoom, factor, window.center())
        });
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
        self.also.clear();
        self.multi_drag = None;
        self.menu_on = None;
        self.shape_drag = None;
        self.label_drag = None;
        self.legend_drag = None;
        self.title_drag = None;
    }

    /// Layers with an index-based shape selection, context menu or drag.
    pub fn shape_interaction_layers(&self) -> Vec<usize> {
        let mut layers = Vec::new();
        if let Some((layer, ObjectKind::Shape(_))) = self.selected {
            layers.push(layer);
        }
        for (layer, kind) in &self.also {
            if matches!(kind, ObjectKind::Shape(_)) && !layers.contains(layer) {
                layers.push(*layer);
            }
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
        if let Some(drag) = &self.resize_drag
            && !layers.contains(&drag.layer)
        {
            layers.push(drag.layer);
        }
        layers
    }

    /// Discard index-based shape interactions after a layer's shape list changes.
    pub fn invalidate_shape_interaction(&mut self, layer: usize, keep_selection: bool) {
        // The other selected objects name shapes by index too; a changed list ends the multi-selection.
        self.also.retain(|(l, kind)| *l != layer || !matches!(kind, ObjectKind::Shape(_)));
        self.multi_drag = None;
        if !keep_selection && matches!(self.selected, Some((l, ObjectKind::Shape(_))) if l == layer) {
            self.selected = None;
        }
        if matches!(self.menu_on, Some(MenuTarget::Hit(Hit::Object { layer: l, kind: ObjectKind::Shape(_) })) if l == layer)
        {
            self.menu_on = None;
        }
        if self.resize_drag.as_ref().is_some_and(|drag| drag.layer == layer) {
            self.resize_drag = None;
        }
        if self.shape_drag.is_some_and(|drag| drag.layer == layer) {
            self.shape_drag = None;
        }
    }

    /// What the Delete key does to the selected object: a legend, an axis title or a text label
    /// goes; other objects have no Delete in Origin's menus either.
    pub fn delete_selected(&mut self) -> Option<GraphOp> {
        let (layer, kind) = self.selected?;
        self.also.clear();
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

    /// The selected drawn objects and text labels in the order they were selected, the reference
    /// (last-selected) last; empty when the selection is something else (T159).
    pub fn multi_selection(&self) -> Vec<(usize, ObjectKind)> {
        match self.selected {
            Some(sel @ (_, ObjectKind::Shape(_) | ObjectKind::Label(_))) => {
                let mut all: Vec<_> = self.also.iter().copied().filter(|o| *o != sel).collect();
                all.push(sel);
                all
            }
            _ => Vec::new(),
        }
    }

    /// Shift+click on a drawn object or text label (T159): adds it to the selection as the new
    /// reference, or takes it out when it is already selected. `members` is the clicked object with
    /// the rest of its group before it (T162): the whole group goes in or out. Other objects select
    /// alone. (Takes the two fields so the caller can hold the picture borrowed.)
    fn toggle_in_selection(
        selected: &mut Option<(usize, ObjectKind)>,
        also: &mut Vec<(usize, ObjectKind)>,
        members: Vec<(usize, ObjectKind)>,
    ) {
        let Some(&hit) = members.last() else { return };
        if !matches!(hit.1, ObjectKind::Shape(_) | ObjectKind::Label(_)) {
            also.clear();
            *selected = Some(hit);
            return;
        }
        let mut all = match *selected {
            Some(sel @ (_, ObjectKind::Shape(_) | ObjectKind::Label(_))) => {
                let mut all: Vec<_> = also.iter().copied().filter(|o| *o != sel).collect();
                all.push(sel);
                all
            }
            _ => Vec::new(),
        };
        let was_in = all.contains(&hit);
        all.retain(|o| !members.contains(o));
        if !was_in {
            all.extend(members);
        }
        *selected = all.pop();
        *also = all;
    }

    /// Selects `members` (the reference last), or nothing when it is empty.
    fn select_all(
        selected: &mut Option<(usize, ObjectKind)>,
        also: &mut Vec<(usize, ObjectKind)>,
        mut members: Vec<(usize, ObjectKind)>,
    ) {
        *selected = members.pop();
        *also = members;
    }

    /// The selected objects as drawn in the last frame, for the Object Edit Align and Make Same
    /// Width/Height buttons: (reference, the others). `None` with fewer than two objects drawn.
    pub fn arrange_members(
        &self,
        project: &Project,
        graph: WindowId,
    ) -> Option<(crate::obj_align::Member, Vec<crate::obj_align::Member>)> {
        use crate::obj_align::{Member, Target};
        let t = self.texture.as_ref()?;
        let g = project.graph(graph)?;
        let mut members: Vec<Member> = self
            .multi_selection()
            .into_iter()
            .filter_map(|(layer, kind)| {
                let target = match kind {
                    ObjectKind::Shape(i) => Target::Shape(i),
                    ObjectKind::Label(i) => Target::Label(i),
                    _ => return None,
                };
                let l = g.layers.get(layer)?;
                let f = t.geometry.get(layer)?.frame;
                let extent = match target {
                    Target::Shape(i) => {
                        let s = l.shapes.get(i)?;
                        Some(((s.x2 - s.x1).abs() * f.w, (s.y2 - s.y1).abs() * f.h))
                    }
                    Target::Label(i) => {
                        l.labels.get(i)?;
                        None
                    }
                };
                let bounds = t
                    .scene
                    .layers
                    .iter()
                    .flat_map(|sl| sl.objects.iter())
                    .filter(|o| o.layer == layer && o.kind == kind)
                    .map(|o| (o.bounds.x, o.bounds.y, o.bounds.x + o.bounds.w, o.bounds.y + o.bounds.h))
                    .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))?;
                Some(Member {
                    layer,
                    target,
                    bounds: (bounds.0, bounds.1, bounds.2 - bounds.0, bounds.3 - bounds.1),
                    frame: (f.x, f.y, f.w, f.h),
                    extent,
                })
            })
            .collect();
        if members.len() < 2 {
            return None;
        }
        let reference = members.pop()?;
        Some((reference, members))
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
    // Browser Graph (T410): the bar that steps the browsed plot through its datasets.
    if let Some(b) = &graph.browser {
        ui.horizontal(|ui| {
            let n = b.items.len();
            if ui.add_enabled(b.current > 0, egui::Button::new("Previous")).clicked() {
                actions.push(Action::BrowseGraph { graph: graph_id, to: b.current - 1 });
            }
            if ui.add_enabled(b.current + 1 < n, egui::Button::new("Next")).clicked() {
                actions.push(Action::BrowseGraph { graph: graph_id, to: b.current + 1 });
            }
            ui.label(b.caption(doc.project()));
        });
    }
    // Time Series Explorer (T416): the bar that steps the zoom band through the intervals, or plays them.
    if let Some(e) = graph.explorer {
        let now = ui.input(|i| i.time);
        let last = e.current + 1 >= e.intervals;
        ui.horizontal(|ui| {
            if ui.add_enabled(e.current > 0, egui::Button::new("Previous")).clicked() {
                actions.push(Action::ExploreInterval { graph: graph_id, to: e.current - 1 });
            }
            if ui.add_enabled(!last, egui::Button::new("Next")).clicked() {
                actions.push(Action::ExploreInterval { graph: graph_id, to: e.current + 1 });
            }
            if state.explorer_play.is_some() {
                if ui.button("Pause").clicked() {
                    state.explorer_play = None;
                }
            } else if ui.button("Play").clicked() {
                // Play from the last interval starts again at the first.
                if last {
                    actions.push(Action::ExploreInterval { graph: graph_id, to: 0 });
                }
                state.explorer_play = Some(now);
            }
            ui.label(e.caption());
            ui.label("Intervals");
            let mut n = e.intervals;
            if ui.add(egui::DragValue::new(&mut n).range(2..=1000)).changed() && n != e.intervals {
                actions.push(Action::ExplorerIntervals { graph: graph_id, intervals: n });
            }
        });
        if let Some(t) = state.explorer_play {
            if now - t >= EXPLORER_STEP_SECS {
                if last {
                    state.explorer_play = None;
                } else {
                    actions.push(Action::ExploreInterval { graph: graph_id, to: e.current + 1 });
                    state.explorer_play = Some(now);
                }
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(EXPLORER_STEP_SECS));
        }
    } else {
        state.explorer_play = None;
    }
    // Set Active Layer By Layer Icon Only (T183): what a click inside the page activates is undone below.
    let icon_only = graph.page.style.activate_by_icon_only;
    let icon_plot_setup = graph.page.style.icon_double_click_plot_setup;
    let (active_before, actions_before) = (state.active_layer, actions.len());

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
    let to_screen = |(x, y): (f64, f64)| image_rect.min + egui::vec2(x as f32 * fit, y as f32 * fit);
    painter.rect_filled(image_rect.translate(egui::vec2(3.0, 3.0)), 0.0, Color32::from_black_alpha(40));
    if let Some(t) = &state.texture {
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(t.texture.id(), image_rect, uv, Color32::WHITE);
        if graph.page.style.active_layer_indicator
            && let Some(geometry) = t.geometry.get(state.active_layer)
        {
            view_modes::draw_active_axes(
                &painter,
                &t.scene,
                state.active_layer,
                geometry,
                &to_screen,
                ui.visuals().selection.bg_fill,
            );
        }
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
    // (a straight line that is not Selectable has no hit object, so clicks pass through it)
    let pick = |p: Pos2| hit_test(&t.scene, to_page(p), f64::from(PICK_TOLERANCE / fit));

    // The selection, drawn over the picture.
    if let Some((layer, kind)) = state.selected {
        let moved = state.label_drag.filter(|d| kind == ObjectKind::Label(d.label) && layer == d.layer);
        let moved_legend = state.legend_drag.filter(|d| kind == ObjectKind::Legend && layer == d.layer);
        let moved_shape = state.shape_drag.filter(|d| kind == ObjectKind::Shape(d.shape) && layer == d.layer);
        let title_kind = |y: bool| if y { ObjectKind::YTitle } else { ObjectKind::XTitle };
        let moved_title = state.title_drag.filter(|d| kind == title_kind(d.y) && layer == d.layer);
        let shift = moved
            .map(|d| d.delta)
            .or(moved_legend.map(|d| d.delta))
            .or(moved_title.map(|d| d.delta))
            .or(moved_shape.map(|d| d.delta))
            .or(state.multi_drag)
            .unwrap_or((0.0, 0.0));
        if let ObjectKind::Shape(index) = kind {
            if let Some(original) = graph.layers.get(layer).and_then(|l| l.shapes.get(index))
                && let Some(f) = t.geometry.get(layer).map(|g| g.frame)
            {
                let s = state
                    .resize_drag
                    .as_ref()
                    .filter(|d| d.layer == layer && d.shape == index)
                    .map(|d| resized(d, (f.w, f.h)))
                    .unwrap_or_else(|| original.clone());
                let point = |(x, y): (f64, f64)| to_screen((f.x + x * f.w + shift.0, f.y + y * f.h + shift.1));
                if state.resize_drag.is_some() {
                    draw_preview(&painter, &s, &point);
                }
                for (_, p) in shape_handles(&s) {
                    let r = Rect::from_center_size(point(p), Vec2::splat(8.0));
                    painter.rect_filled(r, 0.0, Color32::from_rgb(0, 170, 0));
                    painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::BLACK), egui::StrokeKind::Inside);
                }
            }
        } else {
            draw_selection(&painter, &t.scene, layer, kind, &|(x, y)| to_screen((x + shift.0, y + shift.1)));
        }
    }
    // The other objects of a Shift+click selection (T159) or a group (T162), moved with a drag of them all.
    let shift = state.multi_drag.unwrap_or((0.0, 0.0));
    for (layer, kind) in state.multi_selection().iter().rev().skip(1) {
        draw_selection(&painter, &t.scene, *layer, *kind, &|(x, y)| to_screen((x + shift.0, y + shift.1)));
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

    // Analysis locks (T186): on screen only, where Data > Lock Position puts them.
    draw_analysis_locks(ui, actions, &painter, doc.project(), graph_id, &t.geometry, &to_screen, image_rect);

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
                        state.also.clear();
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

    // Escape can arrive on the same frame as the first drag movement. Consume that gesture
    // through release so it cannot start a resize or a move after cancellation.
    if ui.input(|i| i.key_pressed(egui::Key::Escape) && i.pointer.primary_down())
        && matches!(state.selected, Some((_, ObjectKind::Shape(_))))
    {
        state.cancelled_shape_gesture = true;
    }
    // Selected geometric handles have priority over picking any object beneath them.
    if state.tool() != GraphTool::Pointer
        || state.gadget.is_some()
        || state.menu_on.is_some()
        || ui.input(|i| i.key_pressed(egui::Key::Escape))
    {
        state.resize_drag = None;
    }
    if response.drag_started()
        && !state.cancelled_shape_gesture
        && state.tool() == GraphTool::Pointer
        && state.gadget.is_none()
        && state.menu_on.is_none()
        && let Some((layer, ObjectKind::Shape(index))) = state.selected
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Some(s) = graph.layers.get(layer).and_then(|l| l.shapes.get(index))
        && let Some(f) = t.geometry.get(layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0)
        && let Some(handle) = handle_hit(s, pos, &|(x, y)| to_screen((f.x + x * f.w, f.y + y * f.h)))
    {
        state.resize_drag = Some(ResizeDrag { layer, shape: index, original: s.clone(), handle, delta: (0.0, 0.0) });
    }
    if let Some(mut d) = state.resize_drag.take() {
        let page = response.drag_delta() / fit;
        d.delta.0 += f64::from(page.x);
        d.delta.1 += f64::from(page.y);
        if response.drag_stopped() {
            if let Some(f) = t.geometry.get(d.layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0) {
                let after = resized(&d, (f.w, f.h));
                if after != d.original && valid_resize(&after, (f.w * f64::from(fit), f.h * f64::from(fit))) {
                    actions.push(Action::ResizeShape {
                        graph: graph_id,
                        layer: d.layer,
                        shape: d.shape,
                        before: d.original,
                        after,
                    });
                }
            }
        } else if ui.input(|i| i.pointer.primary_down()) {
            state.resize_drag = Some(d);
        }
    }

    // Dragging one of several selected objects, or a grouped one, moves them all (T162); the
    // change is made, as one undo step, when the button is let go.
    if response.drag_started()
        && state.resize_drag.is_none()
        && !state.cancelled_shape_gesture
        && !state.reader
        && !state.text_tool
        && state.shape_tool.is_none()
        && !state.slope_tool
        && state.data_tool.is_none()
        && !state.pan_tool
        && !zoom_took_input
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Hit::Object { layer, kind: kind @ (ObjectKind::Shape(_) | ObjectKind::Label(_)) } = pick(pos)
    {
        if !state.multi_selection().contains(&(layer, kind)) {
            let members = crate::obj_align::with_group(graph, layer, kind);
            if members.len() > 1 {
                GraphUi::select_all(&mut state.selected, &mut state.also, members);
            }
        }
        if state.multi_selection().len() > 1 {
            state.multi_drag = Some((0.0, 0.0));
            actions.push(Action::ActivateLayer { graph: graph_id, layer });
        }
    }
    if let Some(mut d) = state.multi_drag {
        let page = response.drag_delta() / fit;
        d = (d.0 + f64::from(page.x), d.1 + f64::from(page.y));
        state.multi_drag = Some(d);
        if response.drag_stopped() {
            state.multi_drag = None;
            let changes: Vec<_> = state
                .multi_selection()
                .into_iter()
                .filter_map(|(layer, kind)| {
                    let f = t.geometry.get(layer).map(|g| g.frame).filter(|f| f.w > 0.0 && f.h > 0.0)?;
                    Some(crate::obj_align::Change::Move {
                        layer,
                        target: crate::obj_align::Target::of(kind)?,
                        dx: d.0 / f.w,
                        dy: d.1 / f.h,
                    })
                })
                .collect();
            if (d.0 != 0.0 || d.1 != 0.0) && !changes.is_empty() {
                actions.push(Action::MoveObjects { graph: graph_id, changes });
            }
        }
    }

    // Dragging a label moves it; the change is made when the button is let go.
    if response.drag_started()
        && state.multi_drag.is_none()
        && state.resize_drag.is_none()
        && !state.cancelled_shape_gesture
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
        state.also.clear();
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
                    state.also.clear();
                    state.selected = Some((d.layer, ObjectKind::Shape(index)));
                    actions.push(Action::AddShape { graph: graph_id, layer: d.layer, shape: preview });
                }
            }
        }
    }

    // Dragging a drawn object moves it; the change is made when the button is let go.
    if response.drag_started()
        && state.resize_drag.is_none()
        && !state.cancelled_shape_gesture
        && !state.reader
        && !state.text_tool
        && state.shape_tool.is_none()
        && !state.slope_tool
        && state.data_tool.is_none()
        && !state.pan_tool
        && !zoom_took_input
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && state.multi_drag.is_none()
        && let Hit::Object { layer, kind: ObjectKind::Shape(shape) } = pick(pos)
    {
        state.also.clear();
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
        && state.resize_drag.is_none()
        && !state.cancelled_shape_gesture
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
        state.also.clear();
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

    // Dragging an axis title moves it: the move is added to the title's offset (T335), as Origin keeps
    // a dragged title where it was put.
    if response.drag_started()
        && state.resize_drag.is_none()
        && !state.cancelled_shape_gesture
        && !state.reader
        && !state.text_tool
        && state.shape_tool.is_none()
        && !state.slope_tool
        && state.data_tool.is_none()
        && !state.pan_tool
        && !zoom_took_input
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Hit::Object { layer, kind: kind @ (ObjectKind::XTitle | ObjectKind::YTitle) } = pick(pos)
    {
        state.also.clear();
        state.selected = Some((layer, kind));
        state.title_drag = Some(TitleDrag { layer, y: kind == ObjectKind::YTitle, delta: (0.0, 0.0) });
        actions.push(Action::ActivateLayer { graph: graph_id, layer });
    }
    if let Some(mut d) = state.title_drag {
        let page = response.drag_delta() / fit;
        d.delta = (d.delta.0 + f64::from(page.x), d.delta.1 + f64::from(page.y));
        state.title_drag = Some(d);
        if response.drag_stopped() {
            state.title_drag = None;
            if d.delta.0 != 0.0 || d.delta.1 != 0.0 {
                actions.push(Action::MoveAxisTitle {
                    graph: graph_id,
                    layer: d.layer,
                    y: d.y,
                    dx: d.delta.0,
                    dy: d.delta.1,
                });
            }
        }
    }

    if !ui.input(|i| i.pointer.primary_down()) {
        state.cancelled_shape_gesture = false;
    }

    // A click on a selected handle preserves selection even when the corner lies outside the shape.
    let clicked_handle = if response.clicked()
        && state.tool() == GraphTool::Pointer
        && state.gadget.is_none()
        && let Some((layer, ObjectKind::Shape(index))) = state.selected
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(s) = graph.layers.get(layer).and_then(|l| l.shapes.get(index))
        && let Some(f) = t.geometry.get(layer).map(|g| g.frame)
        && handle_hit(s, pos, &|(x, y)| to_screen((f.x + x * f.w, f.y + y * f.h))).is_some()
    {
        true
    } else {
        false
    };
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
        && !clicked_handle
        && !state.reader
        && !slope_took_input
        && !zoom_took_input
        && state.data_tool.is_none()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let hit = pick(pos);
        state.last_click = Some(to_page(pos));
        match hit {
            // Shift+click adds a drawn object or text label to the selection, or takes it out (T159).
            Hit::Object { layer, kind } if ui.input(|i| i.modifiers.shift) => GraphUi::toggle_in_selection(
                &mut state.selected,
                &mut state.also,
                crate::obj_align::with_group(graph, layer, kind),
            ),
            // A click on a grouped object selects its whole group (T162).
            Hit::Object { layer, kind } => GraphUi::select_all(
                &mut state.selected,
                &mut state.also,
                crate::obj_align::with_group(graph, layer, kind),
            ),
            _ => {
                state.also.clear();
                state.selected = None;
            }
        }
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
        let hit = if clicked_handle {
            state.selected.map_or_else(|| pick(pos), |(layer, kind)| Hit::Object { layer, kind })
        } else {
            pick(pos)
        };
        let open = |layer, tab, plot| Action::GraphProperties { graph: graph_id, layer, tab, plot };
        let action = match hit {
            Hit::Object { layer, kind } => Some(properties_of(graph_id, layer, kind)),
            Hit::Layer(layer) => Some(open(layer, PropsTab::Plots, None)),
            Hit::Page => Some(open(state.active_layer, PropsTab::Page, None)),
        };
        actions.extend(action);
    }
    if response.secondary_clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let hit = pick(pos);
        state.resize_drag = None;
        state.menu_on = Some(MenuTarget::Hit(hit));
        state.menu_page = to_page(pos);
        state.last_click = Some(state.menu_page);
        if let Some(layer) = layer_of(hit) {
            state.active_layer = layer;
            actions.push(Action::ActivateLayer { graph: graph_id, layer });
        }
        // A right click on one of several selected objects keeps them selected.
        if let Hit::Object { layer, kind } = hit
            && !state.multi_selection().contains(&(layer, kind))
        {
            state.also.clear();
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

    if icon_only {
        state.active_layer = active_before;
        let rest: Vec<Action> =
            actions.drain(actions_before..).filter(|a| !matches!(a, Action::ActivateLayer { .. })).collect();
        actions.extend(rest);
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
            if icon_plot_setup {
                actions.push(Action::PlotSetup { graph: graph_id, layer: i });
            } else {
                actions.push(Action::GraphProperties { graph: graph_id, layer: i, tab: PropsTab::Plots, plot: None });
            }
        }
        if resp.secondary_clicked() {
            state.active_layer = i;
            state.menu_on = Some(MenuTarget::LayerIcon(i));
            actions.push(Action::ActivateLayer { graph: graph_id, layer: i });
        }
        icon_menu = Some(icon_menu.map_or(resp.clone(), |r: egui::Response| r.union(resp)));
    }

    // The right-click menus; the layer icon menu ticks its switches from the page (T183).
    let mut c = graph_context(graph_id, state.active_layer, state.tool());
    if let Some(g) = doc.project().graph(graph_id) {
        c.activate_by_icon_only = g.page.style.activate_by_icon_only;
        c.icon_double_click_plot_setup = g.page.style.icon_double_click_plot_setup;
        let layer = state.active_layer;
        c.active_layer_only = (g.layers.len() >= 2 && layer < g.layers.len())
            .then(|| g.layers.iter().enumerate().all(|(i, l)| l.display.hidden == (i != layer)));
    }
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
        user_book_templates: &[],
        templates: false,
        recent_imports: &[],
        panels: [true; 4],
        ai_link: false,
        layer,
        tool,
        curve: None,
        graph_pick: None,
        graph_paste: false,
        layer_curves: Vec::new(),
        stack: None,
        can_cumulate: false,
        can_color_scale: false,
        space3d: false,
        selected_shape: None,
        multi_selected: 0,
        can_group: false,
        can_ungroup: false,
        view_mode: Default::default(),
        layer_icons: true,
        active_layer_indicator: false,
        show_frame: None,
        lock_position: Default::default(),
        page_breaks: false,
        activate_by_icon_only: false,
        icon_double_click_plot_setup: false,
        show_labels: true,
        show_data: true,
        active_layer_only: None,
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

/// What opens an object's properties: a double-click on it, or Format > Object Properties... while
/// it is selected (T150).
pub fn properties_of(graph_id: WindowId, layer: usize, kind: ObjectKind) -> Action {
    let open = |tab, plot| Action::GraphProperties { graph: graph_id, layer, tab, plot };
    match kind {
        ObjectKind::XAxis | ObjectKind::XTitle => open(PropsTab::XAxis, None),
        ObjectKind::YAxis | ObjectKind::YTitle => open(PropsTab::YAxis, None),
        ObjectKind::Legend => open(PropsTab::Legend, None),
        ObjectKind::Plot(p) => open(PropsTab::Plots, Some(p)),
        ObjectKind::Label(label) => Action::EditText { graph: graph_id, layer, label },
        ObjectKind::Shape(shape) => Action::EditShape { graph: graph_id, layer, shape },
        ObjectKind::Slope(slope) => Action::EditSlope { graph: graph_id, layer, slope },
        ObjectKind::StraightLine(line) => Action::EditStraightLine { graph: graph_id, layer, line },
        ObjectKind::Scale(_) => open(PropsTab::Plots, None),
    }
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
            ObjectKind::Shape(k) => {
                let region = graph.layers.get(layer).and_then(|l| l.shapes.get(k)).is_some_and(|s| {
                    matches!(s.kind, sheaf_core::shape::ShapeKind::Rectangle | sheaf_core::shape::ShapeKind::Ellipse)
                });
                let regions = graph.layers.get(layer).map_or(0, |l| {
                    l.shapes
                        .iter()
                        .filter(|s| {
                            matches!(
                                s.kind,
                                sheaf_core::shape::ShapeKind::Rectangle | sheaf_core::shape::ShapeKind::Ellipse
                            )
                        })
                        .count()
                });
                menus::graph_shape_menu(layer, k, region, regions)
            }
            ObjectKind::Slope(k) => menus::graph_slope_menu(layer, k),
            ObjectKind::StraightLine(k) => menus::graph_straight_line_menu(layer, k),
            ObjectKind::Scale(k) => menus::graph_scale_menu(layer, k),
        },
        MenuTarget::Hit(Hit::Layer(layer)) => menus::graph_page_menu(layer, page),
        MenuTarget::Hit(Hit::Page) => menus::graph_page_menu(0, page),
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
        | GraphOp::OrderLegend { layer, .. }
        | GraphOp::OrderTitle { layer, .. }
        | GraphOp::LegendBehindData { layer, .. }
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
        | GraphOp::OrderLabel { layer, .. }
        | GraphOp::HideLayer { layer }
        | GraphOp::HideOtherLayers { layer }
        | GraphOp::DataMarkers { layer, .. }
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
            let range = |axis: &sheaf_plot::ticks::ResolvedAxis| {
                let (from, to) = axis.endpoints();
                (from, to, axis.step)
            };
            let ranges: Vec<_> = r.iter().map(|l| (range(&l.x), range(&l.y))).collect();
            g.freeze_linked_ranges(layer, &ranges);
        }
        let _ = g.delete_layer(layer);
        return;
    }
    if let GraphOp::DataMarkers { layer, op } = op {
        use crate::actions::MarkerOp;
        match op {
            MarkerOp::Mark { plot } => {
                project.mark_data_range(graph, layer, plot);
            }
            MarkerOp::Move { plot, a, b } => {
                if let Some(g) = project.graph_mut(graph) {
                    g.set_data_markers(layer, plot, a, b);
                }
            }
            MarkerOp::Clear => {
                project.graph_mut(graph).map(|g| g.clear_data_markers());
            }
            MarkerOp::SetRange => {
                project.graph_mut(graph).map(|g| g.set_display_range());
            }
            MarkerOp::ResetRange => {
                project.graph_mut(graph).map(|g| g.reset_full_range());
            }
            MarkerOp::Show(show) => {
                if let Some(g) = project.graph_mut(graph) {
                    g.set_marker_display(Some(show), None);
                }
            }
            MarkerOp::Size(size) => {
                if let Some(g) = project.graph_mut(graph) {
                    g.set_marker_display(None, Some(size));
                }
            }
        }
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
        GraphOp::OrderLegend { front, .. } => l.legend.on_top = front,
        GraphOp::OrderTitle { y, front, .. } => {
            if y {
                l.y_axis.title_on_top = front;
            } else {
                l.x_axis.title_on_top = front;
            }
        }
        GraphOp::LegendBehindData { behind, .. } => l.legend.behind_data = behind,
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
        GraphOp::OrderLabel { label, front, .. } => {
            let len = l.labels.len();
            if label < len && ((front && label + 1 < len) || (!front && label > 0)) {
                let value = l.labels.remove(label);
                if front {
                    l.labels.push(value);
                } else {
                    l.labels.insert(0, value);
                }
            }
        }
        GraphOp::HideLayer { .. } => l.display.hidden = !l.display.hidden,
        GraphOp::HideOtherLayers { .. }
        | GraphOp::DataMarkers { .. }
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

#[cfg(test)]
mod resize_tests {
    use super::*;

    #[test]
    fn all_corner_identities_and_degenerate_geometry() {
        let original = Shape::new(ShapeKind::Rectangle, (0.8, 0.7), (0.2, 0.3));
        for (handle, _) in shape_handles(&original) {
            let drag = ResizeDrag { layer: 0, shape: 0, original: original.clone(), handle, delta: (10.0, -20.0) };
            let s = resized(&drag, (100.0, 200.0));
            assert!((if handle.0 { s.x2 - original.x2 } else { s.x1 - original.x1 } - 0.1).abs() < 1e-10);
            assert!((if handle.1 { s.y2 - original.y2 } else { s.y1 - original.y1 } + 0.1).abs() < 1e-10);
            assert_eq!(if handle.0 { s.x1 } else { s.x2 }, if handle.0 { original.x1 } else { original.x2 });
            assert_eq!(if handle.1 { s.y1 } else { s.y2 }, if handle.1 { original.y1 } else { original.y2 });
        }
        let mut s = original;
        s.x1 = s.x2;
        assert!(!valid_resize(&s, (100.0, 200.0)));
        s.kind = ShapeKind::Line;
        assert!(valid_resize(&s, (100.0, 200.0)));
        s.y1 = s.y2;
        assert!(!valid_resize(&s, (100.0, 200.0)));
        s.x1 = s.x2 + 0.00002; // 0.002 screen points remains a valid, very short line.
        assert!(valid_resize(&s, (100.0, 200.0)));
        s.x1 = f64::NAN;
        assert!(!valid_resize(&s, (100.0, 200.0)));
    }
}
