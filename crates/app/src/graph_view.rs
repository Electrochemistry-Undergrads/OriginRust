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

use crate::actions::{Action, GraphOp};
use crate::dialogs::graph_props::PropsTab;
use crate::gadget::Integrator;
use crate::menus::{self, MenuContext, WindowKind};
use crate::reader;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use sheaf_core::{Document, Project, WindowId};
use sheaf_plot::LayerGeometry;
use sheaf_plot::hit::{Hit, hit_test};
use sheaf_plot::scene::{Item, ItemList, ObjectKind, Scene};

/// The grey round the page (Origin shows the part of the window the page does not fill in grey).
const GROUND: Color32 = Color32::from_gray(192);
/// How far from a line, in screen points, a click still picks it.
const PICK_TOLERANCE: f32 = 3.0;
const ICON: f32 = 15.0;

/// The cached image of one graph.
pub struct GraphTexture {
    revision: u64,
    /// Raster pixels per page point, quantised, so tiny size changes do not re-render.
    scale_key: u32,
    texture: TextureHandle,
    geometry: Vec<LayerGeometry>,
    scene: Scene,
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
}

impl GraphUi {
    /// What the Delete key does to the selected object: a legend, an axis title or a text label
    /// goes; other objects have no Delete in Origin's menus either.
    pub fn delete_selected(&mut self) -> Option<GraphOp> {
        let (layer, kind) = self.selected?;
        let op = match kind {
            ObjectKind::Legend => GraphOp::HideLegend { layer },
            ObjectKind::XTitle => GraphOp::HideTitle { layer, y: false },
            ObjectKind::YTitle => GraphOp::HideTitle { layer, y: true },
            ObjectKind::Label(label) => GraphOp::DeleteLabel { layer, label },
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

    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click());
    ui.painter().rect_filled(rect, 0.0, GROUND);
    let margin = 12.0;
    let page_w = graph.page.width_pt as f32;
    let page_h = graph.page.height_pt as f32;
    if page_w <= 0.0 || page_h <= 0.0 || rect.width() <= 2.0 * margin || rect.height() <= 2.0 * margin {
        return;
    }
    // UI points per page point, so that the whole page fits in the window ("fit page to window").
    let fit = ((rect.width() - 2.0 * margin) / page_w).min((rect.height() - 2.0 * margin) / page_h).max(0.05);
    let image_rect = Rect::from_center_size(rect.center(), egui::vec2(page_w * fit, page_h * fit));
    state.view = Some((image_rect, fit));

    // Raster pixels per page point.
    let pixel_scale = fit * ui.ctx().pixels_per_point();
    let scale_key = (pixel_scale * 50.0).round() as u32;
    let stale = state.texture.as_ref().is_none_or(|t| t.revision != doc.revision() || t.scale_key != scale_key);
    if stale {
        let scale = f64::from(scale_key) / 50.0;
        let drawn = sheaf_plot::graph_scene(doc.project(), graph_id).and_then(|scene| {
            let raster = sheaf_plot::render_scene_raster(&scene, scale)?;
            Ok((scene, raster, sheaf_plot::layout_geometry(doc.project(), graph_id)?))
        });
        match drawn {
            Ok((scene, raster, geometry)) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [raster.width as usize, raster.height as usize],
                    &raster.rgba,
                );
                let texture = ui.ctx().load_texture(format!("graph-{}", graph_id.0), image, TextureOptions::LINEAR);
                state.texture = Some(GraphTexture { revision: doc.revision(), scale_key, texture, geometry, scene });
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
    let pick = |p: Pos2| hit_test(&t.scene, to_page(p), f64::from(PICK_TOLERANCE / fit));

    // The selection, drawn over the picture.
    if let Some((layer, kind)) = state.selected {
        draw_selection(&painter, &t.scene, layer, kind, &to_screen);
    }

    // Data read-out under the pointer, in the layer under it.
    if let Some(pos) = response.hover_pos() {
        let (px, py) = to_page(pos);
        if let Some(g) = t.geometry.iter().rev().find(|g| {
            let f = &g.frame;
            px >= f.x && px <= f.x + f.w && py >= f.y && py <= f.y + f.h
        }) {
            let (x, y) = g.page_to_data(px, py);
            painter.text(
                rect.left_bottom() + egui::vec2(8.0, -6.0),
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

    // Clicks on the page.
    let layer_of = |hit: Hit| match hit {
        Hit::Object { layer, .. } | Hit::Layer(layer) => Some(layer),
        Hit::Page => None,
    };
    if response.clicked()
        && !state.reader
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
                ObjectKind::Label(_) => None,
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
        let offset = sheaf_plot::stack_offsets(doc.project(), graph_id, gadget.layer)
            .ok()
            .and_then(|o| o.get(gadget.plot).copied())
            .unwrap_or((0.0, 0.0));
        let c = graph_context(graph_id, state.active_layer, state.reader);
        state.gadget_rect = gadget.show(ui, doc.project(), graph_id, geom, fit, &to_screen, offset, &c, actions);
        state.gadget = Some(gadget);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.gadget = None;
        }
    }

    // Layer icons: after the page, so that they get the clicks on them.
    let mut icon_menu = None;
    for i in 0..layers {
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
    let c = graph_context(graph_id, state.active_layer, state.reader);
    let entries = |target: MenuTarget| menu_for(doc.project(), graph_id, target, state.selected);
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

/// The menu context of a graph window's own menus.
fn graph_context(graph_id: WindowId, layer: usize, reader: bool) -> MenuContext<'static> {
    MenuContext {
        kind: WindowKind::Graph,
        book: None,
        graph: Some(graph_id),
        active: Some(graph_id),
        selected: &[],
        cells: false,
        undo: None,
        redo: None,
        windows: Vec::new(),
        open: None,
        panels: [true; 4],
        layer,
        reader,
    }
}

/// The entries of the right-click menu for what was clicked.
fn menu_for(
    project: &Project,
    graph_id: WindowId,
    target: MenuTarget,
    selected: Option<(usize, ObjectKind)>,
) -> Vec<menus::Entry> {
    let Some(graph) = project.graph(graph_id) else {
        return Vec::new();
    };
    match target {
        MenuTarget::LayerIcon(layer) => {
            let plots: Vec<String> = graph
                .layers
                .get(layer)
                .map(|l| l.plots.iter().map(|p| project.plot_label(p)).collect())
                .unwrap_or_default();
            let active = match selected {
                Some((l, ObjectKind::Plot(p))) if l == layer => Some(p),
                _ => None,
            };
            menus::layer_icon_menu(layer, graph.layers.len(), &plots, active)
        }
        MenuTarget::Hit(Hit::Object { layer, kind }) => match kind {
            ObjectKind::Plot(p) => {
                let Some(plot) = graph.layers.get(layer).and_then(|l| l.plots.get(p)) else {
                    return Vec::new();
                };
                let book = project.locate(plot.y).map(|(b, _, _)| (b.id, b.name.clone()));
                menus::graph_plot_menu(layer, p, plot.kind, book)
            }
            ObjectKind::XAxis | ObjectKind::YAxis => {
                let y = kind == ObjectKind::YAxis;
                let grids = graph.layers.get(layer).is_some_and(|l| {
                    let a = if y { &l.y_axis } else { &l.x_axis };
                    a.major_grid.is_some() || a.minor_grid.is_some()
                });
                menus::graph_axis_menu(layer, y, grids)
            }
            ObjectKind::XTitle => menus::graph_title_menu(layer, false),
            ObjectKind::YTitle => menus::graph_title_menu(layer, true),
            ObjectKind::Legend => menus::graph_legend_menu(layer),
            ObjectKind::Label(k) => menus::graph_label_menu(layer, k),
        },
        MenuTarget::Hit(Hit::Layer(layer)) => menus::graph_page_menu(layer),
        MenuTarget::Hit(Hit::Page) => menus::graph_page_menu(0),
    }
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

/// Applies a change made from the graph window to the project. `resolved` is the present layout
/// of the graph (its axes' ranges), used by the axis zoom.
pub fn apply_op(project: &mut Project, graph: WindowId, op: GraphOp, resolved: Option<&[LayerGeometry]>) {
    let Some(g) = project.graph_mut(graph) else { return };
    let layers = g.layers.len();
    let layer_index = match op {
        GraphOp::SetPlotKind { layer, .. }
        | GraphOp::HidePlot { layer, .. }
        | GraphOp::HideAxis { layer, .. }
        | GraphOp::ToggleGrids { layer, .. }
        | GraphOp::Rescale { layer }
        | GraphOp::AxisZoom { layer, .. }
        | GraphOp::HideLegend { layer }
        | GraphOp::HideTitle { layer, .. }
        | GraphOp::DeleteLabel { layer, .. }
        | GraphOp::HideLayer { layer }
        | GraphOp::HideOtherLayers { layer } => layer,
    };
    if layer_index >= layers {
        return;
    }
    if let GraphOp::HideOtherLayers { layer } = op {
        for (i, l) in g.layers.iter_mut().enumerate() {
            if i != layer {
                l.plots.iter_mut().for_each(|p| p.visible = false);
            }
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
        GraphOp::AxisZoom { y, zoom_in, .. } => {
            let Some(r) = resolved.and_then(|r| r.get(layer_index)) else { return };
            let ra = if y { &r.y } else { &r.x };
            // Zoom about the middle in the axis' own scale: a log axis zooms in decades.
            let (u0, u1) = if zoom_in { (0.25, 0.75) } else { (-0.5, 1.5) };
            let (lo, hi) = (ra.from_unit(u0), ra.from_unit(u1));
            if lo.is_finite() && hi.is_finite() && lo != hi {
                let a = if y { &mut l.y_axis } else { &mut l.x_axis };
                a.min = Some(lo.min(hi));
                a.max = Some(lo.max(hi));
            }
        }
        GraphOp::HideLegend { .. } => l.legend.visible = false,
        GraphOp::HideTitle { y, .. } => {
            let a = if y { &mut l.y_axis } else { &mut l.x_axis };
            a.show_title = false;
        }
        GraphOp::DeleteLabel { label, .. } => {
            if label < l.labels.len() {
                l.labels.remove(label);
            }
        }
        GraphOp::HideLayer { .. } => l.plots.iter_mut().for_each(|p| p.visible = false),
        GraphOp::HideOtherLayers { .. } => unreachable!("handled above"),
    }
}

/// The colour of grid lines that "Show Grids" adds: Origin's light grey.
const GRID_COLOR: sheaf_core::Color = sheaf_core::Color { r: 204, g: 204, b: 204, a: 255 };

fn short(v: f64) -> String {
    sheaf_core::format::format_general(v, 5)
}
