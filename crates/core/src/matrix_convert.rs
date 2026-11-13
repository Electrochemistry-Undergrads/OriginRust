//! Conversions between worksheets and matrices (Origin's `w2m`, `m2w` and `xyz_regular`). The
//! rules, and what was measured in Origin 2016, are in `docs/spec/matrices.md`.

use crate::column::{Column, ColumnData, Designation};
use crate::ids::ColumnId;
use crate::matrix::{MAX_CELLS, MatrixError, MatrixSheet};
use crate::worksheet::Worksheet;

/// Where the coordinates of a worksheet that is converted to a matrix (or that a matrix is
/// written to) are kept.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum XyLayout {
    /// No coordinates: the matrix has its columns and rows numbered 1, 2, 3, ...
    None,
    /// X values run across the columns (one per data column); the Y values are in the first
    /// column when [`W2mOptions::coords_in_first_column`] is set. Matrix columns are sheet columns.
    XAcrossColumns,
    /// Y values run across the columns (one per data column); the X values are in the first column
    /// when [`W2mOptions::coords_in_first_column`] is set. Matrix rows are sheet columns.
    YAcrossColumns,
}

/// Which label row of a column holds a coordinate value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnParam {
    LongName,
    Units,
    Comments,
    /// A user-defined parameter row, by name.
    User(String),
}

impl ColumnParam {
    fn read(&self, c: &Column) -> String {
        match self {
            ColumnParam::LongName => c.long_name.clone(),
            ColumnParam::Units => c.units.clone(),
            ColumnParam::Comments => c.comments.clone(),
            ColumnParam::User(name) => c.params.get(name).cloned().unwrap_or_default(),
        }
    }

    fn write(&self, c: &mut Column, text: String) {
        match self {
            ColumnParam::LongName => c.long_name = text,
            ColumnParam::Units => c.units = text,
            ColumnParam::Comments => c.comments = text,
            ColumnParam::User(name) => {
                c.params.insert(name.clone(), text);
            }
        }
    }
}

/// Where the coordinates along the columns (X for [`XyLayout::XAcrossColumns`], Y for
/// [`XyLayout::YAcrossColumns`]) are read from or written to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LabelSource {
    /// Not stored: the coordinates are 1, 2, 3, ...
    None,
    /// The first data row of the worksheet.
    FirstRow,
    /// A label row of the columns.
    Parameter(ColumnParam),
}

/// Options of [`worksheet_to_matrix`]; the default is what Origin's `w2m` does with no options.
#[derive(Clone, Debug, PartialEq)]
pub struct W2mOptions {
    pub layout: XyLayout,
    /// Where the coordinates across the columns are stored (ignored for [`XyLayout::None`]).
    pub labels: LabelSource,
    /// The first column holds the other coordinate (Y for X across columns, X for Y across).
    pub coords_in_first_column: bool,
    /// How unevenly spaced the coordinates may be, as a percentage: the largest deviation of a step
    /// from the mean step, divided by the mean step. Matrix coordinates are always even, so
    /// coordinates that are uneven by more than this make the conversion fail.
    pub tolerance_percent: f64,
    /// Place the first and last coordinate on a straight line fitted through all of them instead
    /// of at the first and last value.
    pub linear_fit: bool,
    /// Leave out rows and columns of the matrix that hold no number at all.
    pub trim: bool,
}

impl Default for W2mOptions {
    fn default() -> Self {
        W2mOptions {
            layout: XyLayout::None,
            labels: LabelSource::None,
            coords_in_first_column: false,
            tolerance_percent: 5.0,
            linear_fit: false,
            trim: true,
        }
    }
}

fn numbers(c: &Column) -> Vec<f64> {
    match c.numeric() {
        Some(v) => v.to_vec(),
        None => vec![f64::NAN; c.len()],
    }
}

