//! Matrix operations on a [`Project`]: each one changes or adds a window, so that
//! [`crate::history::Document::edit`] makes it one undo step, as for workbooks.

use crate::graph::FieldSource;
use crate::ids::{ColumnId, WindowId};
use crate::matrix::{FlipDirection, MatrixError, MatrixSheet, MatrixStats, Rotation};
use crate::matrix_convert::{self, M2wMethod, W2mOptions};
use crate::project::{Project, ProjectError, Window};
use crate::template::FieldTemplate;
use crate::worksheet::Workbook;

/// A change of the shape or the data of a matrix sheet.
#[derive(Clone, Debug, PartialEq)]
pub enum MatrixOp {
    /// Set Matrix Values: fill every cell from a formula in `i`, `j`, `x` and `y`.
    SetValues(String),
    /// New size and, optionally, new X and Y ranges (`x_from, x_to, y_from, y_to`).
    SetDimensions {
        rows: usize,
        cols: usize,
        ranges: Option<(f64, f64, f64, f64)>,
    },
    Transpose,
    Flip {
        direction: FlipDirection,
        coordinates: bool,
    },
    Rotate(Rotation),
    Expand {
        cols: usize,
        rows: usize,
    },
    Shrink {
        cols: usize,
        rows: usize,
    },
}

impl MatrixOp {
    /// The name of the undo step.
    pub fn label(&self) -> &'static str {
        match self {
            MatrixOp::SetValues(_) => "Set Matrix Values",
            MatrixOp::SetDimensions { .. } => "Set Matrix Dimensions",
            MatrixOp::Transpose => "Transpose Matrix",
            MatrixOp::Flip { .. } => "Flip Matrix",
            MatrixOp::Rotate(_) => "Rotate Matrix",
            MatrixOp::Expand { .. } => "Expand Matrix",
            MatrixOp::Shrink { .. } => "Shrink Matrix",
        }
    }

    /// Applies the operation to a sheet and returns the result; the sheet keeps its name,
    /// identity and labels.
    pub fn apply(&self, sheet: &MatrixSheet) -> Result<MatrixSheet, MatrixError> {
        let mut out = match self {
            MatrixOp::SetValues(f) => {
                let mut s = sheet.clone();
                s.set_values(f)?;
                s
            }
            MatrixOp::SetDimensions { rows, cols, ranges } => {
                let mut s = sheet.clone();
                s.set_dimensions(*rows, *cols, *ranges)?;
                s
            }
            MatrixOp::Transpose => sheet.transpose(),
            MatrixOp::Flip { direction, coordinates } => sheet.flip(*direction, *coordinates),
            MatrixOp::Rotate(r) => sheet.rotate90(*r),
            MatrixOp::Expand { cols, rows } => sheet.expand(*cols, *rows)?,
            MatrixOp::Shrink { cols, rows } => sheet.shrink(*cols, *rows)?,
        };
        out.name = sheet.name.clone();
        out.data_id = sheet.data_id;
        Ok(out)
    }
}

impl Project {
    /// A sheet of a matrix book.
    pub fn matrix_sheet(&self, book: WindowId, sheet: usize) -> Result<&MatrixSheet, ProjectError> {
        self.matrix_book(book)
            .ok_or(ProjectError::NoSuchWindow(book))?
            .sheets
            .get(sheet)
            .ok_or(ProjectError::NoSuchMatrixSheet(sheet))
    }

    pub fn matrix_sheet_mut(&mut self, book: WindowId, sheet: usize) -> Result<&mut MatrixSheet, ProjectError> {
        self.matrix_book_mut(book)
            .ok_or(ProjectError::NoSuchWindow(book))?
            .sheets
            .get_mut(sheet)
            .ok_or(ProjectError::NoSuchMatrixSheet(sheet))
    }

    /// Applies an operation to one matrix sheet. On an error the project is not changed.
    pub fn apply_matrix_op(&mut self, book: WindowId, sheet: usize, op: &MatrixOp) -> Result<(), ProjectError> {
        let result = op.apply(self.matrix_sheet(book, sheet)?)?;
        *self.matrix_sheet_mut(book, sheet)? = result;
        Ok(())
    }

    /// Statistics of a matrix sheet (Origin's `mstats`).
    pub fn matrix_stats(&self, book: WindowId, sheet: usize) -> Result<MatrixStats, ProjectError> {
        Ok(self.matrix_sheet(book, sheet)?.stats())
    }

