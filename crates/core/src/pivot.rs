//! Worksheet > Pivot Table (Origin's X-Function `wpivot`): one row per distinct value, or
//! combination of values, of the row source columns, one column per distinct value of the column
//! source, and in each cell the count of the rows or the sum, mean, minimum or maximum of a data
//! column over them. The table goes to a new sheet of the same workbook (one undo step). The
//! options and their defaults are `wpivot`'s; the output's layout details are Sheaf's own and
//! unmeasured (docs/spec/workbook-model.md "Pivot Table").

use crate::analysis::AnalysisError;
use crate::column::{Column, ColumnData, Designation};
use crate::ids::{ColumnId, WindowId};
use crate::project::Project;
use crate::worksheet::Worksheet;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

/// `wpivot`'s `method`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Count,
    Sum,
    Mean,
    Min,
    Max,
}

impl Method {
    pub const ALL: [Method; 5] = [Method::Count, Method::Sum, Method::Mean, Method::Min, Method::Max];

    pub fn label(self) -> &'static str {
        match self {
            Method::Count => "Count",
            Method::Sum => "Sum",
            Method::Mean => "Mean",
            Method::Min => "Min",
            Method::Max => "Max",
        }
    }
}

/// `wpivot`'s `sort_total` (rows) and `sort_col` (columns): by the labels, by the totals, or in
/// order of first appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    LabelsAscending,
    LabelsDescending,
    TotalsAscending,
    TotalsDescending,
    None,
}

impl Sort {
    /// The row choices in `sort_total`'s order (0 to 4).
    pub const ROWS: [Sort; 5] =
        [Sort::LabelsAscending, Sort::TotalsDescending, Sort::TotalsAscending, Sort::LabelsDescending, Sort::None];
    /// The column choices in `sort_col`'s order (0 to 4).
    pub const COLUMNS: [Sort; 5] =
        [Sort::LabelsAscending, Sort::LabelsDescending, Sort::TotalsAscending, Sort::TotalsDescending, Sort::None];

    /// The name of the choice for rows (`row`) or columns.
    pub fn label(self, row: bool) -> &'static str {
        match (self, row) {
            (Sort::LabelsAscending, true) => "Row Labels Ascending",
            (Sort::LabelsDescending, true) => "Row Labels Descending",
            (Sort::LabelsAscending, false) => "Column Labels Ascending",
            (Sort::LabelsDescending, false) => "Column Labels Descending",
            (Sort::TotalsAscending, true) => "Ascending by Row Totals",
            (Sort::TotalsDescending, true) => "Descending by Row Totals",
            (Sort::TotalsAscending, false) => "Ascending by Column Totals",
            (Sort::TotalsDescending, false) => "Descending by Column Totals",
            (Sort::None, _) => "None",
        }
    }
}

/// `wpivot`'s `normalize`: each column divided by its column total.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Normalize {
    #[default]
    None,
    Fraction,
    Percent,
}

/// `wpivot`'s `pos`: where a value column's label goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InfoPosition {
    LongName,
    #[default]
    Comments,
    /// After the data column's Long Name (or the method's name for Count), separated by a space.
    AppendToLongName,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivotOptions {
    pub rows: Vec<ColumnId>,
    pub column: ColumnId,
    /// Not read by Count.
    pub data: Option<ColumnId>,
    pub method: Method,
    pub row_totals: bool,
    pub column_totals: bool,
    pub sort_rows: Sort,
    pub sort_columns: Sort,
    pub normalize: Normalize,
    /// Missing combinations as 0 (`zero`, default on) or empty.
    pub zeros: bool,
    pub info: InfoPosition,
}

impl PivotOptions {
    /// `wpivot`'s defaults for these sources.
    pub fn new(rows: Vec<ColumnId>, column: ColumnId, data: Option<ColumnId>) -> Self {
        PivotOptions {
            rows,
            column,
            data,
            method: Method::Count,
            row_totals: false,
            column_totals: false,
            sort_rows: Sort::TotalsDescending,
            sort_columns: Sort::LabelsAscending,
            normalize: Normalize::None,
            zeros: true,
            info: InfoPosition::Comments,
        }
    }
}

/// A distinct value of a source column: numbers sort before texts, numbers by value, texts by
/// their characters.
#[derive(Clone, Debug)]
pub enum Key {
    Num(f64),
    Text(String),
}

