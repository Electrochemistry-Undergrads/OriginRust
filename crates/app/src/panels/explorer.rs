//! The Project Explorer: a folder tree over a list of the windows of the selected folder, with
//! the columns Name, Long Name, Short Name and Type, the active window in bold. A double click
//! shows and activates a window, the right-click menu has Origin's window commands, and a
//! window can be dragged onto a folder.

use super::PanelData;
use crate::actions::Action;
use crate::theme;
use crate::workspace;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use sheaf_core::project::{Folder, Window};
use sheaf_core::{Project, WindowId};
use std::cmp::Ordering;

const ROW_H: f32 = 18.0;
const HEAD_FILL: Color32 = Color32::from_rgb(240, 240, 243);
const GRID: Color32 = Color32::from_rgb(222, 222, 226);

/// The columns of the window list, in order.
/// T350: Origin's Explorer also lists the window's Comments.
const COLUMNS: [&str; 5] = ["Name", "Long Name", "Short Name", "Type", "Comments"];
/// Share of the list width each column takes.
const SHARES: [f32; 5] = [0.2, 0.21, 0.17, 0.14, 0.28];

pub struct ExplorerState {
    /// The selected folder, as indices into the `folders` of each level; empty is the root.
    pub folder: Vec<usize>,
    /// The highlighted row of the window list.
    pub selected: Option<WindowId>,
    /// The column the list is sorted by and the direction.
    sort: (usize, bool),
    /// True after a click in the explorer, until a click elsewhere: it has the keyboard.
    pub focus: bool,
    /// The highlighted windows (T286): one after a plain click, more with Ctrl+click, Shift+click and
    /// Ctrl+A. `selected` is the last one clicked, where a Shift+click run starts.
    pub picked: Vec<WindowId>,
    /// The search box over the window list (T288): only windows whose name, long name or comments
    /// hold this text (any case) are listed; empty lists them all.
    pub search: String,
    /// The search also looks in the subfolders of the folder (T288).
    pub search_subfolders: bool,
    /// The Favorites folder is selected (T130): the list shows the shortcuts of `Project::favorites`.
    pub favorites: bool,
    /// The windows drawn in bold during the last frame (the active one), for the tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub bold: Vec<WindowId>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        ExplorerState {
            folder: Vec::new(),
            selected: None,
            sort: (0, true),
            focus: false,
            picked: Vec::new(),
            search: String::new(),
            search_subfolders: false,
            favorites: false,
            bold: Vec::new(),
        }
    }
}

impl ExplorerState {
    /// The windows the list shows (T288): those of Favorites or of the folder (and, while searching
    /// with Subfolders ticked, of its subfolders) whose name, long name or comments hold the search
    /// text, in any case. Not sorted.
    pub fn listed(&self, project: &Project) -> Vec<WindowId> {
        let base = if self.favorites {
            project.favorites.clone()
        } else if self.search_subfolders && !self.search.trim().is_empty() {
            project
                .windows
                .iter()
                .map(Window::id)
                .filter(|id| home(&project.root, *id).starts_with(&self.folder))
                .collect()
        } else {
            windows_in(project, &self.folder)
        };
        let text = self.search.trim().to_lowercase();
        if text.is_empty() {
            return base;
        }
        base.into_iter()
            .filter(|id| {
                project.window(*id).is_some_and(|w| {
                    [cell(w, 0), cell(w, 1), project.window_comments(*id).to_string()]
                        .iter()
                        .any(|t| t.to_lowercase().contains(&text))
                })
            })
            .collect()
    }

