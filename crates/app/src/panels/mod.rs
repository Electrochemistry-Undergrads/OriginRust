//! The side panels (ADR 0007, point 4): Project Explorer, Messages Log and Results Log on the
//! left edge and the Object Manager on the right edge. Each panel is a vertical tab name in a
//! thin strip at the edge ("auto-hide"). The mouse on the tab, or a click on it, slides the
//! panel out over the workspace; the pin button docks it next to the workspace for good, and
//! the same button in the docked panel lets it go back to the strip. The X button hides the
//! panel and its tab; View > Project Explorer (Alt+1) and the other entries bring it back.

pub mod explorer;
pub mod logs;
pub mod objects;

use crate::actions::Action;
use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Vec2};
use sheaf_core::{Project, WindowId};

/// Width of the strip that holds the tabs.
const STRIP_W: f32 = 22.0;
const HEADER_H: f32 = 20.0;
const HEADER_FILL: Color32 = Color32::from_rgb(222, 229, 240);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PanelId {
    Explorer,
    Objects,
    Messages,
    Results,
}

impl PanelId {
    pub const ALL: [PanelId; 4] = [PanelId::Explorer, PanelId::Messages, PanelId::Results, PanelId::Objects];

    /// Origin's name, written on the tab and in the header.
    pub fn title(self) -> &'static str {
        match self {
            PanelId::Explorer => "Project Explorer",
            PanelId::Objects => "Object Manager",
            PanelId::Messages => "Messages Log",
            PanelId::Results => "Results Log",
        }
    }

    fn right(self) -> bool {
        self == PanelId::Objects
    }

    pub fn index(self) -> usize {
        PanelId::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    fn width(self) -> f32 {
        match self {
            PanelId::Explorer => 280.0,
            PanelId::Objects => 300.0,
            PanelId::Messages | PanelId::Results => 320.0,
        }
    }

    pub fn from_name(name: &str) -> Option<PanelId> {
        match name {
            "explorer" => Some(PanelId::Explorer),
            "objects" => Some(PanelId::Objects),
            "messages" => Some(PanelId::Messages),
            "results" => Some(PanelId::Results),
            _ => None,
        }
    }
}

/// Sets the text the status bar shows on its left while the mouse is on something (a menu entry,
/// a toolbar button). The status bar reads it once per frame.
pub fn set_hint(ctx: &egui::Context, text: &str) {
    ctx.data_mut(|d| d.insert_temp(Id::new("status_hint"), text.to_string()));
}

/// The hint set since the last call, if any.
pub fn take_hint(ctx: &egui::Context) -> Option<String> {
    ctx.data_mut(|d| d.remove_temp::<String>(Id::new("status_hint")))
}

#[derive(Copy, Clone, Debug)]
pub struct PanelState {
    /// The panel exists: its tab is in the strip, or it is docked.
    pub shown: bool,
    /// Docked next to the workspace instead of sliding out of the strip.
    pub pinned: bool,
}

/// What the panels read from the program.
pub struct PanelData<'a> {
    pub project: &'a Project,
    pub active: Option<WindowId>,
    /// The name of the project as the explorer's root folder shows it.
    pub project_name: String,
    /// The plot selected in the Plot Details window while that is open on the active graph.
    pub props_plot: Option<usize>,
    pub logs: &'a logs::Logs,
}

pub struct Panels {
    state: [PanelState; 4],
    /// The panel that is slid out over the workspace.
    flyout: Option<PanelId>,
    /// Opened by a click or a key: it stays until a click elsewhere, not only while the mouse is
    /// on it.
    sticky: bool,
    tab_rects: Vec<(PanelId, Rect)>,
    hovered_tab: Option<PanelId>,
    clicked_tab: Option<PanelId>,
    /// The space between the two strips and the panels docked next to them: where a panel that
    /// slides out may draw.
    edge: Rect,
    pub explorer: explorer::ExplorerState,
    pub objects: objects::ObjectsState,
}