/// The first and last of `n` coordinates that must be evenly spaced, or the reason they are not.
fn even_range(coords: &[f64], tolerance: f64, linear_fit: bool, what: &str) -> Result<(f64, f64), MatrixError> {
    if coords.iter().any(|v| !v.is_finite()) {
        return Err(MatrixError::Invalid(format!("the {what} coordinates contain a missing value or text")));
    }
    let n = coords.len();
    if n == 1 {
        return Ok((coords[0], coords[0]));
    }
    let steps: Vec<f64> = coords.windows(2).map(|w| w[1] - w[0]).collect();
    let mean = steps.iter().sum::<f64>() / steps.len() as f64;
    if mean == 0.0 {
        return Err(MatrixError::Invalid(format!("the {what} coordinates are all equal")));
    }
    let deviation = steps.iter().map(|s| (s - mean).abs()).fold(0.0, f64::max) / mean.abs() * 100.0;
    if deviation > tolerance {
        return Err(MatrixError::Invalid(format!(
            "the {what} coordinates are not evenly spaced (steps differ from the mean step by up to {deviation:.1} %, \
             more than the tolerance of {tolerance} %)"
        )));
    }
    if !linear_fit {
        return Ok((coords[0], coords[n - 1]));
    }
    // Least-squares line through (k, coordinate k).
    let kmean = (n - 1) as f64 / 2.0;
    let cmean = coords.iter().sum::<f64>() / n as f64;
    let (mut sxy, mut sxx) = (0.0, 0.0);
    for (k, c) in coords.iter().enumerate() {
        sxy += (k as f64 - kmean) * (c - cmean);
        sxx += (k as f64 - kmean) * (k as f64 - kmean);
    }
    let slope = sxy / sxx;
    Ok((cmean - slope * kmean, cmean + slope * ((n - 1) as f64 - kmean)))
}

