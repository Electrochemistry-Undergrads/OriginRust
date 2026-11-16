//! One worksheet column: its labels, its plotting role and its data.

use crate::format::NumberFormat;
use crate::ids::ColumnId;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
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
    /// A grouping column (Origin's G): its values name the group each row belongs to.
    Group,
    /// The subject of a repeated-measures design (Origin's S): its values name who was measured.
    Subject,
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
            Designation::Group => "G",
            Designation::Subject => "S",
            Designation::Disregard => "",
        }
    }

    pub const ALL: [Designation; 9] = [
        Designation::X,
        Designation::Y,
        Designation::Z,
        Designation::YError,
        Designation::XError,
        Designation::Label,
        Designation::Group,
        Designation::Subject,
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
    /// Origin's "Text & Numeric" format: numbers and text cells in one column. `numbers` holds
    /// the number of every row (NaN for a missing value and for a text cell); `texts` holds the
    /// text of the rows that are text cells (keyed by 0-based row). Every numeric view (plots,
    /// analysis, fits, formulas) reads `numbers` and so sees a text cell as missing. A text only
    /// counts for a row whose number is NaN, so a number written over a text cell wins.
    Mixed { numbers: Arc<Vec<f64>>, texts: Arc<BTreeMap<usize, String>> },
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
            (ColumnData::Mixed { numbers: a, texts: ta }, ColumnData::Mixed { numbers: b, texts: tb }) => {
                (Arc::ptr_eq(a, b)
                    || (a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())))
                    && (Arc::ptr_eq(ta, tb) || ta == tb)
            }
            _ => false,
        }
    }
}

impl Default for ColumnData {
    /// An empty Text & Numeric column: the format Origin gives a new column.
    fn default() -> Self {
        ColumnData::mixed_numbers(Vec::new())
    }
}

impl ColumnData {
    pub fn numeric(values: Vec<f64>) -> Self {
        ColumnData::Numeric(Arc::new(values))
    }

    pub fn text(values: Vec<String>) -> Self {
        ColumnData::Text(Arc::new(values))
    }

    /// A Text & Numeric column. Texts at rows whose number is not missing are dropped.
    pub fn mixed(numbers: Vec<f64>, mut texts: BTreeMap<usize, String>) -> Self {
        texts.retain(|r, t| !t.is_empty() && numbers.get(*r).is_none_or(|x| x.is_nan()));
        ColumnData::Mixed { numbers: Arc::new(numbers), texts: Arc::new(texts) }
    }

    /// A Text & Numeric column that holds only numbers so far (a new Origin column is one).
    pub fn mixed_numbers(numbers: Vec<f64>) -> Self {
        ColumnData::mixed(numbers, BTreeMap::new())
    }

    pub fn len(&self) -> usize {
        match self {
            ColumnData::Numeric(v) => v.len(),
            ColumnData::Text(v) => v.len(),
            ColumnData::Mixed { numbers, texts } => numbers.len().max(texts.keys().next_back().map_or(0, |r| r + 1)),
        }
    }

    /// The text of a text cell of a Text & Numeric column, or of any cell of a text column
    /// (when not empty). `None` for a number or a missing value.
    pub fn text_cell(&self, row: usize) -> Option<&str> {
        match self {
            ColumnData::Numeric(_) => None,
            ColumnData::Text(v) => v.get(row).map(String::as_str).filter(|s| !s.is_empty()),
            ColumnData::Mixed { numbers, texts } => {
                if numbers.get(row).is_none_or(|x| x.is_nan()) {
                    texts.get(&row).map(String::as_str)
                } else {
                    None
                }
            }
        }
    }