impl Default for Panels {
    fn default() -> Self {
        let shown = |s| PanelState { shown: s, pinned: false };
        Panels {
            // Explorer, Messages, Results, Object Manager (the order of `PanelId::ALL`).
            state: [shown(true), shown(true), shown(false), shown(true)],
            flyout: None,
            sticky: false,
            tab_rects: Vec::new(),
            hovered_tab: None,
            clicked_tab: None,
            edge: Rect::NOTHING,
            explorer: explorer::ExplorerState::default(),
            objects: objects::ObjectsState::default(),
        }
    }
}

impl Panels {
    pub fn is_shown(&self, id: PanelId) -> bool {
        self.state[id.index()].shown
    }

    pub fn is_pinned(&self, id: PanelId) -> bool {
        self.state[id.index()].pinned
    }

    /// The panel that is slid out, if any.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn flyout(&self) -> Option<PanelId> {
        self.flyout
    }

    /// Which panels are shown, in the order of `PanelId::ALL` (for the ticks of the View menu).
    pub fn shown_flags(&self) -> [bool; 4] {
        PanelId::ALL.map(|p| self.is_shown(p))
    }

    /// View > Project Explorer and its keys: hides a shown panel; shows a hidden one and, when
    /// it is not docked, slides it out so that something visibly happens.
    pub fn toggle(&mut self, id: PanelId) {
        if self.is_shown(id) {
            self.state[id.index()].shown = false;
            if self.flyout == Some(id) {
                self.flyout = None;
            }
        } else {
            self.state[id.index()].shown = true;
            if !self.is_pinned(id) {
                self.flyout = Some(id);
                self.sticky = true;
            }
        }
    }

    /// Makes the tab appear without sliding the panel out (new results arrived).
    pub fn reveal(&mut self, id: PanelId) {
        self.state[id.index()].shown = true;
    }

    /// Slides the panel out and keeps it open (`--show panel:explorer`).
    pub fn open(&mut self, id: PanelId) {
        self.state[id.index()].shown = true;
        if !self.is_pinned(id) {
            self.flyout = Some(id);
            self.sticky = true;
        }
    }

    pub fn set_pinned(&mut self, id: PanelId, pinned: bool) {
        let s = &mut self.state[id.index()];
        s.shown = true;
        s.pinned = pinned;
        if pinned {
            if self.flyout == Some(id) {
                self.flyout = None;
            }
        } else {
            self.flyout = Some(id);
            self.sticky = true;
        }
    }

    pub fn hide(&mut self, id: PanelId) {
        self.state[id.index()].shown = false;
        if self.flyout == Some(id) {
            self.flyout = None;
        }
    }

    // ----- drawing -----------------------------------------------------------------------

    /// Adds the strips with their tabs and the docked panels to the window layout. Call before the
    /// toolbars that sit next to the workspace and before the central panel.
    pub fn strips_and_docks(&mut self, ctx: &egui::Context, data: &PanelData, actions: &mut Vec<Action>) {
        self.tab_rects.clear();
        self.hovered_tab = None;
        self.clicked_tab = None;
        for right in [false, true] {
            self.strip(ctx, right);
            for id in PanelId::ALL {
                if id.right() == right && self.is_shown(id) && self.is_pinned(id) {
                    let panel = if right {
                        egui::SidePanel::right(Id::new(("dock", id)))
                    } else {
                        egui::SidePanel::left(Id::new(("dock", id)))
                    };
                    panel
                        .resizable(true)
                        .default_width(id.width())
                        .width_range(150.0..=600.0)
                        .frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0_f32, theme::BORDER)))
                        .show(ctx, |ui| self.body(ui, id, data, actions));
                }
            }
        }
        self.edge = ctx.available_rect();
    }

    fn strip(&mut self, ctx: &egui::Context, right: bool) {
        let tabs: Vec<PanelId> = PanelId::ALL
            .into_iter()
            .filter(|id| id.right() == right && self.is_shown(*id) && !self.is_pinned(*id))
            .collect();
        if tabs.is_empty() {
            return;
        }
        let panel = if right {
            egui::SidePanel::right(Id::new("strip_right"))
        } else {
            egui::SidePanel::left(Id::new("strip_left"))
        };
        panel.exact_width(STRIP_W).resizable(false).frame(egui::Frame::new().fill(theme::PANEL)).show(ctx, |ui| {
            let strip = ui.max_rect();
            let mut y = strip.top() + 6.0;
            for id in tabs {
                let galley =
                    ui.painter().layout_no_wrap(id.title().to_string(), FontId::proportional(12.0), Color32::BLACK);
                let tab = Rect::from_min_size(Pos2::new(strip.left(), y), Vec2::new(STRIP_W, galley.size().x + 16.0));
                let response = ui.interact(tab, Id::new(("tab", id)), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{} tab", id.title()))
                });
                if self.flyout == Some(id) {
                    ui.painter().rect_filled(tab, 0.0, Color32::from_rgb(204, 228, 247));
                } else if response.hovered() {
                    ui.painter().rect_filled(tab, 0.0, Color32::from_rgb(229, 241, 251));
                }
                // The text runs downwards, its tops towards the right: turn it a quarter
                // clockwise about its start, which then is the top right of the tab.
                let at = Pos2::new(tab.center().x + galley.size().y / 2.0, tab.top() + 8.0);
                let shape =
                    egui::epaint::TextShape::new(at, galley, Color32::BLACK).with_angle(std::f32::consts::FRAC_PI_2);
                ui.painter().add(shape);
                if response.clicked() {
                    self.clicked_tab = Some(id);
                }
                if response.hovered() {
                    self.hovered_tab = Some(id);
                }
                self.tab_rects.push((id, tab));
                y += tab.height() + 4.0;
            }
        });
    }

    /// The header with the title, the pin and the close button, then the panel's content.
    fn body(&mut self, ui: &mut egui::Ui, id: PanelId, data: &PanelData, actions: &mut Vec<Action>) {
        self.header(ui, id);
        match id {
            PanelId::Explorer => explorer::show(ui, &mut self.explorer, data, actions),
            PanelId::Objects => objects::show(ui, &mut self.objects, data, actions),
            PanelId::Messages => logs::show_messages(ui, data.logs, actions),
            PanelId::Results => logs::show_results(ui, data.logs, actions),
        }
    }

    fn header(&mut self, ui: &mut egui::Ui, id: PanelId) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), HEADER_H), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, HEADER_FILL);
        ui.painter().text(
            Pos2::new(rect.left() + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            id.title(),
            FontId::proportional(12.0),
            Color32::BLACK,
        );
        let pinned = self.is_pinned(id);
        let button = |ui: &mut egui::Ui, n: f32, name: String| {
            let r =
                Rect::from_center_size(Pos2::new(rect.right() - 12.0 - 20.0 * n, rect.center().y), Vec2::splat(18.0));
            let response = ui.interact(r, Id::new(("panel_button", id, n as i32, pinned)), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name.clone()));
            if response.hovered() {
                ui.painter().rect_filled(r, 0.0, Color32::from_rgb(229, 241, 251));
            }
            (r, response.on_hover_text(name.clone()))
        };
        let (r, response) = button(ui, 0.0, format!("Close {}", id.title()));
        let x = Stroke::new(1.3_f32, Color32::from_rgb(60, 60, 66));
        let c = r.center();
        ui.painter().line_segment([c + Vec2::new(-4.0, -4.0), c + Vec2::new(4.0, 4.0)], x);
        ui.painter().line_segment([c + Vec2::new(-4.0, 4.0), c + Vec2::new(4.0, -4.0)], x);
        if response.clicked() {
            self.hide(id);
        }
        let name = if pinned { format!("Auto Hide {}", id.title()) } else { format!("Pin {}", id.title()) };
        let (r, response) = button(ui, 1.0, name);
        paint_pin(ui.painter(), r.center(), pinned);
        if response.clicked() {
            self.set_pinned(id, !pinned);
        }
    }

    /// Draws the panel that is slid out over the workspace and decides when it goes away again.
    /// Call after the central panel.
    pub fn flyouts(&mut self, ctx: &egui::Context, data: &PanelData, actions: &mut Vec<Action>) {
        if let Some(t) = self.clicked_tab {
            self.flyout = Some(t);
            self.sticky = true;
        } else if let Some(t) = self.hovered_tab
            && self.flyout != Some(t)
        {
            self.flyout = Some(t);
            self.sticky = false;
        }
        let Some(id) = self.flyout else { return };
        if !self.is_shown(id) || self.is_pinned(id) {
            self.flyout = None;
            return;
        }
        let edge = self.edge;
        let w = id.width().min(edge.width() - 40.0).max(120.0);
        let rect = if id.right() {
            Rect::from_min_max(Pos2::new(edge.right() - w, edge.top()), edge.right_bottom())
        } else {
            Rect::from_min_max(edge.left_top(), Pos2::new(edge.left() + w, edge.bottom()))
        };
        egui::Area::new(Id::new(("flyout", id))).order(egui::Order::Middle).fixed_pos(rect.min).show(ctx, |ui| {
            ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0_f32, theme::BORDER), egui::StrokeKind::Inside);
            // Nothing below the panel gets the mouse.
            ui.interact(rect, Id::new(("flyout_block", id)), Sense::click_and_drag());
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(1.0)), |ui| {
                ui.set_clip_rect(rect);
                self.body(ui, id, data, actions);
            });
            ui.allocate_rect(rect, Sense::hover());
        });
        // Leaving the panel and its tab closes it, unless it was opened by a click (then a click
        // elsewhere does), and not while a menu is open or a window is being dragged.
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let near = |r: Rect| pointer.is_some_and(|p| r.expand(2.0).contains(p));
        let inside = near(rect)
            || self.tab_rects.iter().any(|(t, r)| *t == id && near(*r))
            || egui::Popup::is_any_open(ctx)
            || egui::DragAndDrop::has_any_payload(ctx);
        let pressed = ctx.input(|i| i.pointer.any_pressed());
        if !inside && (!self.sticky || pressed) {
            self.flyout = None;
        }
    }
}

