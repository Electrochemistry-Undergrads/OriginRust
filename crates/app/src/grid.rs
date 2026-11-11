//! The worksheet grid: column headers with label rows, and a virtualised body
//! that lays out only the rows currently visible, so the cost of drawing does
//! not depend on how many rows a sheet has.

use crate::actions::Action;
use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke};
use egui_extras::{Column as TableColumn, TableBuilder};
use sheaf_core::template::Template;
use sheaf_core::{ColumnId, Designation, Document, WindowId};

/// What the user is doing in one workbook window.
#[derive(Default)]
pub struct BookUi {
    /// Selected columns in the order they were clicked.
    pub selected: Vec<ColumnId>,
    /// The cell being edited: column, row, and the text typed so far.
    pub editing: Option<(ColumnId, usize, String)>,
    /// The text the editor opened with; leaving the cell with this text unchanged stores nothing.
    editing_original: String,
    /// True right after editing started, to move keyboard focus into the cell once.
    focus_editor: bool,
}

const ROW_HEIGHT: f32 = 20.0;
const HEADER_HEIGHT: f32 = 78.0;
/// Empty rows shown below the data so that new values can be typed in.
const SPARE_ROWS: usize = 30;

pub fn show(ui: &mut egui::Ui, doc: &Document, book_id: WindowId, state: &mut BookUi, actions: &mut Vec<Action>) {
    let Some(book) = doc.project().workbook(book_id) else {
        ui.label("This workbook no longer exists.");
        return;
    };
    let sheet_index = book.active_sheet.min(book.sheets.len().saturating_sub(1));
    let Some(sheet) = book.sheets.get(sheet_index) else {
        ui.label("This workbook has no sheets.");
        return;
    };

    // Drop selections that point at columns which are gone (after undo, delete, ...).
    state.selected.retain(|id| sheet.column(*id).is_some());

    egui::TopBottomPanel::bottom(ui.id().with("sheet_tabs")).show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            for (i, s) in book.sheets.iter().enumerate() {
                if ui.selectable_label(i == sheet_index, &s.name).clicked() && i != sheet_index {
                    actions.push(Action::SelectSheet { book: book_id, sheet: i });
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.weak(format!("{} columns, {} rows", sheet.columns.len(), sheet.row_count()));
            });
        });
    });

    let rows = sheet.row_count() + SPARE_ROWS;
    let selection_fill = ui.visuals().selection.bg_fill.gamma_multiply(0.35);
    let header_fill = ui.visuals().faint_bg_color;
    let grid_stroke = Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color);

    egui::ScrollArea::horizontal().auto_shrink([false, false]).show(ui, |ui| {
        // Cells touch each other so the grid lines join up.
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let mut table = TableBuilder::new(ui)
            .striped(false)
            .resizable(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(TableColumn::exact(80.0))
            .min_scrolled_height(0.0);
        for _ in &sheet.columns {
            table = table.column(TableColumn::initial(104.0).at_least(40.0).clip(true));
        }
        // One empty column at the end as a target for "add column".
        table = table.column(TableColumn::remainder().at_least(60.0));

        table
            .header(HEADER_HEIGHT, |mut header| {
                header.col(|ui| {
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.add_space(19.0);
                        ui.weak("Long Name");
                        ui.weak("Units");
                        ui.weak("Comments");
                    });
                });
                for column in &sheet.columns {
                    let selected = state.selected.contains(&column.id);
                    header.col(|ui| {
                        let rect = ui.max_rect();
                        ui.painter().rect_filled(rect, 0.0, if selected { selection_fill } else { header_fill });
                        ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, grid_stroke);
                        ui.painter().vline(rect.right() - 0.5, rect.y_range(), grid_stroke);
                        let response = ui.interact(rect, ui.id().with(("col_header", column.id.0)), Sense::click());
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            let tag = column.designation.tag();
                            let head = if tag.is_empty() {
                                column.short_name.clone()
                            } else {
                                format!("{}({})", column.short_name, tag)
                            };
                            ui.label(RichText::new(head).strong());
                            ui.label(&column.long_name);
                            ui.label(&column.units);
                            ui.label(column.comments.lines().next().unwrap_or_default());
                        });
                        if response.clicked() {
                            let modifiers = ui.input(|i| i.modifiers);
                            select_column(state, sheet, column.id, modifiers);
                        }
                        if response.double_clicked() {
                            actions.push(Action::ColumnProperties(column.id));
                        }
                        response.context_menu(|ui| {
                            if !state.selected.contains(&column.id) {
                                state.selected = vec![column.id];
                            }
                            column_menu(ui, book_id, &state.selected, actions);
                        });
                    });
                }
                header.col(|ui| {
                    if ui.button("+").on_hover_text("Add a column").clicked() {
                        actions.push(Action::AddColumn(book_id));
                    }
                });
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, rows, |mut row| {
                    let r = row.index();
                    row.col(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_space(6.0);
                            ui.weak((r + 1).to_string());
                        });
                    });
                    for column in &sheet.columns {
                        let selected = state.selected.contains(&column.id);
                        row.col(|ui| {
                            let rect = ui.max_rect();
                            if selected {
                                ui.painter().rect_filled(rect, 0.0, selection_fill);
                            }
                            let (right, bottom) = (rect.right() - 0.5, rect.bottom() - 0.5);
                            ui.painter().hline(rect.x_range(), bottom, grid_stroke);
                            ui.painter().vline(right, rect.y_range(), grid_stroke);

                            let is_editing = matches!(&state.editing, Some((c, er, _)) if *c == column.id && *er == r);
                            if is_editing {
                                edit_cell(ui, state, sheet, actions);
                                return;
                            }
                            let text = if r < column.len() { column.cell_text(r) } else { String::new() };
                            let response = ui.interact(rect, ui.id().with(("cell", column.id.0, r)), Sense::click());
                            let numeric = column.data.is_numeric();
                            let layout = if numeric {
                                Layout::right_to_left(Align::Center)
                            } else {
                                Layout::left_to_right(Align::Center)
                            };
                            ui.with_layout(layout, |ui| {
                                ui.add_space(4.0);
                                let color = if text == "--" {
                                    ui.visuals().weak_text_color()
                                } else {
                                    ui.visuals().text_color()
                                };
                                ui.label(RichText::new(&text).color(color));
                            });
                            if response.double_clicked() {
                                start_editing(state, column, r);
                            }
                        });
                    }
                    row.col(|_ui| {});
                });
            });
    });
}