    /// Nothing highlighted.
    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.picked.clear();
    }

    /// Whether the window's row is highlighted.
    pub fn is_picked(&self, id: WindowId) -> bool {
        self.picked.contains(&id)
    }

    /// The windows an action on `id`'s row acts on: every highlighted window when the row is one
    /// of them, else that window alone (T286).
    pub fn targets(&self, id: WindowId) -> Vec<WindowId> {
        if self.is_picked(id) { self.picked.clone() } else { vec![id] }
    }

    /// A click on the row of `id` with these modifiers, `shown` being the rows in their shown order
    /// (T286, as in Windows lists): a plain click highlights the row alone, Ctrl+click adds or removes
    /// it, Shift+click highlights the run from the last clicked row to it.
    pub fn click(&mut self, id: WindowId, ctrl: bool, shift: bool, shown: &[WindowId]) {
        let anchor = self.selected.and_then(|a| shown.iter().position(|w| *w == a));
        match (shift, anchor, shown.iter().position(|w| *w == id)) {
            (true, Some(a), Some(b)) => {
                let run = shown[a.min(b)..=a.max(b)].to_vec();
                if ctrl {
                    for w in run {
                        if !self.picked.contains(&w) {
                            self.picked.push(w);
                        }
                    }
                } else {
                    self.picked = run;
                }
                // The run's start stays where it was, as in Windows lists.
                return;
            }
            _ if ctrl => {
                if let Some(i) = self.picked.iter().position(|w| *w == id) {
                    self.picked.remove(i);
                } else {
                    self.picked.push(id);
                }
            }
            _ => self.picked = vec![id],
        }
        self.selected = Some(id);
    }
}

// ----- the folder model ------------------------------------------------------------------

/// The folder at this path of indices.
pub fn folder_at<'a>(root: &'a Folder, path: &[usize]) -> Option<&'a Folder> {
    let mut f = root;
    for i in path {
        f = f.folders.get(*i)?;
    }
    Some(f)
}

pub fn folder_at_mut<'a>(root: &'a mut Folder, path: &[usize]) -> Option<&'a mut Folder> {
    let mut f = root;
    for i in path {
        f = f.folders.get_mut(*i)?;
    }
    Some(f)
}

/// The path of the folder that lists the window; the root (empty path) for a window no folder
/// lists, as the model says.
pub fn home(root: &Folder, id: WindowId) -> Vec<usize> {
    fn find(f: &Folder, id: WindowId, path: &mut Vec<usize>) -> bool {
        for (i, c) in f.folders.iter().enumerate() {
            path.push(i);
            if c.windows.contains(&id) || find(c, id, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = Vec::new();
    if find(root, id, &mut path) { path } else { Vec::new() }
}

/// The windows of one folder (not of its subfolders), in creation order.
pub fn windows_in(project: &Project, path: &[usize]) -> Vec<WindowId> {
    project.windows.iter().map(Window::id).filter(|id| home(&project.root, *id) == path).collect()
}

/// The windows the workspace shows for the active folder `path` under View > View Windows (T100);
/// `None` when every window shows.
pub fn shown_windows(project: &Project, path: &[usize]) -> Option<Vec<WindowId>> {
    use sheaf_core::project::ViewWindows;
    match project.view_windows {
        ViewWindows::None => None,
        ViewWindows::ActiveFolder => Some(windows_in(project, path)),
        ViewWindows::ActiveFolderAndSubfolders => Some(
            project.windows.iter().map(Window::id).filter(|id| home(&project.root, *id).starts_with(path)).collect(),
        ),
    }
}

/// Every folder of the project with its path and its name as Window > Folders shows it
/// (`/Project/Sub/`), the root first.
pub fn folder_list(root: &Folder) -> Vec<(Vec<usize>, String)> {
    fn walk(f: &Folder, path: &mut Vec<usize>, name: String, out: &mut Vec<(Vec<usize>, String)>) {
        out.push((path.clone(), name.clone()));
        for (i, c) in f.folders.iter().enumerate() {
            path.push(i);
            walk(c, path, format!("{name}{}/", c.name), out);
            path.pop();
        }
    }
    let mut out = Vec::new();
    walk(root, &mut Vec::new(), format!("/{}/", root.name), &mut out);
    out
}

/// Every window listed in the folder or below it.
pub fn subtree_windows(f: &Folder) -> Vec<WindowId> {
    let mut out = f.windows.clone();
    for c in &f.folders {
        out.extend(subtree_windows(c));
    }
    out
}

/// What the keys of the explorer ask for: F2 renames the last clicked window, Delete deletes the
/// highlighted windows, Ctrl+A highlights every window of the folder. Only while the explorer
/// has the keyboard focus (the last click was in it).
pub fn handle_keys(ctx: &egui::Context, st: &mut ExplorerState, project: &Project, actions: &mut Vec<Action>) {
    if !st.focus {
        return;
    }
    let (rename, delete, all) = ctx.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::NONE, egui::Key::F2),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::A),
        )
    });
    if all {
        // Every listed window: of the folder or Favorites, as the search leaves them (T288).
        st.picked = st.listed(project);
        st.selected = st.picked.first().copied();
    }
    if rename {
        match st.selected {
            Some(id) => actions.push(Action::RenameWindow(id)),
            None if !st.folder.is_empty() => actions.push(Action::RenameFolder(st.folder.clone())),
            None => {}
        }
    }
    if delete {
        if !st.picked.is_empty() {
            actions.extend(st.picked.iter().copied().map(Action::DeleteWindow));
        } else if let Some(id) = st.selected {
            actions.push(Action::DeleteWindow(id));
        }
        st.clear_selection();
    }
}