/// Converts worksheet columns to a matrix sheet (Origin's `w2m`, Convert to Matrix > Direct).
/// `columns` are the columns to convert, left to right; the matrix gets `name` and data id 0 (the
/// project gives the real id when it adopts the sheet).
pub fn worksheet_to_matrix(columns: &[&Column], opts: &W2mOptions, name: &str) -> Result<MatrixSheet, MatrixError> {
    if columns.is_empty() {
        return Err(MatrixError::Invalid("there are no columns to convert".into()));
    }
    let mut cols: Vec<Vec<f64>> = columns.iter().map(|c| numbers(c)).collect();
    let mut headers: Vec<&Column> = columns.to_vec();
    let nrows_sheet = cols.iter().map(Vec::len).max().unwrap_or(0);
    for c in &mut cols {
        c.resize(nrows_sheet, f64::NAN);
    }
    let layout = opts.layout;

    // The coordinates in the first column.
    let mut first_col: Option<Vec<f64>> = None;
    if layout != XyLayout::None && opts.coords_in_first_column {
        first_col = Some(cols.remove(0));
        headers.remove(0);
    }
    if cols.is_empty() {
        return Err(MatrixError::Invalid("there is no data column after the coordinate column".into()));
    }
    // The coordinates across the columns.
    let mut across: Option<Vec<f64>> = None;
    let mut data_start = 0;
    if layout != XyLayout::None {
        match &opts.labels {
            LabelSource::None => {}
            LabelSource::FirstRow => {
                if nrows_sheet == 0 {
                    return Err(MatrixError::Invalid("the worksheet has no rows".into()));
                }
                across = Some(cols.iter().map(|c| c[0]).collect());
                data_start = 1;
            }
            LabelSource::Parameter(p) => {
                let mut v = Vec::with_capacity(cols.len());
                for h in &headers {
                    let text = p.read(h);
                    v.push(text.trim().parse::<f64>().map_err(|_| {
                        MatrixError::Invalid(format!(
                            "the label '{}' of column {} is not a number, so it cannot be a coordinate",
                            text, h.short_name
                        ))
                    })?);
                }
                across = Some(v);
            }
        }
    }
    let first_col: Option<Vec<f64>> = first_col.map(|c| c[data_start..].to_vec());
    let nrows = nrows_sheet - data_start;
    if nrows == 0 {
        return Err(MatrixError::Invalid("the worksheet has no data rows".into()));
    }
    if let Some(f) = &first_col {
        debug_assert_eq!(f.len(), nrows);
    }

    // Lay the data out as a matrix of `mrows` x `mcols`, with a coordinate for each.
    let sheet_cols = cols.len();
    let one_to = |n: usize| (1..=n).map(|k| k as f64).collect::<Vec<f64>>();
    let (mrows, mcols, xs, ys, values) = match layout {
        XyLayout::None | XyLayout::XAcrossColumns => {
            let xs = across.clone().unwrap_or_else(|| one_to(sheet_cols));
            let ys = match (&first_col, layout) {
                (Some(f), XyLayout::XAcrossColumns) => f.clone(),
                _ => one_to(nrows),
            };
            let mut v = Vec::with_capacity(nrows * sheet_cols);
            for r in 0..nrows {
                for c in &cols {
                    v.push(c[data_start + r]);
                }
            }
            (nrows, sheet_cols, xs, ys, v)
        }
        XyLayout::YAcrossColumns => {
            let ys = across.clone().unwrap_or_else(|| one_to(sheet_cols));
            let xs = first_col.clone().unwrap_or_else(|| one_to(nrows));
            // Matrix row k is sheet column k; matrix column r is sheet row r.
            let mut v = Vec::with_capacity(nrows * sheet_cols);
            for c in &cols {
                for r in 0..nrows {
                    v.push(c[data_start + r]);
                }
            }
            (sheet_cols, nrows, xs, ys, v)
        }
    };
    if mrows.checked_mul(mcols).is_none_or(|n| n > MAX_CELLS) {
        return Err(MatrixError::TooLarge { rows: mrows, cols: mcols });
    }

    let (mut rows_keep, mut cols_keep): (Vec<usize>, Vec<usize>) = ((0..mrows).collect(), (0..mcols).collect());
    if opts.trim {
        rows_keep.retain(|&r| (0..mcols).any(|c| !values[r * mcols + c].is_nan()));
        cols_keep.retain(|&c| (0..mrows).any(|r| !values[r * mcols + c].is_nan()));
        if rows_keep.is_empty() || cols_keep.is_empty() {
            return Err(MatrixError::Invalid("the columns hold no numbers".into()));
        }
    }
    let (x1, x2) = even_range(&xs, opts.tolerance_percent, opts.linear_fit, "X")?;
    let (y1, y2) = even_range(&ys, opts.tolerance_percent, opts.linear_fit, "Y")?;
    // When rows or columns were left out, the range runs from the first to the last that is kept.
    let (x1, x2) = if opts.trim && cols_keep.len() != mcols {
        let (a, b) = (xs[cols_keep[0]], xs[*cols_keep.last().expect("not empty")]);
        if opts.linear_fit { (x1, x2) } else { (a, b) }
    } else {
        (x1, x2)
    };
    let (y1, y2) = if opts.trim && rows_keep.len() != mrows {
        let (a, b) = (ys[rows_keep[0]], ys[*rows_keep.last().expect("not empty")]);
        if opts.linear_fit { (y1, y2) } else { (a, b) }
    } else {
        (y1, y2)
    };
    let mut out = Vec::with_capacity(rows_keep.len() * cols_keep.len());
    for &r in &rows_keep {
        for &c in &cols_keep {
            out.push(values[r * mcols + c]);
        }
    }
    let mut m = MatrixSheet::from_values(name, 0, rows_keep.len(), cols_keep.len(), out, (x1, x2), (y1, y2))?;
    // The labels of the first data column describe the cell values.
    if let Some(first) = headers.first() {
        m.z.long_name = first.long_name.clone();
        m.z.units = first.units.clone();
    }
    Ok(m)
}