    /// True for a Text & Numeric column.
    pub fn is_mixed(&self) -> bool {
        matches!(self, ColumnData::Mixed { .. })
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// True when the column has numbers (Numeric and Text & Numeric), false for a text column.
    pub fn is_numeric(&self) -> bool {
        !matches!(self, ColumnData::Text(_))
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

/// Set on the columns that an analysis tool wrote while its Recalculate choice was Auto or Manual
/// (ADR 0005, "Recalculating results"); the lock in the column head follows it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisMark {
    pub mode: RecalcMode,
    pub state: FormulaState,
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
    /// Present on the result columns of an analysis that recalculates (see [`AnalysisMark`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis: Option<AnalysisMark>,
    /// Extra label rows (name -> value), for example instrument settings.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
    /// True for the "Fitted ..." and "Residual of ..." columns that a fit writes next to its
    /// source column (ADR 0006). They are left out of the default column choice for fitting.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fit_result: bool,
    /// True for a column hidden in the worksheet (Column > Hide/Unhide Columns). It stays in the
    /// sheet, plots and formulas still use it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// The width in characters that the worksheet shows the column at; `None` is the default of
    /// 8 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u16>,
    /// The 0-based rows whose cell is masked (Column > Mask): the point stays in the sheet and in
    /// graphs (drawn red) but analysis, fits and statistics leave it out.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub mask: BTreeSet<usize>,
    /// Column > Mask > Disable Masking: the mask is kept but not applied, so analysis reads the
    /// masked cells and graphs draw them as ordinary points (Origin's `@MM = 1`). A copy of the
    /// project's switch ([`crate::project::Project::masking_disabled`]), kept in line after every edit.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mask_disabled: bool,
    /// Column > Set as Categorical: the values are categories (groups), not quantities. Tools
    /// that need a grouping column, such as Unstack Columns, offer these columns first.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub categorical: bool,
    /// Column > Set Sampling Interval: the column's own X (`x0 + i * inc` for row `i`), which its
    /// plots use instead of an X column (Origin's `colint`; see [`crate::sampling`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<crate::sampling::SamplingInterval>,
    /// The values. Not part of the JSON description: the project file stores
    /// column data separately (see the `sheaf-io` crate).
    #[serde(skip)]
    pub data: ColumnData,
}

/// Origin's default column width in characters.
pub const DEFAULT_WIDTH: u16 = 8;

/// How a column keeps its cells (the data side of Origin's Format list).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Storage {
    Numeric,
    Text,
    /// Text & Numeric.
    Mixed,
}

impl Column {
    /// How the column keeps its cells now.
    pub fn storage(&self) -> Storage {
        match self.data {
            ColumnData::Numeric(_) => Storage::Numeric,
            ColumnData::Text(_) => Storage::Text,
            ColumnData::Mixed { .. } => Storage::Mixed,
        }
    }

    /// Changes how the column keeps its cells, as the Format list of Column Properties does.
    /// Numbers become the text the cells show; text is read as numbers where it can be (what
    /// cannot be is missing in a Numeric column and stays text in a Text & Numeric one).
    /// A Text column shows its text, so its display format goes back to General.
    pub fn convert_storage(&mut self, to: Storage) {
        if self.storage() == to {
            return;
        }
        let rows = self.len();
        let data = match to {
            Storage::Text => {
                let texts: Vec<String> = (0..rows)
                    .map(|r| match self.numeric().and_then(|n| n.get(r)) {
                        Some(x) if x.is_nan() && self.data.text_cell(r).is_none() => String::new(),
                        _ => self.cell_text(r),
                    })
                    .collect();
                ColumnData::text(texts)
            }
            Storage::Numeric | Storage::Mixed => {
                let mut numbers = Vec::with_capacity(rows);
                let mut texts = BTreeMap::new();
                match &self.data {
                    ColumnData::Text(v) => {
                        for (r, t) in v.iter().enumerate() {
                            match t.trim().parse::<f64>() {
                                Ok(x) if x.is_finite() => numbers.push(x),
                                _ => {
                                    numbers.push(f64::NAN);
                                    if !t.is_empty() {
                                        texts.insert(r, t.clone());
                                    }
                                }
                            }
                        }
                    }
                    _ => numbers.extend_from_slice(self.numeric().unwrap_or(&[])),
                }
                if to == Storage::Numeric { ColumnData::numeric(numbers) } else { ColumnData::mixed(numbers, texts) }
            }
        };
        self.data = data;
        if to == Storage::Text {
            self.format = crate::format::NumberFormat::General;
        }
    }

    /// The width in characters the worksheet shows the column at.
    /// The numbers of the column with the masked cells replaced by missing values: what
    /// analysis, fits and statistics read. Borrowed when nothing is masked.
    pub fn numeric_unmasked(&self) -> Option<Cow<'_, [f64]>> {
        let v = self.numeric()?;
        let Some(mask) = self.active_mask() else {
            return Some(Cow::Borrowed(v));
        };
        let mut owned = v.to_vec();
        for r in mask.range(..owned.len()) {
            owned[*r] = f64::NAN;
        }
        Some(Cow::Owned(owned))
    }

    /// The masked rows that are in force: `None` when nothing is masked or masking is disabled.
    pub fn active_mask(&self) -> Option<&BTreeSet<usize>> {
        (!self.mask_disabled && !self.mask.is_empty()).then_some(&self.mask)
    }

