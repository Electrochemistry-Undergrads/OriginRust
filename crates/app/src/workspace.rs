//! The workspace: the grey area in which workbooks and graphs float as child windows
//! (ADR 0007, point 1). `egui::Window` has no minimise, maximise or stacking order of its
//! own, so this type owns, per window, position, size, state and place in the stack, and
//! draws the title bar row itself.
//!
//! Closing a window only hides it; its data stays in the project and the project explorer
//! shows it again. Deleting is a separate command (as in Origin, where the close question
//! offers Hide).

use crate::theme;
use eframe::egui::{self, Color32, Id, Pos2, Rect, Sense, Stroke, Vec2};
use sheaf_core::WindowId;
use sheaf_core::project::{WindowPlace, WindowState};
use std::collections::HashMap;

/// Step of the cascade, in points.
const CASCADE_STEP: f32 = 26.0;
/// Where the first window starts, relative to the workspace corner.
const CASCADE_START: f32 = 20.0;
/// Windows cascaded before the position wraps to the start.
const CASCADE_WRAP: usize = 8;
/// Child windows lie below the dialogs, which are ordinary `Middle` windows, and above the
/// grey of the workspace.
const WINDOW_ORDER: egui::Order = egui::Order::Background;
const TITLE_HEIGHT: f32 = 22.0;
const FRAME: f32 = 3.0;
const MIN_SIZE: Vec2 = Vec2::new(180.0, 90.0);
const BAR_SIZE: Vec2 = Vec2::new(150.0, TITLE_HEIGHT);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Minimised,
    Maximised,
}

#[derive(Copy, Clone, Debug)]
struct State {
    /// Top-left corner relative to the workspace corner (in the normal state).
    pos: Vec2,
    size: Vec2,
    /// The size a new window of this kind has; Cascade gives it back.
    base_size: Vec2,
    mode: Mode,
    /// Closed windows are hidden, not deleted.
    visible: bool,
    /// Has a cascade position been given yet.
    placed: bool,
    /// Changes when the window must take a new size from us (maximise, restore); egui keeps the
    /// size of a window under its id, so a new id is the way to give it another one.
    generation: u32,
}

/// What the workspace needs to know to draw a window.
pub struct WindowInfo {
    pub title: String,
    pub graph: bool,
}

#[derive(Default)]
pub struct Workspace {
    wins: HashMap<WindowId, State>,
    /// Bottom to top.
    z: Vec<WindowId>,
    /// Most recently activated first (the Window Switcher's order). A window that was never
    /// activated is not listed.
    mru: Vec<WindowId>,
    active: Option<WindowId>,
    cascaded: usize,
    /// Size of the area the windows were drawn in last time; the arranging commands use it.
    area: Vec2,
}

impl Workspace {
    /// Starts to track a window. A window added `visible` is placed by the cascade and made
    /// the active one; others wait in the project explorer until they are shown.
    pub fn add(&mut self, id: WindowId, graph: bool, visible: bool) {
        let size = if graph { Vec2::new(520.0, 360.0) } else { Vec2::new(480.0, 330.0) };
        self.wins.insert(
            id,
            State {
                pos: Vec2::ZERO,
                size,
                base_size: size,
                mode: Mode::Normal,
                visible: false,
                placed: false,
                generation: 0,
            },
        );
        self.z.push(id);
        if visible {
            self.activate(id);
        }
    }

    /// Drops windows that are no longer in `keep`.
    pub fn retain(&mut self, keep: impl Fn(WindowId) -> bool) {
        self.wins.retain(|id, _| keep(*id));
        self.z.retain(|id| self.wins.contains_key(id));
        self.mru.retain(|id| self.wins.contains_key(id));
        if self.active.is_some_and(|a| !self.wins.contains_key(&a)) {
            self.active = self.top_visible();
            self.touch_active();
        }
    }

    /// Puts the active window (after another one closed or was minimised) first in the recent list.
    fn touch_active(&mut self) {
        if let Some(a) = self.active {
            self.mru.retain(|m| *m != a);
            self.mru.insert(0, a);
        }
    }

