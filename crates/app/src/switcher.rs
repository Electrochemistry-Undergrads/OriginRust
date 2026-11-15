//! The Window Switcher (Ctrl+Tab): a centred panel of cards, one per child window of the active
//! Project Explorer folder, most recently used first. Each Tab while Ctrl is held moves the
//! selection, releasing Ctrl activates it. See `docs/spec/gui-look.md`, "Window Switcher (T42)".
//!
//! The keys are read here before the grid and the shortcut table see them, so Ctrl+Tab works while
//! a worksheet has the keyboard; plain Tab is left alone and still moves the cell.

use crate::theme;
use eframe::egui::{self, Color32, Event, Key, Rect, Sense, Stroke, TextureHandle, TextureOptions, vec2};
use sheaf_core::{Project, Window, WindowId};
use std::collections::HashMap;

const CARD_W: f32 = 168.0;
const THUMB_W: f32 = 148.0;
const THUMB_H: f32 = 98.0;
const LABEL_H: f32 = 36.0;
const GAP: f32 = 10.0;
/// Graphs beyond this many (in recent order) get a type icon instead of a rendered thumbnail, so
/// opening the switcher on a project with very many graphs stays quick.
const MAX_THUMBNAILS: usize = 24;

/// What a key press means for the switcher.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Nav {
    /// Tab: forward (or opens the switcher with Ctrl).
    Next,
    /// Shift+Tab.
    Prev,
    Left,
    Right,
    Up,
    Down,
    Confirm,
    Cancel,
}

/// Takes the keys of the switcher out of this frame's input. `open` is true while the switcher is
/// shown (then Tab, the arrows, Enter and Esc are all its); otherwise only Ctrl+Tab and
/// Ctrl+Shift+Tab count, and only when `may_open` (no dialog has the keyboard). Ctrl is the
/// physical Control key on every system (on macOS Cmd+Tab belongs to the system).
pub fn take_keys(ctx: &egui::Context, open: bool, may_open: bool) -> Vec<Nav> {
    let mut out = Vec::new();
    if !open && !may_open {
        return out;
    }
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            let Event::Key { key, pressed: true, modifiers, .. } = e else { return true };
            let nav = match key {
                Key::Tab if open || modifiers.ctrl => Some(if modifiers.shift { Nav::Prev } else { Nav::Next }),
                Key::ArrowLeft if open => Some(Nav::Left),
                Key::ArrowRight if open => Some(Nav::Right),
                Key::ArrowUp if open => Some(Nav::Up),
                Key::ArrowDown if open => Some(Nav::Down),
                Key::Enter if open => Some(Nav::Confirm),
                Key::Escape if open => Some(Nav::Cancel),
                _ => None,
            };
            match nav {
                Some(n) => {
                    out.push(n);
                    false
                }
                None => true,
            }
        });
    });
    out
}

/// True while the Control key is held.
pub fn ctrl_down(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.modifiers.ctrl)
}

/// What the overlay's mouse use asks for.
pub enum Click {
    Window(WindowId),
    Dismiss,
}

struct Card {
    id: WindowId,
    title: String,
    graph: bool,
    hidden: bool,
}

/// What a card shows of its window.
enum Thumb {
    /// A graph drawn small (texture, width over height); the texture is dropped with the switcher.
    Image(TextureHandle, f32),
    /// The first cells of a worksheet: header row first.
    Table(Vec<Vec<String>>),
    /// No preview (too many graphs, or the drawing failed).
    Icon,
}

pub struct Switcher {
    cards: Vec<Card>,
    thumbs: HashMap<WindowId, Thumb>,
    selected: usize,
    /// Opened from the menu: stays until a card is chosen, Esc, or a click outside.
    pub sticky: bool,
    /// Cards per row, as drawn last (the Up and Down keys step by it).
    columns: usize,
}

impl Switcher {
    /// Builds the switcher for `ids` (already in most-recent-first order); `hidden` tells the
    /// windows that are closed. Needs two or more windows: with fewer, Ctrl+Tab does nothing.
    /// `backward` starts on the last card (Ctrl+Shift+Tab), otherwise on the second (the
    /// previous window).
    pub fn open(
        ctx: &egui::Context,
        project: &Project,
        ids: &[WindowId],
        hidden: impl Fn(WindowId) -> bool,
        backward: bool,
        sticky: bool,
    ) -> Option<Self> {
        let cards: Vec<Card> = ids
            .iter()
            .filter_map(|id| {
                let w = project.window(*id)?;
                Some(Card { id: *id, title: w.title(), graph: matches!(w, Window::Graph(_)), hidden: hidden(*id) })
            })
            .collect();
        if cards.len() < 2 {
            return None;
        }
        let ppp = ctx.pixels_per_point();
        let mut graphs = 0;
        let mut thumbs = HashMap::new();
        for c in &cards {
            let thumb = if c.graph {
                graphs += 1;
                if graphs <= MAX_THUMBNAILS { graph_thumb(ctx, project, c.id, ppp) } else { Thumb::Icon }
            } else {
                table_thumb(project, c.id)
            };
            thumbs.insert(c.id, thumb);
        }
        let selected = if backward { cards.len() - 1 } else { 1 };
        Some(Self { cards, thumbs, selected, sticky, columns: 1 })
    }

