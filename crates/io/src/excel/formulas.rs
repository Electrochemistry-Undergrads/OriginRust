//! Excel formulas kept as column formulas (T417, Origin 2026's formula-preserving Excel import; Sheaf's own rules,
//! not measured; `docs/spec/import-export.md`, "Excel formulas").
//!
//! A column keeps its cells' formula when every data row that holds a value holds a formula and all of them are
//! one formula written for their own row: `B2 = A2*2`, `B3 = A3*2`, ... becomes `col(A)*2` (the short names of the
//! imported columns). The values stay as Excel stored them; the formula recalculates automatically from then on.
//!
//! What can be read: numbers, `+ - * / ^` and parentheses, references to cells of the same row in imported columns
//! (`A2`, `$A2`), and the functions SQRT, EXP, LN, LOG10, ABS, SIN, COS, TAN, ASIN, ACOS, ATAN, SINH, COSH, TANH,
//! ROUND and PI(). Anything else (other rows, `$` rows, ranges, other sheets, text, comparisons, other functions)
//! keeps only the stored values, with a warning. A unary minus is refused next to `^` (Excel's `-2^2` is 4).

/// Excel functions with the same meaning in Sheaf's formulas, and their Sheaf names.
const FUNCTIONS: [(&str, &str); 14] = [
    ("SQRT", "sqrt"),
    ("EXP", "exp"),
    ("LN", "ln"),
    ("LOG10", "log10"),
    ("ABS", "abs"),
    ("SIN", "sin"),
    ("COS", "cos"),
    ("TAN", "tan"),
    ("ASIN", "asin"),
    ("ACOS", "acos"),
    ("ATAN", "atan"),
    ("SINH", "sinh"),
    ("COSH", "cosh"),
    ("TANH", "tanh"),
];

/// The 0-based column of Excel column letters (`A` 0, `Z` 25, `AA` 26).
fn column_index(letters: &str) -> Option<usize> {
    if letters.is_empty() || letters.len() > 3 {
        return None;
    }
    letters
        .chars()
        .try_fold(0usize, |acc, c| c.is_ascii_uppercase().then(|| acc * 26 + (c as usize - 'A' as usize + 1)))
        .map(|n| n - 1)
}

/// The Sheaf formula for the Excel formula `text` of 0-based sheet row `row`, with `column` naming the imported
/// column of a 0-based sheet column; `None` when it cannot be read (see the module documentation).
pub(crate) fn translate(text: &str, row: usize, column: &dyn Fn(usize) -> Option<String>) -> Option<String> {
    let text = text.trim().trim_start_matches('=');
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    // Whether the previous token ends an operand (so that a following + or - is binary).
    let mut operand = false;
    let (mut unary, mut power) = (false, false);
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            out.push(' ');
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                let mut k = i + 1;
                if k < chars.len() && matches!(chars[k], '+' | '-') {
                    k += 1;
                }
                if k < chars.len() && chars[k].is_ascii_digit() {
                    i = k;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let number: String = chars[start..i].iter().collect();
            number.parse::<f64>().ok()?;
            out.push_str(&number);
            operand = true;
        } else if c.is_ascii_alphabetic() || c == '$' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '$' | '_' | '.')) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let next = chars[i..].iter().find(|c| !c.is_whitespace()).copied();
            if next == Some('(') {
                let upper = word.to_ascii_uppercase();
                if upper == "PI" {
                    // PI() is the constant.
                    let rest: String = chars[i..].iter().filter(|c| !c.is_whitespace()).take(2).collect();
                    if rest != "()" {
                        return None;
                    }
                    while chars[i] != ')' {
                        i += 1;
                    }
                    i += 1;
                    out.push_str("pi");
                    operand = true;
                    continue;
                }
                let name = if upper == "ROUND" { "round" } else { FUNCTIONS.iter().find(|(e, _)| *e == upper)?.1 };
                out.push_str(name);
                operand = false;
                continue;
            }
            if matches!(next, Some(':' | '!')) {
                return None;
            }
            // A reference: optional $, letters, digits without $ (the same row).
            let w = word.strip_prefix('$').unwrap_or(&word);
            let letters: String = w.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            let digits = &w[letters.len()..];
            if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let excel_row: usize = digits.parse().ok()?;
            if excel_row != row + 1 {
                return None;
            }
            let name = column(column_index(&letters.to_ascii_uppercase())?)?;
            out.push_str(&format!("col({name})"));
            operand = true;
        } else {
            match c {
                '+' | '-' => {
                    if !operand {
                        unary = true;
                    }
                    operand = false;
                }
                '*' | '/' | '(' | ',' => operand = false,
                '^' => {
                    power = true;
                    operand = false;
                }
                ')' => operand = true,
                _ => return None,
            }
            out.push(c);
            i += 1;
        }
    }
    if out.trim().is_empty() || (unary && power) {
        return None;
    }
    sheaf_expr::Expr::parse(&out).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(c: usize) -> Option<String> {
        (c < 4).then(|| ["A", "B", "C", "D"][c].to_string())
    }

    fn t(text: &str, row: usize) -> Option<String> {
        translate(text, row, &names)
    }

    #[test]
    fn same_row_arithmetic_and_functions_are_read() {
        assert_eq!(t("=A2*2", 1).as_deref(), Some("col(A)*2"));
        assert_eq!(t("A7^2 + SQRT($B7)", 6).as_deref(), Some("col(A)^2 + sqrt(col(B))"));
        assert_eq!(t("=ROUND(C3/1e-3, 2)*PI()", 2).as_deref(), Some("round(col(C)/1e-3, 2)*pi"));
        assert_eq!(t("=-A2+D2", 1).as_deref(), Some("-col(A)+col(D)"));
        assert_eq!(t("=log10(a2)", 1).as_deref(), Some("log10(col(A))"), "letter case of names is ignored");
    }

    #[test]
    fn other_rows_ranges_and_unknown_things_are_refused() {
        for (text, row) in [
            ("=A3*2", 1),
            ("=A$2*2", 1),
            ("=SUM($A$2:A2)", 1),
            ("=Sheet2!A2", 1),
            ("=IF(A2>1,1,0)", 1),
            ("=A2&\"x\"", 1),
            ("=E2*2", 1),
            ("=-A2^2", 1),
            ("=A2%", 1),
            ("=", 1),
            ("=TRUE", 1),
        ] {
            assert_eq!(t(text, row), None, "{text}");
        }
    }
}
