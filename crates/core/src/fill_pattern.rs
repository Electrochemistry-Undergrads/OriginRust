//! Column > Fill Column With: A set of Numbers, A set of Date/Time values and Arbitrary set of
//! Text & Numeric values (T74), as Origin 2016's `patternN`, `patternD` and `patternT` fill a column.
//! Measured on 2026-11-15 (`docs/spec/workbook-model.md`, "Fill Column With patterns"):
//!
//! - The sequence runs from `from` toward `to` by `inc`, `to` included when a step lands on it; an
//!   increment of 0 or one that points away from `to` gives nothing (5 to 1 by 1 is empty, by -1 is
//!   5, 4, 3, 2, 1).
//! - Repeat mode writes each value `each` times in a row and the whole run `sequence` times
//!   (1, 1, 2, 2, 3, 3, 1, 1, ... for 2 and 3); the set size is not used.
//! - Random mode writes `size` values drawn at random, with replacement, from the sequence. Random
//!   dates have no size: they fill as many rows as the sheet already has (the sequence's length when
//!   the sheet is empty).
//! - Dates step by the unit as a fixed length: second, minute, hour, day, a week of 7 days, a month
//!   of 31 days and a year of 365 days (2024-01-01 plus one year is 2024-12-31). The column becomes
//!   a Date column (a Time column for times).
//! - The fill writes from the first row; rows below what it writes keep their values.
//! - Text items are separated by white space; double quotes keep an item with spaces together
//!   (the hint of Origin's window: `Boston "New York" Chicago`; the quoting is not measured).
//!
//! The same submenu's Row Numbers, Uniform Random Numbers and Normal Random Numbers have no window
//! ([`QuickFill`], T63 group H, documented, not measured): they fill every row of the sheet.

use crate::column::ColumnData;
use crate::datetime::{DatePattern, TimePattern};
use crate::format::NumberFormat;
use crate::ids::ColumnId;
use crate::project::{Project, ProjectError};
use std::collections::BTreeMap;

/// The most values one fill writes.
pub const MAX_FILL_ROWS: usize = 10_000_000;

/// The rows a quick fill writes into a sheet that has none yet (Origin's new sheet has 32).
pub const QUICK_FILL_EMPTY_ROWS: usize = 32;

/// Column > Fill Column With > Row Numbers, Uniform Random Numbers and Normal Random Numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickFill {
    /// 1, 2, 3, ... down the sheet.
    RowNumbers,
    /// Uniform on [0, 1), as `rnd()`.
    Uniform,
    /// Normal with mean 0 and standard deviation 1, as `grnd()`.
    Normal,
}

impl QuickFill {
    /// The menu text.
    pub fn label(self) -> &'static str {
        match self {
            QuickFill::RowNumbers => "Row Numbers",
            QuickFill::Uniform => "Uniform Random Numbers",
            QuickFill::Normal => "Normal Random Numbers",
        }
    }

    /// `rows` values; `seed` drives the random ones.
    pub fn values(self, rows: usize, seed: u64) -> Vec<f64> {
        let mut rng = Rng(seed);
        match self {
            QuickFill::RowNumbers => (1..=rows).map(|r| r as f64).collect(),
            QuickFill::Uniform => (0..rows).map(|_| rng.unit()).collect(),
            QuickFill::Normal => (0..rows)
                .map(|_| {
                    // Box-Muller; 1 - u keeps the logarithm finite.
                    let (u, v) = (1.0 - rng.unit(), rng.unit());
                    (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
                })
                .collect(),
        }
    }
}

/// Repeat the run in order, or draw values at random.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillMode {
    #[default]
    Repeat,
    Random,
}

/// The step unit of a date or time fill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateUnit {
    Second,
    Minute,
    Hour,
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl DateUnit {
    pub const ALL: [DateUnit; 7] = [
        DateUnit::Second,
        DateUnit::Minute,
        DateUnit::Hour,
        DateUnit::Day,
        DateUnit::Week,
        DateUnit::Month,
        DateUnit::Year,
    ];

    /// The length of one unit in days, as Origin steps it (a month is 31 days, a year 365).
    pub fn days(self) -> f64 {
        match self {
            DateUnit::Second => 1.0 / 86_400.0,
            DateUnit::Minute => 1.0 / 1_440.0,
            DateUnit::Hour => 1.0 / 24.0,
            DateUnit::Day => 1.0,
            DateUnit::Week => 7.0,
            DateUnit::Month => 31.0,
            DateUnit::Year => 365.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DateUnit::Second => "Second",
            DateUnit::Minute => "Minute",
            DateUnit::Hour => "Hour",
            DateUnit::Day => "Day",
            DateUnit::Week => "Week",
            DateUnit::Month => "Month",
            DateUnit::Year => "Year",
        }
    }
}