/// Takes the window out of every folder (it then belongs to the root).
pub fn unlist(f: &mut Folder, id: WindowId) {
    f.windows.retain(|w| *w != id);
    f.folders.iter_mut().for_each(|c| unlist(c, id));
}

/// The root folder's name as the explorer shows it: the project's name (Origin shows
/// `UNTITLED` for a project that was not saved yet).
pub fn root_label(project: &Project, project_name: &str) -> String {
    if project.root.name.is_empty() || project.root.name == "Project" {
        project_name.to_string()
    } else {
        project.root.name.clone()
    }
}

/// Compares names so that `Book2` comes before `Book10`.
fn natural(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(p), Some(q)) if p.is_ascii_digit() && q.is_ascii_digit() => {
                let num = |it: &mut std::iter::Peekable<std::str::Chars<'_>>| {
                    let mut n = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        n.push(c);
                        it.next();
                    }
                    n.trim_start_matches('0').to_string()
                };
                let (m, n) = (num(&mut x), num(&mut y));
                let o = m.len().cmp(&n.len()).then_with(|| m.cmp(&n));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(p), Some(q)) => {
                let o = p.to_ascii_lowercase().cmp(&q.to_ascii_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// The text of one cell of the window list, the Comments column (4) included: the first line of the
/// window's comments (T350).
pub fn cell_text(project: &Project, w: &Window, column: usize) -> String {
    if column == 4 {
        return project.window_comments(w.id()).lines().next().unwrap_or("").to_string();
    }
    cell(w, column)
}

/// The text of one cell of the window list (the columns that the window itself holds).
pub fn cell(w: &Window, column: usize) -> String {
    let long = match w {
        Window::Workbook(b) => &b.long_name,
        Window::Graph(g) => &g.long_name,
        Window::Notes(n) => &n.long_name,
        Window::Matrix(m) => &m.long_name,
        Window::Layout(l) => &l.long_name,
    };
    match column {
        0 | 2 => w.name().to_string(),
        1 => long.clone(),
        _ => match w {
            Window::Workbook(_) => "Workbook".to_string(),
            Window::Graph(_) => "Graph".to_string(),
            Window::Notes(_) => "Notes".to_string(),
            Window::Matrix(_) => "Matrix".to_string(),
            Window::Layout(_) => "Layout".to_string(),
        },
    }
}

fn sorted(project: &Project, mut ids: Vec<WindowId>, sort: (usize, bool)) -> Vec<WindowId> {
    ids.sort_by(|a, b| {
        let (Some(wa), Some(wb)) = (project.window(*a), project.window(*b)) else {
            return Ordering::Equal;
        };
        let o = natural(&cell_text(project, wa, sort.0), &cell_text(project, wb, sort.0))
            .then_with(|| natural(wa.name(), wb.name()));
        if sort.1 { o } else { o.reverse() }
    });
    ids
}

// ----- drawing ---------------------------------------------------------------------------

pub fn show(ui: &mut egui::Ui, st: &mut ExplorerState, data: &PanelData, actions: &mut Vec<Action>) {
    if folder_at(&data.project.root, &st.folder).is_none() {
        st.folder.clear();
    }
    st.bold.clear();
    let (pressed, at) = ui.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos()));
    if pressed {
        st.focus = at.is_some_and(|p| ui.max_rect().contains(p));
    }
    egui::TopBottomPanel::top(ui.id().with("tree"))
        .resizable(true)
        .default_height(110.0)
        .height_range(40.0..=400.0)
        .frame(egui::Frame::new().fill(Color32::WHITE))
        .show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                let mut path = Vec::new();
                tree(ui, st, data, &data.project.root, &mut path, actions);
                favorites_row(ui, st);
            });
        });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::WHITE)).show_inside(ui, |ui| {
        list(ui, st, data, actions);
    });
}