/// The text a cell is edited with: for numbers the shortest text that reads back as exactly the
/// same value (the grid shows a rounded version, which would lose digits), empty for a missing
/// value or a row beyond the data.
fn editor_text(column: &sheaf_core::Column, row: usize) -> String {
    match &column.data {
        sheaf_core::ColumnData::Numeric(v) => match v.get(row) {
            Some(x) if !x.is_nan() => format!("{x:?}"),
            _ => String::new(),
        },
        sheaf_core::ColumnData::Text(v) => v.get(row).cloned().unwrap_or_default(),
    }
}

fn start_editing(state: &mut BookUi, column: &sheaf_core::Column, row: usize) {
    let text = editor_text(column, row);
    state.editing_original = text.clone();
    state.editing = Some((column.id, row, text));
    state.focus_editor = true;
}

fn edit_cell(ui: &mut egui::Ui, state: &mut BookUi, sheet: &sheaf_core::Worksheet, actions: &mut Vec<Action>) {
    let Some((column, row, text)) = &mut state.editing else {
        return;
    };
    let response = ui.add(egui::TextEdit::singleline(text).frame(false).desired_width(f32::INFINITY));
    if state.focus_editor {
        response.request_focus();
        state.focus_editor = false;
    }
    let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if cancel {
        state.editing = None;
    } else if response.lost_focus() {
        let (column, row) = (*column, *row);
        if *text != state.editing_original {
            actions.push(Action::SetCell { column, row, text: text.clone() });
        }
        // Enter moves on to the cell below, like a spreadsheet, showing what it holds.
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        match sheet.column(column) {
            Some(c) if enter => start_editing(state, c, row + 1),
            _ => state.editing = None,
        }
    }
}

