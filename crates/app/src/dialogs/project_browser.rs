//! View > Project Browser (T433): the project's books, sheets, columns and graphs in one list with their Long Name,
//! Units and Comments, filtered by a search and a kind; a preview of the chosen item; Go To; and setting one field
//! on every ticked item at once. Rules in `sheaf_core::project_browser`.

use crate::project_find::FindTarget;
use eframe::egui;
use sheaf_core::Project;
use sheaf_core::project_browser::{ItemInfo, ItemKind, ItemRef, MetaField};
use std::collections::HashSet;

/// Rows the list shows at most (the search narrows it).
const MAX_ROWS: usize = 500;

pub enum BrowserOutcome {
    Open,
    Closed,
    GoTo(FindTarget),
    Set { items: Vec<ItemRef>, field: MetaField, value: String },
}

pub struct ProjectBrowserDialog {
    pub search: String,
    /// `None`: every kind.
    pub kind: Option<ItemKind>,
    pub ticked: HashSet<ItemRef>,
    /// The item the preview shows.
    pub focus: Option<ItemRef>,
    pub field: MetaField,
    pub value: String,
    /// Sorted by this column of the list (1 Kind ... 6 Comments; `None` project order), and descending (T466).
    pub sort: Option<(usize, bool)>,
    /// A cell being edited in place: the item, its field and the text typed (T466).
    pub editing: Option<(ItemRef, MetaField, String)>,
    /// The edit box takes the keyboard on its first frame.
    edit_started: bool,
}

impl Default for ProjectBrowserDialog {
    fn default() -> Self {
        ProjectBrowserDialog {
            search: String::new(),
            kind: None,
            ticked: HashSet::new(),
            focus: None,
            field: MetaField::LongName,
            value: String::new(),
            sort: None,
            editing: None,
            edit_started: false,
        }
    }
}

/// Where Go To takes an item.
pub fn target(project: &Project, item: ItemRef) -> Option<FindTarget> {
    Some(match item {
        ItemRef::Book(id) | ItemRef::Graph(id) | ItemRef::Matrix(id) | ItemRef::MatrixSheet(id, _) => {
            FindTarget::Window(id)
        }
        ItemRef::Sheet(book, sheet) => FindTarget::Sheet { book, sheet },
        ItemRef::Column(column) => {
            let (b, _, _) = project.locate(column)?;
            let sheet = b.sheets.iter().position(|s| s.column(column).is_some())?;
            FindTarget::Column { book: b.id, sheet, column }
        }
    })
}

/// A few lines about an item: a column's first values, a sheet's size, a book's sheets, a graph's layers.
fn preview(project: &Project, item: ItemRef) -> String {
    match item {
        ItemRef::Column(id) => {
            let Some(c) = project.column(id) else { return String::new() };
            let first: Vec<String> =
                (0..c.len().min(5)).map(|r| sheaf_core::indexed::level_text(c, r).unwrap_or_default()).collect();
            format!("{} rows: {}{}", c.len(), first.join(", "), if c.len() > 5 { ", ..." } else { "" })
        }
        ItemRef::Sheet(b, i) => project
            .workbook(b)
            .and_then(|b| b.sheets.get(i))
            .map(|s| format!("{} columns, {} rows", s.columns.len(), s.row_count()))
            .unwrap_or_default(),
        ItemRef::Book(b) => project.workbook(b).map(|b| format!("{} sheets", b.sheets.len())).unwrap_or_default(),
        ItemRef::Matrix(m) => project.matrix_book(m).map(|m| format!("{} sheets", m.sheets.len())).unwrap_or_default(),
        ItemRef::MatrixSheet(m, i) => project
            .matrix_book(m)
            .and_then(|m| m.sheets.get(i))
            .map(|s| format!("{} x {} (rows x columns)", s.rows, s.cols))
            .unwrap_or_default(),
        ItemRef::Graph(g) => project
            .graph(g)
            .map(|g| {
                let plots: usize = g.layers.iter().map(|l| l.plots.len()).sum();
                format!("{} layers, {} plots", g.layers.len(), plots)
            })
            .unwrap_or_default(),
    }
}

