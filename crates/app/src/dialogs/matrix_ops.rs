//! The windows of the Matrix menu that ask for something (T192 slice 3, matrices spec 7b): Set Matrix
//! Values..., Set Dimensions..., Expand... and Shrink.... OK applies the operation to the active sheet
//! as one undo step. The window stays open with the problem shown when the operation is refused.

use eframe::egui;
use sheaf_core::matrix_project::MatrixOp;
use sheaf_core::{Document, WindowId};

pub enum Outcome {
    Open,
    Closed,
    Done,
}

/// Which window this is, with what it holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Form {
    /// Set Matrix Values: a formula in i (row), j (column), x and y.
    Values { formula: String },
    /// Set Dimensions: the size and the X and Y ranges.
    Dimensions { cols: usize, rows: usize, x: (f64, f64), y: (f64, f64) },
    /// Expand: each cell becomes `cols` x `rows` cells (bilinear).
    Expand { cols: usize, rows: usize },
    /// Shrink: each block of `cols` x `rows` cells becomes their mean.
    Shrink { cols: usize, rows: usize },
}

impl Form {
    pub fn title(&self) -> &'static str {
        match self {
            Form::Values { .. } => "Set Matrix Values",
            Form::Dimensions { .. } => "Matrix Dimensions and Labels",
            Form::Expand { .. } => "Expand Matrix",
            Form::Shrink { .. } => "Shrink Matrix",
        }
    }

    /// The core operation this form asks for.
    pub fn op(&self) -> MatrixOp {
        match self {
            Form::Values { formula } => MatrixOp::SetValues(formula.clone()),
            Form::Dimensions { cols, rows, x, y } => {
                MatrixOp::SetDimensions { rows: *rows, cols: *cols, ranges: Some((x.0, x.1, y.0, y.1)) }
            }
            Form::Expand { cols, rows } => MatrixOp::Expand { cols: *cols, rows: *rows },
            Form::Shrink { cols, rows } => MatrixOp::Shrink { cols: *cols, rows: *rows },
        }
    }
}

pub struct MatrixOpDialog {
    pub book: WindowId,
    pub sheet: usize,
    pub form: Form,
    problem: Option<String>,
}

/// Applies `op` to sheet `sheet` of `book` as one undo step named after the operation.
pub fn apply(doc: &mut Document, book: WindowId, sheet: usize, op: &MatrixOp) -> Result<(), String> {
    doc.edit(op.label(), |p| p.apply_matrix_op(book, sheet, op).map_err(|e| e.to_string()))
}

impl MatrixOpDialog {
    /// The window for `form` on the active sheet of `book`, filled from that sheet. `None` when
    /// `book` is not a matrix book.
    pub fn new(doc: &Document, book: WindowId, form: Form) -> Option<Self> {
        let b = doc.project().matrix_book(book)?;
        let sheet = b.active_sheet.min(b.sheets.len().saturating_sub(1));
        let s = b.sheets.get(sheet)?;
        let form = match form {
            Form::Dimensions { .. } => {
                Form::Dimensions { cols: s.cols, rows: s.rows, x: (s.x_from, s.x_to), y: (s.y_from, s.y_to) }
            }
            other => other,
        };
        Some(MatrixOpDialog { book, sheet, form, problem: None })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &mut Document) -> Outcome {
        if doc.project().matrix_sheet(self.book, self.sheet).is_err() || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            return Outcome::Closed;
        }
        let mut open = true;
        let mut result = Outcome::Open;
        let mut ok = false;
        egui::Window::new(self.form.title())
            .id(egui::Id::new("matrix_op_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("matrix_op_grid").num_columns(2).show(ui, |ui| match &mut self.form {
                    Form::Values { formula } => {
                        ui.label("Cell(i,j) =");
                        ui.add(egui::TextEdit::singleline(formula).desired_width(260.0));
                        ui.end_row();
                    }
                    Form::Dimensions { cols, rows, x, y } => {
                        ui.label("Columns");
                        ui.add(egui::DragValue::new(cols).range(1..=100_000));
                        ui.end_row();
                        ui.label("Rows");
                        ui.add(egui::DragValue::new(rows).range(1..=100_000));
                        ui.end_row();
                        ui.label("X from / to");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut x.0).speed(0.1));
                            ui.add(egui::DragValue::new(&mut x.1).speed(0.1));
                        });
                        ui.end_row();
                        ui.label("Y from / to");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut y.0).speed(0.1));
                            ui.add(egui::DragValue::new(&mut y.1).speed(0.1));
                        });
                        ui.end_row();
                    }
                    Form::Expand { cols, rows } | Form::Shrink { cols, rows } => {
                        ui.label("Column factor");
                        ui.add(egui::DragValue::new(cols).range(1..=1000));
                        ui.end_row();
                        ui.label("Row factor");
                        ui.add(egui::DragValue::new(rows).range(1..=1000));
                        ui.end_row();
                    }
                });
                if let Some(p) = &self.problem {
                    ui.colored_label(egui::Color32::DARK_RED, p);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ok = ui.button("OK").clicked();
                    if ui.button("Cancel").clicked() {
                        result = Outcome::Closed;
                    }
                });
            });
        if !open {
            return Outcome::Closed;
        }
        if ok {
            match apply(doc, self.book, self.sheet, &self.form.op()) {
                Ok(()) => return Outcome::Done,
                Err(e) => self.problem = Some(e),
            }
        }
        result
    }
}