    /// The windows in the order they were last activated, most recent first; windows that were
    /// never activated follow in their creation order.
    pub fn recent_order(&self, ids: &[WindowId]) -> Vec<WindowId> {
        let mut out: Vec<WindowId> = self.mru.iter().copied().filter(|m| ids.contains(m)).collect();
        out.extend(ids.iter().copied().filter(|i| !self.mru.contains(i)));
        out
    }

    pub fn contains(&self, id: WindowId) -> bool {
        self.wins.contains_key(&id)
    }

    pub fn active(&self) -> Option<WindowId> {
        self.active
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn mode(&self, id: WindowId) -> Option<Mode> {
        self.wins.get(&id).map(|s| s.mode)
    }

    /// True when the window is on screen or minimised (not closed).
    pub fn is_shown(&self, id: WindowId) -> bool {
        self.wins.get(&id).is_some_and(|s| s.visible)
    }

    /// The windows that are shown, bottom to top.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn shown(&self) -> Vec<WindowId> {
        self.z.iter().copied().filter(|id| self.is_shown(*id)).collect()
    }

    /// Shows the window if it was closed or minimised, brings it to the front and makes it the
    /// active one.
    pub fn activate(&mut self, id: WindowId) {
        let Some(s) = self.wins.get_mut(&id) else {
            return;
        };
        if !s.placed {
            let n = self.cascaded % CASCADE_WRAP;
            self.cascaded += 1;
            s.pos = Vec2::splat(CASCADE_START + CASCADE_STEP * n as f32);
            s.placed = true;
        }
        s.visible = true;
        if s.mode == Mode::Minimised {
            s.mode = Mode::Normal;
            s.generation += 1;
        }
        self.z.retain(|z| *z != id);
        self.z.push(id);
        self.active = Some(id);
        self.touch_active();
    }

    /// Closes the window: it disappears from the workspace, its data stays.
    pub fn hide(&mut self, id: WindowId) {
        if let Some(s) = self.wins.get_mut(&id) {
            s.visible = false;
        }
        self.mru.retain(|m| *m != id);
        self.mru.push(id);
        if self.active == Some(id) {
            self.active = self.top_visible();
            self.touch_active();
        }
    }

    pub fn minimise(&mut self, id: WindowId) {
        if let Some(s) = self.wins.get_mut(&id) {
            s.mode = Mode::Minimised;
        }
        if self.active == Some(id) {
            self.active = self.top_visible();
            self.touch_active();
        }
    }

    /// Maximises a normal window and restores a maximised one.
    pub fn toggle_maximise(&mut self, id: WindowId) {
        if let Some(s) = self.wins.get_mut(&id) {
            s.mode = if s.mode == Mode::Maximised { Mode::Normal } else { Mode::Maximised };
            s.generation += 1;
        }
        self.activate(id);
    }

    /// The windows an arranging command moves: shown and not minimised, bottom to top.
    fn arrangeable(&self) -> Vec<WindowId> {
        self.z
            .iter()
            .copied()
            .filter(|id| self.wins.get(id).is_some_and(|s| s.visible && s.mode != Mode::Minimised))
            .collect()
    }

    /// The size of the workspace; before the first frame a plain guess.
    fn area(&self) -> Vec2 {
        if self.area.x > 1.0 && self.area.y > 1.0 { self.area } else { Vec2::new(800.0, 500.0) }
    }

    /// Gives a window a place and size and makes it a normal window again. The generation
    /// changes so that egui takes the new size.
    fn place(&mut self, id: WindowId, pos: Vec2, size: Vec2) {
        if let Some(s) = self.wins.get_mut(&id) {
            s.pos = pos;
            s.size = size;
            s.mode = Mode::Normal;
            s.placed = true;
            s.generation += 1;
        }
    }

    /// Window > Cascade: all open windows at their starting size, each 26 points right of and
    /// below the one behind it, the active window in front.
    pub fn cascade(&mut self) {
        for (k, id) in self.arrangeable().into_iter().enumerate() {
            let step = Vec2::splat(CASCADE_START + CASCADE_STEP * (k % CASCADE_WRAP) as f32);
            let size = self.wins[&id].base_size;
            self.place(id, step, size);
        }
        self.cascaded = self.arrangeable().len();
    }