/// The push pin: standing (docked) or lying (auto-hide).
fn paint_pin(painter: &egui::Painter, c: Pos2, docked: bool) {
    let s = Stroke::new(1.4_f32, Color32::from_rgb(60, 60, 66));
    // Drawn standing; lying is the same turned a quarter.
    let p = |x: f32, y: f32| if docked { c + Vec2::new(x, y) } else { c + Vec2::new(y, x) };
    painter.line_segment([p(-3.0, -5.0), p(3.0, -5.0)], s);
    painter.line_segment([p(-2.0, -5.0), p(-2.0, -1.0)], s);
    painter.line_segment([p(2.0, -5.0), p(2.0, -1.0)], s);
    painter.line_segment([p(-4.0, -1.0), p(4.0, -1.0)], s);
    painter.line_segment([p(0.0, -1.0), p(0.0, 5.0)], s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_hides_a_shown_panel_and_slides_a_hidden_one_out() {
        let mut p = Panels::default();
        assert!(p.is_shown(PanelId::Explorer) && p.flyout().is_none());
        p.toggle(PanelId::Explorer);
        assert!(!p.is_shown(PanelId::Explorer));
        p.toggle(PanelId::Explorer);
        assert!(p.is_shown(PanelId::Explorer));
        assert_eq!(p.flyout(), Some(PanelId::Explorer));
    }

    #[test]
    fn a_pinned_panel_does_not_slide_and_unpinning_slides_it_out() {
        let mut p = Panels::default();
        p.open(PanelId::Objects);
        p.set_pinned(PanelId::Objects, true);
        assert!(p.flyout().is_none() && p.is_pinned(PanelId::Objects));
        p.toggle(PanelId::Objects);
        p.toggle(PanelId::Objects);
        assert!(p.flyout().is_none(), "docked panels need no slide");
        p.set_pinned(PanelId::Objects, false);
        assert_eq!(p.flyout(), Some(PanelId::Objects));
    }

    #[test]
    fn results_log_starts_hidden_and_the_other_three_have_tabs() {
        let p = Panels::default();
        assert_eq!(p.shown_flags(), [true, true, false, true]);
    }
}