    pub fn selected(&self) -> WindowId {
        self.cards[self.selected].id
    }

    /// The titles of the cards in order (tests).
    #[cfg(test)]
    pub fn titles(&self) -> Vec<&str> {
        self.cards.iter().map(|c| c.title.as_str()).collect()
    }

    /// Moves the selection; Confirm and Cancel are the caller's business.
    pub fn go(&mut self, nav: Nav) {
        let n = self.cards.len();
        let cols = self.columns.max(1);
        self.selected = match nav {
            Nav::Next | Nav::Right => (self.selected + 1) % n,
            Nav::Prev | Nav::Left => (self.selected + n - 1) % n,
            Nav::Down => (self.selected + cols).min(n - 1),
            Nav::Up => self.selected.saturating_sub(cols),
            Nav::Confirm | Nav::Cancel => self.selected,
        };
    }

    /// Draws the overlay. Returns what the mouse chose, if anything.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Click> {
        let screen = ctx.content_rect();
        let mut result = None;
        // A backdrop that dims the workspace and keeps clicks from reaching it.
        egui::Area::new(egui::Id::new("switcher_backdrop")).order(egui::Order::Foreground).fixed_pos(screen.min).show(
            ctx,
            |ui| {
                let (rect, response) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(90));
                if response.clicked() {
                    result = Some(Click::Dismiss);
                }
            },
        );
        let max_cols = (((screen.width() * 0.85) / (CARD_W + GAP)).floor() as usize).clamp(1, 6);
        let cols = self.cards.len().min(max_cols);
        self.columns = cols;
        let rows = self.cards.len().div_ceil(cols);
        let card_h = THUMB_H + LABEL_H + 10.0;
        let size = vec2(cols as f32 * (CARD_W + GAP) + GAP, rows as f32 * (card_h + GAP) + GAP + 24.0);
        let top_left = screen.center() - size / 2.0;
        egui::Area::new(egui::Id::new("switcher_panel")).order(egui::Order::Tooltip).fixed_pos(top_left).show(
            ctx,
            |ui| {
                let (panel, _) = ui.allocate_exact_size(size, Sense::hover());
                let p = ui.painter();
                p.rect_filled(panel, 6.0, theme::PANEL);
                p.rect_stroke(panel, 6.0, Stroke::new(1.0_f32, theme::BORDER), egui::StrokeKind::Inside);
                p.text(
                    panel.center_top() + vec2(0.0, 14.0),
                    egui::Align2::CENTER_CENTER,
                    "Window Switcher",
                    egui::FontId::proportional(12.0),
                    Color32::from_gray(90),
                );
                for (k, card) in self.cards.iter().enumerate() {
                    let (r, c) = (k / cols, k % cols);
                    let min = panel.min + vec2(GAP + c as f32 * (CARD_W + GAP), 24.0 + GAP + r as f32 * (card_h + GAP));
                    let rect = Rect::from_min_size(min, vec2(CARD_W, card_h));
                    let response = ui.interact(rect, egui::Id::new(("switcher_card", card.id.0)), Sense::click());
                    let name = card.title.clone();
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Switch to {name}"))
                    });
                    if response.clicked() {
                        result = Some(Click::Window(card.id));
                    }
                    paint_card(ui, rect, card, self.thumbs.get(&card.id), k == self.selected, response.hovered());
                }
            },
        );
        result
    }
}