    /// Window > Tile Horizontally: the open windows one above the other, each as wide as the
    /// workspace (the meaning of the word in Windows' MDI).
    pub fn tile_horizontally(&mut self) {
        let ids = self.arrangeable();
        let n = ids.len().max(1) as f32;
        let area = self.area();
        let h = area.y / n;
        for (k, id) in ids.into_iter().enumerate() {
            self.place(id, Vec2::new(0.0, h * k as f32), Vec2::new(area.x, h));
        }
    }

    /// Window > Tile Vertically: the open windows side by side, each as high as the workspace.
    pub fn tile_vertically(&mut self) {
        let ids = self.arrangeable();
        let n = ids.len().max(1) as f32;
        let area = self.area();
        let w = area.x / n;
        for (k, id) in ids.into_iter().enumerate() {
            self.place(id, Vec2::new(w * k as f32, 0.0), Vec2::new(w, area.y));
        }
    }

    /// Where every window that has a place is, bottom of the stack first, for saving in the
    /// project. Windows that were never shown have no place yet and are left out.
    pub fn places(&self) -> Vec<WindowPlace> {
        self.z
            .iter()
            .filter_map(|id| {
                let s = self.wins.get(id)?;
                if !s.placed {
                    return None;
                }
                let state = match (s.visible, s.mode) {
                    (false, _) => WindowState::Hidden,
                    (true, Mode::Normal) => WindowState::Normal,
                    (true, Mode::Minimised) => WindowState::Minimised,
                    (true, Mode::Maximised) => WindowState::Maximised,
                };
                Some(WindowPlace {
                    window: *id,
                    x: f64::from(s.pos.x),
                    y: f64::from(s.pos.y),
                    width: f64::from(s.size.x),
                    height: f64::from(s.size.y),
                    state,
                })
            })
            .collect()
    }

    /// Puts the listed windows back where `places` says (a project being opened), in its stacking
    /// order; the topmost shown window becomes the active one. Windows the workspace does not
    /// track, and sizes that are not finite, are skipped.
    pub fn restore(&mut self, places: &[WindowPlace]) {
        let mut restored = Vec::new();
        for p in places {
            let Some(s) = self.wins.get_mut(&p.window) else { continue };
            let (pos, size) = (Vec2::new(p.x as f32, p.y as f32), Vec2::new(p.width as f32, p.height as f32));
            if !(pos.x.is_finite() && pos.y.is_finite() && size.x.is_finite() && size.y.is_finite()) {
                continue;
            }
            s.pos = pos.max(Vec2::ZERO);
            s.size = size.max(MIN_SIZE);
            s.placed = true;
            s.visible = p.state != WindowState::Hidden;
            s.mode = match p.state {
                WindowState::Minimised => Mode::Minimised,
                WindowState::Maximised => Mode::Maximised,
                WindowState::Normal | WindowState::Hidden => Mode::Normal,
            };
            s.generation += 1;
            restored.push(p.window);
        }
        if restored.is_empty() {
            return;
        }
        // The listed windows go to the top in their saved order; others stay below them.
        self.z.retain(|id| !restored.contains(id));
        self.z.extend(restored);
        self.active = self.top_visible();
    }

    /// The topmost window that is shown and not minimised.
    fn top_visible(&self) -> Option<WindowId> {
        self.z
            .iter()
            .rev()
            .copied()
            .find(|id| self.wins.get(id).is_some_and(|s| s.visible && s.mode != Mode::Minimised))
    }

    fn layer(id: WindowId, generation: u32) -> egui::LayerId {
        egui::LayerId::new(WINDOW_ORDER, Id::new(("workspace_window", id.0, generation)))
    }

