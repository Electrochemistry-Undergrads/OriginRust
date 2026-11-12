//! One worksheet column: its labels, its plotting role and its data.

use crate::format::NumberFormat;
use crate::ids::ColumnId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The plotting role of a column. A Y column is plotted against the nearest
/// X column to its left; an error column belongs to the nearest Y column to
/// its left.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Designation {
    X,
    #[default]
    Y,
    Z,
    /// Error bar values for the Y column to the left.
    YError,
    /// Error bar values for the X column to the left.
    XError,
    /// Text labels for data points.
    Label,
    /// Not used for plotting.
    Disregard,
}

impl Designation {
    /// The short tag shown in a column header, for example `X` or `yEr±`.
    pub fn tag(&self) -> &'static str {
        match self {
            Designation::X => "X",
            Designation::Y => "Y",
            Designation::Z => "Z",
            Designation::YError => "yEr\u{b1}",
            Designation::XError => "xEr\u{b1}",
            Designation::Label => "L",
            Designation::Disregard => "",
        }
    }

    pub const ALL: [Designation; 7] = [
        Designation::X,
        Designation::Y,
        Designation::Z,
        Designation::YError,
        Designation::XError,
        Designation::Label,
        Designation::Disregard,
    ];
}

/// The values of a column. Data is behind an `Arc` so that copying a whole
/// project (which is how undo snapshots work) does not copy the numbers.
#[derive(Clone, Debug)]
pub enum ColumnData {
    /// Numbers. A missing value is stored as NaN.
    Numeric(Arc<Vec<f64>>),
    /// Text. A missing value is stored as an empty string.
    Text(Arc<Vec<String>>),
}

/// Two columns are equal when they share storage or hold the same bit
/// patterns. Comparing bits (not values) makes a missing value equal to
/// itself, so an untouched column never looks changed.
impl PartialEq for ColumnData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ColumnData::Numeric(a), ColumnData::Numeric(b)) => {
                Arc::ptr_eq(a, b)
                    || (a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits()))
            }
            (ColumnData::Text(a), ColumnData::Text(b)) => Arc::ptr_eq(a, b) || a == b,
            _ => false,
        }
    }
}

impl Default for ColumnData {
    fn default() -> Self {
        ColumnData::Numeric(Arc::new(Vec::new()))
    }
}

impl ColumnData {
    pub fn numeric(values: Vec<f64>) -> Self {
        ColumnData::Numeric(Arc::new(values))
    }

    pub fn text(values: Vec<String>) -> Self {
        ColumnData::Text(Arc::new(values))
    }

    pub fn len(&self) -> usize {
        match self {
            ColumnData::Numeric(v) => v.len(),
            ColumnData::Text(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, ColumnData::Numeric(_))
    }
}

/// When a column formula runs again after the columns it reads have changed (the "Recalculate"
/// choice of Set Column Values).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RecalcMode {
    /// The formula ran once; the values are plain data afterwards.
    #[default]
    None,
    /// The column is recalculated whenever a column it reads changes.
    Auto,
    /// The column is only marked as out of date; the user recalculates it.
    Manual,
}

impl RecalcMode {
    pub const ALL: [RecalcMode; 3] = [RecalcMode::None, RecalcMode::Auto, RecalcMode::Manual];

    pub fn label(&self) -> &'static str {
        match self {
            RecalcMode::None => "None",
            RecalcMode::Auto => "Auto",
            RecalcMode::Manual => "Manual",
        }
    }
}

/// How up to date a recalculating formula column is (the colour of Origin's lock icon).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FormulaState {
    /// The values match the current inputs.
    #[default]
    Current,
    /// An input changed and the user has not recalculated yet.
    Pending,
    /// The formula cannot run: a column it reads is gone or renamed, or the formula is circular.
    Broken,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub id: ColumnId,
    /// Short name, unique within the worksheet (`A`, `B`, ... by default).
    pub short_name: String,
    /// Descriptive name, used for axis titles and legends when present.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub long_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub units: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comments: String,
    #[serde(default)]
    pub designation: Designation,
    #[serde(default)]
    pub format: NumberFormat,
    /// The "set column values" expression that produced this column, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// Whether and when the formula runs again. Only meaningful with a formula.
    #[serde(default, skip_serializing_if = "is_default")]
    pub recalc: RecalcMode,
    /// Whether the values are current. Always `Current` for mode `None`.
    #[serde(default, skip_serializing_if = "is_default")]
    pub formula_state: FormulaState,
    /// The 0-based rows the formula fills; `None` means all rows its inputs have.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula_rows: Option<std::ops::Range<usize>>,
    /// Extra label rows (name -> value), for example instrument settings.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
    /// True for the "Fitted ..." and "Residual of ..." columns that a fit writes next to its
    /// source column (ADR 0006). They are left out of the default column choice for fitting.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fit_result: bool,
    /// The values. Not part of the JSON description: the project file stores
    /// column data separately (see the `sheaf-io` crate).
    #[serde(skip)]
    pub data: ColumnData,
}

