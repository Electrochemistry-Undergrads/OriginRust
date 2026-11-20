//! Worksheet > Transpose, Stack Columns, Unstack Columns, Split Columns and Reduce Rows. Each
//! reads columns of a sheet and writes a new sheet of the same workbook, so nothing is lost and
//! one undo step takes it back.

use crate::analysis::AnalysisError;
use crate::column::{Column, ColumnData, Designation};
use crate::ids::{ColumnId, WindowId};
use crate::project::Project;
use crate::worksheet::Worksheet;
use std::collections::BTreeMap;

/// One cell of a column, whatever the column's type.
#[derive(Clone, Debug, PartialEq)]
enum Cell {
    Num(f64),
    Text(String),
    Empty,
}

fn cells(c: &Column) -> Vec<Cell> {
    (0..c.len())
        .map(|r| match c.data.text_cell(r) {
            Some(t) => Cell::Text(t.to_string()),
            None => match c.numeric().and_then(|v| v.get(r).copied()) {
                Some(x) if !x.is_nan() => Cell::Num(x),
                _ => Cell::Empty,
            },
        })
        .collect()
}

/// A column's data from cells: numbers only gives a numeric column, texts only a text column,
/// both a Text & Numeric column.
fn data_of(cells: &[Cell]) -> ColumnData {
    let has_text = cells.iter().any(|c| matches!(c, Cell::Text(_)));
    let has_num = cells.iter().any(|c| matches!(c, Cell::Num(_)));
    let numbers: Vec<f64> = cells.iter().map(|c| if let Cell::Num(x) = c { *x } else { f64::NAN }).collect();
    let texts: BTreeMap<usize, String> = cells
        .iter()
        .enumerate()
        .filter_map(|(i, c)| if let Cell::Text(t) = c { Some((i, t.clone())) } else { None })
        .collect();
    match (has_text, has_num) {
        (false, _) => ColumnData::numeric(numbers),
        (true, false) => ColumnData::text(
            cells.iter().map(|c| if let Cell::Text(t) = c { t.clone() } else { String::new() }).collect(),
        ),
        (true, true) => ColumnData::mixed(numbers, texts),
    }
}

/// How Reduce Rows combines the rows of a group.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Reduce {
    Mean,
    Sum,
    Min,
    Max,
    Median,
    First,
}

impl Reduce {
    pub const ALL: [Reduce; 6] = [Reduce::Mean, Reduce::Sum, Reduce::Min, Reduce::Max, Reduce::Median, Reduce::First];

    pub fn label(&self) -> &'static str {
        match self {
            Reduce::Mean => "Average",
            Reduce::Sum => "Sum",
            Reduce::Min => "Minimum",
            Reduce::Max => "Maximum",
            Reduce::Median => "Median",
            Reduce::First => "First",
        }
    }

    fn apply(&self, group: &[f64]) -> f64 {
        let v: Vec<f64> = group.iter().copied().filter(|x| !x.is_nan()).collect();
        if v.is_empty() {
            return f64::NAN;
        }
        match self {
            Reduce::Mean => v.iter().sum::<f64>() / v.len() as f64,
            Reduce::Sum => v.iter().sum(),
            Reduce::Min => v.iter().copied().fold(f64::INFINITY, f64::min),
            Reduce::Max => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            Reduce::Median => {
                let mut s = v;
                s.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
                let n = s.len();
                if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 }
            }
            Reduce::First => v[0],
        }
    }
}

/// A name for a new sheet that the workbook does not use yet: `base1`, `base2`, ...
fn free_name(sheets: &[Worksheet], base: &str) -> String {
    (1..)
        .map(|n| format!("{base}{n}"))
        .find(|n| !sheets.iter().any(|s| s.name.eq_ignore_ascii_case(n)))
        .expect("unbounded")
}

impl Project {
    /// The columns to work on: the given ones in sheet order, or every column of the sheet.
    fn source_columns(&self, book: WindowId, sheet: usize, ids: &[ColumnId]) -> Result<Vec<Column>, AnalysisError> {
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        let cols: Vec<Column> = ws.columns.iter().filter(|c| ids.is_empty() || ids.contains(&c.id)).cloned().collect();
        if cols.is_empty() {
            return Err(AnalysisError::BadOption("the sheet has no columns to work on".into()));
        }
        Ok(cols)
    }

    /// Adds a sheet to the workbook and returns its index.
    fn push_sheet(&mut self, book: WindowId, base: &str, columns: Vec<Column>) -> usize {
        let b = self.workbook_mut(book).expect("checked by the caller");
        let mut ws = Worksheet::new(free_name(&b.sheets, base));
        ws.columns = columns;
        b.sheets.push(ws);
        b.sheets.len() - 1
    }

