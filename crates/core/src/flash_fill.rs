//! Column > Flash Fill (T430; Origin 2026's Flash Fill and Smart Formula, Sheaf's own rules): the cells typed in a
//! column are examples of what it should hold; Flash Fill finds one rule that builds every example from the other
//! cells of its row, and fills the column's other rows by it, either as values or as a column formula that follows
//! its sources.
//!
//! A rule joins pieces, each one of: a fixed text; a whole cell of a source column; a word of one (a run of
//! letters and digits, counted from the start or from the end), or its first or last one to four characters (T468);
//! each read as it is, in upper case or in lower case. The search tries the longest pieces first, prefers a cell to a word, a word to a
//! letter and a letter to fixed text, and a word counted from the nearer end (from the start when equal), so that
//! "John Smith" -> "Smith, J." gives `token(col(A)$, -1) + ", " + left(token(col(A)$, 1), 1) + "."`.

use crate::ids::ColumnId;
use crate::project::{Project, ProjectError};
use sheaf_expr::{token, tokens};

/// How a piece's text is cased.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    AsIs,
    Upper,
    Lower,
}

impl Case {
    fn apply(self, s: &str) -> String {
        match self {
            Case::AsIs => s.to_string(),
            Case::Upper => s.to_uppercase(),
            Case::Lower => s.to_lowercase(),
        }
    }
}

/// The part of a word a piece takes (T468: the first or last few characters as well as the whole word).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Whole,
    /// The first n characters (1 is the initial).
    First(u8),
    /// The last n characters.
    Last(u8),
}

/// One piece of a rule; `source` indexes the rule's source columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Cell {
        source: usize,
        case: Case,
    },
    /// Word `index` (1 the first, -1 the last), or the part of it `part` takes.
    Word {
        source: usize,
        index: i32,
        case: Case,
        part: Part,
    },
}

impl Piece {
    fn eval(&self, row: &[String]) -> String {
        match self {
            Piece::Text(t) => t.clone(),
            Piece::Cell { source, case } => case.apply(&row[*source]),
            Piece::Word { source, index, case, part } => {
                let w = token(&row[*source], f64::from(*index));
                let n = w.chars().count();
                let w: String = match part {
                    Part::Whole => w,
                    Part::First(k) => w.chars().take(*k as usize).collect(),
                    Part::Last(k) => w.chars().skip(n.saturating_sub(*k as usize)).collect(),
                };
                case.apply(&w)
            }
        }
    }

    /// The formula of the piece, `names` being the source columns' short names.
    fn formula(&self, names: &[String]) -> String {
        let cased = |e: String, case: Case| match case {
            Case::AsIs => e,
            Case::Upper => format!("upper({e})"),
            Case::Lower => format!("lower({e})"),
        };
        match self {
            Piece::Text(t) => format!("\"{t}\""),
            Piece::Cell { source, case } => cased(format!("col({})$", names[*source]), *case),
            Piece::Word { source, index, case, part } => {
                let w = format!("token(col({})$, {index})", names[*source]);
                let w = match part {
                    Part::Whole => w,
                    Part::First(k) => format!("left({w}, {k})"),
                    Part::Last(k) => format!("right({w}, {k})"),
                };
                cased(w, *case)
            }
        }
    }
}

/// A Flash Fill rule: the pieces joined in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlashRule {
    pub pieces: Vec<Piece>,
}

impl FlashRule {
    /// The rule's text for one row (`row[k]` the text of source k).
    pub fn apply(&self, row: &[String]) -> String {
        self.pieces.iter().map(|p| p.eval(row)).collect()
    }

    /// The rule as a column formula, or `None` when a fixed text holds a `"` (formulas cannot write one).
    pub fn formula(&self, names: &[String]) -> Option<String> {
        if self.pieces.iter().any(|p| matches!(p, Piece::Text(t) if t.contains('"'))) {
            return None;
        }
        Some(self.pieces.iter().map(|p| p.formula(names)).collect::<Vec<_>>().join(" + "))
    }
}

/// How many partial rules the search may look at before it gives up.
const BUDGET: usize = 2_000;

/// The pieces worth trying in a row: every cell, word and initial of its sources in every case, the more
/// general first (cells, then words from the nearer end, then initials).
fn candidates(sources: usize, rows: &[&[String]]) -> Vec<Piece> {
    let mut out = Vec::new();
    for source in 0..sources {
        for case in [Case::AsIs, Case::Upper, Case::Lower] {
            out.push(Piece::Cell { source, case });
        }
    }
    let most = rows.iter().flat_map(|r| r.iter().map(|s| tokens(s).len())).max().unwrap_or(0) as i32;
    let mut words = Vec::new();
    for source in 0..sources {
        for k in 1..=most {
            for index in [k, -k] {
                let parts = [Part::Whole].into_iter().chain((1..=4).map(Part::First)).chain((1..=4).map(Part::Last));
                for part in parts {
                    for case in [Case::AsIs, Case::Upper, Case::Lower] {
                        words.push(Piece::Word { source, index, case, part });
                    }
                }
            }
        }
    }
    // Whole words before parts (an initial first among them), then the nearer end, then from the start, then as-is
    // before cased.
    words.sort_by_key(|p| match p {
        Piece::Word { index, part, case, .. } => (*part, index.unsigned_abs(), *index < 0, *case as u8),
        _ => unreachable!(),
    });
    out.extend(words);
    out
}