impl Column {
    /// True for a column whose values come from a formula that keeps running (mode Auto or
    /// Manual). Origin shows a lock on such a column and refuses hand edits of its cells.
    pub fn is_locked(&self) -> bool {
        self.formula.is_some() && self.recalc != RecalcMode::None
    }

    pub fn new(id: ColumnId, short_name: impl Into<String>, designation: Designation, data: ColumnData) -> Self {
        Column {
            id,
            short_name: short_name.into(),
            long_name: String::new(),
            units: String::new(),
            comments: String::new(),
            designation,
            format: NumberFormat::default(),
            formula: None,
            recalc: RecalcMode::None,
            formula_state: FormulaState::Current,
            formula_rows: None,
            params: BTreeMap::new(),
            fit_result: false,
            data,
        }
    }

    /// Marks the column as a result column written by a fit.
    pub fn as_fit_result(mut self) -> Self {
        self.fit_result = true;
        self
    }

    pub fn with_labels(mut self, long_name: &str, units: &str, comments: &str) -> Self {
        self.long_name = long_name.to_string();
        self.units = units.to_string();
        self.comments = comments.to_string();
        self
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The numbers of a numeric column, or `None` for a text column.
    pub fn numeric(&self) -> Option<&[f64]> {
        match &self.data {
            ColumnData::Numeric(v) => Some(v.as_slice()),
            ColumnData::Text(_) => None,
        }
    }

    /// Mutable access to the numbers. If the data is shared with an undo
    /// snapshot it is copied first, so the snapshot stays intact.
    pub fn numeric_mut(&mut self) -> Option<&mut Vec<f64>> {
        match &mut self.data {
            ColumnData::Numeric(v) => Some(Arc::make_mut(v)),
            ColumnData::Text(_) => None,
        }
    }

    pub fn text(&self) -> Option<&[String]> {
        match &self.data {
            ColumnData::Text(v) => Some(v.as_slice()),
            ColumnData::Numeric(_) => None,
        }
    }

    pub fn text_mut(&mut self) -> Option<&mut Vec<String>> {
        match &mut self.data {
            ColumnData::Text(v) => Some(Arc::make_mut(v)),
            ColumnData::Numeric(_) => None,
        }
    }

    /// The best human-readable name: the long name when set, otherwise the short name.
    pub fn display_name(&self) -> &str {
        if self.long_name.is_empty() { &self.short_name } else { &self.long_name }
    }

    /// Text for an axis title: `Long Name (units)`, or just the name when there are no units.
    pub fn axis_title(&self) -> String {
        if self.units.is_empty() {
            self.display_name().to_string()
        } else {
            format!("{} ({})", self.display_name(), self.units)
        }
    }

    /// The text shown in a worksheet cell. Missing numbers show as `--`.
    pub fn cell_text(&self, row: usize) -> String {
        match &self.data {
            ColumnData::Numeric(v) => match v.get(row) {
                Some(x) => self.format.format(*x),
                None => String::new(),
            },
            ColumnData::Text(v) => v.get(row).cloned().unwrap_or_default(),
        }
    }

    /// The text of a cell as the worksheet shows it. A column in the General format shows up to
    /// 5 decimal digits (Origin's default, Tools > Options > Numeric Format), so 0.841470985 is
    /// shown as 0.84147; very small and very large numbers use scientific notation. Other
    /// formats show what `cell_text` gives. The cell keeps every digit.
    pub fn shown_text(&self, row: usize) -> String {
        match (&self.data, self.format) {
            (ColumnData::Numeric(v), crate::format::NumberFormat::General) => match v.get(row) {
                Some(x) if x.is_finite() && *x != 0.0 => {
                    if x.abs() >= 1e-4 && x.abs() < 1e15 {
                        let text = format!("{x:.5}");
                        text.trim_end_matches('0').trim_end_matches('.').to_string()
                    } else {
                        crate::format::format_general(*x, 5)
                    }
                }
                _ => self.cell_text(row),
            },
            _ => self.cell_text(row),
        }
    }

    /// Sets one cell from typed text. Numeric columns grow with missing
    /// values as needed; text that is not a number becomes a missing value.
    /// Returns `false` when the text could not be read as a number.
    pub fn set_cell_text(&mut self, row: usize, text: &str) -> bool {
        let is_date = self.format.is_date_time();
        match &mut self.data {
            ColumnData::Numeric(v) => {
                let v = Arc::make_mut(v);
                let t = text.trim();
                let (value, ok) = if t.is_empty() || t == "--" {
                    (f64::NAN, true)
                } else {
                    match t.parse::<f64>() {
                        Ok(x) => (x, true),
                        // A date column also takes dates and times as typed text.
                        Err(_) => match is_date.then(|| crate::datetime::parse(t, Default::default())).flatten() {
                            Some(p) => (p.value, true),
                            None => (f64::NAN, false),
                        },
                    }
                };
                if row >= v.len() {
                    v.resize(row + 1, f64::NAN);
                }
                v[row] = value;
                ok
            }
            ColumnData::Text(v) => {
                let v = Arc::make_mut(v);
                if row >= v.len() {
                    v.resize(row + 1, String::new());
                }
                v[row] = text.to_string();
                true
            }
        }
    }
}

/// Default short names in worksheet order: `A`..`Z`, then `AA`, `AB`, ...
pub fn default_short_name(index: usize) -> String {
    let mut n = index;
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).expect("ASCII letters")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_follow_spreadsheet_order() {
        assert_eq!(default_short_name(0), "A");
        assert_eq!(default_short_name(25), "Z");
        assert_eq!(default_short_name(26), "AA");
        assert_eq!(default_short_name(27), "AB");
        assert_eq!(default_short_name(51), "AZ");
        assert_eq!(default_short_name(52), "BA");
        assert_eq!(default_short_name(701), "ZZ");
        assert_eq!(default_short_name(702), "AAA");
    }

