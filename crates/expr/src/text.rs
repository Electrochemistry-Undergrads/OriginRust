//! Text values and the string functions of column formulas (`left`, `mid`, `text`, `value`, ...).
//!
//! The behaviour follows what Origin gave for the same formulas; the measurements are in
//! `docs/spec/expression-language.md`, section 3.

use crate::funcs::NA;

/// The result of a formula in one row: a number (NaN is a missing value) or a text.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Text(String),
}

impl Value {
    /// The number, or a missing value for a text (text counts as missing in calculations).
    pub fn number(&self) -> f64 {
        match self {
            Value::Num(x) => *x,
            Value::Text(_) => NA,
        }
    }

    /// The text of the value: a text as it is, a number as `text(x)` writes it (`--` when missing).
    pub fn into_text(self) -> String {
        match self {
            Value::Text(s) => s,
            Value::Num(x) => format_general(x),
        }
    }
}

/// The string functions. Arguments are texts or numbers; a number is written as text first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrFn {
    Left,
    Right,
    Mid,
    Len,
    Upper,
    Lower,
    Trim,
    Find,
    Search,
    Replace,
    Substitute,
    Char,
    Code,
    Compare,
    Exact,
    Text,
    Value,
}

impl StrFn {
    /// `(fewest, most)` arguments.
    pub(crate) fn arity(self) -> (usize, usize) {
        match self {
            StrFn::Left | StrFn::Right | StrFn::Exact => (2, 2),
            StrFn::Mid | StrFn::Find | StrFn::Search | StrFn::Compare => (2, 3),
            StrFn::Len | StrFn::Upper | StrFn::Lower | StrFn::Trim | StrFn::Value | StrFn::Char | StrFn::Code => (1, 1),
            StrFn::Replace => (4, 4),
            StrFn::Substitute => (3, 4),
            StrFn::Text => (1, 2),
        }
    }
}

impl StrFn {
    /// Whether the result is a text (else a number).
    pub(crate) fn returns_text(self) -> bool {
        !matches!(
            self,
            StrFn::Len | StrFn::Find | StrFn::Search | StrFn::Compare | StrFn::Exact | StrFn::Value | StrFn::Code
        )
    }

    /// Whether argument `i` is read as text (a bare column is its text) rather than as a number.
    /// `text(col(A), fmt)` formats the number of the cell, as Origin does.
    pub(crate) fn text_arg(self, i: usize) -> bool {
        match self {
            StrFn::Text => i > 0,
            StrFn::Char => false,
            StrFn::Left | StrFn::Right | StrFn::Mid => i == 0,
            StrFn::Replace => i == 0 || i == 3,
            StrFn::Substitute => i < 3,
            StrFn::Find | StrFn::Search | StrFn::Compare => i <= 1,
            _ => true,
        }
    }
}

fn count(v: &Value) -> f64 {
    v.number()
}

/// Whole number of characters from a count argument; negative or missing counts are zero.
fn chars_of(n: f64) -> usize {
    if n.is_nan() { 0 } else { n.round().clamp(0.0, 1.0e9) as usize }
}

/// A 1-based start position; anything below 1 counts as 1 (Origin: `mid(s,0,3)` is `mid(s,1,3)`).
fn start_of(n: f64) -> usize {
    if n.is_nan() { 1 } else { n.round().clamp(1.0, 1.0e9) as usize }
}

