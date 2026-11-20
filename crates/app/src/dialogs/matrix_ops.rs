//! The windows of the Matrix menu that ask for something (T192 slice 3, matrices spec 7b): Set Matrix
//! Values..., Set Dimensions..., Expand... and Shrink.... OK applies the operation to the active sheet
//! as one undo step. The window stays open with the problem shown when the operation is refused.

use eframe::egui;
use sheaf_core::matrix::{FlipDirection, Rotation};
use sheaf_core::matrix_image::{ImageOp, MathOp};
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
    /// An Image menu operation with one setting (T207).
    Image(ImageForm),
}

/// The settings of an Image menu window (T207).
#[derive(Clone, Debug, PartialEq)]
pub enum ImageForm {
    Brightness(f64),
    Contrast(f64),
    Gamma(f64),
    Math(MathOp, f64),
    ToBinary(f64),
    Rotate(Rotation),
    Flip(FlipDirection),
    /// Levels (T210): input low/high to output low/high.
    Levels {
        input: (f64, f64),
        output: (f64, f64),
    },
    /// Crop (T210): 1-based first and last row and column.
    Crop {
        rows: (usize, usize),
        cols: (usize, usize),
    },
    /// Resize (T210): the new size.
    Resize {
        rows: usize,
        cols: usize,
    },
    /// Subtract (T210): another matrix book (its active sheet), from `choices`.
    Subtract {
        choices: Vec<(WindowId, String)>,
        pick: usize,
    },
}

impl ImageForm {
    fn op(&self) -> ImageOp {
        match *self {
            ImageForm::Brightness(a) => ImageOp::Brightness(a),
            ImageForm::Contrast(k) => ImageOp::Contrast(k),
            ImageForm::Gamma(g) => ImageOp::Gamma(g),
            ImageForm::Math(op, k) => ImageOp::Math(op, k),
            ImageForm::ToBinary(t) => ImageOp::ToBinary(t),
            ImageForm::Rotate(r) => ImageOp::Rotate(r),
            ImageForm::Flip(d) => ImageOp::Flip(d),
            ImageForm::Levels { input, output } => ImageOp::Levels { input, output },
            ImageForm::Crop { rows, cols } => ImageOp::Crop {
                rows: (rows.0.saturating_sub(1), rows.1.saturating_sub(1)),
                cols: (cols.0.saturating_sub(1), cols.1.saturating_sub(1)),
            },
            ImageForm::Resize { rows, cols } => ImageOp::Resize { rows, cols },
            ImageForm::Subtract { .. } => unreachable!("Subtract runs through subtract_image"),
        }
    }
}

/// Image > Arithmetic Transform > Subtract (T210): the active sheet of `other` is taken from sheet
/// `sheet` of `book`, in place, as one undo step.
pub fn subtract_image(doc: &mut Document, book: WindowId, sheet: usize, other: WindowId) -> Result<(), String> {
    let o = doc.project().matrix_book(other).ok_or("the other matrix book is gone")?;
    let o = o.sheets.get(o.active_sheet).ok_or("the other matrix book has no sheet")?.clone();
    doc.edit("Image Subtract", |p| {
        let s = p.matrix_sheet_mut(book, sheet).map_err(|e| e.to_string())?;
        *s = sheaf_core::matrix_image::subtract(s, &o).map_err(|e| e.to_string())?;
        Ok(())
    })
}

impl Form {
    pub fn title(&self) -> &'static str {
        match self {
            Form::Values { .. } => "Set Matrix Values",
            Form::Dimensions { .. } => "Matrix Dimensions and Labels",
            Form::Expand { .. } => "Expand Matrix",
            Form::Shrink { .. } => "Shrink Matrix",
            Form::Image(f) => match f {
                ImageForm::Brightness(_) => "Brightness",
                ImageForm::Contrast(_) => "Contrast",
                ImageForm::Gamma(_) => "Gamma",
                ImageForm::Math(..) => "Math",
                ImageForm::ToBinary(_) => "To Binary",
                ImageForm::Rotate(_) => "Rotate",
                ImageForm::Flip(_) => "Flip",
                ImageForm::Levels { .. } => "Levels",
                ImageForm::Crop { .. } => "Crop",
                ImageForm::Resize { .. } => "Resize",
                ImageForm::Subtract { .. } => "Subtract",
            },
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
            Form::Image(_) => unreachable!("an Image form runs through apply_image"),
        }
    }

    /// Runs the form's operation on sheet `sheet` of `book` as one undo step.
    fn run(&self, doc: &mut Document, book: WindowId, sheet: usize) -> Result<(), String> {
        match self {
            Form::Image(ImageForm::Subtract { choices, pick }) => match choices.get(*pick) {
                Some((other, _)) => subtract_image(doc, book, sheet, *other),
                None => Err("there is no other matrix book to subtract".to_string()),
            },
            Form::Image(f) => apply_image(doc, book, sheet, &f.op()),
            _ => apply(doc, book, sheet, &self.op()),
        }
    }
}