    /// Draws all shown windows inside the rectangle of `ui`, bottom to top, then the bars of the
    /// minimised ones. `changed` is the project's unsaved-changes flag; `content` draws the body
    /// of a window.
    pub fn show(
        &mut self,
        ui: &egui::Ui,
        changed: bool,
        info: &dyn Fn(WindowId) -> Option<WindowInfo>,
        content: &mut dyn FnMut(WindowId, &mut egui::Ui),
    ) {
        let ctx = ui.ctx().clone();
        let area = ui.max_rect();
        self.area = area.size();
        let mut events: Vec<(WindowId, Event)> = Vec::new();
        for id in self.z.clone() {
            let Some(mut s) = self.wins.get(&id).copied() else { continue };
            if !s.visible || s.mode == Mode::Minimised {
                continue;
            }
            let Some(win_info) = info(id) else { continue };
            let active = self.active == Some(id);
            let maximised = s.mode == Mode::Maximised;
            let layer = Self::layer(id, s.generation);
            let frame = egui::Frame::new()
                .fill(if active { FRAME_ACTIVE } else { FRAME_INACTIVE })
                .inner_margin(FRAME)
                .stroke(Stroke::new(1.0_f32, if active { FRAME_ACTIVE_EDGE } else { BORDER_INACTIVE }));
            let margin = frame.total_margin().sum();
            let origin = area.min + s.pos;
            let mut window = egui::Window::new("")
                .id(layer.id)
                .order(WINDOW_ORDER)
                .title_bar(false)
                .frame(frame)
                .movable(false)
                .constrain_to(area)
                .min_size(MIN_SIZE - margin);
            if maximised {
                window = window.current_pos(area.min).fixed_size(area.size() - margin);
            } else {
                window = window.current_pos(origin).default_size(s.size - margin).resizable(true);
            }
            let mut drag = Vec2::ZERO;
            let mut ev: Vec<Event> = Vec::new();
            let shown = window.show(&ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let bar = title_bar(ui, id, &win_info, changed, active, maximised);
                drag = bar.drag;
                ev.extend(bar.event);
                let body = ui.available_rect_before_wrap();
                ui.painter().rect_filled(body, 0.0, Color32::WHITE);
                ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| content(id, ui));
                // The body always fills the window, so that the frame goes round all of it.
                ui.allocate_rect(body, Sense::hover());
            });
            if let Some(shown) = shown {
                let rect = shown.response.rect;
                if !maximised {
                    s.pos = rect.min - area.min + drag;
                    s.size = rect.size();
                    self.wins.insert(id, s);
                }
                // A press anywhere in the window (and not in a window above it) activates it.
                let pressed = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten());
                if let Some(p) = pressed
                    && rect.contains(p)
                    && ctx.layer_id_at(p) == Some(shown.response.layer_id)
                {
                    ev.push(Event::Activate);
                }
            }
            events.extend(ev.into_iter().map(|e| (id, e)));
        }
        self.minimised_bars(&ctx, area, info, &mut events);
        for (id, event) in events {
            match event {
                Event::Activate => {
                    if self.active != Some(id) {
                        self.activate(id);
                    }
                }
                Event::Minimise => self.minimise(id),
                Event::ToggleMaximise => self.toggle_maximise(id),
                Event::Close => self.hide(id),
                Event::Restore => self.activate(id),
            }
        }
        // The stack follows `z`: the active window is on top of the layers.
        if let Some(id) = self.active
            && let Some(s) = self.wins.get(&id)
        {
            ctx.move_to_top(Self::layer(id, s.generation));
        }
    }

    fn minimised_bars(
        &self,
        ctx: &egui::Context,
        area: Rect,
        info: &dyn Fn(WindowId) -> Option<WindowInfo>,
        events: &mut Vec<(WindowId, Event)>,
    ) {
        let bars: Vec<WindowId> = self
            .z
            .iter()
            .copied()
            .filter(|id| self.wins.get(id).is_some_and(|s| s.visible && s.mode == Mode::Minimised))
            .collect();
        for (k, id) in bars.into_iter().enumerate() {
            let Some(win_info) = info(id) else { continue };
            let pos = Pos2::new(area.left() + 4.0 + k as f32 * (BAR_SIZE.x + 4.0), area.bottom() - 4.0 - BAR_SIZE.y);
            egui::Area::new(Id::new(("workspace_bar", id.0)))
                .order(WINDOW_ORDER)
                .fixed_pos(pos)
                .constrain_to(area)
                .show(ctx, |ui| {
                    let (rect, _) = ui.allocate_exact_size(BAR_SIZE, Sense::hover());
                    let response = ui.interact(rect, Id::new(("workspace_bar_body", id.0)), Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Restore {}", win_info.title))
                    });
                    let p = ui.painter();
                    p.rect_filled(rect, 0.0, TITLE_INACTIVE);
                    p.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, BORDER_INACTIVE), egui::StrokeKind::Inside);
                    let icon = Rect::from_min_size(rect.min + Vec2::new(4.0, 4.0), Vec2::splat(14.0));
                    paint_window_icon(p, icon, win_info.graph);
                    let text_rect =
                        Rect::from_min_max(rect.min + Vec2::new(22.0, 0.0), rect.max - Vec2::new(22.0, 0.0));
                    p.with_clip_rect(text_rect).text(
                        text_rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        &win_info.title,
                        egui::FontId::proportional(12.0),
                        TEXT_INACTIVE,
                    );
                    let close = Rect::from_min_size(rect.right_top() + Vec2::new(-20.0, 3.0), Vec2::new(16.0, 16.0));
                    let close_response = ui.interact(close, Id::new(("workspace_bar_close", id.0)), Sense::click());
                    close_response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Close {}", win_info.title))
                    });
                    paint_button(p, close, Glyph::Close, false, close_response.hovered());
                    if close_response.clicked() {
                        events.push((id, Event::Close));
                    } else if response.clicked() {
                        events.push((id, Event::Restore));
                    }
                });
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Event {
    Activate,
    Minimise,
    ToggleMaximise,
    Close,
    Restore,
}

