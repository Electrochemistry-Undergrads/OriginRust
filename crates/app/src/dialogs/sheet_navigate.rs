//! The sheet tab's Navigate... (T93): Origin 2016's "Navigate Worksheets - Book1" window lists the
//! workbook's sheets in a table (Index, Creation Index, Name, Columns, Size(KB), Comments, Type, Source),
//! the active one selected; OK goes to the selected sheet, Close closes. Sheaf shows the columns it can
//! fill: Index, Name, Long Name, Columns, Rows and Comments. Rules in `docs/spec/workbook-model.md` 13h.

use eframe::egui::{self, RichText};
use sheaf_core::{Project, WindowId, Worksheet};

pub enum NavigateOutcome {
    Open,
    Closed,
    Go { book: WindowId, sheet: usize },
    Reorder { book: WindowId, order: Vec<usize> },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Index,
    Name,
    LongName,
    Columns,
    Rows,
    Comments,
}

impl SortColumn {
    const HEADERS: [(Self, &'static str); 6] = [
        (Self::Index, "Index"),
        (Self::Name, "Name"),
        (Self::LongName, "Long Name"),
        (Self::Columns, "Columns"),
        (Self::Rows, "Rows"),
        (Self::Comments, "Comments"),
    ];

    fn order(self, sheets: &[Worksheet], descending: bool) -> Vec<usize> {
        let mut order: Vec<_> = (0..sheets.len()).collect();
        order.sort_by(|&a, &b| {
            let (first, second) = (&sheets[a], &sheets[b]);
            let compared = match self {
                Self::Index => a.cmp(&b),
                Self::Name => first.name.cmp(&second.name),
                Self::LongName => first.long_name.cmp(&second.long_name),
                Self::Columns => first.columns.len().cmp(&second.columns.len()),
                Self::Rows => first.row_count().cmp(&second.row_count()),
                Self::Comments => first.comments.cmp(&second.comments),
            };
            if descending { compared.reverse() } else { compared }
        });
        order
    }
}

pub struct SheetNavigateDialog {
    pub book: WindowId,
    pub selected: usize,
    sort: Option<(SortColumn, bool)>,
}

impl SheetNavigateDialog {
    pub fn new(book: WindowId, active: usize) -> Self {
        SheetNavigateDialog { book, selected: active, sort: None }
    }

    pub fn reordered(&mut self, order: &[usize]) {
        if let Some(selected) = order.iter().position(|&index| index == self.selected) {
            self.selected = selected;
        }
        self.sort = None;
    }

    pub fn show(&mut self, ctx: &egui::Context, project: &Project, can_reorder: bool) -> NavigateOutcome {
        let Some(book) = project.workbook(self.book) else { return NavigateOutcome::Closed };
        let mut outcome = NavigateOutcome::Open;
        let mut open = true;
        egui::Window::new(format!("Navigate Worksheets - {}", book.name))
            .id(egui::Id::new("sheet_navigate"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                let order = egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| self.table(ui, &book.sheets, &mut outcome))
                    .inner;
                ui.horizontal(|ui| {
                    let has_reorder = order.iter().copied().ne(0..book.sheets.len());
                    let mut apply = ui.add_enabled(has_reorder && can_reorder, egui::Button::new("Apply Reorder"));
                    if has_reorder && !can_reorder {
                        apply = apply.on_disabled_hover_text("Close the other dialog before reordering worksheets.");
                    }
                    if apply.clicked() {
                        outcome = NavigateOutcome::Reorder { book: self.book, order };
                    }
                    if ui.button("OK").clicked() {
                        outcome = NavigateOutcome::Go { book: self.book, sheet: self.selected };
                    }
                    if ui.button("Close").clicked() {
                        outcome = NavigateOutcome::Closed;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    outcome = NavigateOutcome::Closed;
                }
            });
        if !open { NavigateOutcome::Closed } else { outcome }
    }

    fn table(&mut self, ui: &mut egui::Ui, sheets: &[Worksheet], outcome: &mut NavigateOutcome) -> Vec<usize> {
        egui::Grid::new("sheet_navigate_grid")
            .striped(true)
            .num_columns(SortColumn::HEADERS.len())
            .show(ui, |ui| {
                for (column, title) in SortColumn::HEADERS {
                    let text = match self.sort {
                        Some((current, descending)) if column == current => {
                            format!("{title} {}", if descending { "↓" } else { "↑" })
                        }
                        _ => title.to_string(),
                    };
                    if ui.button(RichText::new(text).strong()).clicked() {
                        self.sort = Some((column, self.sort == Some((column, false))));
                    }
                }
                ui.end_row();

                let order = self.sort.map_or_else(
                    || (0..sheets.len()).collect(),
                    |(column, descending)| column.order(sheets, descending),
                );
                for &i in &order {
                    let sheet = &sheets[i];
                    let picked = i == self.selected;
                    let index = ui.selectable_label(picked, (i + 1).to_string());
                    let name = ui.selectable_label(picked, &sheet.name);
                    ui.label(&sheet.long_name);
                    ui.label(sheet.columns.len().to_string());
                    ui.label(sheet.row_count().to_string());
                    let first_line = sheet.comments.lines().next().unwrap_or_default();
                    let comment =
                        ui.add_sized([180.0, ui.spacing().interact_size.y], egui::Label::new(first_line).truncate());
                    if !sheet.comments.is_empty() {
                        comment.on_hover_text(&sheet.comments);
                    }
                    ui.end_row();
                    if index.clicked() || name.clicked() {
                        self.selected = i;
                    }
                    if index.double_clicked() || name.double_clicked() {
                        *outcome = NavigateOutcome::Go { book: self.book, sheet: i };
                    }
                }
                order
            })
            .inner
    }
}