/// Finds the rule that builds `examples[i].1` from `examples[i].0` (the row's source texts) for every example.
/// `None` when no rule of the pieces above does within the search budget.
pub fn infer(examples: &[(Vec<String>, String)]) -> Option<FlashRule> {
    let first = examples.first()?;
    let sources = first.0.len();
    let rows: Vec<&[String]> = examples.iter().map(|e| e.0.as_slice()).collect();
    let pieces = candidates(sources, &rows);
    let mut budget = BUDGET;
    let mut found = Vec::new();
    let start = vec![0; examples.len()];
    if search(examples, &pieces, &start, &mut found, &mut budget) {
        Some(FlashRule { pieces: merge(found) })
    } else {
        None
    }
}

/// Depth-first: at positions `at` (one per example) try each piece that every example continues with, longest
/// first; fixed text (one character, the same in every example) last.
fn search(
    examples: &[(Vec<String>, String)],
    pieces: &[Piece],
    at: &[usize],
    found: &mut Vec<Piece>,
    budget: &mut usize,
) -> bool {
    if examples.iter().zip(at).all(|(e, &i)| i == e.1.len()) {
        return true;
    }
    if *budget == 0 || examples.iter().zip(at).any(|(e, &i)| i >= e.1.len()) {
        return false;
    }
    *budget -= 1;
    // The pieces that fit every example here, the longest (in the first example) first.
    let mut fits: Vec<(usize, &Piece, Vec<usize>)> = Vec::new();
    for p in pieces {
        let mut next = Vec::with_capacity(at.len());
        for (e, &i) in examples.iter().zip(at) {
            let t = p.eval(&e.0);
            if t.is_empty() || !e.1[i..].starts_with(&t) {
                break;
            }
            next.push(i + t.len());
        }
        if next.len() == at.len() {
            fits.push((next[0] - at[0], p, next));
        }
    }
    fits.sort_by_key(|f| std::cmp::Reverse(f.0));
    for (_, p, next) in fits {
        found.push(p.clone());
        if search(examples, pieces, &next, found, budget) {
            return true;
        }
        found.pop();
    }
    // One character of fixed text, when it is the same in every example.
    let Some(c) = examples[0].1[at[0]..].chars().next() else { return false };
    if examples.iter().zip(at).all(|(e, &i)| e.1[i..].starts_with(c)) {
        let next: Vec<usize> = at.iter().map(|i| i + c.len_utf8()).collect();
        found.push(Piece::Text(c.to_string()));
        if search(examples, pieces, &next, found, budget) {
            return true;
        }
        found.pop();
    }
    false
}

/// Joins runs of fixed text into one piece.
fn merge(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for p in pieces {
        match (out.last_mut(), p) {
            (Some(Piece::Text(a)), Piece::Text(b)) => a.push_str(&b),
            (_, p) => out.push(p),
        }
    }
    out
}

/// A cell as Flash Fill reads it: as the sheet shows it for a date or time (T493), else its text or number.
fn shown(c: &crate::column::Column, r: usize) -> String {
    if c.format.is_date_time()
        && c.data.text_cell(r).is_none()
        && c.numeric().and_then(|v| v.get(r)).is_some_and(|x| !x.is_nan())
    {
        c.cell_text(r)
    } else {
        crate::indexed::level_text(c, r).unwrap_or_default()
    }
}

/// What Flash Fill found for a column: the rule, its sources, the example rows and the column's new texts.
#[derive(Clone, Debug, PartialEq)]
pub struct FlashFill {
    pub rule: FlashRule,
    pub sources: Vec<ColumnId>,
    /// The formula that keeps the rule (`None` when it cannot be written as one).
    pub formula: Option<String>,
    /// Rows (0-based) whose cells were the examples.
    pub examples: Vec<usize>,
    /// The column's texts by the rule, for every row of the sheet.
    pub values: Vec<String>,
}