fn tree(
    ui: &mut egui::Ui,
    st: &mut ExplorerState,
    data: &PanelData,
    folder: &Folder,
    path: &mut Vec<usize>,
    actions: &mut Vec<Action>,
) {
    let name = if path.is_empty() { root_label(data.project, &data.project_name) } else { folder.name.clone() };
    let mut row = |ui: &mut egui::Ui, st: &mut ExplorerState| {
        let selected = !st.favorites && st.folder == *path;
        let galley = ui.painter().layout_no_wrap(name.clone(), FontId::proportional(12.0), Color32::BLACK);
        let size = Vec2::new(galley.size().x + 26.0, ROW_H);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &name));
        let dropping = response.dnd_hover_payload::<WindowId>().is_some();
        if selected || dropping || response.hovered() {
            let fill = if dropping { Color32::from_rgb(204, 228, 247) } else { Color32::from_rgb(229, 241, 251) };
            ui.painter().rect_filled(rect, 0.0, fill);
        }
        if selected || dropping {
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0_f32, theme::ACCENT), egui::StrokeKind::Inside);
        }
        paint_folder(ui.painter(), Rect::from_min_size(rect.min + Vec2::new(3.0, 3.0), Vec2::splat(12.0)));
        ui.painter().galley(
            Pos2::new(rect.min.x + 20.0, rect.center().y - galley.size().y / 2.0),
            galley,
            Color32::BLACK,
        );
        if response.clicked() {
            st.folder = path.clone();
            st.favorites = false;
            st.clear_selection();
        }
        if let Some(window) = response.dnd_release_payload::<WindowId>() {
            // T286: dragging a highlighted row moves every highlighted window.
            for window in st.targets(*window) {
                actions.push(Action::MoveWindow { window, folder: path.clone() });
            }
        }
        response.context_menu(|ui| {
            st.folder = path.clone();
            if ui.button("New Folder").clicked() {
                actions.push(Action::NewFolder(path.clone()));
                ui.close();
            }
            let not_root = !path.is_empty();
            if ui.add_enabled(not_root, egui::Button::new("Rename")).clicked() {
                actions.push(Action::RenameFolder(path.clone()));
                ui.close();
            }
            if ui.add_enabled(not_root, egui::Button::new("Delete Folder")).clicked() {
                actions.push(Action::DeleteFolder(path.clone()));
                ui.close();
            }
            if ui.button("Properties...").clicked() {
                actions.push(Action::FolderProperties(path.clone()));
                ui.close();
            }
        });
    };
    if folder.folders.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().icon_width + ui.spacing().item_spacing.x);
            row(ui, st);
        });
    } else {
        let id = ui.make_persistent_id(("explorer_folder", path.clone()));
        let state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);
        state.show_header(ui, |ui| row(ui, st)).body(|ui| {
            for (i, child) in folder.folders.iter().enumerate() {
                path.push(i);
                tree(ui, st, data, child, path, actions);
                path.pop();
            }
        });
    }
}