/// What to fill a column with.
#[derive(Clone, Debug, PartialEq)]
pub enum FillPattern {
    /// A set of Numbers.
    Numbers { from: f64, to: f64, inc: f64, mode: FillMode, each: usize, sequence: usize, size: usize },
    /// A set of Date/Time values: date numbers (or fractions of a day with `time`) stepped by `inc` units.
    Dates { time: bool, from: f64, to: f64, inc: f64, unit: DateUnit, mode: FillMode },
    /// Arbitrary set of Text & Numeric values: the items of `text`.
    Text { text: String, mode: FillMode, each: usize, sequence: usize, size: usize },
}

impl FillPattern {
    /// Origin's defaults for a set of numbers: 1 to 10 by 1, each once, the run once, a random set of 10.
    pub fn numbers() -> Self {
        FillPattern::Numbers { from: 1.0, to: 10.0, inc: 1.0, mode: FillMode::Repeat, each: 1, sequence: 1, size: 10 }
    }
}

/// Why a fill could not be made.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum FillError {
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("the start, end and increment must be numbers")]
    NotFinite,
    #[error("the pattern gives no value: the increment does not lead from the start to the end")]
    Empty,
    #[error("the pattern gives {0} values, more than the {MAX_FILL_ROWS} one fill can write")]
    TooMany(usize),
    #[error("give at least one text item")]
    NoItems,
}

/// The run from `from` toward `to` by `inc`, `to` included when a step lands on it (to within a
/// millionth of the step). Empty for a step of 0 or one that points away from `to`.
pub fn number_run(from: f64, to: f64, inc: f64) -> Result<Vec<f64>, FillError> {
    if !(from.is_finite() && to.is_finite() && inc.is_finite()) {
        return Err(FillError::NotFinite);
    }
    if inc == 0.0 || (to - from) * inc < 0.0 {
        return Ok(if from == to { vec![from] } else { Vec::new() });
    }
    let steps = ((to - from) / inc + 1e-6).floor();
    let count = steps as usize + 1;
    if !steps.is_finite() || count > MAX_FILL_ROWS {
        return Err(FillError::TooMany(count));
    }
    Ok((0..count).map(|k| from + k as f64 * inc).collect())
}

/// The items of a text fill: separated by white space, double quotes keep spaces inside an item.
pub fn text_items(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in text.chars() {
        if c == '"' {
            quoted = !quoted;
            started = true;
        } else if c.is_whitespace() && !quoted {
            if started {
                items.push(std::mem::take(&mut cur));
                started = false;
            }
        } else {
            cur.push(c);
            started = true;
        }
    }
    if started {
        items.push(cur);
    }
    items
}

/// A small splitmix64 generator for the random modes.
struct Rng(u64);

impl Rng {
    /// A number in [0, 1).
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (((z >> 11) as f64 / (1u64 << 53) as f64) * n as f64) as usize % n.max(1)
    }
}

/// The run laid out as Repeat or Random mode lays it out.
fn lay_out<T: Clone>(
    run: &[T],
    mode: FillMode,
    each: usize,
    sequence: usize,
    size: usize,
    seed: u64,
) -> Result<Vec<T>, FillError> {
    if run.is_empty() {
        return Err(FillError::Empty);
    }
    let count = match mode {
        FillMode::Repeat => run.len().saturating_mul(each.max(1)).saturating_mul(sequence.max(1)),
        FillMode::Random => size,
    };
    if count > MAX_FILL_ROWS {
        return Err(FillError::TooMany(count));
    }
    Ok(match mode {
        FillMode::Repeat => {
            let one: Vec<T> = run.iter().flat_map(|v| std::iter::repeat_n(v.clone(), each.max(1))).collect();
            (0..sequence.max(1)).flat_map(|_| one.iter().cloned()).collect()
        }
        FillMode::Random => {
            let mut rng = Rng(seed);
            (0..size).map(|_| run[rng.below(run.len())].clone()).collect()
        }
    })
}

/// The values a pattern writes: numbers with the column's new format, or text items. `rows` is the
/// row count of the sheet (random dates fill that many); `seed` drives the random modes.
pub enum FillValues {
    Numbers(Vec<f64>, Option<NumberFormat>),
    Texts(Vec<String>),
}

