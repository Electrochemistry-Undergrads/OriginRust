//! Worksheet > Conditional Formatting > Conditional Highlight: the cells that meet a condition, or
//! two conditions joined by And or Or. The conditions are those of Origin's X-Function `wcolor`
//! (`trrule.cond1` codes 0 to 8: equal to, not equal to, greater than, greater than or equal to,
//! less than, less than or equal to, between, text that contains, custom). How each one treats text
//! and missing cells is Sheaf's own choice and unmeasured (docs/spec/gui-look.md).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One condition. Numeric conditions match numbers only; `Contains` matches the cell's text as
/// the worksheet shows it; `Custom` is an expression in `x` (the cell's number) that is true when
/// it is not zero.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Cond {
    Equal(f64),
    NotEqual(f64),
    Greater(f64),
    GreaterOrEqual(f64),
    Less(f64),
    LessOrEqual(f64),
    /// Both bounds included, in either order.
    Between(f64, f64),
    Contains(String),
    Custom(String),
}

impl Default for Cond {
    /// Origin's default: equal to 0.
    fn default() -> Self {
        Cond::Equal(0.0)
    }
}

/// Origin's `join`: how a second condition combines with the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Join {
    And,
    Or,
}

/// What a cell holds, for the conditions.
#[derive(Clone, Copy, Debug)]
pub struct Cell<'a> {
    /// The number, `None` for a text, empty or missing cell.
    pub number: Option<f64>,
    /// The text as the worksheet shows it (empty for an empty cell).
    pub text: &'a str,
}

/// Conditions ready to test cells: custom expressions are parsed once and their results kept per
/// number, since a column often repeats its values.
pub struct Tester<'r> {
    first: &'r Cond,
    second: Option<(Join, &'r Cond)>,
    exprs: [Option<sheaf_expr::Expr>; 2],
    cache: [HashMap<u64, bool>; 2],
}

/// Checks a custom condition: it must parse and use no variable other than `x`. The error says why not.
pub fn check_custom(src: &str) -> Result<sheaf_expr::Expr, String> {
    let expr = sheaf_expr::Expr::parse(src).map_err(|e| e.to_string())?;
    if let Some(v) = expr.variables().into_iter().find(|v| v != "x") {
        return Err(format!("unknown name '{v}': the condition can use x, the cell's value"));
    }
    if !expr.columns().is_empty() {
        return Err("the condition cannot read columns; use x, the cell's value".into());
    }
    Ok(expr)
}