impl Project {
    /// Flash Fill for column `target`: its non-empty cells are the examples, the columns left of it in its sheet
    /// (those with any text or number) the sources. An error when there is no example or no rule fits.
    pub fn flash_fill(&self, target: ColumnId) -> Result<FlashFill, ProjectError> {
        let (_, sheet, col) = self.locate(target).ok_or(ProjectError::NoSuchColumn(target))?;
        let rows = sheet.row_count();
        let text = shown;
        let at = sheet.columns.iter().position(|c| c.id == target).unwrap_or(0);
        let sources: Vec<&crate::column::Column> =
            sheet.columns[..at].iter().filter(|c| (0..c.len()).any(|r| !text(c, r).is_empty())).collect();
        let row_of = |r: usize| sources.iter().map(|c| text(c, r)).collect::<Vec<String>>();
        let examples: Vec<usize> = (0..col.len()).filter(|&r| !text(col, r).is_empty()).collect();
        if examples.is_empty() {
            return Err(ProjectError::FlashFillNoExample);
        }
        let pairs: Vec<(Vec<String>, String)> = examples.iter().map(|&r| (row_of(r), text(col, r))).collect();
        let rule = infer(&pairs).ok_or(ProjectError::FlashFillNoRule)?;
        let names: Vec<String> = sources.iter().map(|c| c.short_name.clone()).collect();
        Ok(FlashFill {
            formula: rule.formula(&names),
            // A row with no source text stays empty.
            values: (0..rows)
                .map(|r| {
                    let row = row_of(r);
                    if row.iter().all(String::is_empty) { String::new() } else { rule.apply(&row) }
                })
                .collect(),
            sources: sources.iter().map(|c| c.id).collect(),
            examples,
            rule,
        })
    }