pub fn generate(pattern: &FillPattern, rows: usize, seed: u64) -> Result<FillValues, FillError> {
    match pattern {
        FillPattern::Numbers { from, to, inc, mode, each, sequence, size } => {
            let run = number_run(*from, *to, *inc)?;
            Ok(FillValues::Numbers(lay_out(&run, *mode, *each, *sequence, *size, seed)?, None))
        }
        FillPattern::Dates { time, from, to, inc, unit, mode } => {
            let run = number_run(*from, *to, inc * unit.days())?;
            let size = if rows == 0 { run.len() } else { rows };
            let values = lay_out(&run, *mode, 1, 1, size, seed)?;
            let format = if *time { NumberFormat::Time(TimePattern::Hm) } else { NumberFormat::Date(DatePattern::Mdy) };
            Ok(FillValues::Numbers(values, Some(format)))
        }
        FillPattern::Text { text, mode, each, sequence, size } => {
            let items = text_items(text);
            if items.is_empty() {
                return Err(FillError::NoItems);
            }
            Ok(FillValues::Texts(lay_out(&items, *mode, *each, *sequence, *size, seed)?))
        }
    }
}

impl Project {
    /// Fills every row of `column`'s sheet (32 rows when the sheet has none) with row numbers or
    /// random numbers; the column's text cells and formula go. Returns how many rows were written.
    pub fn fill_quick(&mut self, column: ColumnId, kind: QuickFill, seed: u64) -> Result<usize, FillError> {
        let rows = {
            let (_, sheet, _) = self.locate(column).ok_or(ProjectError::NoSuchColumn(column))?;
            match sheet.row_count() {
                0 => QUICK_FILL_EMPTY_ROWS,
                n => n,
            }
        };
        let col = self.column_mut(column).ok_or(ProjectError::NoSuchColumn(column))?;
        col.data = ColumnData::numeric(kind.values(rows, seed));
        col.formula = None;
        Ok(rows)
    }

    /// Fills `column` with `pattern` from its first row; rows below what the fill writes keep their
    /// values. A date fill makes the column a Date (or Time) column; a text fill keeps the numbers
    /// below it in a Text & Numeric column. Returns how many rows were written.
    pub fn fill_pattern(&mut self, column: ColumnId, pattern: &FillPattern, seed: u64) -> Result<usize, FillError> {
        let rows = {
            let (_, sheet, _) = self.locate(column).ok_or(ProjectError::NoSuchColumn(column))?;
            sheet.row_count()
        };
        let values = generate(pattern, rows, seed)?;
        let col = self.column_mut(column).ok_or(ProjectError::NoSuchColumn(column))?;
        let old_numbers: Vec<f64> = col.numeric().map(<[f64]>::to_vec).unwrap_or_default();
        let old = &col.data;
        let len = old.len();
        let old_number = |r: usize| old_numbers.get(r).copied().unwrap_or(f64::NAN);
        let old_text = |r: usize| old.text_cell(r).map(str::to_string);
        let (data, written) = match values {
            FillValues::Numbers(v, format) => {
                let n = v.len();
                let mut numbers = v;
                let mut texts = BTreeMap::new();
                for r in n..len {
                    numbers.push(old_number(r));
                    if let Some(t) = old_text(r) {
                        texts.insert(r, t);
                    }
                }
                if let Some(f) = format {
                    col.format = f;
                }
                let data =
                    if texts.is_empty() { ColumnData::numeric(numbers) } else { ColumnData::mixed(numbers, texts) };
                (data, n)
            }
            FillValues::Texts(v) => {
                let n = v.len();
                let mut numbers = vec![f64::NAN; n.max(len)];
                let mut texts: BTreeMap<usize, String> = BTreeMap::new();
                for (r, t) in v.into_iter().enumerate() {
                    match t.trim().parse::<f64>() {
                        Ok(x) => numbers[r] = x,
                        Err(_) => {
                            texts.insert(r, t);
                        }
                    }
                }
                for (r, slot) in numbers.iter_mut().enumerate().skip(n) {
                    *slot = old_number(r);
                    if let Some(t) = old_text(r) {
                        texts.insert(r, t);
                    }
                }
                (ColumnData::mixed(numbers, texts), n)
            }
        };
        col.data = data;
        col.formula = None;
        Ok(written)
    }
}

#[cfg(test)]
mod tests;