const FRAME_ACTIVE: Color32 = Color32::from_rgb(120, 165, 220);
const FRAME_ACTIVE_EDGE: Color32 = Color32::from_rgb(70, 120, 190);
const FRAME_INACTIVE: Color32 = Color32::from_rgb(214, 214, 218);
const BORDER_INACTIVE: Color32 = Color32::from_rgb(170, 170, 176);
const TITLE_ACTIVE: Color32 = Color32::from_rgb(190, 214, 240);
const TITLE_INACTIVE: Color32 = Color32::from_rgb(232, 232, 235);
const TEXT_ACTIVE: Color32 = Color32::BLACK;
const TEXT_INACTIVE: Color32 = Color32::from_rgb(120, 120, 124);
const CLOSE_ACTIVE: Color32 = Color32::from_rgb(204, 64, 56);

struct Bar {
    drag: Vec2,
    event: Option<Event>,
}

/// The row at the top of a window: icon, name, three buttons. Dragging it moves the window,
/// a double click maximises or restores it.
fn title_bar(ui: &mut egui::Ui, id: WindowId, info: &WindowInfo, changed: bool, active: bool, maximised: bool) -> Bar {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), TITLE_HEIGHT), Sense::hover());
    let title = format!("{}{}", info.title, if changed { " *" } else { "" });
    let drag_response = ui.interact(rect, Id::new(("workspace_title", id.0)), Sense::click_and_drag());
    drag_response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Title {}", info.title)));
    let mut bar = Bar { drag: Vec2::ZERO, event: None };
    if drag_response.dragged() && !maximised {
        bar.drag = drag_response.drag_delta();
    }
    if drag_response.double_clicked() {
        bar.event = Some(Event::ToggleMaximise);
    }

    let width = 20.0;
    let button = |k: f32| {
        Rect::from_min_size(rect.right_top() + Vec2::new(-(k + 1.0) * width - 2.0, 3.0), Vec2::new(18.0, 16.0))
    };
    let close = button(0.0);
    let maximise = button(1.0);
    let minimise = button(2.0);
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 0.0, if active { TITLE_ACTIVE } else { TITLE_INACTIVE });
    paint_window_icon(&painter, Rect::from_min_size(rect.min + Vec2::new(4.0, 4.0), Vec2::splat(14.0)), info.graph);
    let text_rect =
        Rect::from_min_max(rect.min + Vec2::new(24.0, 0.0), Pos2::new(minimise.left() - 4.0, rect.bottom()));
    painter.with_clip_rect(text_rect).text(
        text_rect.left_center(),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(12.0),
        if active { TEXT_ACTIVE } else { TEXT_INACTIVE },
    );
    let max_label = if maximised { "Restore" } else { "Maximise" };
    let max_glyph = if maximised { Glyph::Restore } else { Glyph::Maximise };
    for (r, glyph, label, key) in [
        (minimise, Glyph::Minimise, "Minimise", 1),
        (maximise, max_glyph, max_label, 2),
        (close, Glyph::Close, "Close", 3),
    ] {
        let response = ui.interact(r, Id::new(("workspace_button", id.0, key)), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{label} {}", info.title))
        });
        paint_button(&painter, r, glyph, active && key == 3, response.hovered());
        if response.clicked() {
            bar.event = Some(match key {
                1 => Event::Minimise,
                2 => Event::ToggleMaximise,
                _ => Event::Close,
            });
        }
    }
    bar
}