/// The Favorites folder under the project's folders (T130): a click selects it and the list shows the
/// shortcuts.
fn favorites_row(ui: &mut egui::Ui, st: &mut ExplorerState) {
    ui.horizontal(|ui| {
        ui.add_space(ui.spacing().icon_width + ui.spacing().item_spacing.x);
        let name = "Favorites";
        let galley = ui.painter().layout_no_wrap(name.into(), FontId::proportional(12.0), Color32::BLACK);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(galley.size().x + 26.0, ROW_H), Sense::click());
        response
            .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, st.favorites, name));
        if st.favorites || response.hovered() {
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(229, 241, 251));
        }
        if st.favorites {
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0_f32, theme::ACCENT), egui::StrokeKind::Inside);
        }
        paint_folder(ui.painter(), Rect::from_min_size(rect.min + Vec2::new(3.0, 3.0), Vec2::splat(12.0)));
        let at = Pos2::new(rect.min.x + 20.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, Color32::BLACK);
        if response.clicked() {
            st.favorites = true;
            st.clear_selection();
        }
    });
}

fn paint_folder(painter: &egui::Painter, r: Rect) {
    let edge = Stroke::new(1.0_f32, Color32::from_rgb(160, 130, 40));
    let fill = Color32::from_rgb(250, 215, 110);
    let tab = Rect::from_min_size(r.min, Vec2::new(r.width() * 0.45, 3.0));
    painter.rect_filled(tab, 0.0, fill);
    painter.rect_stroke(tab, 0.0, edge, egui::StrokeKind::Inside);
    let body = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 2.0), r.max);
    painter.rect_filled(body, 0.0, fill);
    painter.rect_stroke(body, 0.0, edge, egui::StrokeKind::Inside);
}

fn list(ui: &mut egui::Ui, st: &mut ExplorerState, data: &PanelData, actions: &mut Vec<Action>) {
    let width = ui.available_width();
    let mut edges = [0.0_f32; 6];
    for i in 0..5 {
        edges[i + 1] = edges[i] + SHARES[i] * width;
    }
    edges[5] = width;
    // T288: the search box, and whether it looks in the subfolders too.
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(&mut st.search)
            .hint_text("Search windows")
            .desired_width((width - 110.0).max(60.0));
        let response = ui.add(edit).on_hover_text("Name, long name or comments holding this text (any case)");
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search windows"));
        if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            st.search.clear();
        }
        ui.checkbox(&mut st.search_subfolders, "Subfolders")
            .on_hover_text("Search the folder and every folder below it");
    });
    // The header row; a click on a name sorts by it.
    let (head, _) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::hover());
    ui.painter().rect_filled(head, 0.0, HEAD_FILL);
    for (c, title) in COLUMNS.iter().enumerate() {
        let cell_rect = Rect::from_min_max(
            Pos2::new(head.min.x + edges[c], head.min.y),
            Pos2::new(head.min.x + edges[c + 1], head.max.y),
        );
        let response = ui.interact(cell_rect, ui.id().with(("column_head", c)), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Sort by {title}")));
        if response.hovered() {
            ui.painter().rect_filled(cell_rect, 0.0, Color32::from_rgb(229, 241, 251));
        }
        ui.painter().vline(cell_rect.right(), cell_rect.y_range(), Stroke::new(1.0_f32, GRID));
        let text_pos = Pos2::new(cell_rect.left() + 4.0, cell_rect.center().y);
        ui.painter().with_clip_rect(cell_rect).text(
            text_pos,
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(12.0),
            Color32::BLACK,
        );
        if st.sort.0 == c {
            paint_arrow(ui.painter(), Pos2::new(cell_rect.right() - 9.0, cell_rect.center().y), st.sort.1);
        }
        if response.clicked() {
            st.sort = if st.sort.0 == c { (c, !st.sort.1) } else { (c, true) };
        }
    }
    ui.painter().hline(head.x_range(), head.bottom(), Stroke::new(1.0_f32, GRID));

    // Deleted windows leave the selection.
    st.picked.retain(|w| data.project.window(*w).is_some());
    let ids = sorted(data.project, st.listed(data.project), st.sort);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for &id in &ids {
            let Some(w) = data.project.window(id) else { continue };
            row(ui, st, data, w, &edges, &ids, actions);
        }
    });
}