    /// Writes `fill`'s texts into column `target` as values (the examples are kept as typed). A number or date
    /// column stays one when every other value reads as a number (a date or time in a date column) as typed into
    /// it (T493); a Text & Numeric column takes each value as typed; otherwise the column becomes text.
    pub fn apply_flash_fill(&mut self, target: ColumnId, fill: &FlashFill) -> Result<(), ProjectError> {
        use crate::column::ColumnData;
        let col = self.column_mut(target).ok_or(ProjectError::NoSuchColumn(target))?;
        let examples = |r: usize| fill.examples.contains(&r);
        let kept = match &col.data {
            ColumnData::Numeric(_) | ColumnData::Mixed { .. } => {
                let mut typed = col.clone();
                typed.data = if matches!(col.data, ColumnData::Mixed { .. }) {
                    ColumnData::mixed_numbers(Vec::new())
                } else {
                    ColumnData::numeric(Vec::new())
                };
                let old = col.numeric().unwrap_or_default();
                let mut fits = true;
                for (r, v) in fill.values.iter().enumerate() {
                    if examples(r) {
                        // The example as it is: its number, or its text in a Text & Numeric column.
                        let text = shown(col, r);
                        match old.get(r).filter(|x| !x.is_nan()) {
                            Some(x) => typed.set_cell_text(r, &format!("{x:?}")),
                            None => typed.set_cell_text(r, &text),
                        };
                    } else {
                        fits &= typed.set_cell_text(r, v);
                    }
                }
                (fits || matches!(col.data, ColumnData::Mixed { .. })).then_some(typed.data)
            }
            ColumnData::Text(_) => None,
        };
        col.data = match kept {
            Some(data) => data,
            None => {
                let mut values = fill.values.clone();
                for &r in &fill.examples {
                    if let Some(v) = values.get_mut(r) {
                        *v = shown(col, r);
                    }
                }
                ColumnData::text(values)
            }
        };
        col.trim_missing_tail();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    fn ex(rows: &[(&[&str], &str)]) -> Vec<(Vec<String>, String)> {
        rows.iter().map(|(s, o)| (s.iter().map(|x| x.to_string()).collect(), o.to_string())).collect()
    }

    #[test]
    fn parts_of_words_are_read() {
        // T468: the first two letters of a first name and the last two digits of a year.
        let rule = infer(&ex(&[(&["John Smith", "2017"], "Jo-17")])).unwrap();
        let names = ["A".to_string(), "B".to_string()];
        assert_eq!(rule.formula(&names).unwrap(), r#"left(token(col(A)$, 1), 2) + "-" + right(token(col(B)$, 1), 2)"#);
        assert_eq!(rule.apply(&["Grace Hopper".into(), "1952".into()]), "Gr-52");
    }

    #[test]
    fn a_name_turns_round_with_an_initial() {
        let rule = infer(&ex(&[(&["John Smith"], "Smith, J.")])).unwrap();
        let names = ["A".to_string()];
        assert_eq!(rule.formula(&names).unwrap(), r#"token(col(A)$, -1) + ", " + left(token(col(A)$, 1), 1) + ".""#);
        assert_eq!(rule.apply(&["Mary Ann Lee".into()]), "Lee, M.");
    }

    #[test]
    fn two_columns_join_and_case_changes() {
        let rule = infer(&ex(&[(&["ada", "Lovelace"], "LOVELACE ada")])).unwrap();
        assert_eq!(rule.apply(&["alan".into(), "Turing".into()]), "TURING alan");
        let rule = infer(&ex(&[(&["2024-03-17"], "17/03")])).unwrap();
        assert_eq!(rule.apply(&["1999-12-31".into()]), "31/12");
    }

    #[test]
    fn a_second_example_settles_an_ambiguous_one() {
        // "b" is word 2 and the last word of "a b"; "c d e" shows it is word 2.
        let rule = infer(&ex(&[(&["a b"], "b"), (&["c d e"], "d")])).unwrap();
        assert_eq!(rule.apply(&["x y z w".into()]), "y");
        assert!(infer(&ex(&[(&["a"], "b"), (&["c"], "q")])).is_none(), "no rule makes b and q");
    }

    #[test]
    fn the_column_fills_from_its_examples() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let names = ["John Smith", "Grace Hopper", "Alan Turing"].iter().map(|s| s.to_string()).collect();
        p.add_column(b, 0, Designation::Y, ColumnData::text(names)).unwrap();
        let out = p.add_column(b, 0, Designation::Y, ColumnData::text(vec!["Smith, J.".into()])).unwrap();
        let fill = p.flash_fill(out).unwrap();
        assert_eq!(fill.values, ["Smith, J.", "Hopper, G.", "Turing, A."]);
        assert_eq!(fill.examples, [0]);
        assert!(fill.formula.as_deref().unwrap().starts_with("token(col(A)$, -1)"));
        // The formula gives the same texts.
        let check = p.add_column(b, 0, Designation::Y, ColumnData::text(Vec::new())).unwrap();
        p.set_column_values(check, fill.formula.as_deref().unwrap(), None).unwrap();
        let by_formula: Vec<String> =
            (0..3).map(|r| crate::indexed::level_text(p.column(check).unwrap(), r).unwrap_or_default()).collect();
        assert_eq!(by_formula, fill.values);
        p.apply_flash_fill(out, &fill).unwrap();
        assert_eq!(crate::indexed::level_text(p.column(out).unwrap(), 2).as_deref(), Some("Turing, A."));
        let empty = p.add_column(b, 0, Designation::Y, ColumnData::text(Vec::new())).unwrap();
        assert!(p.flash_fill(empty).is_err(), "no example");
    }

    #[test]
    fn numbers_and_dates_stay_numbers_and_dates() {
        use crate::datetime::DatePattern;
        use crate::format::NumberFormat;
        let mut p = Project::new();
        let b = p.add_workbook("");
        let ids = ["run 2017 a", "run 2020 b", "run 1999 c"].iter().map(|s| s.to_string()).collect();
        p.add_column(b, 0, Designation::Y, ColumnData::text(ids)).unwrap();
        // T493: a year typed as a number: the column stays numeric.
        let year = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2017.0])).unwrap();
        let fill = p.flash_fill(year).unwrap();
        assert_eq!(fill.values, ["2017", "2020", "1999"]);
        p.apply_flash_fill(year, &fill).unwrap();
        assert_eq!(p.column(year).unwrap().data, ColumnData::numeric(vec![2017.0, 2020.0, 1999.0]));
        // Not every value a number: the column turns to text, as before.
        let word = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![f64::NAN; 3])).unwrap();
        p.column_mut(word).unwrap().data = ColumnData::text(vec!["2017a".into()]);
        let fill = p.flash_fill(word).unwrap();
        p.apply_flash_fill(word, &fill).unwrap();
        assert!(matches!(p.column(word).unwrap().data, ColumnData::Text(_)));
        // A date column: dates built from text stay dates.
        let days = ["2024-03-17 x", "1999-12-31 y"].iter().map(|s| s.to_string()).collect();
        let b2 = p.add_workbook("");
        p.add_column(b2, 0, Designation::Y, ColumnData::text(days)).unwrap();
        let date = p.add_column(b2, 0, Designation::Y, ColumnData::numeric(Vec::new())).unwrap();
        let c = p.column_mut(date).unwrap();
        c.format = NumberFormat::Date(DatePattern::Iso);
        assert!(c.set_cell_text(0, "2024-03-17"));
        let first = c.numeric().unwrap()[0];
        let fill = p.flash_fill(date).unwrap();
        assert_eq!(fill.values, ["2024-03-17", "1999-12-31"]);
        p.apply_flash_fill(date, &fill).unwrap();
        let c = p.column(date).unwrap();
        let v = c.numeric().expect("still a date column");
        assert_eq!(v[0], first, "the example as typed");
        assert_eq!(c.cell_text(1), "1999-12-31");
    }
}