impl<'r> Tester<'r> {
    pub fn new(first: &'r Cond, second: Option<(Join, &'r Cond)>) -> Self {
        let parse = |c: &Cond| match c {
            Cond::Custom(src) => check_custom(src).ok(),
            _ => None,
        };
        let exprs = [parse(first), second.and_then(|(_, c)| parse(c))];
        Tester { first, second, exprs, cache: [HashMap::new(), HashMap::new()] }
    }

    /// True when a condition reads the cell's text (so the caller must supply it).
    pub fn needs_text(&self) -> bool {
        matches!(self.first, Cond::Contains(_)) || self.second.is_some_and(|(_, c)| matches!(c, Cond::Contains(_)))
    }

    /// True when the cell meets the condition(s).
    pub fn matches(&mut self, cell: Cell<'_>) -> bool {
        let a = self.one(0, cell);
        match self.second {
            None => a,
            Some((Join::And, _)) => a && self.one(1, cell),
            Some((Join::Or, _)) => a || self.one(1, cell),
        }
    }

    fn one(&mut self, which: usize, cell: Cell<'_>) -> bool {
        let cond = if which == 0 { self.first } else { self.second.expect("second condition").1 };
        let n = cell.number;
        match cond {
            Cond::Equal(t) => n.is_some_and(|v| v == *t),
            Cond::NotEqual(t) => n.is_some_and(|v| v != *t),
            Cond::Greater(t) => n.is_some_and(|v| v > *t),
            Cond::GreaterOrEqual(t) => n.is_some_and(|v| v >= *t),
            Cond::Less(t) => n.is_some_and(|v| v < *t),
            Cond::LessOrEqual(t) => n.is_some_and(|v| v <= *t),
            Cond::Between(a, b) => n.is_some_and(|v| v >= a.min(*b) && v <= a.max(*b)),
            Cond::Contains(s) => !s.is_empty() && cell.text.contains(s.as_str()),
            Cond::Custom(_) => {
                let (Some(v), Some(expr)) = (n, &self.exprs[which]) else { return false };
                *self.cache[which].entry(v.to_bits()).or_insert_with(|| {
                    let host = sheaf_expr::SimpleHost::new().var("x", v);
                    expr.eval(&host).is_ok_and(|r| r != 0.0 && !r.is_nan())
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(v: f64) -> Cell<'static> {
        Cell { number: Some(v), text: "" }
    }

    fn one(c: &Cond, cell: Cell<'_>) -> bool {
        Tester::new(c, None).matches(cell)
    }

    #[test]
    fn numeric_conditions() {
        assert!(one(&Cond::Equal(2.0), num(2.0)) && !one(&Cond::Equal(2.0), num(2.5)));
        assert!(one(&Cond::NotEqual(2.0), num(2.5)) && !one(&Cond::NotEqual(2.0), num(2.0)));
        assert!(one(&Cond::Greater(2.0), num(3.0)) && !one(&Cond::Greater(2.0), num(2.0)));
        assert!(one(&Cond::GreaterOrEqual(2.0), num(2.0)));
        assert!(one(&Cond::Less(2.0), num(1.0)) && !one(&Cond::Less(2.0), num(2.0)));
        assert!(one(&Cond::LessOrEqual(2.0), num(2.0)));
        // Between includes both bounds, in either order.
        for b in [Cond::Between(1.0, 3.0), Cond::Between(3.0, 1.0)] {
            assert!(one(&b, num(1.0)) && one(&b, num(3.0)) && one(&b, num(2.0)));
            assert!(!one(&b, num(3.5)));
        }
        // Text and empty cells never meet a numeric condition, not even "not equal to".
        let text = Cell { number: None, text: "abc" };
        assert!(!one(&Cond::NotEqual(0.0), text));
        assert!(!one(&Cond::Less(10.0), Cell { number: None, text: "" }));
    }

    #[test]
    fn text_that_contains_reads_the_shown_text() {
        let c = Cond::Contains("pan".into());
        assert!(one(&c, Cell { number: None, text: "Japan" }));
        assert!(!one(&c, Cell { number: None, text: "JAPAN" }), "case matters");
        assert!(one(&Cond::Contains("5".into()), Cell { number: Some(2.5), text: "2.5" }));
        assert!(!one(&Cond::Contains(String::new()), Cell { number: None, text: "x" }));
    }

    #[test]
    fn custom_conditions_are_expressions_in_x() {
        let c = Cond::Custom("x > 2 && mod(x, 2) == 0".into());
        let mut t = Tester::new(&c, None);
        assert!(t.matches(num(4.0)));
        assert!(!t.matches(num(3.0)));
        assert!(!t.matches(num(2.0)));
        assert!(t.matches(num(4.0)), "cached");
        assert!(!t.matches(Cell { number: None, text: "4" }));
        // A condition that does not parse, or uses other names, matches nothing.
        assert!(!one(&Cond::Custom("x >".into()), num(4.0)));
        assert!(!one(&Cond::Custom("y > 1".into()), num(4.0)));
        assert!(check_custom("y > 1").unwrap_err().contains("'y'"));
        assert!(check_custom("col(A) > 1").is_err());
        assert!(check_custom("abs(x) < 1").is_ok());
    }

    #[test]
    fn and_or_join_two_conditions() {
        let (a, b) = (Cond::Greater(1.0), Cond::Less(3.0));
        let mut and = Tester::new(&a, Some((Join::And, &b)));
        assert!(and.matches(num(2.0)) && !and.matches(num(4.0)) && !and.matches(num(0.0)));
        let (a, b) = (Cond::Less(1.0), Cond::Greater(3.0));
        let mut or = Tester::new(&a, Some((Join::Or, &b)));
        assert!(or.matches(num(0.0)) && or.matches(num(4.0)) && !or.matches(num(2.0)));
    }
}