/// Converts XYZ columns that lie on a regular grid to a matrix (Origin's `xyz_regular`).
///
/// The distinct X values and the distinct Y values are sorted. The grid spacing is the smallest
/// difference between neighbouring values, so a grid with gaps keeps its gaps (the cells that no
/// point falls in are missing). Several points in one cell are averaged. Rows with a missing X or Y
/// are ignored; a missing Z leaves its cell missing. Points that do not sit on the grid make the
/// conversion fail, because the points are scattered, not regular.
pub fn xyz_to_matrix(x: &[f64], y: &[f64], z: &[f64], name: &str) -> Result<MatrixSheet, MatrixError> {
    let n = x.len().min(y.len()).min(z.len());
    let pts: Vec<usize> = (0..n).filter(|&i| x[i].is_finite() && y[i].is_finite()).collect();
    if pts.is_empty() {
        return Err(MatrixError::Invalid("the XYZ columns hold no points".into()));
    }
    let axis = |v: &dyn Fn(usize) -> f64, what: &str| -> Result<(f64, f64, usize), MatrixError> {
        let mut vals: Vec<f64> = pts.iter().map(|&i| v(i)).collect();
        vals.sort_by(f64::total_cmp);
        let (lo, hi) = (vals[0], vals[vals.len() - 1]);
        let eps = (hi - lo).abs() * 1e-9;
        let mut step = f64::INFINITY;
        for w in vals.windows(2) {
            let d = w[1] - w[0];
            if d > eps && d < step {
                step = d;
            }
        }
        if !step.is_finite() {
            return Ok((lo, hi, 1));
        }
        let count = ((hi - lo) / step).round() + 1.0;
        if !(count.is_finite() && count >= 1.0 && count <= MAX_CELLS as f64) {
            return Err(MatrixError::Invalid(format!("the {what} values do not form a regular grid")));
        }
        let count = count as usize;
        // Every value must sit on the grid.
        let spacing = (hi - lo) / (count - 1).max(1) as f64;
        for &val in &vals {
            let k = ((val - lo) / spacing).round();
            if (lo + k * spacing - val).abs() > spacing * 1e-6 {
                return Err(MatrixError::Invalid(format!(
                    "the points are not on a regular grid: the {what} value {val} does not fall on the grid of spacing {spacing}; \
                     the points are scattered and need gridding by interpolation"
                )));
            }
        }
        Ok((lo, hi, count))
    };
    let (x1, x2, cols) = axis(&|i| x[i], "X")?;
    let (y1, y2, rows) = axis(&|i| y[i], "Y")?;
    if rows.checked_mul(cols).is_none_or(|c| c > MAX_CELLS) {
        return Err(MatrixError::TooLarge { rows, cols });
    }
    let mut sum = vec![0.0; rows * cols];
    let mut cnt = vec![0u32; rows * cols];
    for &i in &pts {
        if !z[i].is_finite() {
            continue;
        }
        let c = if cols > 1 { ((x[i] - x1) / (x2 - x1) * (cols - 1) as f64).round() as usize } else { 0 };
        let r = if rows > 1 { ((y[i] - y1) / (y2 - y1) * (rows - 1) as f64).round() as usize } else { 0 };
        sum[r * cols + c] += z[i];
        cnt[r * cols + c] += 1;
    }
    let values = sum.iter().zip(&cnt).map(|(s, &c)| if c > 0 { s / f64::from(c) } else { f64::NAN }).collect();
    MatrixSheet::from_values(name, 0, rows, cols, values, (x1, x2), (y1, y2))
}

/// Order of the points when a matrix is written as XYZ columns.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum XyzOrder {
    /// X stays the same while Y runs through all rows (Origin's default, "X Const 1st").
    #[default]
    XConstFirst,
    /// Y stays the same while X runs through all columns ("Y Const 1st").
    YConstFirst,
}

/// How a matrix is written to a worksheet.
#[derive(Clone, Debug, PartialEq)]
pub enum M2wMethod {
    /// One worksheet column per matrix column (or per matrix row for Y across columns), with the
    /// coordinates in the first row or column, or not at all.
    Direct { layout: XyLayout, labels: LabelSource, coords_in_first_column: bool },
    /// Three columns X, Y and Z with a row per matrix cell.
    Xyz { order: XyzOrder, skip_missing: bool },
}