    /// Converts columns of a worksheet to a new matrix book (Convert to Matrix > Direct) and
    /// returns its id. With no `columns` every column of the sheet is converted. The book's long
    /// name is the workbook's.
    pub fn worksheet_to_matrix(
        &mut self,
        book: WindowId,
        sheet: usize,
        columns: Option<&[ColumnId]>,
        opts: &W2mOptions,
    ) -> Result<WindowId, ProjectError> {
        let wb = self.workbook(book).ok_or(ProjectError::NoSuchWindow(book))?;
        let ws = wb.sheets.get(sheet).ok_or(ProjectError::NoSuchLayer(sheet))?;
        let cols: Vec<&crate::column::Column> = match columns {
            None => ws.columns.iter().collect(),
            Some(ids) => {
                ids.iter().map(|id| ws.column(*id).ok_or(ProjectError::NoSuchColumn(*id))).collect::<Result<_, _>>()?
            }
        };
        let m = matrix_convert::worksheet_to_matrix(&cols, opts, "MSheet1")?;
        let long = wb.long_name.clone();
        Ok(self.adopt_matrix_book(&long, vec![m]))
    }

    /// Converts XYZ columns on a regular grid to a new matrix book and returns its id.
    pub fn xyz_to_matrix(&mut self, x: ColumnId, y: ColumnId, z: ColumnId) -> Result<WindowId, ProjectError> {
        let num = |p: &Project, id: ColumnId| -> Result<Vec<f64>, ProjectError> {
            Ok(p.numeric(id).ok_or(ProjectError::NoSuchColumn(id))?.to_vec())
        };
        let (xs, ys, zs) = (num(self, x)?, num(self, y)?, num(self, z)?);
        let mut m = matrix_convert::xyz_to_matrix(&xs, &ys, &zs, "MSheet1")?;
        let label = |p: &Project, id: ColumnId| {
            p.column(id).map(|c| (c.long_name.clone(), c.units.clone(), c.comments.clone())).unwrap_or_default()
        };
        (m.x.long_name, m.x.units, m.x.comments) = label(self, x);
        (m.y.long_name, m.y.units, m.y.comments) = label(self, y);
        (m.z.long_name, m.z.units, m.z.comments) = label(self, z);
        let long = self.locate(z).map(|(b, _, _)| b.long_name.clone()).unwrap_or_default();
        Ok(self.adopt_matrix_book(&long, vec![m]))
    }

    /// Creates a heat map or contour graph of a matrix sheet and returns its id. The graph's long
    /// name is the matrix book's.
    pub fn plot_matrix(
        &mut self,
        template: FieldTemplate,
        book: WindowId,
        sheet: usize,
    ) -> Result<WindowId, ProjectError> {
        self.matrix_sheet(book, sheet)?;
        let long = self.matrix_book(book).map(|b| b.long_name.clone()).unwrap_or_default();
        Ok(self.add_field_graph(template, FieldSource::Matrix { window: book, sheet }, &long))
    }

    /// Creates a contour graph of XYZ columns (a heat map needs the points on a regular grid) and
    /// returns its id. The long name is the first column's workbook's.
    pub fn plot_xyz(
        &mut self,
        template: FieldTemplate,
        x: ColumnId,
        y: ColumnId,
        z: ColumnId,
    ) -> Result<WindowId, ProjectError> {
        for id in [x, y, z] {
            self.numeric(id).ok_or(ProjectError::NoSuchColumn(id))?;
        }
        if matches!(template, FieldTemplate::HeatMap | FieldTemplate::GrayScale) {
            // A heat map has a block per grid cell: the points must lie on a regular grid.
            let (xs, ys, zs) = (self.numeric(x), self.numeric(y), self.numeric(z));
            if let (Some(xs), Some(ys), Some(zs)) = (xs, ys, zs) {
                matrix_convert::xyz_to_matrix(xs, ys, zs, "")?;
            }
        }
        let long = self.locate(z).map(|(b, _, _)| b.long_name.clone()).unwrap_or_default();
        Ok(self.add_field_graph(template, FieldSource::Xyz { x, y, z }, &long))
    }