fn paint_arrow(painter: &egui::Painter, c: Pos2, up: bool) {
    let d = if up { -1.5 } else { 1.5 };
    painter.add(egui::Shape::convex_polygon(
        vec![c + Vec2::new(-3.0, -d), c + Vec2::new(3.0, -d), c + Vec2::new(0.0, d)],
        Color32::from_rgb(90, 90, 96),
        Stroke::NONE,
    ));
}

fn row(
    ui: &mut egui::Ui,
    st: &mut ExplorerState,
    data: &PanelData,
    w: &Window,
    edges: &[f32; 6],
    shown: &[WindowId],
    actions: &mut Vec<Action>,
) {
    let id = w.id();
    let graph = matches!(w, Window::Graph(_));
    let active = data.active == Some(id);
    let selected = st.is_picked(id);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(edges[5], ROW_H), Sense::click_and_drag());
    let title = w.title();
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &title));
    // T249: the window's Comments as the row's tooltip; while searching the subfolders, the folder
    // the window is in (T288).
    let mut tip = data.project.window_comments(id).to_string();
    let path = home(&data.project.root, id);
    if st.search_subfolders && !st.search.trim().is_empty() && path != st.folder {
        let folder = folder_list(&data.project.root).into_iter().find(|(p, _)| *p == path).map(|(_, n)| n);
        let line = format!("In {}", folder.unwrap_or_default());
        tip = if tip.is_empty() { line } else { format!("{tip}\n{line}") };
    }
    // Sticky notes (T450): the window's note, or its first sheet note, marks the row and joins the tooltip.
    let note = match w {
        Window::Graph(g) => g.sticky.as_ref(),
        Window::Workbook(b) => b.sticky.as_ref().or_else(|| b.sheets.iter().find_map(|s| s.sticky.as_ref())),
        Window::Matrix(m) => m.sticky.as_ref(),
        Window::Layout(l) => l.sticky.as_ref(),
        _ => None,
    };
    if let Some(n) = note {
        let line = format!("Note: {}", n.first_line());
        tip = if tip.is_empty() { line } else { format!("{tip}\n{line}") };
    }
    let response = if tip.is_empty() { response } else { response.on_hover_text(tip) };
    if selected {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(204, 228, 247));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(229, 241, 251));
    }
    if active {
        st.bold.push(id);
    }
    for c in 0..5 {
        let cell_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + edges[c], rect.min.y),
            Pos2::new(rect.min.x + edges[c + 1], rect.max.y),
        );
        let mut x = cell_rect.left() + 4.0;
        if c == 0 {
            let icon = Rect::from_min_size(Pos2::new(x, rect.center().y - 7.0), Vec2::splat(14.0));
            workspace::paint_window_icon(ui.painter(), icon, graph);
            if let Some(n) = note {
                let c = n.color.rgb();
                let dot = Rect::from_min_size(icon.right_bottom() - Vec2::splat(6.0), Vec2::splat(7.0));
                ui.painter().rect_filled(dot, 1.0, Color32::from_rgb(c.r, c.g, c.b));
                ui.painter().rect_stroke(
                    dot,
                    1.0,
                    egui::Stroke::new(1.0_f32, Color32::from_gray(90)),
                    egui::StrokeKind::Inside,
                );
            }
            x += 18.0;
        }
        let text = cell_text(data.project, w, c);
        let painter = ui.painter().with_clip_rect(cell_rect);
        let at = Pos2::new(x, rect.center().y);
        painter.text(at, Align2::LEFT_CENTER, &text, FontId::proportional(12.0), Color32::BLACK);
        if active {
            // egui has no bold face: the same text a little to the right makes it heavier.
            painter.text(
                at + Vec2::new(0.6, 0.0),
                Align2::LEFT_CENTER,
                &text,
                FontId::proportional(12.0),
                Color32::BLACK,
            );
        }
    }
    response.dnd_set_drag_payload(id);
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if response.clicked() {
        let m = crate::grid::click_modifiers(ui);
        st.click(id, m.command, m.shift, shown);
    } else if (response.secondary_clicked() || response.drag_started()) && !st.is_picked(id) {
        // A right-click or a drag on a row that is not highlighted takes it alone; on a highlighted
        // row it keeps the selection, so the menu and the drop act on all of it.
        st.click(id, false, false, shown);
    }
    if response.double_clicked() {
        actions.push(Action::Focus(id));
    }
    // T286: Show, Hide and Delete act on every highlighted window.
    let targets = st.targets(id);
    let favorites = st.favorites;
    response.context_menu(|ui| {
        let mut item = |ui: &mut egui::Ui, label: &str, chosen: Vec<Action>| {
            if ui.button(label).clicked() {
                actions.extend(chosen);
                ui.close();
            }
        };
        let each = |make: fn(WindowId) -> Action| targets.iter().copied().map(make).collect::<Vec<_>>();
        item(ui, "Show", each(Action::Focus));
        item(ui, "Hide", each(Action::HideWindow));
        item(ui, "Rename...", vec![Action::RenameWindow(id)]);
        item(ui, "Delete", each(Action::DeleteWindow));
        item(ui, "Duplicate", vec![Action::DuplicateWindow(id)]);
        if favorites {
            item(ui, "Remove from Favorites", vec![Action::RemoveFavorite(id)]);
        } else {
            item(ui, "Add Shortcut to Favorites", vec![Action::AddFavorite(id)]);
        }
        ui.separator();
        item(ui, "Export...", vec![if graph { Action::ExportGraph(id) } else { Action::ExportData(id) }]);
        ui.separator();
        item(ui, "Properties...", vec![Action::RenameWindow(id)]);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::project::Folder;

    fn project() -> (Project, WindowId, WindowId) {
        let mut p = Project::new();
        let a = p.add_workbook("alpha");
        let b = p.add_workbook("beta");
        (p, a, b)
    }

    #[test]
    fn a_window_no_folder_lists_belongs_to_the_root() {
        let (mut p, a, b) = project();
        assert_eq!(windows_in(&p, &[]), [a, b]);
        p.root.folders.push(Folder { name: "Runs".into(), windows: vec![b], ..Folder::default() });
        assert_eq!(windows_in(&p, &[]), [a]);
        assert_eq!(windows_in(&p, &[0]), [b]);
        assert_eq!(home(&p.root, b), [0]);
        assert_eq!(home(&p.root, a), Vec::<usize>::new());
    }

    #[test]
    fn nested_folders_have_their_own_windows() {
        let (mut p, a, b) = project();
        let inner = Folder { name: "Deep".into(), windows: vec![a], ..Folder::default() };
        p.root.folders.push(Folder {
            name: "Outer".into(),
            folders: vec![inner],
            windows: vec![b],
            ..Folder::default()
        });
        assert_eq!(windows_in(&p, &[0, 0]), [a]);
        assert_eq!(windows_in(&p, &[0]), [b]);
        assert!(folder_at(&p.root, &[0, 0]).is_some());
        assert!(folder_at(&p.root, &[0, 1]).is_none());
        unlist(&mut p.root, a);
        assert_eq!(windows_in(&p, &[]), [a]);
    }

    #[test]
    fn ctrl_and_shift_clicks_pick_windows_as_windows_lists_do() {
        // T286.
        let (mut p, a, b) = project();
        let c = p.add_workbook("gamma");
        let d = p.add_workbook("delta");
        let shown = [a, b, c, d];
        let mut st = ExplorerState::default();
        st.click(b, false, false, &shown);
        assert_eq!(st.picked, [b]);
        st.click(d, false, true, &shown);
        assert_eq!(st.picked, [b, c, d], "Shift+click: the run from the last click");
        assert_eq!(st.selected, Some(b), "the run keeps its start");
        st.click(a, false, true, &shown);
        assert_eq!(st.picked, [a, b], "a new run from the same start replaces the old");
        st.click(d, true, false, &shown);
        assert_eq!(st.picked, [a, b, d], "Ctrl+click adds");
        st.click(a, true, false, &shown);
        assert_eq!(st.picked, [b, d], "Ctrl+click removes");
        assert_eq!(st.targets(b), [b, d], "an action on a highlighted row takes them all");
        assert_eq!(st.targets(c), [c], "on another row only that one");
        st.click(c, false, false, &shown);
        assert_eq!(st.picked, [c]);
        st.clear_selection();
        assert!(st.picked.is_empty() && st.selected.is_none());
    }

    #[test]
    fn the_search_lists_the_windows_whose_names_or_comments_hold_the_text() {
        // T288.
        let (mut p, a, b) = project();
        let c = p.add_workbook("Gamma run");
        p.set_window_comments(b, "calibration of the Gamma detector");
        p.root.folders.push(Folder { name: "Runs".into(), windows: vec![c], ..Folder::default() });
        let mut st = ExplorerState::default();
        assert_eq!(st.listed(&p), [a, b], "no search: the root's own windows");
        st.search = "gamma".into();
        assert_eq!(st.listed(&p), [b], "the comments hold it; Gamma run is in a subfolder");
        st.search_subfolders = true;
        assert_eq!(st.listed(&p), [b, c], "with the subfolders");
        st.search = "  ALPHA ".into();
        assert_eq!(st.listed(&p), [a], "long name, any case, spaces trimmed");
        st.search = "Book3".into();
        assert_eq!(st.listed(&p), [c], "the short name");
        st.search = "nothing".into();
        assert!(st.listed(&p).is_empty());
    }

    #[test]
    fn names_sort_with_numbers_in_their_natural_order() {
        assert_eq!(natural("Book2", "Book10"), Ordering::Less);
        assert_eq!(natural("book2", "Book2"), Ordering::Equal);
        assert_eq!(natural("Graph1", "Book9"), Ordering::Greater);
        assert_eq!(natural("Book", "Book1"), Ordering::Less);
    }

    #[test]
    fn the_root_takes_the_project_name_unless_the_model_names_it() {
        let (mut p, _, _) = project();
        assert_eq!(root_label(&p, "UNTITLED"), "UNTITLED");
        p.root.name = "Lab".into();
        assert_eq!(root_label(&p, "UNTITLED"), "Lab");
    }

    #[test]
    fn the_list_cells_hold_name_long_name_short_name_type_and_comments() {
        let (mut p, a, b) = project();
        let cells = |p: &Project, id| {
            let w = p.window(id).unwrap();
            COLUMNS.iter().enumerate().map(|(i, _)| cell_text(p, w, i)).collect::<Vec<_>>()
        };
        assert_eq!(cells(&p, a), ["Book1", "alpha", "Book1", "Workbook", ""]);
        // T350: the first line of the window's comments, and the list sorts by it.
        p.set_window_comments(a, "zeta run\nsecond line");
        p.set_window_comments(b, "alpha run");
        assert_eq!(cells(&p, a)[4], "zeta run");
        assert_eq!(sorted(&p, vec![a, b], (4, true)), [b, a]);
        assert_eq!(sorted(&p, vec![a, b], (4, false)), [a, b]);
    }
}
