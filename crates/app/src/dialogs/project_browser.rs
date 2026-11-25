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
        }
    }
}

/// Where Go To takes an item.
pub fn target(project: &Project, item: ItemRef) -> Option<FindTarget> {
    Some(match item {
        ItemRef::Book(id) | ItemRef::Graph(id) => FindTarget::Window(id),
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
        items.iter().filter(|i| self.kind.is_none_or(|k| i.kind == k) && i.matches(&self.search)).collect()
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
                    ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("name, long name, units, comments"));
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
                    egui::Grid::new("project_browser_rows").striped(true).num_columns(7).show(ui, |ui| {
                        for h in ["", "Kind", "Name", "Where", "Long Name", "Units", "Comments"] {
                            ui.strong(h);
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
                            ui.label(&info.long_name);
                            ui.label(&info.units);
                            ui.label(info.comments.lines().next().unwrap_or_default());
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
