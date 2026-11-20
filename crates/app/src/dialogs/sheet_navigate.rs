//! The sheet tab's Navigate... (T93): Origin 2016's "Navigate Worksheets - Book1" window lists the
//! workbook's sheets in a table (Index, Creation Index, Name, Columns, Size(KB), Comments, Type, Source),
//! the active one selected; OK goes to the selected sheet, Close closes. Sheaf shows the columns it can
//! fill: Index, Name, Long Name, Columns, Rows and Comments. Rules in `docs/spec/workbook-model.md` 13h.
//! The Actions menu and a row's right-click menu (T180) act on the selected sheets: Ctrl+click adds or
//! removes a sheet, Shift+click selects the run from the last click.

use eframe::egui::{self, RichText};
use sheaf_core::sheet_tab::SheetMove;
use sheaf_core::{Project, WindowId, Worksheet};
use std::collections::BTreeSet;

pub enum NavigateOutcome {
    Open,
    Closed,
    Go {
        book: WindowId,
        sheet: usize,
    },
    Reorder {
        book: WindowId,
        order: Vec<usize>,
    },
    /// An item of the Actions menu (T180) on these sheets (indices in the workbook, ascending).
    Act {
        book: WindowId,
        sheets: Vec<usize>,
        action: NavAction,
    },
}

/// The items of Origin's Actions menu that Sheaf has (T180). Set Current Order as Sheet Order is
/// [`NavigateOutcome::Reorder`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavAction {
    Activate,
    Duplicate,
    DuplicateWithoutData,
    ClearData,
    Delete,
    Move(SheetMove),
    /// Move to > New Book and Duplicate to New Book (T184): the sheets go, or are copied, to one new
    /// workbook.
    MoveToNewBook,
    DuplicateToNewBook,
    /// Exclude from Plotting dialogs (T184): set on every sheet, or cleared when all have it.
    ExcludeFromPlotting,
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
    /// The sheet clicked last (OK goes to it).
    pub selected: usize,
    /// The sheets the Actions work on (T180); only `selected` when it does not hold it.
    pub picked: BTreeSet<usize>,
    sort: Option<(SortColumn, bool)>,
}

impl SheetNavigateDialog {
    pub fn new(book: WindowId, active: usize) -> Self {
        SheetNavigateDialog { book, selected: active, picked: BTreeSet::from([active]), sort: None }
    }

    pub fn reordered(&mut self, order: &[usize]) {
        if let Some(selected) = order.iter().position(|&index| index == self.selected) {
            self.selected = selected;
        }
        self.picked = self.picked.iter().filter_map(|p| order.iter().position(|i| i == p)).collect();
        self.sort = None;
    }

    /// After sheets were added or removed: the active sheet alone is selected, unsorted.
    pub fn reset(&mut self, active: usize) {
        self.selected = active;
        self.picked = BTreeSet::from([active]);
        self.sort = None;
    }

    /// The sheets the Actions work on, ascending.
    pub fn targets(&self) -> Vec<usize> {
        if self.picked.contains(&self.selected) { self.picked.iter().copied().collect() } else { vec![self.selected] }
    }

    /// The Actions items (T180), as Origin's list them; the chosen one ends in `outcome`.
    fn actions_menu(&self, ui: &mut egui::Ui, all: &[Worksheet], order: &[usize], outcome: &mut NavigateOutcome) {
        let sheets = all.len();
        let targets = self.targets();
        let mut act = |ui: &mut egui::Ui, label: &str, enabled: bool, action: NavAction| {
            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                *outcome = NavigateOutcome::Act { book: self.book, sheets: targets.clone(), action };
                ui.close();
            }
        };
        act(ui, "Activate", targets.len() == 1, NavAction::Activate);
        act(ui, "Duplicate", true, NavAction::Duplicate);
        act(ui, "Duplicate without Data", true, NavAction::DuplicateWithoutData);
        act(ui, "Clear all Data", true, NavAction::ClearData);
        act(ui, "Delete", targets.len() < sheets, NavAction::Delete);
        ui.separator();
        act(ui, "Move Before", true, NavAction::Move(SheetMove::Before));
        act(ui, "Move to Behind", true, NavAction::Move(SheetMove::Behind));
        act(ui, "Move to End", true, NavAction::Move(SheetMove::End));
        ui.menu_button("Move to", |ui| {
            act(ui, "New Book", targets.len() < sheets, NavAction::MoveToNewBook);
        });
        act(ui, "Duplicate to New Book", true, NavAction::DuplicateToNewBook);
        ui.separator();
        let excluded = targets.iter().all(|&i| all.get(i).is_some_and(|s| s.exclude_from_plotting));
        let label = if excluded { "\u{2713} Exclude from Plotting dialogs" } else { "Exclude from Plotting dialogs" };
        act(ui, label, true, NavAction::ExcludeFromPlotting);
        ui.separator();
        let reorder = order.iter().copied().ne(0..sheets);
        if ui.add_enabled(reorder, egui::Button::new("Set Current Order as Sheet Order")).clicked() {
            *outcome = NavigateOutcome::Reorder { book: self.book, order: order.to_vec() };
            ui.close();
        }
    }

    /// A click on row `i` of the shown `order`: alone, Ctrl adds or removes it, Shift takes the run.
    fn click(&mut self, ui: &egui::Ui, i: usize, order: &[usize]) {
        let m = ui.input(|input| input.modifiers);
        if m.shift {
            let (a, b) = (order.iter().position(|&x| x == self.selected), order.iter().position(|&x| x == i));
            if let (Some(a), Some(b)) = (a, b) {
                self.picked = order[a.min(b)..=a.max(b)].iter().copied().collect();
            }
        } else if m.command || m.ctrl {
            // Ctrl+click on a selected sheet takes it out (the last one stays); the focus moves to
            // a sheet that is still selected.
            if self.picked.contains(&i) && self.picked.len() > 1 {
                self.picked.remove(&i);
                if self.selected == i {
                    self.selected = *self.picked.iter().next_back().expect("one is left");
                }
            } else {
                self.picked.insert(i);
                self.selected = i;
            }
        } else {
            self.picked = BTreeSet::from([i]);
            self.selected = i;
        }
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
                // Origin's Actions menu above the list (T180).
                let shown = self.sort.map_or_else(
                    || (0..book.sheets.len()).collect::<Vec<_>>(),
                    |(column, descending)| column.order(&book.sheets, descending),
                );
                ui.menu_button("Actions", |ui| self.actions_menu(ui, &book.sheets, &shown, &mut outcome));
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
                    let picked = self.targets().contains(&i);
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
                        self.click(ui, i, &order);
                    }
                    // A right-click on a sheet not selected picks it first, as a list does.
                    if (index.secondary_clicked() || name.secondary_clicked()) && !self.targets().contains(&i) {
                        self.picked = BTreeSet::from([i]);
                        self.selected = i;
                    }
                    for row in [&index, &name] {
                        row.context_menu(|ui| self.actions_menu(ui, sheets, &order, outcome));
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