    #[test]
    fn axis_title_uses_long_name_and_units() {
        let c = Column::new(ColumnId(1), "A", Designation::X, ColumnData::default()).with_labels("Potential", "V", "");
        assert_eq!(c.axis_title(), "Potential (V)");
        let d = Column::new(ColumnId(2), "B", Designation::Y, ColumnData::default());
        assert_eq!(d.axis_title(), "B");
    }

    #[test]
    fn the_worksheet_shows_five_decimals_for_the_general_format_and_the_cell_keeps_all_digits() {
        let values = vec![0.841470984807897, -0.7568, 10.0, 1234567.891, 0.00001234, 0.0, f64::NAN, 2.5e20];
        let c = Column::new(ColumnId(1), "A", Designation::Y, ColumnData::numeric(values));
        let shown: Vec<String> = (0..8).map(|r| c.shown_text(r)).collect();
        assert_eq!(shown, ["0.84147", "-0.7568", "10", "1234567.891", "1.234E-5", "0", "--", "2.5E20"]);
        assert_eq!(c.cell_text(0), "0.841470985");
        let mut d = c.clone();
        d.format = crate::format::NumberFormat::Decimal(2);
        assert_eq!(d.shown_text(0), "0.84");
    }

    #[test]
    fn set_cell_grows_and_marks_missing() {
        let mut c = Column::new(ColumnId(1), "A", Designation::Y, ColumnData::numeric(vec![1.0]));
        assert!(c.set_cell_text(3, "2.5"));
        assert_eq!(c.len(), 4);
        assert!(c.numeric().unwrap()[1].is_nan());
        assert_eq!(c.numeric().unwrap()[3], 2.5);
        assert!(!c.set_cell_text(0, "abc"));
        assert_eq!(c.cell_text(0), "--");
    }

    #[test]
    fn editing_shared_data_leaves_the_other_copy_alone() {
        let mut a = Column::new(ColumnId(1), "A", Designation::Y, ColumnData::numeric(vec![1.0, 2.0]));
        let snapshot = a.clone();
        a.numeric_mut().unwrap()[0] = 9.0;
        assert_eq!(snapshot.numeric().unwrap(), &[1.0, 2.0]);
        assert_eq!(a.numeric().unwrap(), &[9.0, 2.0]);
    }
}
