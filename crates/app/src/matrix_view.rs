//! A matrix window (T192 slice 2, `docs/spec/matrices.md`): the active sheet's cells in a grid, with
//! the column numbers across the top and the row numbers down the left as Origin's matrix window
//! shows them, and the sheet tabs below when the book has more than one sheet.
//!
//! A double click on a cell edits its number. Enter or a click elsewhere writes it as one undo step
//! ("Edit matrix cell"), and Esc cancels. Empty text clears the cell (a missing value, shown `--`);
//! text that is not a number is refused with a note on the status bar.

use crate::actions::Action;
use eframe::egui::{self, Align, Layout, vec2};
use egui_extras::{Column as TableColumn, TableBuilder};
use sheaf_core::format::NumberFormat;
use sheaf_core::{Document, WindowId};

const ROW_HEIGHT: f32 = 20.0;
const ROW_HEADER: f32 = 44.0;
const CELL_WIDTH: f32 = 72.0;

/// What a matrix window keeps between frames.
#[derive(Default)]
pub struct MatrixUi {
    /// The cell being edited: (sheet, row, column, text typed).
    editing: Option<(usize, usize, usize, String)>,
    /// The editor of that cell is to take the keyboard in the next frame (once, when editing starts).
    focus_pending: bool,
}

impl MatrixUi {
    /// The cell being edited, as (sheet, row, column).
    #[cfg(test)]
    pub fn editing_cell(&self) -> Option<(usize, usize, usize)> {
        self.editing.as_ref().map(|e| (e.0, e.1, e.2))
    }
}

/// File > Import > Image (T204): reads a picture file as gray levels into a new matrix book named
/// after the file, as one undo step, and returns its id.
pub fn import_image(doc: &mut Document, path: &std::path::Path) -> Result<WindowId, String> {
    let img = sheaf_io::image_import::read_gray(path)?;
    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    doc.edit("Import Image", |p| {
        let mut sheet = sheaf_core::matrix::MatrixSheet::new("MSheet1", 0, img.rows, img.cols);
        sheet.values_mut().copy_from_slice(&img.values);
        Ok::<_, String>(p.adopt_matrix_book(&name, vec![sheet]))
    })
}

/// What a typed cell text means: `Some(Some(v))` a number, `Some(None)` an empty cell (missing),
/// `None` not a number.
pub fn parse_cell(text: &str) -> Option<Option<f64>> {
    let t = text.trim();
    if t.is_empty() || t == "--" {
        return Some(None);
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some)
}

pub fn show(ui: &mut egui::Ui, doc: &Document, id: WindowId, state: &mut MatrixUi, actions: &mut Vec<Action>) {
    let Some(book) = doc.project().matrix_book(id) else { return };
    let active = book.active_sheet.min(book.sheets.len().saturating_sub(1));
    let Some(sheet) = book.sheets.get(active) else { return };
    if state.editing.as_ref().is_some_and(|e| e.0 != active || e.1 >= sheet.rows || e.2 >= sheet.cols) {
        state.editing = None;
    }
    let format = NumberFormat::default();
    let tabs = book.sheets.len() > 1;
    let tab_height = if tabs { 24.0 } else { 0.0 };
    let table_height = (ui.available_height() - tab_height).max(ROW_HEIGHT * 2.0);
    let mut commit: Option<(usize, usize, String)> = None;
    let mut cancel = false;
    ui.allocate_ui(vec2(ui.available_width(), table_height), |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let mut table = TableBuilder::new(ui)
            .id_salt(("matrix_window", id.0, active))
            .striped(false)
            .resizable(true)
            .cell_layout(Layout::right_to_left(Align::Center))
            .column(TableColumn::exact(ROW_HEADER))
            .min_scrolled_height(0.0)
            .auto_shrink([false, false]);
        for _ in 0..sheet.cols {
            table = table.column(TableColumn::initial(CELL_WIDTH).at_least(30.0).clip(true));
        }
        table
            .header(ROW_HEIGHT, |mut header| {
                header.col(|_| {});
                for c in 0..sheet.cols {
                    header.col(|ui| {
                        ui.with_layout(Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.strong((c + 1).to_string());
                        });
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, sheet.rows, |mut row| {
                    let r = row.index();
                    row.col(|ui| {
                        ui.with_layout(Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.strong((r + 1).to_string());
                        });
                    });
                    for c in 0..sheet.cols {
                        row.col(|ui| {
                            let editing_here =
                                state.editing.as_ref().is_some_and(|e| e.0 == active && e.1 == r && e.2 == c);
                            if editing_here {
                                let text = &mut state.editing.as_mut().expect("editing").3;
                                let edit = egui::TextEdit::singleline(text)
                                    .id_salt(("matrix_cell", id.0, active, r, c))
                                    .horizontal_align(Align::RIGHT);
                                let resp = ui.add_sized(ui.available_size(), edit);
                                if std::mem::take(&mut state.focus_pending) {
                                    resp.request_focus();
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    cancel = true;
                                } else if resp.lost_focus() {
                                    commit = Some((r, c, text.clone()));
                                }
                            } else {
                                // The whole cell takes the clicks; its text is painted at the right.
                                let shown = format.format(sheet.value(r, c));
                                let rect = ui.max_rect();
                                let resp = ui.interact(rect, ui.id().with(("matrix_cell", r, c)), egui::Sense::click());
                                ui.painter().text(
                                    rect.right_center() - vec2(4.0, 0.0),
                                    egui::Align2::RIGHT_CENTER,
                                    &shown,
                                    egui::TextStyle::Body.resolve(ui.style()),
                                    ui.visuals().text_color(),
                                );
                                let info = shown.clone();
                                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &info));
                                if resp.double_clicked() {
                                    let v = sheet.value(r, c);
                                    let text = if v.is_nan() { String::new() } else { format.format(v) };
                                    state.editing = Some((active, r, c, text));
                                    state.focus_pending = true;
                                }
                            }
                        });
                    }
                });
            });
    });
    if cancel {
        state.editing = None;
    } else if let Some((row, col, text)) = commit {
        state.editing = None;
        match parse_cell(&text) {
            Some(value) => {
                let old = sheet.value(row, col);
                let same = match value {
                    None => old.is_nan(),
                    Some(v) => v == old,
                };
                if !same {
                    actions.push(Action::SetMatrixCell { id, sheet: active, row, col, value });
                }
            }
            None => actions.push(Action::Hint(format!("'{}' is not a number", text.trim()))),
        }
    }
    if tabs {
        ui.horizontal(|ui| {
            for (k, s) in book.sheets.iter().enumerate() {
                if ui.selectable_label(k == active, &s.name).clicked() && k != active {
                    actions.push(Action::SetMatrixSheet { id, sheet: k });
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::parse_cell;

    #[test]
    fn typed_cell_text_is_a_number_an_empty_cell_or_refused() {
        assert_eq!(parse_cell(" 2.5 "), Some(Some(2.5)));
        assert_eq!(parse_cell("-1e3"), Some(Some(-1000.0)));
        assert_eq!(parse_cell(""), Some(None));
        assert_eq!(parse_cell("--"), Some(None));
        assert_eq!(parse_cell("abc"), None);
        assert_eq!(parse_cell("inf"), None);
    }
}