    fn new_column(&mut self, name: String, d: Designation, data: ColumnData) -> Column {
        Column::new(ColumnId(self.fresh_id()), name, d, data)
    }

    /// Transpose: row j of the chosen columns becomes column j of a new sheet `Transpose1`. As in
    /// Origin (measured with `wtranspose`), the new columns are named A, B, ... and the label
    /// rows of the first columns stay where they were. Returns the new sheet's index.
    pub fn transpose(&mut self, book: WindowId, sheet: usize, ids: &[ColumnId]) -> Result<usize, AnalysisError> {
        let src = self.source_columns(book, sheet, ids)?;
        let data: Vec<Vec<Cell>> = src.iter().map(cells).collect();
        let rows = data.iter().map(Vec::len).max().unwrap_or(0);
        if rows == 0 {
            return Err(AnalysisError::Empty(src[0].short_name.clone()));
        }
        let mut out = Vec::with_capacity(rows);
        for r in 0..rows {
            let column: Vec<Cell> = data.iter().map(|c| c.get(r).cloned().unwrap_or(Cell::Empty)).collect();
            let (d, labels) = match src.get(r) {
                Some(s) => (s.designation, Some((s.long_name.clone(), s.units.clone(), s.comments.clone()))),
                None => (Designation::Y, None),
            };
            let name = crate::column::default_short_name(r);
            let mut c = self.new_column(name, d, data_of(&column));
            if let Some((l, u, m)) = labels {
                c = c.with_labels(&l, &u, &m);
            }
            out.push(c);
        }
        Ok(self.push_sheet(book, "Transpose", out))
    }

    /// Stack Columns: the chosen columns one below the other in one Y column `Value`, next to a
    /// text column `Source` that names the column each value came from (its long name, else its
    /// short name). Returns the new sheet's index.
    pub fn stack_columns(&mut self, book: WindowId, sheet: usize, ids: &[ColumnId]) -> Result<usize, AnalysisError> {
        let src = self.source_columns(book, sheet, ids)?;
        let (mut values, mut names) = (Vec::new(), Vec::new());
        for c in &src {
            let label = if c.long_name.is_empty() { c.short_name.clone() } else { c.long_name.clone() };
            for cell in cells(c) {
                values.push(cell);
                names.push(Cell::Text(label.clone()));
            }
        }
        let a = self.new_column("A".into(), Designation::X, data_of(&names)).with_labels("Source", "", "");
        let b = self.new_column("B".into(), Designation::Y, data_of(&values)).with_labels("Value", "", "");
        Ok(self.push_sheet(book, "Stack", vec![a, b]))
    }