/// Applies a string function to already evaluated arguments.
pub(crate) fn apply(f: StrFn, args: Vec<Value>) -> Value {
    let flag = |v: Option<&Value>, default: bool| v.map_or(default, |v| v.number() != 0.0 && !v.number().is_nan());
    match f {
        StrFn::Char => {
            let n = args[0].number();
            // The documented range is 1..65535. Invalid numbers and lone surrogate code
            // points give empty text; Rust strings always remain valid Unicode.
            let ch = if n.is_finite() && n.fract() == 0.0 && (1.0..=65535.0).contains(&n) {
                char::from_u32(n as u32)
            } else {
                None
            };
            Value::Text(ch.map_or_else(String::new, |c| c.to_string()))
        }
        StrFn::Text => {
            let fmt = args.get(1).map(|v| v.clone().into_text());
            match &args[0] {
                Value::Text(s) => Value::Text(s.clone()),
                Value::Num(x) => Value::Text(format_number(*x, fmt.as_deref().unwrap_or("*"))),
            }
        }
        StrFn::Value => match &args[0] {
            Value::Num(x) => Value::Num(*x),
            Value::Text(s) => Value::Num(s.trim().parse::<f64>().ok().filter(|v| v.is_finite()).unwrap_or(NA)),
        },
        _ => {
            let mut it = args.into_iter();
            let s_val = it.next().unwrap_or(Value::Num(NA));
            let rest: Vec<Value> = it.collect();
            let s = s_val.into_text();
            let text_arg = |i: usize| rest.get(i).cloned().map(Value::into_text).unwrap_or_default();
            match f {
                StrFn::Left => Value::Text(s.chars().take(chars_of(count(&rest[0]))).collect()),
                StrFn::Right => {
                    let n = chars_of(count(&rest[0]));
                    let len = s.chars().count();
                    Value::Text(s.chars().skip(len.saturating_sub(n)).collect())
                }
                StrFn::Mid => {
                    let start = start_of(count(&rest[0]));
                    let it = s.chars().skip(start - 1);
                    Value::Text(match rest.get(1) {
                        Some(n) => it.take(chars_of(count(n))).collect(),
                        None => it.collect(),
                    })
                }
                StrFn::Len => Value::Num(s.chars().count() as f64),
                StrFn::Code => Value::Num(s.chars().next().map_or(0, u32::from) as f64),
                StrFn::Upper => Value::Text(s.to_uppercase()),
                StrFn::Lower => Value::Text(s.to_lowercase()),
                StrFn::Trim => Value::Text(s.trim().to_string()),
                StrFn::Find | StrFn::Search => {
                    let (hay, needle) = if f == StrFn::Search {
                        (s.to_lowercase(), text_arg(0).to_lowercase())
                    } else {
                        (s, text_arg(0))
                    };
                    let start = rest.get(1).map_or(1, |v| start_of(count(v)));
                    let from: usize = hay.chars().take(start - 1).map(char::len_utf8).sum();
                    Value::Num(match hay.get(from..).and_then(|t| t.find(&needle)) {
                        Some(byte) => (hay[..from + byte].chars().count() + 1) as f64,
                        None => -1.0,
                    })
                }
                StrFn::Replace => {
                    let (pos, len) = (start_of(count(&rest[0])), chars_of(count(&rest[1])));
                    let new = text_arg(2);
                    if pos > s.chars().count() {
                        return Value::Text(s);
                    }
                    let head: String = s.chars().take(pos - 1).collect();
                    let tail: String = s.chars().skip(pos - 1 + len).collect();
                    Value::Text(format!("{head}{new}{tail}"))
                }
                StrFn::Substitute => {
                    // Origin's order is (within, replacement, search, occurrence), not
                    // (within, search, replacement). Zero or no occurrence replaces all.
                    let new = text_arg(0);
                    let find = text_arg(1);
                    let n = rest.get(2).map_or(0.0, Value::number);
                    // These cases are not specified by Origin's docs. Leave the text intact
                    // rather than inserting between characters or coercing an invalid index.
                    if find.is_empty() || !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
                        return Value::Text(s);
                    }
                    if n == 0.0 {
                        return Value::Text(s.replace(&find, &new));
                    }
                    let at = s.match_indices(&find).nth(n as usize - 1).map(|(at, _)| at);
                    let mut s = s;
                    if let Some(at) = at {
                        s.replace_range(at..at + find.len(), &new);
                    }
                    Value::Text(s)
                }
                // Origin: 1 when the texts are equal, else 0; the third argument says whether the
                // case matters (default: it does).
                StrFn::Compare => {
                    let case = flag(rest.get(1), true);
                    let other = text_arg(0);
                    Value::Num(f64::from(if case { s == other } else { s.to_lowercase() == other.to_lowercase() }))
                }
                StrFn::Exact => Value::Num(f64::from(s == text_arg(0))),
                StrFn::Text | StrFn::Value | StrFn::Char => unreachable!("handled above"),
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Numbers as text
// ---------------------------------------------------------------------------------------------

/// Rounds half away from zero to `decimals` places (Origin writes 2.5 with no decimals as 3).
fn round_to(x: f64, decimals: usize) -> f64 {
    let p = 10f64.powi(decimals.min(300) as i32);
    let y = (x * p).round() / p;
    if y.is_finite() { y } else { x }
}

/// Decimal exponent of a positive number and its digits after rounding to `n` significant digits.
fn sci_parts(a: f64, n: usize) -> (i32, String) {
    let s = format!("{:.*e}", n.saturating_sub(1), a);
    let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
    (exp.parse().unwrap_or(0), mant.replace('.', ""))
}

fn sci_text(neg: bool, a: f64, decimals: usize, plain: bool) -> String {
    let (e, digits) = sci_parts(a, decimals + 1);
    let digits = if plain { digits.trim_end_matches('0').to_string() } else { digits };
    let digits = if digits.is_empty() { "0".to_string() } else { digits };
    let mant = if digits.len() > 1 { format!("{}.{}", &digits[..1], &digits[1..]) } else { digits };
    let sign = if neg { "-" } else { "" };
    if plain {
        format!("{sign}{mant}E{e}")
    } else {
        format!("{sign}{mant}E{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
    }
}

/// Fixed or scientific: fixed for exponents -3 to 5, and 6 when nothing is lost at `n` digits.
fn is_fixed(a: f64, e: i32, n: usize) -> bool {
    (-3..=5).contains(&e) || (e == 6 && round_sig(a, n) == a)
}

fn round_sig(a: f64, n: usize) -> f64 {
    format!("{:.*e}", n.saturating_sub(1), a).parse().unwrap_or(a)
}

/// `text(x)` and `text(x, "*")`: up to five decimals (six digits in scientific form), `--` for a
/// missing value.
pub(crate) fn format_general(x: f64) -> String {
    if x.is_nan() {
        return "--".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    let (neg, a) = (x < 0.0, x.abs());
    let (e, _) = sci_parts(a, 15);
    if is_fixed(a, e, 6) {
        // Up to five decimals, trailing zeros dropped.
        let text = format!("{:.5}", round_to(a, 5));
        let text = text.trim_end_matches('0').trim_end_matches('.');
        format!("{}{text}", if neg { "-" } else { "" })
    } else {
        sci_text(neg, a, 5, true)
    }
}

/// A number written by one of Origin's format codes: `*` (general), `*n` (n significant digits),
/// `.n` (n decimals), `E*n` (engineering notation with a unit prefix). Other codes fall back to
/// the general form.
pub(crate) fn format_number(x: f64, fmt: &str) -> String {
    if x.is_nan() {
        return "--".into();
    }
    if x.is_infinite() {
        return format_general(x);
    }
    let fmt = fmt.trim();
    let (neg, a) = (x < 0.0, x.abs());
    let sign = if neg { "-" } else { "" };
    let number = |s: &str| s.parse::<usize>().ok().filter(|n| (1..=15).contains(n));
    if let Some(n) = fmt.strip_prefix("E*").and_then(number) {
        return engineering(neg, a, n);
    }
    if let Some(n) = fmt.strip_prefix('*').and_then(number) {
        if a == 0.0 {
            return format!("0.{}", "0".repeat(n - 1)).trim_end_matches('.').to_string();
        }
        let (e, _) = sci_parts(a, 15);
        let fixed = (-3..n as i32).contains(&e) || (e == n as i32 && round_sig(a, n) == a);
        return if fixed {
            let decimals = (n as i32 - 1 - e).max(0) as usize;
            format!("{sign}{:.*}", decimals, round_to(a, decimals))
        } else {
            sci_text(neg, a, n - 1, false)
        };
    }
    if let Some(n) = fmt.strip_prefix('.').and_then(|s| s.parse::<usize>().ok()).filter(|n| *n <= 15) {
        if a == 0.0 {
            return format!("{:.*}", n, 0.0);
        }
        let (e, _) = sci_parts(a, 15);
        return if is_fixed(a, e, 6) { format!("{sign}{:.*}", n, round_to(a, n)) } else { sci_text(neg, a, n, false) };
    }
    format_general(x)
}

fn engineering(neg: bool, a: f64, n: usize) -> String {
    let sign = if neg { "-" } else { "" };
    if a == 0.0 {
        return format!("{:.*}", n - 1, 0.0);
    }
    let (e, _) = sci_parts(a, 15);
    let scale = e.div_euclid(3) * 3;
    let prefix = match scale {
        -12 => "p",
        -9 => "n",
        -6 => "\u{b5}",
        -3 => "m",
        0 => "",
        3 => "k",
        6 => "M",
        9 => "G",
        12 => "T",
        15 => "P",
        _ => return sci_text(neg, a, n - 1, false),
    };
    let decimals = (n as i32 - 1 - (e - scale)).max(0) as usize;
    format!("{sign}{:.*}{prefix}", decimals, round_to(a / 10f64.powi(scale), decimals))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_form_matches_origin() {
        let cases = [
            (3.14259265, "3.14259"),
            (1234.5678, "1234.5678"),
            (12.3456789, "12.34568"),
            (0.00123456789, "0.00123"),
            (123456.789, "123456.789"),
            (0.001234, "0.00123"),
            (0.01234, "0.01234"),
            (0.000123456, "1.23456E-4"),
            (12345.0, "12345"),
            (123456.0, "123456"),
            (1234567.0, "1.23457E6"),
            (100000.5, "100000.5"),
            (1e6, "1000000"),
            (1e9, "1E9"),
            (1e15, "1E15"),
            (123456789012.0, "1.23457E11"),
            (1e-5, "1E-5"),
            (-0.5, "-0.5"),
            (100.0, "100"),
            (0.0, "0"),
            (-42.0, "-42"),
            (999.5, "999.5"),
        ];
        for (x, want) in cases {
            assert_eq!(format_general(x), want, "{x}");
        }
        assert_eq!(format_general(f64::NAN), "--");
    }
}