    /// Puts the rows in the order `order` (new row i takes old row `order[i]`), data and mask
    /// together. `rows` is the length of the sheet the order was made for.
    pub fn reorder_rows(&mut self, order: &[usize], rows: usize) {
        self.data = crate::analysis::take_rows(&self.data, order, rows);
        if !self.mask.is_empty() {
            self.mask = order.iter().enumerate().filter(|(_, src)| self.mask.contains(src)).map(|(i, _)| i).collect();
        }
    }

    pub fn shown_width(&self) -> u16 {
        self.width.unwrap_or(DEFAULT_WIDTH)
    }

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
            analysis: None,
            params: BTreeMap::new(),
            fit_result: false,
            hidden: false,
            width: None,
            mask: BTreeSet::new(),
            mask_disabled: false,
            categorical: false,
            sampling: None,
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

    /// The numbers of a numeric column, or `None` for a text column. A Text & Numeric column
    /// gives its numbers with every text cell as NaN (missing), as Origin does in calculations.
    pub fn numeric(&self) -> Option<&[f64]> {
        match &self.data {
            ColumnData::Numeric(v) => Some(v.as_slice()),
            ColumnData::Mixed { numbers, .. } => Some(numbers.as_slice()),
            ColumnData::Text(_) => None,
        }
    }

    /// Mutable access to the numbers. If the data is shared with an undo
    /// snapshot it is copied first, so the snapshot stays intact.
    pub fn numeric_mut(&mut self) -> Option<&mut Vec<f64>> {
        match &mut self.data {
            ColumnData::Numeric(v) => Some(Arc::make_mut(v)),
            ColumnData::Mixed { numbers, .. } => Some(Arc::make_mut(numbers)),
            ColumnData::Text(_) => None,
        }
    }

    pub fn text(&self) -> Option<&[String]> {
        match &self.data {
            ColumnData::Text(v) => Some(v.as_slice()),
            ColumnData::Numeric(_) | ColumnData::Mixed { .. } => None,
        }
    }

    pub fn text_mut(&mut self) -> Option<&mut Vec<String>> {
        match &mut self.data {
            ColumnData::Text(v) => Some(Arc::make_mut(v)),
            ColumnData::Numeric(_) | ColumnData::Mixed { .. } => None,
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
            ColumnData::Mixed { numbers, .. } => match (self.data.text_cell(row), numbers.get(row)) {
                (Some(t), _) => t.to_string(),
                (None, Some(x)) => self.format.format(*x),
                (None, None) => String::new(),
            },
        }
    }

    /// The text of a cell as the worksheet shows it. A column in the General format shows up to
    /// 5 decimal digits (Origin's default, Tools > Options > Numeric Format), so 0.841470985 is
    /// shown as 0.84147; very small and very large numbers use scientific notation. Other
    /// formats show what `cell_text` gives. The cell keeps every digit.
    pub fn shown_text(&self, row: usize) -> String {
        let numbers = match &self.data {
            ColumnData::Numeric(v) => Some(v),
            ColumnData::Mixed { numbers, .. } if self.data.text_cell(row).is_none() => Some(numbers),
            _ => None,
        };
        match (numbers, self.format) {
            (Some(v), crate::format::NumberFormat::General) => match v.get(row) {
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
    /// A Text & Numeric column keeps such text as a text cell. Returns `false` when the text
    /// could not be read as a number and was not kept (always `true` for Text & Numeric).
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
            ColumnData::Mixed { numbers, texts } => {
                let numbers = Arc::make_mut(numbers);
                let texts = Arc::make_mut(texts);
                let t = text.trim();
                let (value, kept) = if t.is_empty() || t == "--" {
                    (f64::NAN, None)
                } else {
                    match t.parse::<f64>() {
                        Ok(x) => (x, None),
                        Err(_) => match is_date.then(|| crate::datetime::parse(t, Default::default())).flatten() {
                            Some(p) => (p.value, None),
                            None => (f64::NAN, Some(text.to_string())),
                        },
                    }
                };
                if row >= numbers.len() {
                    numbers.resize(row + 1, f64::NAN);
                }
                numbers[row] = value;
                match kept {
                    Some(k) => texts.insert(row, k),
                    None => texts.remove(&row),
                };
                true
            }
        }
    }
}

/// The longest column short name; Origin cuts a longer one to this length without a message.
pub const MAX_SHORT_NAME: usize = 18;

/// A column short name as Origin keeps it when it is typed in Column Properties or set from a
/// script (measured 2026-11-13): letters, digits and dots stay, every other character (space,
/// underscore, hyphen, brackets, slash, ...) is dropped, and at most [`MAX_SHORT_NAME`] characters
/// are kept. A name may start with a digit there. The result can be empty.
pub fn clean_short_name(text: &str) -> String {
    text.chars().filter(|c| c.is_alphanumeric() || *c == '.').take(MAX_SHORT_NAME).collect()
}

/// A short name from a header cell of an imported text file: [`clean_short_name`], and a first
/// character that is a digit or a dot becomes `A` (`3rd-col` gives `Ardcol`, `9lives` gives
/// `Alives`, `1.5` gives `A.5`). The result can be empty.
pub fn imported_short_name(text: &str) -> String {
    let mut name = clean_short_name(text);
    if name.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        name.replace_range(..1, "A");
    }
    name
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
    #[test]
    fn text_to_numeric_keeps_words_like_inf_and_nan_as_text() {
        let mut c = Column::new(
            ColumnId(1),
            "A",
            Designation::Y,
            ColumnData::text(vec!["1.5".into(), "inf".into(), "NaN".into(), "1e999".into()]),
        );
        c.convert_storage(Storage::Mixed);
        assert_eq!(c.numeric().unwrap()[0], 1.5);
        assert_eq!(c.data.text_cell(1), Some("inf"));
        assert_eq!(c.data.text_cell(2), Some("NaN"));
        assert_eq!(c.data.text_cell(3), Some("1e999"));
    }