pub struct MatrixOpDialog {
    pub book: WindowId,
    pub sheet: usize,
    pub form: Form,
    problem: Option<String>,
}

/// Applies an Image menu operation to sheet `sheet` of `book`, in place, as one undo step (T207).
pub fn apply_image(doc: &mut Document, book: WindowId, sheet: usize, op: &ImageOp) -> Result<(), String> {
    doc.edit(op.label(), |p| {
        let s = p.matrix_sheet_mut(book, sheet).map_err(|e| e.to_string())?;
        *s = op.apply(s).map_err(|e| e.to_string())?;
        Ok(())
    })
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
                    Form::Image(f) => image_controls(ui, f),
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
            match self.form.run(doc, self.book, self.sheet) {
                Ok(()) => return Outcome::Done,
                Err(e) => self.problem = Some(e),
            }
        }
        result
    }
}

/// The controls of an Image menu window (T207).
fn image_controls(ui: &mut egui::Ui, f: &mut ImageForm) {
    match f {
        ImageForm::Brightness(a) => {
            ui.label("Brightness (added)");
            ui.add(egui::DragValue::new(a).speed(1.0));
        }
        ImageForm::Contrast(k) => {
            ui.label("Contrast factor");
            ui.add(egui::DragValue::new(k).speed(0.05).range(0.0..=100.0));
        }
        ImageForm::Gamma(g) => {
            ui.label("Gamma");
            ui.add(egui::DragValue::new(g).speed(0.05).range(0.01..=100.0));
        }
        ImageForm::Math(op, k) => {
            ui.label("Operator");
            ui.horizontal(|ui| {
                for (o, name) in
                    [(MathOp::Add, "+"), (MathOp::Subtract, "-"), (MathOp::Multiply, "*"), (MathOp::Divide, "/")]
                {
                    ui.radio_value(op, o, name);
                }
            });
            ui.end_row();
            ui.label("Operand");
            ui.add(egui::DragValue::new(k).speed(0.1));
        }
        ImageForm::ToBinary(t) => {
            ui.label("Threshold");
            ui.add(egui::DragValue::new(t).speed(1.0));
        }
        ImageForm::Rotate(r) => {
            ui.label("Angle");
            ui.horizontal(|ui| {
                ui.radio_value(r, Rotation::Ccw90, "90 CCW");
                ui.radio_value(r, Rotation::Cw90, "90 CW");
                ui.radio_value(r, Rotation::Half, "180");
            });
        }
        ImageForm::Levels { input, output } => {
            ui.label("Input low / high");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut input.0).speed(1.0));
                ui.add(egui::DragValue::new(&mut input.1).speed(1.0));
            });
            ui.end_row();
            ui.label("Output low / high");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut output.0).speed(1.0));
                ui.add(egui::DragValue::new(&mut output.1).speed(1.0));
            });
        }
        ImageForm::Crop { rows, cols } => {
            ui.label("Rows from / to");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut rows.0).range(1..=1_000_000));
                ui.add(egui::DragValue::new(&mut rows.1).range(1..=1_000_000));
            });
            ui.end_row();
            ui.label("Columns from / to");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut cols.0).range(1..=1_000_000));
                ui.add(egui::DragValue::new(&mut cols.1).range(1..=1_000_000));
            });
        }
        ImageForm::Resize { rows, cols } => {
            ui.label("Columns");
            ui.add(egui::DragValue::new(cols).range(1..=100_000));
            ui.end_row();
            ui.label("Rows");
            ui.add(egui::DragValue::new(rows).range(1..=100_000));
        }
        ImageForm::Subtract { choices, pick } => {
            ui.label("Subtract");
            let shown = choices.get(*pick).map_or("(no other matrix)", |c| c.1.as_str()).to_string();
            egui::ComboBox::from_id_salt("image_subtract_pick").selected_text(shown).show_ui(ui, |ui| {
                for (k, (_, name)) in choices.iter().enumerate() {
                    ui.selectable_value(pick, k, name);
                }
            });
        }
        ImageForm::Flip(d) => {
            ui.label("Direction");
            ui.horizontal(|ui| {
                ui.radio_value(d, FlipDirection::Horizontal, "Horizontal");
                ui.radio_value(d, FlipDirection::Vertical, "Vertical");
            });
        }
    }
    ui.end_row();
}