    /// Unstack Columns: the values of `value` are split into one column per distinct entry of
    /// `key`, in order of first appearance; each new column is named after its entry. Rows with
    /// an empty key are left out. Returns the new sheet's index.
    pub fn unstack_columns(
        &mut self,
        book: WindowId,
        sheet: usize,
        key: ColumnId,
        value: ColumnId,
    ) -> Result<usize, AnalysisError> {
        let keys = cells(self.source_columns(book, sheet, &[key])?.first().expect("one column"));
        let vals = cells(self.source_columns(book, sheet, &[value])?.first().expect("one column"));
        let label = |c: &Cell| match c {
            Cell::Num(x) => Some(x.to_string()),
            Cell::Text(t) => Some(t.clone()),
            Cell::Empty => None,
        };
        let mut groups: Vec<(String, Vec<Cell>)> = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let Some(name) = label(k) else { continue };
            let v = vals.get(i).cloned().unwrap_or(Cell::Empty);
            match groups.iter_mut().find(|g| g.0 == name) {
                Some(g) => g.1.push(v),
                None => groups.push((name, vec![v])),
            }
        }
        if groups.is_empty() {
            return Err(AnalysisError::Empty("the key column".into()));
        }
        let mut out = Vec::new();
        for (i, (name, v)) in groups.into_iter().enumerate() {
            let c = self.new_column(crate::column::default_short_name(i), Designation::Y, data_of(&v));
            out.push(c.with_labels(&name, "", ""));
        }
        Ok(self.push_sheet(book, "Unstack", out))
    }

    /// Plot by group (Origin's `plotgroup`, T224): the rows of the Y columns `ys` (and of the X column
    /// each one is plotted against) split by the values of `group`, in order of first appearance, into
    /// a new sheet `Grouped`: per value an X column (when there is one) and one Y column per Y, whose
    /// long name is the value (followed by the Y's own name when there are several Ys). Rows with an
    /// empty group value are left out. Returns the new sheet and its Y columns, ready to plot.
    pub fn group_columns(
        &mut self,
        book: WindowId,
        sheet: usize,
        ys: &[ColumnId],
        group: ColumnId,
    ) -> Result<(usize, Vec<ColumnId>), AnalysisError> {
        let keys = cells(self.source_columns(book, sheet, &[group])?.first().expect("one column"));
        let src = self.source_columns(book, sheet, ys)?;
        let x = {
            let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
            let first = ys.first().and_then(|y| ws.column_index(*y)).ok_or(AnalysisError::NoColumns)?;
            ws.x_for(first).cloned()
        };
        let label = |c: &Cell| match c {
            Cell::Num(x) => Some(x.to_string()),
            Cell::Text(t) => Some(t.clone()),
            Cell::Empty => None,
        };
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let Some(name) = label(k) else { continue };
            match groups.iter_mut().find(|g| g.0 == name) {
                Some(g) => g.1.push(i),
                None => groups.push((name, vec![i])),
            }
        }
        if groups.is_empty() {
            return Err(AnalysisError::Empty("the group column".into()));
        }
        let pick = |c: &Column, rows: &[usize]| -> Vec<Cell> {
            let all = cells(c);
            rows.iter().map(|&r| all.get(r).cloned().unwrap_or(Cell::Empty)).collect()
        };
        let mut out = Vec::new();
        let mut plotted = Vec::new();
        let mut n = 0;
        for (name, rows) in &groups {
            if let Some(x) = &x {
                let col =
                    self.new_column(crate::column::default_short_name(n), Designation::X, data_of(&pick(x, rows)));
                out.push(col.with_labels(&x.long_name, &x.units, name));
                n += 1;
            }
            for c in &src {
                let own = if c.long_name.is_empty() { c.short_name.clone() } else { c.long_name.clone() };
                let long = if src.len() == 1 { name.clone() } else { format!("{name} {own}") };
                let col =
                    self.new_column(crate::column::default_short_name(n), Designation::Y, data_of(&pick(c, rows)));
                plotted.push(col.id);
                out.push(col.with_labels(&long, &c.units, &c.comments));
                n += 1;
            }
        }
        Ok((self.push_sheet(book, "Grouped", out), plotted))
    }

    /// Split Columns: each chosen column is cut into pieces of `rows` rows, and the pieces become
    /// columns of a new sheet (the last piece may be shorter). Returns the new sheet's index.
    pub fn split_columns(
        &mut self,
        book: WindowId,
        sheet: usize,
        ids: &[ColumnId],
        rows: usize,
    ) -> Result<usize, AnalysisError> {
        if rows == 0 {
            return Err(AnalysisError::BadOption("the number of rows per piece must be at least 1".into()));
        }
        let src = self.source_columns(book, sheet, ids)?;
        let mut out = Vec::new();
        let mut name = 0;
        for c in &src {
            let all = cells(c);
            for (k, piece) in all.chunks(rows).enumerate() {
                let col = self.new_column(crate::column::default_short_name(name), c.designation, data_of(piece));
                let long = if c.long_name.is_empty() { c.short_name.clone() } else { c.long_name.clone() };
                out.push(col.with_labels(&format!("{long} {}", k + 1), &c.units, &c.comments));
                name += 1;
            }
        }
        Ok(self.push_sheet(book, "Split", out))
    }

    /// Reduce Rows: every `n` rows of the chosen numeric columns become one row, combined by
    /// `how`; the last group may be shorter. Returns the new sheet's index.
    pub fn reduce_rows(
        &mut self,
        book: WindowId,
        sheet: usize,
        ids: &[ColumnId],
        n: usize,
        how: Reduce,
    ) -> Result<usize, AnalysisError> {
        if n == 0 {
            return Err(AnalysisError::BadOption("the number of rows per group must be at least 1".into()));
        }
        let src = self.source_columns(book, sheet, ids)?;
        let mut out = Vec::new();
        for (i, c) in src.iter().enumerate() {
            let v = c.numeric().ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
            let reduced: Vec<f64> = v.chunks(n).map(|g| how.apply(g)).collect();
            let mut col =
                self.new_column(crate::column::default_short_name(i), c.designation, ColumnData::numeric(reduced));
            col = col.with_labels(&c.long_name, &c.units, &c.comments);
            out.push(col);
        }
        Ok(self.push_sheet(book, "Reduce", out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> (Project, WindowId, Vec<ColumnId>) {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let ids = [vec![1.0, 2.0], vec![4.0, 5.0], vec![7.0, 8.0]]
            .into_iter()
            .map(|v| p.add_column(b, 0, Designation::Y, ColumnData::numeric(v)).unwrap())
            .collect();
        (p, b, ids)
    }

    fn sheet_values(p: &Project, b: WindowId, s: usize) -> Vec<Vec<f64>> {
        p.workbook(b).unwrap().sheets[s].columns.iter().map(|c| c.numeric().unwrap().to_vec()).collect()
    }

    #[test]
    fn transpose_turns_rows_into_columns_as_origin_does() {
        let (mut p, b, _) = book();
        p.workbook_mut(b).unwrap().sheets[0].columns[0].long_name = "La".into();
        let s = p.transpose(b, 0, &[]).unwrap();
        // Measured: three columns of two rows give two columns of three rows.
        assert_eq!(sheet_values(&p, b, s), vec![vec![1.0, 4.0, 7.0], vec![2.0, 5.0, 8.0]]);
        let ws = &p.workbook(b).unwrap().sheets[s];
        assert_eq!(ws.name, "Transpose1");
        assert_eq!((ws.columns[0].short_name.as_str(), ws.columns[1].short_name.as_str()), ("A", "B"));
        assert_eq!(ws.columns[0].long_name, "La", "the first column keeps the first label");
        assert_eq!(p.workbook(b).unwrap().sheets[0].columns.len(), 3, "the source is untouched");
    }

    #[test]
    fn stack_and_unstack_are_inverse() {
        let (mut p, b, ids) = book();
        let s = p.stack_columns(b, 0, &ids).unwrap();
        let ws = &p.workbook(b).unwrap().sheets[s];
        assert_eq!(ws.columns[1].numeric().unwrap(), [1.0, 2.0, 4.0, 5.0, 7.0, 8.0]);
        assert_eq!(ws.columns[0].data.text_cell(2), Some("B"));
        let (key, val) = (ws.columns[0].id, ws.columns[1].id);
        let u = p.unstack_columns(b, s, key, val).unwrap();
        assert_eq!(sheet_values(&p, b, u), vec![vec![1.0, 2.0], vec![4.0, 5.0], vec![7.0, 8.0]]);
        assert_eq!(p.workbook(b).unwrap().sheets[u].columns[1].long_name, "B");
    }

    #[test]
    fn split_cuts_columns_into_pieces_and_reduce_combines_groups() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let c = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
        let s = p.split_columns(b, 0, &[c], 2).unwrap();
        assert_eq!(sheet_values(&p, b, s), vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0]]);
        let r = p.reduce_rows(b, 0, &[c], 2, Reduce::Mean).unwrap();
        assert_eq!(sheet_values(&p, b, r), vec![vec![1.5, 3.5, 5.0]]);
        let r = p.reduce_rows(b, 0, &[c], 2, Reduce::Median).unwrap();
        assert_eq!(sheet_values(&p, b, r), vec![vec![1.5, 3.5, 5.0]]);
        assert!(p.reduce_rows(b, 0, &[c], 0, Reduce::Sum).is_err());
    }

    #[test]
    fn group_columns_splits_rows_by_the_group_value_with_their_x() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0, 4.0, 5.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![10.0, 20.0, 30.0, 40.0, 50.0])).unwrap();
        let g = p
            .add_column(
                b,
                0,
                Designation::Y,
                ColumnData::text(vec!["b".into(), "a".into(), "b".into(), String::new(), "a".into()]),
            )
            .unwrap();
        let (s, ys) = p.group_columns(b, 0, &[y], g).unwrap();
        let sheet = &p.workbook(b).unwrap().sheets[s];
        assert_eq!(sheet.name, "Grouped1");
        let cols: Vec<(Designation, &str, Vec<f64>)> = sheet
            .columns
            .iter()
            .map(|c| (c.designation, c.long_name.as_str(), c.numeric().unwrap().to_vec()))
            .collect();
        assert_eq!(cols[0], (Designation::X, "", vec![1.0, 3.0]));
        assert_eq!(cols[1], (Designation::Y, "b", vec![10.0, 30.0]));
        assert_eq!(cols[3], (Designation::Y, "a", vec![20.0, 50.0]), "the empty group value is left out");
        assert_eq!(ys, vec![sheet.columns[1].id, sheet.columns[3].id]);
    }
}