impl Key {
    pub fn label(&self) -> String {
        match self {
            Key::Num(x) => x.to_string(),
            Key::Text(t) => t.clone(),
        }
    }

    fn id(&self) -> (bool, u64, &str) {
        match self {
            Key::Num(x) => (false, if *x == 0.0 { 0 } else { x.to_bits() }, ""),
            Key::Text(t) => (true, 0, t.as_str()),
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for Key {}

impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Key::Num(a), Key::Num(b)) => a.total_cmp(b),
            (Key::Num(_), Key::Text(_)) => Ordering::Less,
            (Key::Text(_), Key::Num(_)) => Ordering::Greater,
            (Key::Text(a), Key::Text(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The table before it becomes columns: labels in their final order and the cells (NaN for an
/// empty one), plus the totals when asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub rows: Vec<Vec<Key>>,
    pub columns: Vec<Key>,
    /// `cells[r][c]`.
    pub cells: Vec<Vec<f64>>,
    pub row_totals: Option<Vec<f64>>,
    pub column_totals: Option<Vec<f64>>,
    /// The total of the Row Total column, when both totals are on.
    pub grand_total: Option<f64>,
}

/// One source row: its row keys, column key and data value (NaN when missing).
pub type Record = (Vec<Key>, Key, f64);

fn sum(v: impl Iterator<Item = f64>) -> f64 {
    v.filter(|x| !x.is_nan()).sum()
}

/// Builds the table from the usable source rows (in sheet order).
pub fn build(records: &[Record], o: &PivotOptions) -> Table {
    #[derive(Default, Clone, Copy)]
    struct Acc {
        n: usize,
        sum: f64,
        min: f64,
        max: f64,
    }
    impl Acc {
        fn add(&mut self, v: f64) {
            self.n += 1;
            if !v.is_nan() {
                self.sum += v;
                self.min = self.min.min(v);
                self.max = self.max.max(v);
            }
        }
        fn value(&self, method: Method) -> f64 {
            match method {
                Method::Count => self.n as f64,
                Method::Sum => self.sum,
                Method::Mean => self.sum / self.n as f64,
                Method::Min => self.min,
                Method::Max => self.max,
            }
        }
    }
    let mut row_keys: Vec<Vec<Key>> = Vec::new();
    let mut row_index: HashMap<Vec<Key>, usize> = HashMap::new();
    let mut col_keys: Vec<Key> = Vec::new();
    let mut col_index: HashMap<Key, usize> = HashMap::new();
    let mut acc: HashMap<(usize, usize), Acc> = HashMap::new();
    // Every record of a row, of a column and of the table, for the totals (T276: Origin applies the
    // method to them: the Row Total of a Mean table is the mean of the row's records).
    let mut row_acc: HashMap<usize, Acc> = HashMap::new();
    let mut col_acc: HashMap<usize, Acc> = HashMap::new();
    let mut all_acc = Acc { n: 0, sum: 0.0, min: f64::INFINITY, max: f64::NEG_INFINITY };
    for (rk, ck, v) in records {
        if o.method != Method::Count && v.is_nan() {
            continue;
        }
        let r = *row_index.entry(rk.clone()).or_insert_with(|| {
            row_keys.push(rk.clone());
            row_keys.len() - 1
        });
        let c = *col_index.entry(ck.clone()).or_insert_with(|| {
            col_keys.push(ck.clone());
            col_keys.len() - 1
        });
        let empty = Acc { n: 0, sum: 0.0, min: f64::INFINITY, max: f64::NEG_INFINITY };
        for a in
            [acc.entry((r, c)).or_insert(empty), row_acc.entry(r).or_insert(empty), col_acc.entry(c).or_insert(empty)]
        {
            a.add(*v);
        }
        all_acc.add(*v);
    }
    let missing = if o.zeros { 0.0 } else { f64::NAN };
    let mut cells: Vec<Vec<f64>> = (0..row_keys.len())
        .map(|r| {
            (0..col_keys.len())
                .map(|c| match acc.get(&(r, c)) {
                    None => missing,
                    Some(a) => a.value(o.method),
                })
                .collect()
        })
        .collect();
    // The totals apply the method to every record of the row or column (for Count and Sum the same as
    // adding the cells); sorting by totals uses them before normalizing.
    let raw_row_total: Vec<f64> = (0..row_keys.len()).map(|r| row_acc[&r].value(o.method)).collect();
    let raw_col_total: Vec<f64> = (0..col_keys.len()).map(|c| col_acc[&c].value(o.method)).collect();
    // Normalize each column by its total.
    if o.normalize != Normalize::None {
        let scale = if o.normalize == Normalize::Percent { 100.0 } else { 1.0 };
        for c in 0..col_keys.len() {
            let total = sum(cells.iter().map(|row| row[c]));
            for row in &mut cells {
                if !row[c].is_nan() {
                    row[c] = if total != 0.0 { row[c] / total * scale } else { f64::NAN };
                }
            }
        }
    }
    // The Row Total column is the raw row totals; normalized, it is divided by its own total as the other
    // columns are (Origin 2016, T276: East 16 of 36 is 44.44 %, not the sum of East's percentages).
    let row_total: Vec<f64> = if o.normalize == Normalize::None {
        raw_row_total.clone()
    } else {
        let scale = if o.normalize == Normalize::Percent { 100.0 } else { 1.0 };
        let total = sum(raw_row_total.iter().copied());
        raw_row_total.iter().map(|t| if total != 0.0 { t / total * scale } else { f64::NAN }).collect()
    };
    // Normalized, each column's total is its share (100 % or 1); otherwise the method's total.
    let col_total: Vec<f64> = if o.normalize == Normalize::None {
        raw_col_total.clone()
    } else {
        (0..col_keys.len()).map(|c| sum(cells.iter().map(|row| row[c]))).collect()
    };
    let grand = if o.normalize == Normalize::None { all_acc.value(o.method) } else { sum(row_total.iter().copied()) };
    // Order: stable sorts, so equal totals keep the labels' ascending order.
    let mut row_order: Vec<usize> = (0..row_keys.len()).collect();
    let by_label_r = |a: &usize, b: &usize| row_keys[*a].cmp(&row_keys[*b]);
    match o.sort_rows {
        Sort::LabelsAscending => row_order.sort_by(by_label_r),
        Sort::LabelsDescending => row_order.sort_by(|a, b| by_label_r(b, a)),
        Sort::TotalsAscending | Sort::TotalsDescending => {
            row_order.sort_by(by_label_r);
            let desc = o.sort_rows == Sort::TotalsDescending;
            row_order.sort_by(|a, b| {
                let o = raw_row_total[*a].total_cmp(&raw_row_total[*b]);
                if desc { o.reverse() } else { o }
            });
        }
        Sort::None => {}
    }
    let mut col_order: Vec<usize> = (0..col_keys.len()).collect();
    let by_label_c = |a: &usize, b: &usize| col_keys[*a].cmp(&col_keys[*b]);
    match o.sort_columns {
        Sort::LabelsAscending => col_order.sort_by(by_label_c),
        Sort::LabelsDescending => col_order.sort_by(|a, b| by_label_c(b, a)),
        Sort::TotalsAscending | Sort::TotalsDescending => {
            col_order.sort_by(by_label_c);
            let desc = o.sort_columns == Sort::TotalsDescending;
            col_order.sort_by(|a, b| {
                let o = raw_col_total[*a].total_cmp(&raw_col_total[*b]);
                if desc { o.reverse() } else { o }
            });
        }
        Sort::None => {}
    }
    let row_totals_v: Vec<f64> = row_order.iter().map(|r| row_total[*r]).collect();
    Table {
        rows: row_order.iter().map(|r| row_keys[*r].clone()).collect(),
        columns: col_order.iter().map(|c| col_keys[*c].clone()).collect(),
        cells: row_order.iter().map(|r| col_order.iter().map(|c| cells[*r][*c]).collect()).collect(),
        grand_total: (o.row_totals && o.column_totals).then_some(grand),
        row_totals: o.row_totals.then_some(row_totals_v),
        column_totals: o.column_totals.then(|| col_order.iter().map(|c| col_total[*c]).collect()),
    }
}

/// The key of a cell, or `None` for an empty one.
fn key_of(c: &Column, row: usize) -> Option<Key> {
    match c.data.text_cell(row) {
        Some(t) => Some(Key::Text(t.to_string())),
        None => c.numeric().and_then(|v| v.get(row).copied()).filter(|x| !x.is_nan()).map(Key::Num),
    }
}

/// The usable source rows: rows the filter shows, with every row source and the column source set
/// and none of their cells masked. A masked or text data cell counts as missing.
pub fn records(sheet: &Worksheet, o: &PivotOptions) -> Result<Vec<Record>, AnalysisError> {
    let find = |id: ColumnId| sheet.columns.iter().find(|c| c.id == id).ok_or(AnalysisError::NoSuchColumn(id));
    if o.rows.is_empty() {
        return Err(AnalysisError::BadOption("choose at least one row source column".into()));
    }
    let rows: Vec<&Column> = o.rows.iter().map(|id| find(*id)).collect::<Result<_, _>>()?;
    let col = find(o.column)?;
    let data = match (o.method, o.data) {
        (Method::Count, _) => None,
        (_, Some(id)) => Some(find(id)?),
        (_, None) => return Err(AnalysisError::BadOption("choose a data column to summarize".into())),
    };
    if let Some(d) = data
        && d.numeric().is_none()
    {
        return Err(AnalysisError::NotNumeric(d.display_name().to_string()));
    }
    let masked = |c: &Column, r: usize| c.active_mask().is_some_and(|m| m.contains(&r));
    let mut out = Vec::new();
    for r in 0..sheet.row_count() {
        if !sheet.row_shown(r) || rows.iter().chain([&col]).any(|c| masked(c, r)) {
            continue;
        }
        let Some(rk) = rows.iter().map(|c| key_of(c, r)).collect::<Option<Vec<Key>>>() else { continue };
        let Some(ck) = key_of(col, r) else { continue };
        let v = data
            .filter(|d| !masked(d, r) && d.data.text_cell(r).is_none())
            .and_then(|d| d.numeric().and_then(|v| v.get(r).copied()))
            .unwrap_or(f64::NAN);
        out.push((rk, ck, v));
    }
    if out.is_empty() {
        return Err(AnalysisError::BadOption("no row has values in every source column".into()));
    }
    Ok(out)
}

/// A column from keys and a trailing label: numbers stay numbers.
fn key_column(keys: Vec<Option<Key>>) -> ColumnData {
    let has_text = keys.iter().any(|k| matches!(k, Some(Key::Text(_))));
    let numbers: Vec<f64> = keys.iter().map(|k| if let Some(Key::Num(x)) = k { *x } else { f64::NAN }).collect();
    if !has_text {
        return ColumnData::numeric(numbers);
    }
    let texts: BTreeMap<usize, String> = keys
        .iter()
        .enumerate()
        .filter_map(|(i, k)| if let Some(Key::Text(t)) = k { Some((i, t.clone())) } else { None })
        .collect();
    ColumnData::mixed(numbers, texts)
}

impl Project {
    /// Builds the pivot table of a sheet into a new sheet `Pivot1`, `Pivot2`, ... of the same
    /// workbook and returns its index. Columns: one label column per row source (Long Name the
    /// source's name), one value column per column key (its label placed as `info` says), then
    /// Row Total; a last row Column Total when asked for.
    pub fn pivot_table(&mut self, book: WindowId, sheet: usize, o: &PivotOptions) -> Result<usize, AnalysisError> {
        let ws = self.workbook(book).and_then(|b| b.sheets.get(sheet)).ok_or(AnalysisError::NoSuchSheet)?;
        let table = build(&records(ws, o)?, o);
        let name_of = |id: ColumnId| {
            ws.columns.iter().find(|c| c.id == id).map(|c| c.display_name().to_string()).unwrap_or_default()
        };
        let row_names: Vec<String> = o.rows.iter().map(|id| name_of(*id)).collect();
        // Origin 2016's names (T276, measured): "Count", or "Sum of C", "Mean of C", ... after the data column;
        // the Row Total column is "Total_<row source>" and the Column Total row "Total_<column source>".
        let method_label = match (o.method, o.data) {
            (Method::Count, _) | (_, None) => o.method.label().to_string(),
            (_, Some(id)) => format!("{} of {}", o.method.label(), name_of(id)),
        };
        // pos 2 appends "_<key>" to the data column's own Long Name (empty for none).
        let data_long = o
            .data
            .and_then(|id| ws.columns.iter().find(|c| c.id == id))
            .map(|c| c.long_name.clone())
            .unwrap_or_default();
        let row_total_name = format!("Total_{}", row_names.first().map(String::as_str).unwrap_or(""));
        let column_total_name = format!("Total_{}", name_of(o.column));
        let total_row = table.column_totals.is_some();
        let mut out: Vec<(String, String, String, Designation, ColumnData)> = Vec::new();
        for (i, name) in row_names.iter().enumerate() {
            let mut keys: Vec<Option<Key>> = table.rows.iter().map(|k| Some(k[i].clone())).collect();
            if total_row {
                keys.push((i == 0).then(|| Key::Text(column_total_name.clone())));
            }
            let d = if i == 0 { Designation::X } else { Designation::Y };
            // With the key in Comments the source's name is the Long Name; otherwise it moves to Comments.
            let (long, comments) = match o.info {
                InfoPosition::Comments => (name.clone(), String::new()),
                _ => (String::new(), name.clone()),
            };
            out.push((long, String::new(), comments, d, key_column(keys)));
        }
        // `label` is a column key (or the Row Total name); the method's label takes the other place.
        let labelled = |label: &str| -> (String, String) {
            match o.info {
                InfoPosition::LongName => (label.to_string(), method_label.clone()),
                InfoPosition::Comments => (method_label.clone(), label.to_string()),
                InfoPosition::AppendToLongName => (format!("{data_long}_{label}"), method_label.clone()),
            }
        };
        for (c, key) in table.columns.iter().enumerate() {
            let mut v: Vec<f64> = table.cells.iter().map(|row| row[c]).collect();
            if let Some(t) = &table.column_totals {
                v.push(t[c]);
            }
            let (long, comments) = labelled(&key.label());
            out.push((long, String::new(), comments, Designation::Y, ColumnData::numeric(v)));
        }
        if let Some(t) = &table.row_totals {
            let mut v = t.clone();
            if total_row {
                v.push(table.grand_total.unwrap_or(f64::NAN));
            }
            let (long, comments) = labelled(&row_total_name);
            out.push((long, String::new(), comments, Designation::Y, ColumnData::numeric(v)));
        }
        let columns: Vec<Column> = out
            .into_iter()
            .enumerate()
            .map(|(i, (long, units, comments, d, data))| {
                Column::new(ColumnId(self.fresh_id()), crate::column::default_short_name(i), d, data)
                    .with_labels(&long, &units, &comments)
            })
            .collect();
        let b = self.workbook_mut(book).expect("checked above");
        let name = (1..)
            .map(|n| format!("Pivot{n}"))
            .find(|n| !b.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(n)))
            .expect("unbounded");
        let mut new = Worksheet::new(name);
        new.columns = columns;
        b.sheets.push(new);
        Ok(b.sheets.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Key {
        Key::Text(s.into())
    }

    /// Region x product with sales: the shape of Origin's examples.
    fn sales() -> Vec<Record> {
        vec![
            (vec![t("East")], t("Pen"), 10.0),
            (vec![t("West")], t("Pen"), 5.0),
            (vec![t("East")], t("Ink"), 2.0),
            (vec![t("East")], t("Pen"), 4.0),
            (vec![t("North")], t("Ink"), 7.0),
            (vec![t("West")], t("Ink"), f64::NAN),
        ]
    }

    fn opts(method: Method) -> PivotOptions {
        PivotOptions { method, ..PivotOptions::new(vec![ColumnId(1)], ColumnId(2), Some(ColumnId(3))) }
    }

    #[test]
    fn count_with_wpivots_default_sorts() {
        let tb = build(&sales(), &opts(Method::Count));
        // Columns by label ascending; rows by row total descending (East 3, West 2, North 1).
        assert_eq!(tb.columns, vec![t("Ink"), t("Pen")]);
        assert_eq!(tb.rows, vec![vec![t("East")], vec![t("West")], vec![t("North")]]);
        assert_eq!(tb.cells, vec![vec![1.0, 2.0], vec![1.0, 1.0], vec![1.0, 0.0]]);
        assert!(tb.row_totals.is_none() && tb.column_totals.is_none());
    }

    #[test]
    fn sum_mean_min_max_skip_missing_data() {
        let o = PivotOptions { sort_rows: Sort::LabelsAscending, ..opts(Method::Sum) };
        let tb = build(&sales(), &o);
        assert_eq!(tb.rows, vec![vec![t("East")], vec![t("North")], vec![t("West")]]);
        // West/Ink has only a missing value: no entry, shown as 0.
        assert_eq!(tb.cells, vec![vec![2.0, 14.0], vec![7.0, 0.0], vec![0.0, 5.0]]);
        let mean = build(&sales(), &PivotOptions { method: Method::Mean, ..o.clone() });
        assert_eq!(mean.cells[0], vec![2.0, 7.0]);
        let min = build(&sales(), &PivotOptions { method: Method::Min, ..o.clone() });
        assert_eq!(min.cells[0], vec![2.0, 4.0]);
        let max = build(&sales(), &PivotOptions { method: Method::Max, zeros: false, ..o.clone() });
        assert_eq!(max.cells[0], vec![2.0, 10.0]);
        assert!(max.cells[1][1].is_nan(), "without zeros a missing combination is empty");
    }

    #[test]
    fn totals_normalize_and_column_sorts() {
        let o = PivotOptions {
            row_totals: true,
            column_totals: true,
            sort_rows: Sort::LabelsAscending,
            sort_columns: Sort::TotalsDescending,
            ..opts(Method::Sum)
        };
        let tb = build(&sales(), &o);
        assert_eq!(tb.columns, vec![t("Pen"), t("Ink")], "Pen 19 before Ink 9");
        assert_eq!(tb.row_totals, Some(vec![16.0, 7.0, 5.0]));
        assert_eq!(tb.column_totals, Some(vec![19.0, 9.0]));
        assert_eq!(tb.grand_total, Some(28.0));
        let pct = build(&sales(), &PivotOptions { normalize: Normalize::Percent, ..o.clone() });
        assert!(pct.column_totals.unwrap().iter().all(|t| (t - 100.0).abs() < 1e-9));
        assert!((pct.cells[0][1] - 2.0 / 9.0 * 100.0).abs() < 1e-12);
        let frac = build(&sales(), &PivotOptions { normalize: Normalize::Fraction, ..o.clone() });
        assert!(frac.column_totals.unwrap().iter().all(|t| (t - 1.0).abs() < 1e-12));
        let none = build(&sales(), &PivotOptions { sort_rows: Sort::None, sort_columns: Sort::None, ..o });
        assert_eq!(none.rows[0], vec![t("East")]);
        assert_eq!(none.columns, vec![t("Pen"), t("Ink")], "first appearance");
    }

    #[test]
    fn keys_sort_numbers_before_text_and_combine_row_sources() {
        let recs = vec![
            (vec![Key::Num(2.0), t("b")], Key::Num(10.0), 1.0),
            (vec![Key::Num(1.0), t("z")], Key::Num(2.0), 1.0),
            (vec![t("x"), t("a")], Key::Num(10.0), 1.0),
            (vec![Key::Num(1.0), t("a")], Key::Num(10.0), 1.0),
        ];
        let o = PivotOptions { sort_rows: Sort::LabelsAscending, ..opts(Method::Count) };
        let tb = build(&recs, &o);
        assert_eq!(tb.columns, vec![Key::Num(2.0), Key::Num(10.0)], "numeric order, not text order");
        assert_eq!(
            tb.rows,
            vec![
                vec![Key::Num(1.0), t("a")],
                vec![Key::Num(1.0), t("z")],
                vec![Key::Num(2.0), t("b")],
                vec![t("x"), t("a")]
            ]
        );
        let desc = build(&recs, &PivotOptions { sort_rows: Sort::LabelsDescending, ..o });
        assert_eq!(desc.rows[0], vec![t("x"), t("a")]);
    }

    #[test]
    fn project_writes_a_new_sheet_with_labels_and_totals() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let region = p
            .add_column(
                book,
                0,
                Designation::X,
                ColumnData::text(["East", "West", "East", ""].map(String::from).to_vec()),
            )
            .unwrap();
        let product = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.0, 2.0, 2.0])).unwrap();
        let sales = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 6.0, 7.0, 8.0])).unwrap();
        let o = PivotOptions {
            method: Method::Sum,
            row_totals: true,
            column_totals: true,
            sort_rows: Sort::LabelsAscending,
            ..PivotOptions::new(vec![region], product, Some(sales))
        };
        let s = p.pivot_table(book, 0, &o).unwrap();
        let ws = &p.workbook(book).unwrap().sheets[s];
        assert_eq!(ws.name, "Pivot1");
        let names: Vec<&str> = ws.columns.iter().map(|c| c.short_name.as_str()).collect();
        assert_eq!(names, ["A", "B", "C", "D"]);
        // Labels: the row source's name, then the column keys in Comments (wpivot's pos 1).
        assert_eq!(ws.columns[1].comments, "1");
        assert_eq!(ws.columns[2].comments, "2");
        assert_eq!(ws.columns[3].comments, "Total_A");
        assert_eq!(ws.columns[1].long_name, "Sum of C");
        assert_eq!(ws.columns[0].long_name, "A");
        assert_eq!(ws.columns[0].cell_text(0), "East");
        assert_eq!(ws.columns[0].cell_text(2), "Total_B");
        // East: 5 (product 1) and 7 (product 2); West: 6; the empty region is left out.
        assert_eq!(ws.columns[1].numeric().unwrap(), &[5.0, 6.0, 11.0]);
        assert_eq!(ws.columns[2].numeric().unwrap(), &[7.0, 0.0, 7.0]);
        assert_eq!(ws.columns[3].numeric().unwrap(), &[12.0, 6.0, 18.0]);
        // A second table is Pivot2; a missing data column is refused for Sum.
        let o2 = PivotOptions { data: None, ..o };
        assert!(p.pivot_table(book, 0, &o2).is_err());
        let s2 = p.pivot_table(book, 0, &PivotOptions::new(vec![region], product, None)).unwrap();
        assert_eq!(p.workbook(book).unwrap().sheets[s2].name, "Pivot2");
    }

    /// Origin 2016's `wpivot` on Region (A), Product (B), Sales (C), measured 2026-11-21 (T276;
    /// `re-workspace/scratch/pivot-measure/p3.out.txt`).
    #[test]
    fn origin_2016_wpivot_cases() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let text = |v: [&str; 8]| ColumnData::text(v.map(String::from).to_vec());
        let a = p
            .add_column(
                book,
                0,
                Designation::X,
                text(["East", "West", "East", "North", "West", "East", "North", "East"]),
            )
            .unwrap();
        let b = p
            .add_column(book, 0, Designation::Y, text(["pen", "pen", "ink", "pen", "ink", "pad", "ink", "pen"]))
            .unwrap();
        let c = p
            .add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 3.0, 2.0, 7.0, 4.0, 1.0, 6.0, 8.0]))
            .unwrap();
        let sheet = |p: &Project, s: usize| p.workbook(book).unwrap().sheets[s].clone();
        let nums = |ws: &Worksheet, k: usize| ws.columns[k].numeric().unwrap().to_vec();
        // Count, defaults: rows East, North, West (totals 4, 2, 2; ties by label), Long Name "Count".
        let s = p.pivot_table(book, 0, &PivotOptions::new(vec![a], b, Some(c))).unwrap();
        let ws = sheet(&p, s);
        assert_eq!((0..3).map(|r| ws.columns[0].cell_text(r)).collect::<Vec<_>>(), ["East", "North", "West"]);
        assert_eq!(
            (1..4).map(|k| (ws.columns[k].long_name.as_str(), ws.columns[k].comments.as_str())).collect::<Vec<_>>(),
            [("Count", "ink"), ("Count", "pad"), ("Count", "pen")]
        );
        assert_eq!(nums(&ws, 3), [2.0, 1.0, 1.0]);
        // Sum with both totals.
        let sum = PivotOptions {
            method: Method::Sum,
            row_totals: true,
            column_totals: true,
            ..PivotOptions::new(vec![a], b, Some(c))
        };
        let s = p.pivot_table(book, 0, &sum).unwrap();
        let ws = sheet(&p, s);
        assert_eq!(nums(&ws, 4), [16.0, 13.0, 7.0, 36.0]);
        assert_eq!(nums(&ws, 3), [13.0, 7.0, 3.0, 23.0]);
        assert_eq!((ws.columns[4].long_name.as_str(), ws.columns[4].comments.as_str()), ("Sum of C", "Total_A"));
        assert_eq!(ws.columns[0].cell_text(3), "Total_B");
        // Percent: the Row Total column divided by its own total.
        let s = p.pivot_table(book, 0, &PivotOptions { normalize: Normalize::Percent, ..sum.clone() }).unwrap();
        let ws = sheet(&p, s);
        let close = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        assert!(close(&nums(&ws, 1), &[16.666_666_666_667, 50.0, 33.333_333_333_333, 100.0]), "{:?}", nums(&ws, 1));
        assert!(close(&nums(&ws, 4), &[44.444_444_444_444, 36.111_111_111_111, 19.444_444_444_444, 100.0]));
        // Mean without zeros: rows by the totals of the means (North 13, East 9.5, West 7).
        let mean = PivotOptions { method: Method::Mean, zeros: false, ..PivotOptions::new(vec![a], b, Some(c)) };
        let s = p.pivot_table(book, 0, &mean).unwrap();
        let ws = sheet(&p, s);
        assert_eq!((0..3).map(|r| ws.columns[0].cell_text(r)).collect::<Vec<_>>(), ["North", "East", "West"]);
        assert_eq!(nums(&ws, 3), [7.0, 6.5, 3.0]);
        assert!(nums(&ws, 2)[0].is_nan());
        assert_eq!(ws.columns[1].long_name, "Mean of C");
        // Labels descending, columns by descending totals.
        let s = p
            .pivot_table(
                book,
                0,
                &PivotOptions {
                    sort_rows: Sort::LabelsDescending,
                    sort_columns: Sort::TotalsDescending,
                    ..sum.clone()
                },
            )
            .unwrap();
        let ws = sheet(&p, s);
        assert_eq!(ws.columns[0].cell_text(0), "West");
        assert_eq!((1..4).map(|k| ws.columns[k].comments.as_str()).collect::<Vec<_>>(), ["pen", "ink", "pad"]);
        // pos 0: the key is the Long Name, the method in Comments, the source's name in Comments.
        let s = p
            .pivot_table(
                book,
                0,
                &PivotOptions {
                    info: InfoPosition::LongName,
                    method: Method::Sum,
                    ..PivotOptions::new(vec![a], b, Some(c))
                },
            )
            .unwrap();
        let ws = sheet(&p, s);
        assert_eq!((ws.columns[0].long_name.as_str(), ws.columns[0].comments.as_str()), ("", "A"));
        assert_eq!((ws.columns[1].long_name.as_str(), ws.columns[1].comments.as_str()), ("ink", "Sum of C"));
        // pos 2: "_<key>" after the data column's (empty) Long Name.
        let s = p
            .pivot_table(
                book,
                0,
                &PivotOptions {
                    info: InfoPosition::AppendToLongName,
                    method: Method::Max,
                    ..PivotOptions::new(vec![a], b, Some(c))
                },
            )
            .unwrap();
        let ws = sheet(&p, s);
        assert_eq!((ws.columns[1].long_name.as_str(), ws.columns[1].comments.as_str()), ("_ink", "Max of C"));
        assert_eq!(nums(&ws, 3), [8.0, 7.0, 3.0], "rows East, North, West: by the max of each row's records");
        // Totals apply the method to the records: Mean table, North 6.5, ink 4, pen 5.75, grand 4.5.
        let s = p.pivot_table(book, 0, &PivotOptions { row_totals: true, column_totals: true, ..mean }).unwrap();
        let ws = sheet(&p, s);
        assert_eq!(nums(&ws, 4), [6.5, 4.0, 3.5, 4.5]);
        assert_eq!(nums(&ws, 1), [6.0, 2.0, 4.0, 4.0]);
        assert_eq!(nums(&ws, 3), [7.0, 6.5, 3.0, 5.75]);
    }

    #[test]
    fn masked_and_filtered_rows_are_left_out() {
        let mut p = Project::new();
        let book = p.add_workbook("");
        let a = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 1.0, 2.0])).unwrap();
        let b = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![5.0, 5.0, 5.0])).unwrap();
        p.workbook_mut(book).unwrap().sheets[0].columns[0].mask.insert(1);
        let o = PivotOptions::new(vec![a], b, None);
        let recs = records(&p.workbook(book).unwrap().sheets[0], &o).unwrap();
        assert_eq!(recs.len(), 2);
        p.set_row_filter(book, 0, "col(A) > 1").unwrap();
        let recs = records(&p.workbook(book).unwrap().sheets[0], &o).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!((&recs[0].0, &recs[0].1), (&vec![Key::Num(2.0)], &Key::Num(5.0)));
    }
}
