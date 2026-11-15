//! Go To's Column Browser. Choosing a range changes the parent field, not the worksheet.

use crate::panels::explorer::home;
use eframe::egui;
use egui_extras::{Column as TableColumn, TableBuilder};
use sheaf_core::format::NumberFormat;
use sheaf_core::{Column, ColumnId, Project, WindowId, Workbook, Worksheet};
use std::collections::HashSet;

mod rows;

struct AddedRange {
    columns: Vec<Choice>,
    rows: Option<(usize, usize)>,
}

impl AddedRange {
    fn rows_label(&self) -> String {
        self.rows.map_or_else(|| "[1:end]".into(), |(first, last)| format!("[{first}:{last}]"))
    }

    fn suffix(&self) -> String {
        if self.rows.is_some() { self.rows_label() } else { String::new() }
    }

    fn reference(&self, project: &Project) -> Option<String> {
        let entries = ColumnBrowser::range(&self.columns, project)?;
        Some(format!("{}{}", entries.first()?.prefix(), self.local_reference(&entries)?))
    }

    fn local_reference(&self, entries: &[Entry<'_>]) -> Option<String> {
        let start = super::column_reference(entries.first()?.column);
        Some(if entries.len() == 1 {
            format!("{start}{}", self.suffix())
        } else {
            let end = super::column_reference(entries.last()?.column);
            match self.rows {
                Some((first, last)) => format!("{start}[{first}]:{end}[{last}]"),
                None => format!("{start}:{end}"),
            }
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Project,
    Folder,
    RecursiveFolder,
    Book,
    Sheet,
}

impl Scope {
    const ALL: [Self; 5] = [Self::Project, Self::Folder, Self::RecursiveFolder, Self::Book, Self::Sheet];

    fn label(self) -> &'static str {
        match self {
            Self::Project => "in Current Project",
            Self::Folder => "in Current Folder",
            Self::RecursiveFolder => "in Current Folder (recursive)",
            Self::Book => "in Current Book",
            Self::Sheet => "in Current Sheet",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Choice {
    book: WindowId,
    sheet: usize,
    column: ColumnId,
}

struct Entry<'a> {
    choice: Choice,
    book: &'a Workbook,
    sheet: &'a Worksheet,
    column: &'a Column,
    index: usize,
}

impl Choice {
    fn resolve(self, project: &Project) -> Option<Entry<'_>> {
        let book = project.workbook(self.book)?;
        let sheet = book.sheets.get(self.sheet)?;
        let index = sheet.column_index(self.column)?;
        Some(Entry { choice: self, book, sheet, column: &sheet.columns[index], index })
    }
}

impl Entry<'_> {
    fn prefix(&self) -> String {
        format!("[{}]{}!", self.book.name, self.sheet.name)
    }

    fn row_label(&self) -> String {
        format!(
            "{} / {} / {}({}): {}",
            self.book.name,
            self.sheet.name,
            self.column.short_name,
            self.column.designation.tag(),
            self.column.long_name
        )
    }
}

pub(super) enum BrowserOutcome {
    Open,
    Closed,
    /// Native Go To uses an ordinal in its original sheet. Multiple ranges yield zero (end).
    Selected {
        reference: String,
        ordinal: usize,
    },
}

pub(super) struct ColumnBrowser {
    book: WindowId,
    sheet: usize,
    scope: Scope,
    selected: Vec<Choice>,
    excluded: HashSet<Choice>,
    anchor: Option<Choice>,
    cursor: Option<Choice>,
    table_focus: Option<egui::Id>,
    added: Vec<AddedRange>,
    row_editor: Option<(usize, rows::RowsDialog)>,
    selected_range: Option<usize>,
    expanded: bool,
    first_frame: bool,
}

impl ColumnBrowser {
    pub fn new(book: WindowId, sheet: usize) -> Self {
        Self {
            book,
            sheet,
            scope: Scope::Sheet,
            selected: Vec::new(),
            excluded: HashSet::new(),
            anchor: None,
            cursor: None,
            table_focus: None,
            added: Vec::new(),
            row_editor: None,
            selected_range: None,
            expanded: true,
            first_frame: true,
        }
    }

    fn entries<'a>(&self, project: &'a Project) -> Vec<Entry<'a>> {
        let folder = home(&project.root, self.book);
        project
            .workbooks()
            .filter(|book| match self.scope {
                Scope::Project => true,
                Scope::Folder => home(&project.root, book.id) == folder,
                Scope::RecursiveFolder => home(&project.root, book.id).starts_with(&folder),
                Scope::Book | Scope::Sheet => book.id == self.book,
            })
            .flat_map(|book| {
                book.sheets.iter().enumerate().filter_map(move |(index, sheet)| {
                    (self.scope != Scope::Sheet || index == self.sheet).then_some((book, index, sheet))
                })
            })
            .flat_map(|(book, sheet_index, sheet)| {
                sheet.columns.iter().enumerate().map(move |(index, column)| Entry {
                    choice: Choice { book: book.id, sheet: sheet_index, column: column.id },
                    book,
                    sheet,
                    column,
                    index,
                })
            })
            .filter(|entry| !self.excluded.contains(&entry.choice))
            .collect()
    }

    fn exclude(&mut self, project: &Project) -> Option<Choice> {
        if self.selected.is_empty() {
            return None;
        }
        let entries = self.entries(project);
        let first = entries.iter().position(|entry| self.selected.contains(&entry.choice)).unwrap_or(0);
        self.excluded.extend(self.selected.drain(..));
        let remaining = |entry: &&Entry<'_>| !self.excluded.contains(&entry.choice);
        let next = entries
            .iter()
            .filter(remaining)
            .nth(first)
            .or_else(|| entries.iter().rev().find(remaining))
            .map(|entry| entry.choice);
        self.selected = next.into_iter().collect();
        self.anchor = next;
        self.cursor = next;
        self.table_focus = None;
        next
    }

    fn select(&mut self, entries: &[Entry<'_>], index: usize, modifiers: egui::Modifiers) {
        let choice = entries[index].choice;
        let toggle = modifiers.command || modifiers.ctrl;
        if modifiers.shift {
            let anchor = self.anchor.and_then(|anchor| entries.iter().position(|entry| entry.choice == anchor));
            let anchor = anchor.unwrap_or(index);
            if !toggle {
                self.selected.clear();
            }
            for entry in &entries[anchor.min(index)..=anchor.max(index)] {
                if !self.selected.contains(&entry.choice) {
                    self.selected.push(entry.choice);
                }
            }
            self.anchor = Some(entries[anchor].choice);
        } else {
            if toggle {
                if let Some(index) = self.selected.iter().position(|selected| *selected == choice) {
                    self.selected.remove(index);
                } else {
                    self.selected.push(choice);
                }
            } else {
                self.selected = vec![choice];
            }
            self.anchor = Some(choice);
        }
        self.cursor = Some(choice);
    }

    fn range<'a>(choices: &[Choice], project: &'a Project) -> Option<Vec<Entry<'a>>> {
        choices.iter().map(|choice| choice.resolve(project)).collect()
    }