fn paint_card(ui: &egui::Ui, rect: Rect, card: &Card, thumb: Option<&Thumb>, selected: bool, hovered: bool) {
    let p = ui.painter();
    let fill = if hovered && !selected { Color32::from_rgb(232, 240, 250) } else { Color32::WHITE };
    p.rect_filled(rect, 3.0, fill);
    p.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, theme::BORDER), egui::StrokeKind::Inside);
    let area = Rect::from_min_size(rect.min + vec2((CARD_W - THUMB_W) / 2.0, 6.0), vec2(THUMB_W, THUMB_H));
    let dim = |c: Color32| if card.hidden { c.gamma_multiply(0.55) } else { c };
    match thumb {
        Some(Thumb::Image(texture, aspect)) => {
            let size = if *aspect > THUMB_W / THUMB_H {
                vec2(THUMB_W, THUMB_W / aspect)
            } else {
                vec2(THUMB_H * aspect, THUMB_H)
            };
            let img = Rect::from_center_size(area.center(), size);
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            p.image(texture.id(), img, uv, dim(Color32::WHITE));
            p.rect_stroke(img, 0.0, Stroke::new(1.0_f32, Color32::from_gray(200)), egui::StrokeKind::Outside);
        }
        Some(Thumb::Table(rows)) => paint_table(p, area, rows, card.hidden),
        _ => {
            p.rect_filled(area, 0.0, Color32::from_gray(245));
            crate::workspace::paint_window_icon(p, Rect::from_center_size(area.center(), vec2(32.0, 32.0)), card.graph);
        }
    }
    let kind = if card.graph { "Graph" } else { "Workbook" };
    let kind = if card.hidden { format!("{kind} (hidden)") } else { kind.to_string() };
    let name_at = egui::pos2(rect.left() + 8.0, rect.top() + THUMB_H + 16.0);
    let name = fit_text(&card.title, 22);
    p.text(name_at, egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(12.0), dim(Color32::from_gray(20)));
    p.text(
        name_at + vec2(0.0, 14.0),
        egui::Align2::LEFT_CENTER,
        kind,
        egui::FontId::proportional(10.0),
        Color32::from_gray(120),
    );
    if selected {
        p.rect_stroke(rect.shrink(1.0), 3.0, Stroke::new(3.0_f32, theme::ACCENT), egui::StrokeKind::Inside);
    }
}

fn fit_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}...", text.chars().take(max - 3).collect::<String>())
    }
}

/// A miniature worksheet: grid lines and the first cells in small text.
fn paint_table(p: &egui::Painter, area: Rect, rows: &[Vec<String>], hidden: bool) {
    p.rect_filled(area, 0.0, Color32::WHITE);
    let cols = rows.first().map_or(0, Vec::len).max(1);
    let row_h = area.height() / (rows.len().max(1) as f32);
    let col_w = area.width() / cols as f32;
    let ink = if hidden { Color32::from_gray(150) } else { Color32::from_gray(60) };
    let line = Stroke::new(1.0_f32, Color32::from_gray(214));
    for (r, row) in rows.iter().enumerate() {
        let y = area.top() + r as f32 * row_h;
        if r == 0 {
            let head = Rect::from_min_size(egui::pos2(area.left(), y), vec2(area.width(), row_h));
            p.rect_filled(head, 0.0, Color32::from_gray(232));
        }
        p.hline(area.x_range(), y + row_h, line);
        for (c, text) in row.iter().enumerate() {
            let x = area.left() + c as f32 * col_w;
            p.vline(x, area.y_range(), line);
            let clipped = p.with_clip_rect(Rect::from_min_size(egui::pos2(x, y), vec2(col_w, row_h)));
            let at = egui::pos2(x + 3.0, y + row_h / 2.0);
            clipped.text(at, egui::Align2::LEFT_CENTER, text, egui::FontId::proportional(9.0), ink);
        }
    }
    p.rect_stroke(area, 0.0, Stroke::new(1.0_f32, Color32::from_gray(190)), egui::StrokeKind::Outside);
}

fn graph_thumb(ctx: &egui::Context, project: &Project, id: WindowId, ppp: f32) -> Thumb {
    let Some(g) = project.graph(id) else { return Thumb::Icon };
    let (w, h) = (g.page.width_pt as f32, g.page.height_pt as f32);
    if w <= 0.0 || h <= 0.0 {
        return Thumb::Icon;
    }
    let scale = (THUMB_W / w).min(THUMB_H / h) * ppp;
    match sheaf_plot::render_raster(project, id, f64::from(scale)) {
        Ok(r) => {
            let image = egui::ColorImage::from_rgba_unmultiplied([r.width as usize, r.height as usize], &r.rgba);
            let aspect = r.width as f32 / r.height.max(1) as f32;
            Thumb::Image(ctx.load_texture(format!("switcher-{}", id.0), image, TextureOptions::LINEAR), aspect)
        }
        Err(_) => Thumb::Icon,
    }
}

fn table_thumb(project: &Project, id: WindowId) -> Thumb {
    let Some(book) = project.workbook(id) else { return Thumb::Icon };
    let sheet = book.sheet();
    if sheet.columns.is_empty() {
        return Thumb::Icon;
    }
    let cols: Vec<_> = sheet.columns.iter().take(4).collect();
    let mut rows = vec![cols.iter().map(|c| c.short_name.clone()).collect::<Vec<_>>()];
    for r in 0..5 {
        rows.push(cols.iter().map(|c| c.cell_text(r)).collect());
    }
    Thumb::Table(rows)
}