#[derive(Copy, Clone, PartialEq)]
enum Glyph {
    Minimise,
    Maximise,
    Restore,
    Close,
}

fn paint_button(p: &egui::Painter, r: Rect, glyph: Glyph, red: bool, hovered: bool) {
    let (fill, ink) = if red {
        (CLOSE_ACTIVE, Color32::WHITE)
    } else if hovered {
        (Color32::from_rgb(229, 241, 251), Color32::BLACK)
    } else {
        (Color32::from_rgb(236, 236, 238), Color32::from_rgb(60, 60, 64))
    };
    p.rect_filled(r, 0.0, fill);
    let edge = if red { Color32::from_rgb(150, 40, 36) } else { theme::BORDER };
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, edge), egui::StrokeKind::Inside);
    let c = r.center();
    let line = Stroke::new(1.2_f32, ink);
    let inside = egui::StrokeKind::Inside;
    match glyph {
        Glyph::Minimise => {
            p.line_segment([c + Vec2::new(-4.0, 3.0), c + Vec2::new(4.0, 3.0)], line);
        }
        Glyph::Maximise => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::splat(8.0)), 0.0, line, inside);
        }
        Glyph::Restore => {
            p.rect_stroke(Rect::from_min_size(c + Vec2::new(-4.0, -1.0), Vec2::splat(6.0)), 0.0, line, inside);
            p.rect_stroke(Rect::from_min_size(c + Vec2::new(-2.0, -4.0), Vec2::splat(6.0)), 0.0, line, inside);
        }
        Glyph::Close => {
            p.line_segment([c + Vec2::new(-3.5, -3.5), c + Vec2::new(3.5, 3.5)], line);
            p.line_segment([c + Vec2::new(-3.5, 3.5), c + Vec2::new(3.5, -3.5)], line);
        }
    }
}