    /// Plot > Contour/Heat Map > Polar Contour (T222): a colour-fill contour of `z` over the angle
    /// column `theta` (degrees) and the radius column `r`, in Origin's polar layer. Returns the id.
    pub fn plot_xyz_polar(&mut self, theta: ColumnId, r: ColumnId, z: ColumnId) -> Result<WindowId, ProjectError> {
        for id in [theta, r, z] {
            self.numeric(id).ok_or(ProjectError::NoSuchColumn(id))?;
        }
        let long = self.locate(z).map(|(b, _, _)| b.long_name.clone()).unwrap_or_default();
        let id = self.new_window_id();
        let name = self.unique_name("Graph");
        let mut g = FieldTemplate::ContourFill.new_polar_graph(id, &name, FieldSource::Xyz { x: theta, y: r, z });
        g.long_name = long;
        self.windows.push(Window::Graph(g));
        Ok(id)
    }

    fn add_field_graph(&mut self, template: FieldTemplate, source: FieldSource, long_name: &str) -> WindowId {
        let id = self.new_window_id();
        let name = self.unique_name("Graph");
        let mut g = template.new_graph(id, &name, source);
        g.long_name = long_name.to_string();
        self.windows.push(Window::Graph(g));
        id
    }

    /// Analysis > Mathematics > Inverse (T193): the inverse of a square matrix sheet as a new matrix
    /// book, whose long name says what it is. Returns the new book's id.
    pub fn matrix_inverse(&mut self, book: WindowId, sheet: usize) -> Result<WindowId, ProjectError> {
        let inv = self.matrix_sheet(book, sheet)?.inverse()?;
        let name = self.matrix_book(book).map(|b| b.name.clone()).unwrap_or_default();
        Ok(self.adopt_matrix_book(&format!("Inverse of {name}"), vec![inv]))
    }

    /// Analysis > Signal Processing > FFT > 2D FFT and 2D IFFT (T196): a new matrix book with the
    /// transform of a sheet, numpy's `fft2`/`ifft2` conventions (`sheaf_calc::fft2`). The forward
    /// transform of the sheet's values gives the sheets Real, Imaginary, Amplitude and Phase (radians).
    /// The inverse reads the book's sheets named Real and Imaginary when it has both, else the sheet
    /// as real values, and gives Real and Imaginary. Missing values are refused.
    pub fn matrix_fft2(&mut self, book: WindowId, sheet: usize, inverse: bool) -> Result<WindowId, ProjectError> {
        let b = self.matrix_book(book).ok_or(ProjectError::NoSuchWindow(book))?;
        let named = |n: &str| b.sheets.iter().find(|s| s.name.eq_ignore_ascii_case(n));
        let src = self.matrix_sheet(book, sheet)?;
        let (re_sheet, im_sheet) = match (inverse, named("Real"), named("Imaginary")) {
            (true, Some(r), Some(i)) if r.rows == i.rows && r.cols == i.cols => (r, Some(i)),
            _ => (src, None),
        };
        let (rows, cols) = (re_sheet.rows, re_sheet.cols);
        let re: Vec<f64> = re_sheet.values().to_vec();
        let im: Vec<f64> = im_sheet.map(|s| s.values().to_vec()).unwrap_or_default();
        if re.len() != rows * cols || re.iter().chain(&im).any(|v| !v.is_finite()) {
            return Err(crate::matrix::MatrixError::Invalid(
                "the matrix has missing values; fill them before a 2D FFT".into(),
            )
            .into());
        }
        let (out_re, out_im) = sheaf_calc::fft2::fft2(&re, &im, rows, cols, inverse);
        let named_like = |name: &str, data: Vec<f64>| {
            let mut s = re_sheet.like(rows, cols, data);
            s.name = name.to_string();
            s
        };
        let mut sheets = Vec::new();
        if !inverse {
            let amp = out_re.iter().zip(&out_im).map(|(a, b)| a.hypot(*b)).collect();
            let phase = out_re.iter().zip(&out_im).map(|(a, b)| b.atan2(*a)).collect();
            sheets.push(named_like("Real", out_re));
            sheets.push(named_like("Imaginary", out_im));
            sheets.push(named_like("Amplitude", amp));
            sheets.push(named_like("Phase", phase));
        } else {
            sheets.push(named_like("Real", out_re));
            sheets.push(named_like("Imaginary", out_im));
        }
        let name = b.name.clone();
        let what = if inverse { "2D IFFT" } else { "2D FFT" };
        Ok(self.adopt_matrix_book(&format!("{what} of {name}"), sheets))
    }