    fn selection(&self, project: &Project) -> Option<(String, usize)> {
        let ranges = self.added.iter().map(|range| Self::range(&range.columns, project)).collect::<Option<Vec<_>>>()?;
        let first = ranges.first()?.first()?;
        if ranges.len() == 1 {
            return Some((self.added[0].reference(project)?, if ranges[0].len() == 1 { first.index + 1 } else { 0 }));
        }
        let reference = if ranges
            .iter()
            .flatten()
            .all(|entry| entry.choice.book == first.choice.book && entry.choice.sheet == first.choice.sheet)
        {
            format!(
                "{}({})",
                first.prefix(),
                ranges
                    .iter()
                    .zip(&self.added)
                    .map(|(entries, range)| range.local_reference(entries))
                    .collect::<Option<Vec<_>>>()?
                    .join(",")
            )
        } else {
            format!(
                "({})",
                self.added.iter().map(|range| range.reference(project)).collect::<Option<Vec<_>>>()?.join(",")
            )
        };
        Some((reference, 0))
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> BrowserOutcome {
        let mut open = true;
        let mut outcome = BrowserOutcome::Open;
        let editing_rows = self.row_editor.is_some();
        let popup_was_open = egui::Popup::is_any_open(ctx);
        let window = egui::Window::new("Column Browser").enabled(!editing_rows);
        window.open(&mut open).collapsible(false).default_width(700.0).show(ctx, |ui| {
            let mut focus_after_exclude = None;
            ui.horizontal(|ui| {
                let label = ui.label("List Columns");
                let previous = self.scope;
                let scope = egui::ComboBox::from_id_salt("goto_browser_scope")
                    .selected_text(self.scope.label())
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for scope in Scope::ALL {
                            ui.selectable_value(&mut self.scope, scope, scope.label());
                        }
                    })
                    .response
                    .labelled_by(label.id);
                if self.first_frame {
                    scope.request_focus();
                    self.first_frame = false;
                }
                if previous != self.scope {
                    self.excluded.clear();
                    self.selected.clear();
                    self.anchor = None;
                    self.cursor = None;
                    self.table_focus = None;
                }
                if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Exclude")).clicked() {
                    focus_after_exclude = self.exclude(project);
                }
            });
            let entries = self.entries(project);
            let mut reveal =
                focus_after_exclude.and_then(|choice| entries.iter().position(|entry| entry.choice == choice));
            if !editing_rows
                && !popup_was_open
                && !egui::Popup::is_any_open(ctx)
                && self.table_focus.is_some_and(|id| ctx.memory(|memory| memory.has_focus(id)))
            {
                let modifiers = ctx.input(|input| input.modifiers);
                if (modifiers.command || modifiers.ctrl)
                    && ctx.input_mut(|input| input.consume_key(modifiers, egui::Key::A))
                {
                    self.selected = entries.iter().map(|entry| entry.choice).collect();
                }
                for (key, down) in [(egui::Key::ArrowDown, true), (egui::Key::ArrowUp, false)] {
                    if !entries.is_empty() && ctx.input_mut(|input| input.consume_key(modifiers, key)) {
                        let index = self
                            .cursor
                            .and_then(|cursor| entries.iter().position(|entry| entry.choice == cursor))
                            .unwrap_or(0);
                        let index = if down { (index + 1).min(entries.len() - 1) } else { index.saturating_sub(1) };
                        self.select(&entries, index, modifiers);
                        reveal = Some(index);
                    }
                }
            }
            egui::ScrollArea::horizontal().id_salt("goto_browser_columns").show(ui, |ui| {
                // Leave the scroll bar below every row, rather than over the final row's hit area.
                ui.set_min_height(270.0);
                let mut table = TableBuilder::new(ui)
                    .striped(true)
                    .sense(egui::Sense::click())
                    .columns(TableColumn::auto().at_least(60.0), 10)
                    .min_scrolled_height(215.0)
                    .max_scroll_height(235.0);
                if let Some(index) = reveal {
                    table = table.scroll_to_row(index, None);
                }
                table
                    .header(22.0, |mut header| {
                        for label in [
                            "Sheet",
                            "Book Long Name",
                            "Book Short Name",
                            "Index",
                            "SName",
                            "LName",
                            "Comments",
                            "Format",
                            "Size",
                            "1st Value",
                        ] {
                            header.col(|ui| {
                                ui.strong(label);
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(20.0, entries.len(), |mut row| {
                            let index = row.index();
                            let entry = &entries[index];
                            let selected = self.selected.contains(&entry.choice);
                            row.set_selected(selected);
                            let mut clicked = false;
                            let mut focus = None;
                            row.col(|ui| {
                                let response = ui.selectable_label(selected, &entry.sheet.name);
                                let _ =
                                    ctx.accesskit_node_builder(response.id, |node| node.set_label(entry.row_label()));
                                clicked = response.clicked();
                                focus = Some(response.id);
                                if reveal == Some(index) {
                                    response.request_focus();
                                    self.table_focus = Some(response.id);
                                }
                            });
                            for value in [
                                entry.book.long_name.clone(),
                                entry.book.name.clone(),
                                (entry.index + 1).to_string(),
                                entry.column.short_name.clone(),
                                entry.column.long_name.clone(),
                                entry.column.comments.clone(),
                                format_kind(entry.column).into(),
                                entry.column.len().to_string(),
                                entry.column.shown_text(0),
                            ] {
                                row.col(|ui| {
                                    ui.label(value);
                                });
                            }
                            if clicked || row.response().clicked() {
                                self.select(&entries, index, ctx.input(|input| input.modifiers));
                                if let Some(id) = focus {
                                    ctx.memory_mut(|memory| memory.request_focus(id));
                                    self.table_focus = Some(id);
                                }
                            }
                        });
                    });
            });
            // Arrow keys extend/move this table's selection, rather than traversing other widgets.
            if let Some(id) = self.table_focus {
                ctx.memory_mut(|memory| {
                    memory.set_focus_lock_filter(id, egui::EventFilter { vertical_arrows: true, ..Default::default() })
                });
            }
            ui.horizontal(|ui| {
                ui.weak("Column Selected");
                let selected = entries.iter().filter(|entry| self.selected.contains(&entry.choice)).collect::<Vec<_>>();
                let block = selected.len() > 1
                    && selected.windows(2).all(|pair| {
                        pair[0].choice.book == pair[1].choice.book
                            && pair[0].choice.sheet == pair[1].choice.sheet
                            && pair[0].index + 1 == pair[1].index
                    });
                if ui.add_enabled(block, egui::Button::new("Add as One Block")).clicked() {
                    self.added
                        .push(AddedRange { columns: selected.iter().map(|entry| entry.choice).collect(), rows: None });
                }
                let add_response = ui.add_enabled(!selected.is_empty(), egui::Button::new("Add"));
                if add_response.clicked() {
                    self.added
                        .extend(selected.iter().map(|entry| AddedRange { columns: vec![entry.choice], rows: None }));
                }
                if ui.add_enabled(self.selected_range.is_some(), egui::Button::new("Remove")).clicked()
                    && let Some(index) = self.selected_range.take().filter(|index| *index < self.added.len())
                {
                    self.added.remove(index);
                }
                let selection = self.selection(project);
                if ui.add_enabled(selection.is_some(), egui::Button::new("OK")).clicked()
                    && let Some((reference, ordinal)) = selection
                {
                    outcome = BrowserOutcome::Selected { reference, ordinal };
                }
                if ui.button("Cancel").clicked() {
                    outcome = BrowserOutcome::Closed;
                }
                if ui
                    .button(if self.expanded { "▴" } else { "▾" })
                    .on_hover_text("Show or hide selected ranges")
                    .clicked()
                {
                    self.expanded = !self.expanded;
                }
            });
            if self.expanded {
                egui::ScrollArea::vertical()
                    .id_salt("goto_browser_ranges")
                    .max_height(150.0)
                    .min_scrolled_height(120.0)
                    .show(ui, |ui| {
                        egui::Grid::new("goto_browser_added").striped(true).show(ui, |ui| {
                            ui.strong("Range");
                            ui.strong("Rows");
                            ui.end_row();
                            for (index, range) in self.added.iter().enumerate() {
                                let reference = range.reference(project).unwrap_or_else(|| "Unavailable column".into());
                                if ui.selectable_label(self.selected_range == Some(index), &reference).clicked() {
                                    self.selected_range = Some(index);
                                }
                                ui.horizontal(|ui| {
                                    ui.label(range.rows_label());
                                    let maximum =
                                        range.columns.first().and_then(|choice| choice.resolve(project)).map(|entry| {
                                            entry.sheet.columns.iter().map(Column::len).max().unwrap_or(0)
                                        });
                                    let edit = ui.add_enabled(maximum.is_some(), egui::Button::new("…"));
                                    let _ = ctx.accesskit_node_builder(edit.id, |node| {
                                        node.set_label(format!("Edit rows for {reference}"))
                                    });
                                    if edit.clicked()
                                        && let Some(maximum) = maximum
                                    {
                                        self.row_editor = Some((index, rows::RowsDialog::new(range.rows, maximum)));
                                    }
                                });
                                ui.end_row();
                            }
                        });
                    });
            }
            if matches!(outcome, BrowserOutcome::Open)
                && !editing_rows
                && self.row_editor.is_none()
                && !popup_was_open
                && !egui::Popup::is_any_open(ctx)
                && ctx.input(|input| input.key_pressed(egui::Key::Enter))
                && let Some((reference, ordinal)) = self.selection(project)
            {
                outcome = BrowserOutcome::Selected { reference, ordinal };
            }
        });
        if let Some((index, editor)) = &mut self.row_editor {
            match editor.show(ctx) {
                rows::Outcome::Open => {}
                rows::Outcome::Closed => self.row_editor = None,
                rows::Outcome::Selected(rows) => {
                    if let Some(range) = self.added.get_mut(*index) {
                        range.rows = rows;
                    }
                    self.row_editor = None;
                }
            }
            return BrowserOutcome::Open;
        }
        if !editing_rows && (!open || (!popup_was_open && ctx.input(|input| input.key_pressed(egui::Key::Escape)))) {
            BrowserOutcome::Closed
        } else {
            outcome
        }
    }
}

fn format_kind(column: &Column) -> &'static str {
    match column.format {
        NumberFormat::Date(_) => "Date",
        NumberFormat::Time(_) => "Time",
        NumberFormat::DateTime(_, _) => "Date/Time",
        NumberFormat::Month(_) => "Month",
        NumberFormat::Weekday(_) => "Day of Week",
        _ if column.numeric().is_none() => "Text",
        _ => "T&N",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::project::Folder;
    use sheaf_core::{ColumnData, Designation};

    #[test]
    fn scopes_distinguish_the_original_sheet_book_folder_descendants_and_unlisted_root_books() {
        let mut project = Project::default();
        let mut books = Vec::new();
        for name in ["Current", "Peer", "Child", "Elsewhere", "Unlisted root"] {
            let book = project.add_workbook(name);
            project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
            books.push(book);
        }
        let current = books[0];
        project.add_column(current, 0, Designation::Y, ColumnData::numeric(vec![2.0])).unwrap();
        project.workbook_mut(current).unwrap().sheets.push(Worksheet::new("Second"));
        project.add_column(current, 1, Designation::Y, ColumnData::numeric(vec![3.0])).unwrap();
        project.root.folders = vec![
            Folder {
                name: "Current folder".into(),
                windows: vec![current, books[1]],
                folders: vec![Folder { name: "Child".into(), windows: vec![books[2]], ..Folder::default() }],
            },
            Folder { name: "Other folder".into(), windows: vec![books[3]], ..Folder::default() },
        ];
        let mut browser = ColumnBrowser::new(current, 0);
        assert_eq!(browser.entries(&project).len(), 2, "default scope is the opening sheet");
        for (scope, count) in [(Scope::Book, 3), (Scope::Folder, 4), (Scope::RecursiveFolder, 5), (Scope::Project, 7)] {
            browser.scope = scope;
            assert_eq!(browser.entries(&project).len(), count, "{}", scope.label());
        }
        browser.book = books[4];
        browser.scope = Scope::Folder;
        let entries = browser.entries(&project);
        assert_eq!(entries.len(), 1, "unlisted windows belong to root, not a child");
        assert_eq!(entries[0].book.id, books[4]);
        browser.scope = Scope::RecursiveFolder;
        assert_eq!(browser.entries(&project).len(), 7, "root recursion includes unlisted and explicitly filed windows");
    }
}
