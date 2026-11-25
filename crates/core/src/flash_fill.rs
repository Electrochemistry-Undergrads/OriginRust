//! Column > Flash Fill (T430; Origin 2026's Flash Fill and Smart Formula, Sheaf's own rules): the cells typed in a
//! column are examples of what it should hold; Flash Fill finds one rule that builds every example from the other
//! cells of its row, and fills the column's other rows by it, either as values or as a column formula that follows
//! its sources.
//!
//! A rule joins pieces, each one of: a fixed text; a whole cell of a source column; a word of one (a run of
//! letters and digits, counted from the start or from the end) or the word's first letter; each read as it is, in
//! upper case or in lower case. The search tries the longest pieces first, prefers a cell to a word, a word to a
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

/// One piece of a rule; `source` indexes the rule's source columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Cell {
        source: usize,
        case: Case,
    },
    /// Word `index` (1 the first, -1 the last); its first letter only when `initial`.
    Word {
        source: usize,
        index: i32,
        case: Case,
        initial: bool,
    },
}

impl Piece {
    fn eval(&self, row: &[String]) -> String {
        match self {
            Piece::Text(t) => t.clone(),
            Piece::Cell { source, case } => case.apply(&row[*source]),
            Piece::Word { source, index, case, initial } => {
                let w = token(&row[*source], f64::from(*index));
                let w = if *initial { w.chars().take(1).collect() } else { w };
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
            Piece::Word { source, index, case, initial } => {
                let w = format!("token(col({})$, {index})", names[*source]);
                cased(if *initial { format!("left({w}, 1)") } else { w }, *case)
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
const BUDGET: usize = 5_000;

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
                for initial in [false, true] {
                    for case in [Case::AsIs, Case::Upper, Case::Lower] {
                        words.push(Piece::Word { source, index, case, initial });
                    }
                }
            }
        }
    }
    // Whole words before initials, then the nearer end, then from the start, then as-is before cased.
    words.sort_by_key(|p| match p {
        Piece::Word { index, initial, case, .. } => (*initial, index.unsigned_abs(), *index < 0, *case as u8),
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
        let text = |c: &crate::column::Column, r: usize| crate::indexed::level_text(c, r).unwrap_or_default();
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

    /// Writes `fill`'s texts into column `target` as values (the examples are kept as typed).
    pub fn apply_flash_fill(&mut self, target: ColumnId, fill: &FlashFill) -> Result<(), ProjectError> {
        let col = self.column_mut(target).ok_or(ProjectError::NoSuchColumn(target))?;
        let mut values = fill.values.clone();
        for &r in &fill.examples {
            if let Some(v) = values.get_mut(r) {
                *v = crate::indexed::level_text(col, r).unwrap_or_default();
            }
        }
        col.data = crate::column::ColumnData::text(values);
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
}