    /// Writes a matrix sheet to a new workbook and returns its id.
    pub fn matrix_to_worksheet(
        &mut self,
        book: WindowId,
        sheet: usize,
        method: &M2wMethod,
    ) -> Result<WindowId, ProjectError> {
        let m = self.matrix_sheet(book, sheet)?.clone();
        let long = self.matrix_book(book).map(|b| b.long_name.clone()).unwrap_or_default();
        let ws = matrix_convert::matrix_to_worksheet(&m, method, "Sheet1", || self.new_column_id());
        let wb_id = self.new_window_id();
        let name = self.unique_name("Book");
        let mut wb = Workbook::new(wb_id, name);
        wb.long_name = long;
        wb.sheets = vec![ws];
        self.windows.push(Window::Workbook(wb));
        Ok(wb_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::history::Document;
    use crate::matrix_convert::{LabelSource, XyLayout};

    #[test]
    fn matrix_operations_undo_like_workbook_edits() {
        let mut doc = Document::new(Project::new());
        let m = doc.apply("New matrix", |p| p.add_matrix_book("demo"));
        doc.edit("Set Matrix Values", |p| {
            p.apply_matrix_op(m, 0, &MatrixOp::SetDimensions { rows: 2, cols: 3, ranges: Some((0.0, 2.0, 0.0, 1.0)) })?;
            p.apply_matrix_op(m, 0, &MatrixOp::SetValues("i*10+j".into()))
        })
        .unwrap();
        doc.edit("Transpose", |p| p.apply_matrix_op(m, 0, &MatrixOp::Transpose)).unwrap();
        let s = doc.project().matrix_sheet(m, 0).unwrap();
        assert_eq!((s.rows, s.cols), (3, 2));
        assert_eq!(s.value(2, 1), 23.0);
        assert_eq!(doc.undo().as_deref(), Some("Transpose"));
        let s = doc.project().matrix_sheet(m, 0).unwrap();
        assert_eq!((s.rows, s.cols), (2, 3));
        assert_eq!(s.value(1, 2), 23.0);
        assert_eq!(doc.undo().as_deref(), Some("Set Matrix Values"));
        assert_eq!(doc.project().matrix_sheet(m, 0).unwrap().rows, 32);
        assert_eq!(doc.redo().as_deref(), Some("Set Matrix Values"));
        assert_eq!(doc.project().matrix_sheet(m, 0).unwrap().rows, 2);
    }

    #[test]
    fn a_failing_matrix_operation_changes_nothing() {
        let mut doc = Document::new(Project::new());
        let m = doc.apply("New matrix", |p| p.add_matrix_book(""));
        let r = doc.edit("Set Matrix Values", |p| p.apply_matrix_op(m, 0, &MatrixOp::SetValues("i +".into())));
        assert!(r.is_err());
        assert_eq!(doc.undo_label(), Some("New matrix"));
        assert!(doc.edit("x", |p| p.apply_matrix_op(WindowId(999), 0, &MatrixOp::Transpose)).is_err());
    }

    #[test]
    fn worksheet_and_matrix_convert_through_the_project() {
        let mut p = Project::new();
        let book = p.add_workbook("data");
        for vals in [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]] {
            p.add_column(book, 0, Designation::Y, ColumnData::numeric(vals.to_vec())).unwrap();
        }
        let m = p.worksheet_to_matrix(book, 0, None, &W2mOptions::default()).unwrap();
        let s = p.matrix_sheet(m, 0).unwrap();
        assert_eq!((s.rows, s.cols), (2, 3));
        assert_eq!(p.matrix_book(m).unwrap().long_name, "data");
        let method =
            M2wMethod::Direct { layout: XyLayout::None, labels: LabelSource::None, coords_in_first_column: false };
        let back = p.matrix_to_worksheet(m, 0, &method).unwrap();
        let ws = &p.workbook(back).unwrap().sheets[0];
        assert_eq!(ws.columns.len(), 3);
        assert_eq!(ws.columns[2].numeric().unwrap(), &[5.0, 6.0]);
        // all identifiers differ
        let mut ids: Vec<u64> = p.windows.iter().map(|w| w.id().0).collect();
        ids.extend(p.workbooks().flat_map(|b| b.sheets.iter().flat_map(|s| s.columns.iter().map(|c| c.id.0))));
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