impl ProjectBrowserDialog {
    fn shown<'a>(&self, items: &'a [ItemInfo]) -> Vec<&'a ItemInfo> {
        let mut rows: Vec<&ItemInfo> =
            items.iter().filter(|i| self.kind.is_none_or(|k| i.kind == k) && i.matches(&self.search)).collect();
        // T466: sorted by the chosen column, ignoring case (a stable sort keeps project order among equals).
        if let Some((column, descending)) = self.sort {
            let key = |i: &ItemInfo| -> String {
                match column {
                    1 => i.kind.name().to_string(),
                    2 => i.name.to_lowercase(),
                    3 => i.location.to_lowercase(),
                    4 => i.folder.to_lowercase(),
                    5 => i.long_name.to_lowercase(),
                    6 => i.units.to_lowercase(),
                    _ => i.comments.to_lowercase(),
                }
            };
            rows.sort_by_key(|i| key(i));
            if descending {
                rows.reverse();
            }
        }
        rows
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> BrowserOutcome {
        let mut open = true;
        let mut outcome = BrowserOutcome::Open;
        let items = project.project_items();
        // Ticks on items that are gone are dropped.
        self.ticked.retain(|t| items.iter().any(|i| i.item == *t));
        egui::Window::new("Project Browser")
            .id(egui::Id::new("project_browser"))
            .collapsible(false)
            .default_width(640.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("name, folder, long name, units, comments"),
                    );
                    ui.label("Show");
                    let shown = self.kind.map_or("All", ItemKind::name);
                    egui::ComboBox::from_id_salt("project_browser_kind").selected_text(shown).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.kind, None, "All");
                        for k in ItemKind::ALL {
                            ui.selectable_value(&mut self.kind, Some(k), k.name());
                        }
                    });
                });
                let rows = self.shown(&items);
                ui.horizontal(|ui| {
                    ui.label(format!("{} items, {} ticked", rows.len(), self.ticked.len()));
                    if ui.small_button("Tick All Shown").clicked() {
                        self.ticked.extend(rows.iter().map(|i| i.item));
                    }
                    if ui.small_button("Clear Ticks").clicked() {
                        self.ticked.clear();
                    }
                });
                egui::ScrollArea::vertical().max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
                    egui::Grid::new("project_browser_rows").striped(true).num_columns(8).show(ui, |ui| {
                        // Column headers sort the list (T466): a click sorts by that column, again reverses it.
                        ui.label("");
                        for (k, h) in
                            ["Kind", "Name", "Where", "Folder", "Long Name", "Units", "Comments"].iter().enumerate()
                        {
                            let column = k + 1;
                            let mark = match self.sort {
                                Some((c, false)) if c == column => " \u{25B2}",
                                Some((c, true)) if c == column => " \u{25BC}",
                                _ => "",
                            };
                            let header = ui.add(
                                egui::Button::new(egui::RichText::new(format!("{h}{mark}")).strong()).frame(false),
                            );
                            if header.clicked() {
                                self.sort = match self.sort {
                                    Some((c, false)) if c == column => Some((column, true)),
                                    _ => Some((column, false)),
                                };
                            }
                        }
                        ui.end_row();
                        for info in rows.iter().take(MAX_ROWS) {
                            let mut on = self.ticked.contains(&info.item);
                            if ui.checkbox(&mut on, "").changed() {
                                if on {
                                    self.ticked.insert(info.item);
                                } else {
                                    self.ticked.remove(&info.item);
                                }
                            }
                            ui.label(info.kind.name());
                            let name = ui.selectable_label(self.focus == Some(info.item), &info.name);
                            if name.clicked() {
                                self.focus = Some(info.item);
                            }
                            if name.double_clicked()
                                && let Some(t) = target(project, info.item)
                            {
                                outcome = BrowserOutcome::GoTo(t);
                            }
                            ui.label(&info.location);
                            ui.label(&info.folder);
                            // Long Name, Units and Comments edit in place on a double click (T466).
                            for (field, text) in [
                                (MetaField::LongName, info.long_name.as_str()),
                                (MetaField::Units, info.units.as_str()),
                                (MetaField::Comments, info.comments.lines().next().unwrap_or_default()),
                            ] {
                                let here =
                                    self.editing.as_ref().is_some_and(|(i, f, _)| *i == info.item && *f == field);
                                if here {
                                    let (_, _, typed) = self.editing.as_mut().expect("checked");
                                    let r = ui.add(egui::TextEdit::singleline(typed).desired_width(120.0));
                                    if std::mem::take(&mut self.edit_started) {
                                        r.request_focus();
                                    }
                                    if r.lost_focus() {
                                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            outcome = BrowserOutcome::Set {
                                                items: vec![info.item],
                                                field,
                                                value: typed.clone(),
                                            };
                                        }
                                        self.editing = None;
                                    }
                                } else {
                                    let r = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                                    let r = if field.applies_to(info.kind) {
                                        r.on_hover_text("Double-click to edit")
                                    } else {
                                        r
                                    };
                                    if r.double_clicked() && field.applies_to(info.kind) {
                                        let full = match field {
                                            MetaField::LongName => info.long_name.clone(),
                                            MetaField::Units => info.units.clone(),
                                            MetaField::Comments => info.comments.clone(),
                                        };
                                        self.editing = Some((info.item, field, full));
                                        self.edit_started = true;
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    });
                });
                ui.separator();
                if let Some(item) = self.focus {
                    ui.horizontal(|ui| {
                        ui.label(preview(project, item));
                        if ui.button("Go To").clicked()
                            && let Some(t) = target(project, item)
                        {
                            outcome = BrowserOutcome::GoTo(t);
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Set");
                    egui::ComboBox::from_id_salt("project_browser_field").selected_text(self.field.name()).show_ui(
                        ui,
                        |ui| {
                            for f in MetaField::ALL {
                                ui.selectable_value(&mut self.field, f, f.name());
                            }
                        },
                    );
                    ui.label("to");
                    ui.add(egui::TextEdit::singleline(&mut self.value).id_salt("project_browser_value"));
                    let fits: Vec<ItemRef> = items
                        .iter()
                        .filter(|i| self.ticked.contains(&i.item) && self.field.applies_to(i.kind))
                        .map(|i| i.item)
                        .collect();
                    let label = format!("Apply to {} Ticked", fits.len());
                    if ui.add_enabled(!fits.is_empty(), egui::Button::new(label)).clicked() {
                        outcome = BrowserOutcome::Set { items: fits, field: self.field, value: self.value.clone() };
                    }
                });
                if ui.button("Close").clicked() {
                    outcome = BrowserOutcome::Closed;
                }
            });
        if !open { BrowserOutcome::Closed } else { outcome }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{ColumnData, Designation};

    #[test]
    fn a_header_click_sorts_by_its_column_and_again_reverses() {
        // T466: three columns named C, A, B by long name.
        let mut p = Project::new();
        let b = p.add_workbook("");
        for name in ["gamma", "Alpha", "beta"] {
            let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
            p.column_mut(c).unwrap().long_name = name.into();
        }
        let items = p.project_items();
        let mut d = ProjectBrowserDialog { kind: Some(ItemKind::Column), ..Default::default() };
        let names = |d: &ProjectBrowserDialog| d.shown(&items).iter().map(|i| i.long_name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&d), ["gamma", "Alpha", "beta"], "project order");
        d.sort = Some((5, false)); // Long Name, after Kind, Name, Where and Folder (T486)
        assert_eq!(names(&d), ["Alpha", "beta", "gamma"], "ignoring case");
        d.sort = Some((5, true));
        assert_eq!(names(&d), ["gamma", "beta", "Alpha"]);
    }
}
