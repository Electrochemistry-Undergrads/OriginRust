//! The Formula Bar (T448; Origin 2021's Formula Bar): a strip above a workbook's grid naming the current cell and
//! showing its cell formula (`=B1*2`) or its value; Enter writes what was typed as if typed into the cell (a text
//! starting with `=` makes or changes the cell formula). Rules in `sheaf_core::cell_formula`.

use crate::actions::Action;
use crate::grid::{BookUi, Sel};
use eframe::egui;
use sheaf_core::{ColumnId, Document, WindowId};

/// What the bar is editing: the cell, and the text being typed.
#[derive(Default)]
pub struct FormulaBar {
    pub editing: Option<(ColumnId, usize, String)>,
}

/// The current cell of a workbook: the one cell selected, as (column, 0-based row).
pub fn current_cell(doc: &Document, book: WindowId, state: &BookUi) -> Option<(ColumnId, usize)> {
    let Sel::Cells { anchor, end } = &state.sel else { return None };
    if anchor != end {
        return None;
    }
    let b = doc.project().workbook(book)?;
    let column = b.sheet().columns.get(anchor.0)?;
    Some((column.id, anchor.1))
}

/// The text the bar shows for a cell: `=` and its formula, else the cell as the sheet shows it.
pub fn cell_text(doc: &Document, column: ColumnId, row: usize) -> String {
    let p = doc.project();
    match p.cell_formula(column, row) {
        Some(f) => format!("={f}"),
        None => p.column(column).and_then(|c| sheaf_core::indexed::level_text(c, row)).unwrap_or_default(),
    }
}

pub fn show(
    ui: &mut egui::Ui,
    doc: &Document,
    book: WindowId,
    state: Option<&BookUi>,
    bar: &mut FormulaBar,
    actions: &mut Vec<Action>,
) {
    let cell = state.and_then(|s| current_cell(doc, book, s));
    // The bar follows the current cell; typing in it keeps it until Enter or Esc.
    if bar.editing.as_ref().is_some_and(|(c, r, _)| Some((*c, *r)) != cell) {
        bar.editing = None;
    }
    ui.horizontal(|ui| {
        let name = cell.and_then(|(c, r)| {
            let col = doc.project().column(c)?;
            Some(format!("{}{}", col.short_name, r + 1))
        });
        ui.add_sized([56.0, 18.0], egui::Label::new(name.clone().unwrap_or_default()));
        ui.label("fx");
        let Some((column, row)) = cell else {
            ui.add_enabled(false, egui::TextEdit::singleline(&mut String::new()).desired_width(f32::INFINITY));
            return;
        };
        // Fill Down (T455): the cell's formula copied to the rows below, its relative rows shifted.
        if doc.project().cell_formula(column, row).is_some()
            && ui
                .button("Fill Down")
                .on_hover_text("Copy this formula to the rows below, as far as the sheet has data")
                .clicked()
        {
            actions.push(Action::FillFormulaDown { column, row });
        }
        let (_, _, text) = bar.editing.get_or_insert_with(|| (column, row, cell_text(doc, column, row)));
        let r = ui.add(
            egui::TextEdit::singleline(text)
                .id_salt(("formula_bar", book.0))
                .desired_width(f32::INFINITY)
                .hint_text("a value, or =formula such as =B1*2"),
        );
        if r.lost_focus() {
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && *text != cell_text(doc, column, row) {
                actions.push(Action::SetCell { column, row, text: text.clone() });
            }
            bar.editing = None;
        }
    });
}