/// A small drawn icon: a table for workbooks, a curve on axes for graphs.
pub fn paint_window_icon(painter: &egui::Painter, rect: Rect, graph: bool) {
    let r = rect.shrink(1.0);
    let line = Stroke::new(1.0_f32, Color32::from_rgb(70, 70, 74));
    if graph {
        painter.line_segment([r.left_top(), r.left_bottom()], line);
        painter.line_segment([r.left_bottom(), r.right_bottom()], line);
        let accent = Stroke::new(1.5_f32, Color32::from_rgb(40, 110, 200));
        let pts: Vec<Pos2> = [(0.1, 0.8), (0.35, 0.45), (0.6, 0.6), (0.95, 0.1)]
            .iter()
            .map(|(x, y)| Pos2::new(r.left() + x * r.width(), r.top() + y * r.height()))
            .collect();
        painter.add(egui::Shape::line(pts, accent));
    } else {
        painter.rect_stroke(r, 0.0, line, egui::StrokeKind::Inside);
        for k in 1..3 {
            let x = r.left() + r.width() / 3.0 * k as f32;
            let y = r.top() + r.height() / 3.0 * k as f32;
            painter.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], line);
            painter.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_windows_cascade_and_the_last_one_is_active_and_on_top() {
        let mut w = Workspace::default();
        for k in 1..=3 {
            w.add(WindowId(k), false, true);
        }
        assert_eq!(w.active(), Some(WindowId(3)));
        assert_eq!(w.shown(), vec![WindowId(1), WindowId(2), WindowId(3)]);
        let p = |k| w.wins[&WindowId(k)].pos;
        assert_eq!(p(2) - p(1), Vec2::splat(CASCADE_STEP));
        assert_eq!(p(3) - p(2), Vec2::splat(CASCADE_STEP));
    }

    #[test]
    fn activate_raises_and_hide_keeps_the_window() {
        let mut w = Workspace::default();
        w.add(WindowId(1), false, true);
        w.add(WindowId(2), true, true);
        w.activate(WindowId(1));
        assert_eq!(w.shown(), vec![WindowId(2), WindowId(1)]);
        w.hide(WindowId(1));
        assert!(w.contains(WindowId(1)) && !w.is_shown(WindowId(1)));
        assert_eq!(w.active(), Some(WindowId(2)));
        w.activate(WindowId(1));
        assert!(w.is_shown(WindowId(1)));
    }

    #[test]
    fn maximise_and_minimise_change_the_mode_and_restore_by_activating() {
        let mut w = Workspace::default();
        w.add(WindowId(1), false, true);
        w.toggle_maximise(WindowId(1));
        assert_eq!(w.mode(WindowId(1)), Some(Mode::Maximised));
        w.toggle_maximise(WindowId(1));
        assert_eq!(w.mode(WindowId(1)), Some(Mode::Normal));
        w.minimise(WindowId(1));
        assert_eq!(w.mode(WindowId(1)), Some(Mode::Minimised));
        assert_eq!(w.active(), None);
        w.activate(WindowId(1));
        assert_eq!(w.mode(WindowId(1)), Some(Mode::Normal));
    }

    #[test]
    fn cascade_restores_the_start_size_and_steps_each_window() {
        let mut w = Workspace::default();
        for k in 1..=3 {
            w.add(WindowId(k), k == 2, true);
        }
        w.toggle_maximise(WindowId(1));
        w.minimise(WindowId(3));
        w.cascade();
        // The minimised window is left alone, the maximised one is a normal window again.
        assert_eq!(w.mode(WindowId(1)), Some(Mode::Normal));
        assert_eq!(w.mode(WindowId(3)), Some(Mode::Minimised));
        // Each window sits one step right of and below the one behind it.
        let order = w.arrangeable();
        assert_eq!(order.len(), 2);
        let (a, b) = (&w.wins[&order[0]], &w.wins[&order[1]]);
        assert_eq!(b.pos - a.pos, Vec2::splat(CASCADE_STEP));
        assert_eq!(a.size, a.base_size);
        assert_eq!(b.size, b.base_size);
        assert_eq!(w.active(), Some(WindowId(1)), "the active window stays in front");
        assert_eq!(order[1], WindowId(1));
    }

    #[test]
    fn places_are_saved_and_restored_with_state_and_order() {
        let mut w = Workspace::default();
        for (id, graph) in [(WindowId(1), false), (WindowId(2), true), (WindowId(3), true)] {
            w.add(id, graph, true);
        }
        w.minimise(WindowId(2));
        w.hide(WindowId(3));
        w.activate(WindowId(1));
        let places = w.places();
        assert_eq!(places.iter().map(|p| p.window).collect::<Vec<_>>(), vec![WindowId(2), WindowId(3), WindowId(1)]);
        assert_eq!(places[0].state, WindowState::Minimised);
        assert_eq!(places[1].state, WindowState::Hidden);

        let mut fresh = Workspace::default();
        for (id, graph) in [(WindowId(1), false), (WindowId(2), true), (WindowId(3), true)] {
            fresh.add(id, graph, true);
        }
        fresh.restore(&places);
        assert_eq!(fresh.places(), places, "the same places, states and order");
        assert_eq!(fresh.active(), Some(WindowId(1)));
        assert!(!fresh.is_shown(WindowId(3)));
    }

    #[test]
    fn tiling_divides_the_workspace_between_the_open_windows() {
        let mut w = Workspace { area: Vec2::new(900.0, 600.0), ..Workspace::default() };
        for k in 1..=3 {
            w.add(WindowId(k), false, true);
        }
        w.tile_horizontally();
        for k in 1..=3u64 {
            let s = &w.wins[&WindowId(k)];
            assert_eq!(s.size, Vec2::new(900.0, 200.0));
            assert_eq!(s.pos, Vec2::new(0.0, 200.0 * (k - 1) as f32));
        }
        w.tile_vertically();
        for k in 1..=3u64 {
            let s = &w.wins[&WindowId(k)];
            assert_eq!(s.size, Vec2::new(300.0, 600.0));
            assert_eq!(s.pos, Vec2::new(300.0 * (k - 1) as f32, 0.0));
        }
        // The windows are not stacked on each other afterwards: the order of the stack stays.
        assert_eq!(w.shown(), vec![WindowId(1), WindowId(2), WindowId(3)]);
    }
}