    #[test]
    fn storage_conversions_keep_what_can_be_kept() {
        let mut c = Column::new(ColumnId(1), "A", Designation::Y, ColumnData::numeric(vec![1.5, f64::NAN, 3.0]));
        c.format = crate::format::NumberFormat::Decimal(2);
        c.convert_storage(Storage::Text);
        assert_eq!(c.text().unwrap(), ["1.50", "", "3.00"]);
        assert_eq!(c.format, crate::format::NumberFormat::General);
        c.set_cell_text(1, "abc");
        c.convert_storage(Storage::Mixed);
        assert_eq!(c.data.text_cell(1), Some("abc"));
        assert_eq!(c.numeric().unwrap()[0], 1.5);
        c.convert_storage(Storage::Numeric);
        assert!(c.numeric().unwrap()[1].is_nan());
        assert_eq!(c.storage(), Storage::Numeric);
    }

    use super::*;

    #[test]
    fn short_names_are_cleaned_like_origin() {
        // Measured in Origin: the Column Properties box and `wks.col1.name$` give the same results.
        assert_eq!(clean_short_name("a b_c-d.e"), "abcd.e");
        assert_eq!(clean_short_name("my_name_1"), "myname1");
        assert_eq!(clean_short_name("9abc"), "9abc");
        assert_eq!(clean_short_name("a/b(c)+d%e,f:g#h"), "abcdefgh");
        assert_eq!(clean_short_name("abcdefghijklmnopqrstuvwxyz"), "abcdefghijklmnopqr");
        assert_eq!(clean_short_name("_ ."), ".");
        assert_eq!(clean_short_name("__"), "");
        // From an imported header cell a first digit or dot becomes A.
        assert_eq!(imported_short_name("3rd-col"), "Ardcol");
        assert_eq!(imported_short_name("9lives"), "Alives");
        assert_eq!(imported_short_name("12x"), "A2x");
        assert_eq!(imported_short_name("1.5"), "A.5");
        assert_eq!(imported_short_name(".5x"), "A5x");
        assert_eq!(imported_short_name("..a"), "A.a");
        assert_eq!(imported_short_name("x1"), "x1");
        assert_eq!(imported_short_name("Second Col"), "SecondCol");
        assert_eq!(imported_short_name("---"), "");
    }

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
    fn text_and_numeric_keeps_text_and_numbers_see_it_as_missing() {
        let mut c = Column::new(ColumnId(1), "A", Designation::Y, ColumnData::mixed_numbers(vec![1.0]));
        assert!(c.set_cell_text(2, "four"));
        assert!(c.set_cell_text(0, "2.5"));
        assert_eq!(c.len(), 3);
        assert_eq!(c.cell_text(2), "four");
        assert_eq!(c.shown_text(2), "four");
        assert_eq!(c.shown_text(0), "2.5");
        let n = c.numeric().unwrap();
        assert_eq!(n[0], 2.5);
        assert!(n[1].is_nan() && n[2].is_nan());
        assert_eq!(c.cell_text(1), "--");
        // a number typed over the text replaces it; so does an empty cell
        assert!(c.set_cell_text(2, "7"));
        assert_eq!(c.cell_text(2), "7");
        assert!(c.set_cell_text(0, "x"));
        assert!(c.set_cell_text(0, ""));
        assert_eq!(c.cell_text(0), "--");
        assert!(c.data.is_numeric());
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
