//! The Project Explorer: a folder tree over a list of the windows of the selected folder, with
//! the columns Name, Long Name, Short Name and Type, the active window in bold. A double click
//! shows and activates a window, the right-click menu has Origin's window commands, and a
//! window can be dragged onto a folder.

use super::PanelData;
use crate::actions::Action;
use crate::menus::NOT_YET;
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
const COLUMNS: [&str; 4] = ["Name", "Long Name", "Short Name", "Type"];
/// Share of the list width each column takes.
const SHARES: [f32; 4] = [0.24, 0.27, 0.27, 0.22];

pub struct ExplorerState {
    /// The selected folder, as indices into the `folders` of each level; empty is the root.
    pub folder: Vec<usize>,
    /// The highlighted row of the window list.
    pub selected: Option<WindowId>,
    /// The column the list is sorted by and the direction.
    sort: (usize, bool),
    /// The windows drawn in bold during the last frame (the active one), for the tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub bold: Vec<WindowId>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        ExplorerState { folder: Vec::new(), selected: None, sort: (0, true), bold: Vec::new() }
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

/// The text of one cell of the window list.
pub fn cell(w: &Window, column: usize) -> String {
    let long = match w {
        Window::Workbook(b) => &b.long_name,
        Window::Graph(g) => &g.long_name,
    };
    match column {
        0 | 2 => w.name().to_string(),
        1 => long.clone(),
        _ => match w {
            Window::Workbook(_) => "Workbook".to_string(),
            Window::Graph(_) => "Graph".to_string(),
        },
    }
}

fn sorted(project: &Project, mut ids: Vec<WindowId>, sort: (usize, bool)) -> Vec<WindowId> {
    ids.sort_by(|a, b| {
        let (Some(wa), Some(wb)) = (project.window(*a), project.window(*b)) else {
            return Ordering::Equal;
        };
        let o = natural(&cell(wa, sort.0), &cell(wb, sort.0)).then_with(|| natural(wa.name(), wb.name()));
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
    egui::TopBottomPanel::top(ui.id().with("tree"))
        .resizable(true)
        .default_height(110.0)
        .height_range(40.0..=400.0)
        .frame(egui::Frame::new().fill(Color32::WHITE))
        .show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                let mut path = Vec::new();
                tree(ui, st, data, &data.project.root, &mut path, actions);
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
        let selected = st.folder == *path;
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
        }
        if let Some(window) = response.dnd_release_payload::<WindowId>() {
            actions.push(Action::MoveWindow { window: *window, folder: path.clone() });
        }
        response.context_menu(|ui| {
            st.folder = path.clone();
            if ui.button("New Folder").clicked() {
                actions.push(Action::NewFolder(path.clone()));
                ui.close();
            }
            for label in ["Rename", "Delete Folder", "Properties..."] {
                ui.add_enabled(false, egui::Button::new(label)).on_disabled_hover_text(NOT_YET);
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
    let mut edges = [0.0_f32; 5];
    for i in 0..4 {
        edges[i + 1] = edges[i] + SHARES[i] * width;
    }
    edges[4] = width;
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

    let ids = sorted(data.project, windows_in(data.project, &st.folder), st.sort);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for id in ids {
            let Some(w) = data.project.window(id) else { continue };
            row(ui, st, data, w, &edges, actions);
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
    edges: &[f32; 5],
    actions: &mut Vec<Action>,
) {
    let id = w.id();
    let graph = matches!(w, Window::Graph(_));
    let active = data.active == Some(id);
    let selected = st.selected == Some(id);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(edges[4], ROW_H), Sense::click_and_drag());
    let title = w.title();
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &title));
    if selected {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(204, 228, 247));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(229, 241, 251));
    }
    if active {
        st.bold.push(id);
    }
    for c in 0..4 {
        let cell_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + edges[c], rect.min.y),
            Pos2::new(rect.min.x + edges[c + 1], rect.max.y),
        );
        let mut x = cell_rect.left() + 4.0;
        if c == 0 {
            let icon = Rect::from_min_size(Pos2::new(x, rect.center().y - 7.0), Vec2::splat(14.0));
            workspace::paint_window_icon(ui.painter(), icon, graph);
            x += 18.0;
        }
        let text = cell(w, c);
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
    if response.clicked() || response.secondary_clicked() || response.drag_started() {
        st.selected = Some(id);
    }
    if response.double_clicked() {
        actions.push(Action::Focus(id));
    }
    response.context_menu(|ui| {
        let mut item = |ui: &mut egui::Ui, label: &str, action: Option<Action>| {
            match action {
                Some(a) => {
                    if ui.button(label).clicked() {
                        actions.push(a);
                        ui.close();
                    }
                }
                None => {
                    ui.add_enabled(false, egui::Button::new(label)).on_disabled_hover_text(NOT_YET);
                }
            };
        };
        item(ui, "Show", Some(Action::Focus(id)));
        item(ui, "Hide", Some(Action::HideWindow(id)));
        item(ui, "Rename...", Some(Action::RenameWindow(id)));
        item(ui, "Delete", Some(Action::DeleteWindow(id)));
        item(ui, "Duplicate", None);
        ui.separator();
        item(ui, "Export...", Some(if graph { Action::ExportGraph(id) } else { Action::ExportData(id) }));
        ui.separator();
        item(ui, "Properties...", Some(Action::RenameWindow(id)));
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
        p.root.folders.push(Folder { name: "Runs".into(), folders: vec![], windows: vec![b] });
        assert_eq!(windows_in(&p, &[]), [a]);
        assert_eq!(windows_in(&p, &[0]), [b]);
        assert_eq!(home(&p.root, b), [0]);
        assert_eq!(home(&p.root, a), Vec::<usize>::new());
    }

    #[test]
    fn nested_folders_have_their_own_windows() {
        let (mut p, a, b) = project();
        let inner = Folder { name: "Deep".into(), folders: vec![], windows: vec![a] };
        p.root.folders.push(Folder { name: "Outer".into(), folders: vec![inner], windows: vec![b] });
        assert_eq!(windows_in(&p, &[0, 0]), [a]);
        assert_eq!(windows_in(&p, &[0]), [b]);
        assert!(folder_at(&p.root, &[0, 0]).is_some());
        assert!(folder_at(&p.root, &[0, 1]).is_none());
        unlist(&mut p.root, a);
        assert_eq!(windows_in(&p, &[]), [a]);
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
    fn the_list_cells_hold_name_long_name_short_name_and_type() {
        let (p, a, _) = project();
        let w = p.window(a).unwrap();
        assert_eq!(
            COLUMNS.iter().enumerate().map(|(i, _)| cell(w, i)).collect::<Vec<_>>(),
            ["Book1", "alpha", "Book1", "Workbook"]
        );
    }
}