fn select_column(state: &mut BookUi, sheet: &sheaf_core::Worksheet, id: ColumnId, modifiers: egui::Modifiers) {
    if modifiers.command {
        if let Some(pos) = state.selected.iter().position(|c| *c == id) {
            state.selected.remove(pos);
        } else {
            state.selected.push(id);
        }
    } else if modifiers.shift && !state.selected.is_empty() {
        let last = *state.selected.last().expect("not empty");
        if let (Some(a), Some(b)) = (sheet.column_index(last), sheet.column_index(id)) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            state.selected = sheet.columns[lo..=hi].iter().map(|c| c.id).collect();
        }
    } else {
        state.selected = vec![id];
    }
}

/// The right-click menu of a column header. Also used by the Column menu.
/// Menu entries that change the sheet as a whole: sorting by the selected column, filtering rows.
pub fn sheet_menu(ui: &mut egui::Ui, book: WindowId, selected: &[ColumnId], actions: &mut Vec<Action>) {
    ui.menu_button("Sort Sheet by Column", |ui| {
        for (label, descending) in [("Ascending", false), ("Descending", true)] {
            if ui.add_enabled(selected.len() == 1, egui::Button::new(label)).clicked() {
                actions.push(Action::SortSheet { column: selected[0], descending });
                ui.close();
            }
        }
        if selected.len() != 1 {
            ui.weak("Select the one column to sort by");
        }
    });
    if ui.button("Filter Rows...").clicked() {
        actions.push(Action::FilterRows(book));
        ui.close();
    }
}

pub fn column_menu(ui: &mut egui::Ui, book: WindowId, selected: &[ColumnId], actions: &mut Vec<Action>) {
    let any = !selected.is_empty();
    ui.menu_button("Plot", |ui| {
        for t in Template::ALL {
            if ui.add_enabled(any, egui::Button::new(t.label())).clicked() {
                actions.push(Action::PlotColumns { book, columns: selected.to_vec(), template: t });
                ui.close();
            }
        }
    });
    ui.menu_button("Set as", |ui| {
        for d in Designation::ALL {
            let label = match d {
                Designation::X => "X",
                Designation::Y => "Y",
                Designation::Z => "Z",
                Designation::YError => "Y error",
                Designation::XError => "X error",
                Designation::Label => "Label",
                Designation::Disregard => "Disregard",
            };
            if ui.add_enabled(any, egui::Button::new(label)).clicked() {
                actions.push(Action::SetDesignation { columns: selected.to_vec(), designation: d });
                ui.close();
            }
        }
    });
    ui.separator();
    if ui.add_enabled(selected.len() == 1, egui::Button::new("Properties...")).clicked() {
        actions.push(Action::ColumnProperties(selected[0]));
        ui.close();
    }
    if ui.add_enabled(selected.len() == 1, egui::Button::new("Set Column Values...")).clicked() {
        actions.push(Action::SetColumnValues(selected[0]));
        ui.close();
    }
    ui.separator();
    if ui.button("Add Column").clicked() {
        actions.push(Action::AddColumn(book));
        ui.close();
    }
    if ui.add_enabled(any, egui::Button::new("Delete Columns")).clicked() {
        actions.push(Action::DeleteColumns(selected.to_vec()));
        ui.close();
    }
}

/// A small colored square, used in menus and lists to show a plot color.
pub fn color_swatch(ui: &mut egui::Ui, color: sheaf_core::Color) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
    ui.painter().rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a));
}