impl Default for M2wMethod {
    fn default() -> Self {
        M2wMethod::Direct { layout: XyLayout::None, labels: LabelSource::None, coords_in_first_column: false }
    }
}

fn label_text(v: f64) -> String {
    format!("{v}")
}

/// Writes a matrix sheet as worksheet columns (Origin's `m2w`). `new_id` gives each column an
/// identifier.
pub fn matrix_to_worksheet(
    m: &MatrixSheet,
    method: &M2wMethod,
    sheet_name: &str,
    mut new_id: impl FnMut() -> ColumnId,
) -> Worksheet {
    let mut ws = Worksheet::new(sheet_name);
    let mut push = |ws: &mut Worksheet, d: Designation, data: Vec<f64>| -> usize {
        let name = ws.next_short_name();
        ws.columns.push(Column::new(new_id(), name, d, ColumnData::numeric(data)));
        ws.columns.len() - 1
    };
    match method {
        M2wMethod::Direct { layout, labels, coords_in_first_column } => {
            let layout = *layout;
            let first_row = layout != XyLayout::None && *labels == LabelSource::FirstRow;
            let with_first = layout != XyLayout::None && *coords_in_first_column;
            // `lines` are the worksheet columns of data; `across` the coordinate of each of them and
            // `along` the coordinate of each of their rows.
            let (lines, across, along): (Vec<Vec<f64>>, Vec<f64>, Vec<f64>) = if layout == XyLayout::YAcrossColumns {
                let lines = (0..m.rows).map(|r| (0..m.cols).map(|c| m.value(r, c)).collect()).collect();
                ((lines), (0..m.rows).map(|r| m.y_at(r)).collect(), (0..m.cols).map(|c| m.x_at(c)).collect())
            } else {
                let lines = (0..m.cols).map(|c| (0..m.rows).map(|r| m.value(r, c)).collect()).collect();
                (lines, (0..m.cols).map(|c| m.x_at(c)).collect(), (0..m.rows).map(|r| m.y_at(r)).collect())
            };
            if with_first {
                let mut col = Vec::new();
                if first_row {
                    col.push(f64::NAN);
                }
                col.extend(&along);
                push(&mut ws, Designation::X, col);
            }
            for (k, line) in lines.into_iter().enumerate() {
                let mut col = Vec::new();
                if first_row {
                    col.push(across[k]);
                }
                col.extend(line);
                let idx = push(&mut ws, Designation::Y, col);
                if layout != XyLayout::None
                    && let LabelSource::Parameter(p) = labels
                {
                    p.write(&mut ws.columns[idx], label_text(across[k]));
                }
            }
        }
        M2wMethod::Xyz { order, skip_missing } => {
            let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
            let mut add = |r: usize, c: usize| {
                let z = m.value(r, c);
                if *skip_missing && z.is_nan() {
                    return;
                }
                xs.push(m.x_at(c));
                ys.push(m.y_at(r));
                zs.push(z);
            };
            match order {
                XyzOrder::XConstFirst => (0..m.cols).for_each(|c| (0..m.rows).for_each(|r| add(r, c))),
                XyzOrder::YConstFirst => (0..m.rows).for_each(|r| (0..m.cols).for_each(|c| add(r, c))),
            }
            let xi = push(&mut ws, Designation::X, xs);
            let yi = push(&mut ws, Designation::Y, ys);
            let zi = push(&mut ws, Designation::Z, zs);
            ws.columns[xi].long_name = m.x.long_name.clone();
            ws.columns[xi].units = m.x.units.clone();
            ws.columns[yi].long_name = m.y.long_name.clone();
            ws.columns[yi].units = m.y.units.clone();
            ws.columns[zi].long_name = m.z.long_name.clone();
            ws.columns[zi].units = m.z.units.clone();
            ws.columns[zi].comments = m.z.comments.clone();
        }
    }
    ws
}
