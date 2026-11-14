//! The two sort windows that need a choice: the nested sort of Worksheet > Sort Worksheet >
//! Custom... and Sort Range > Custom... (a list of key columns, each with its own order), and
//! Worksheet > Sort Columns by Label... (which label row to order the columns by).

use super::frame::{DialogButton, button_row};
use eframe::egui;
use sheaf_core::sheetops::{ColumnLabel, SortKey};
use sheaf_core::{ColumnId, Project, WindowId};
use std::ops::Range;

/// What a sort window sorts.
pub enum SortKind {
    /// The rows of these columns (every column of the sheet when empty) by a list of keys.
    Nested { columns: Vec<ColumnId> },
    /// The columns of the active sheet by one of their labels.
    ByLabel,
}

pub struct SortDialog {
    pub book: WindowId,
    kind: SortKind,
    /// The key columns of a nested sort, first key first.
    levels: Vec<SortKey>,
    label: ColumnLabel,
    descending: bool,
    rows: Option<Range<usize>>,
}

pub enum SortOutcome {
    Open,
    Closed,
    Nested { book: WindowId, columns: Vec<ColumnId>, keys: Vec<SortKey>, rows: Option<Range<usize>> },
    ByLabel { book: WindowId, label: ColumnLabel, descending: bool },
}

fn label_name(l: ColumnLabel) -> &'static str {
    match l {
        ColumnLabel::LongName => "Long Name",
        ColumnLabel::Units => "Units",
        ColumnLabel::Comments => "Comments",
        ColumnLabel::ShortName => "Short Name",
    }
}

impl SortDialog {
    /// A nested sort; `columns` empty sorts the whole sheet. The first key is the first column
    /// that is part of the sort.
    pub fn nested_in_rows(
        project: &Project,
        book: WindowId,
        columns: Vec<ColumnId>,
        rows: Option<Range<usize>>,
    ) -> Option<Self> {
        let sheet = project.workbook(book)?.sheet();
        let first = columns.first().copied().or_else(|| sheet.columns.first().map(|c| c.id))?;
        Some(SortDialog {
            book,
            kind: SortKind::Nested { columns },
            levels: vec![SortKey { column: first, descending: false }],
            label: ColumnLabel::LongName,
            descending: false,
            rows,
        })
    }

    pub fn by_label(book: WindowId) -> Self {
        SortDialog {
            book,
            kind: SortKind::ByLabel,
            levels: Vec::new(),
            label: ColumnLabel::LongName,
            descending: false,
            rows: None,
        }
    }

    /// The columns a key may be chosen from: the range, or the whole sheet.
    fn choices(&self, project: &Project) -> Vec<(ColumnId, String)> {
        let Some(book) = project.workbook(self.book) else { return Vec::new() };
        book.sheet()
            .columns
            .iter()
            .filter(|c| match &self.kind {
                SortKind::Nested { columns } => columns.is_empty() || columns.contains(&c.id),
                SortKind::ByLabel => true,
            })
            .map(|c| {
                let text = if c.long_name.is_empty() {
                    c.short_name.clone()
                } else {
                    format!("{} ({})", c.short_name, c.long_name)
                };
                (c.id, text)
            })
            .collect()
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project) -> SortOutcome {
        if project.workbook(self.book).is_none() {
            return SortOutcome::Closed;
        }
        let choices = self.choices(project);
        let title = match self.kind {
            SortKind::Nested { .. } => "Nested Sort",
            SortKind::ByLabel => "Sort Columns by Label",
        };
        let mut open = true;
        let mut pressed = None;
        egui::Window::new(title)
            .id(egui::Id::new("sort_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                match &self.kind {
                    SortKind::Nested { .. } => {
                        let mut remove = None;
                        egui::Grid::new("sort_levels").num_columns(3).spacing([10.0, 6.0]).show(ui, |ui| {
                            for (i, level) in self.levels.iter_mut().enumerate() {
                                ui.label(if i == 0 { "Sort by" } else { "Then by" });
                                let shown = choices.iter().find(|c| c.0 == level.column).map(|c| c.1.clone());
                                egui::ComboBox::from_id_salt(("sort_key", i))
                                    .selected_text(shown.unwrap_or_default())
                                    .show_ui(ui, |ui| {
                                        for (id, text) in &choices {
                                            ui.selectable_value(&mut level.column, *id, text);
                                        }
                                    });
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut level.descending, false, "Ascending");
                                    ui.selectable_value(&mut level.descending, true, "Descending");
                                    if i > 0 && ui.small_button("Remove").clicked() {
                                        remove = Some(i);
                                    }
                                });
                                ui.end_row();
                            }
                        });
                        if let Some(i) = remove {
                            self.levels.remove(i);
                        }
                        let unused = choices.iter().find(|c| !self.levels.iter().any(|l| l.column == c.0));
                        if let Some((id, _)) = unused
                            && ui.button("Add level").clicked()
                        {
                            self.levels.push(SortKey { column: *id, descending: false });
                        }
                    }
                    SortKind::ByLabel => {
                        egui::Grid::new("sort_label").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label("Label row");
                            egui::ComboBox::from_id_salt("sort_label_row")
                                .selected_text(label_name(self.label))
                                .show_ui(ui, |ui| {
                                    for l in [
                                        ColumnLabel::LongName,
                                        ColumnLabel::Units,
                                        ColumnLabel::Comments,
                                        ColumnLabel::ShortName,
                                    ] {
                                        ui.selectable_value(&mut self.label, l, label_name(l));
                                    }
                                });
                            ui.end_row();
                            ui.label("Order");
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut self.descending, false, "Ascending");
                                ui.selectable_value(&mut self.descending, true, "Descending");
                            });
                            ui.end_row();
                        });
                    }
                }
                ui.add_space(6.0);
                pressed = button_row(ui, false);
            });
        match pressed {
            Some(DialogButton::Ok) => match &self.kind {
                SortKind::Nested { columns } => SortOutcome::Nested {
                    book: self.book,
                    columns: columns.clone(),
                    keys: self.levels.clone(),
                    rows: self.rows.clone(),
                },
                SortKind::ByLabel => {
                    SortOutcome::ByLabel { book: self.book, label: self.label, descending: self.descending }
                }
            },
            Some(DialogButton::Cancel) => SortOutcome::Closed,
            _ if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) => SortOutcome::Closed,
            _ => SortOutcome::Open,
        }
    }
}
