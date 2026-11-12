//! Import of Origin `.FDF` fit-function definition files. See `docs/spec/fdf-format.md`.
//!
//! [`parse_fdf`] reads the text of a file into an [`FdfFunction`]; [`FdfFunction::to_fit_function`]
//! turns it into a [`FitFunction`] that [`crate::nlsf::fit`] can use. The formula section of a
//! file is written in a C-like dialect. It is translated to the expression language of
//! `sheaf-expr`: comments are dropped, temporaries (`double t = ...;`) are substituted,
//! `if / else if / else` blocks that assign the dependent variable become conditional
//! expressions, `log` becomes `ln` in a formula (Origin C) but `log10` in a derived parameter (LabTalk), and `PI` and `NANUM` are
//! mapped to `pi` and `na()`. Functions whose body calls a native routine, uses a loop or leaves
//! the result unassigned on some path cannot be translated; they import with `formula: None`.
//!
//! The starting-value code (`Parameters Initialization`) is Origin C and is not executed. Only
//! the numeric `Initial Values` of a file are used.

use sheaf_expr::Expr;

use crate::nlsf::{FitFunction, Model, ParamSpec};
use crate::report::FitError;

/// A bound as written in a file: the number and whether the file switches it on by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FdfBound {
    pub value: f64,
    pub enabled: bool,
}

/// One parameter of an imported function.
#[derive(Debug, Clone, PartialEq)]
pub struct FdfParam {
    pub name: String,
    pub initial: Option<f64>,
    /// The file starts this parameter held fixed (`(F)` after the initial value).
    pub fixed: bool,
    pub lower: Option<FdfBound>,
    pub upper: Option<FdfBound>,
}

/// What an `.FDF` file defines.
#[derive(Debug, Clone, PartialEq)]
pub struct FdfFunction {
    pub name: String,
    pub independent: Vec<String>,
    pub dependent: Vec<String>,
    pub params: Vec<FdfParam>,
    /// The right-hand side of the model in the expression language, if the body could be
    /// translated.
    pub formula: Option<String>,
    /// Why `formula` is `None`.
    pub formula_note: Option<String>,
    /// Derived quantities `(name, expression)` that could be translated.
    pub derived: Vec<(String, String)>,
}

fn clean_value(v: &str) -> &str {
    // An inline comment in a key section starts at whitespace followed by ';'.
    let mut prev_space = false;
    for (i, c) in v.char_indices() {
        if c == ';' && prev_space {
            return v[..i].trim();
        }
        prev_space = c.is_whitespace();
    }
    v.trim()
}

/// Splits on commas that are not inside parentheses.
fn split_list(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in v.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() || !out.is_empty() {
        out.push(cur.trim().to_string());
    }
    // A trailing comma leaves an empty last element; drop it.
    while out.last().is_some_and(|s| s.is_empty()) && out.len() > 1 {
        out.pop();
    }
    out
}

/// `value(FLAG,STATE)` pieces of a bound or initial-value element.
fn split_element(e: &str) -> (Option<f64>, Vec<String>) {
    let e = e.trim();
    let (num, flags) = match e.find('(') {
        Some(i) => (&e[..i], e[i + 1..].trim_end_matches(')')),
        None => (e, ""),
    };
    let num = num.trim();
    let value = if num.is_empty() || num == "--" { None } else { num.parse::<f64>().ok() };
    (value, flags.split(',').map(|s| s.trim().to_ascii_lowercase()).collect())
}

fn parse_bound(e: &str) -> Option<FdfBound> {
    let (value, flags) = split_element(e);
    value.map(|value| FdfBound { value, enabled: flags.iter().any(|f| f == "on") })
}

struct Sections {
    /// (lowercased name, lines) in file order.
    list: Vec<(String, Vec<String>)>,
}

impl Sections {
    fn parse(text: &str) -> Self {
        let mut list: Vec<(String, Vec<String>)> = Vec::new();
        for raw in text.lines() {
            let line = raw.trim_end_matches('\r');
            let t = line.trim();
            if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
                list.push((t[1..t.len() - 1].trim().to_ascii_lowercase(), Vec::new()));
            } else if let Some((_, lines)) = list.last_mut() {
                lines.push(line.to_string());
            }
        }
        Sections { list }
    }

    fn lines(&self, names: &[&str]) -> Option<&Vec<String>> {
        self.list.iter().find(|(n, _)| names.contains(&n.as_str())).map(|(_, l)| l)
    }

    /// Key/value pairs of a section (keys lowercased and trimmed).
    fn keys(&self, names: &[&str]) -> Vec<(String, String)> {
        let Some(lines) = self.lines(names) else {
            return Vec::new();
        };
        lines
            .iter()
            .filter_map(|l| {
                let (k, v) = l.split_once('=')?;
                Some((k.trim().to_ascii_lowercase(), clean_value(v).to_string()))
            })
            .collect()
    }

    fn text(&self, names: &[&str]) -> String {
        self.lines(names).map(|l| l.join("\n")).unwrap_or_default()
    }
}

fn key<'a>(keys: &'a [(String, String)], name: &str) -> Option<&'a str> {
    keys.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// Parses the text of an `.FDF` file.
pub fn parse_fdf(text: &str) -> Result<FdfFunction, FitError> {
    let sec = Sections::parse(text);
    let general = sec.keys(&["general information"]);
    let name = key(&general, "function name")
        .filter(|n| !n.is_empty())
        .ok_or_else(|| FitError::BadFunction("the file has no Function Name".into()))?
        .to_string();
    let independent: Vec<String> = sec.keys(&["independent variables"]).into_iter().map(|(k, _)| k).collect();
    let dependent: Vec<String> = sec.keys(&["dependent variables"]).into_iter().map(|(k, _)| k).collect();

    // Parameter names keep their case (formulas are case-sensitive in them), so read the raw line.
    let fp_lines = sec.lines(&["fitting parameters"]).cloned().unwrap_or_default();
    let raw_value = |k: &str| -> Option<String> {
        fp_lines.iter().find_map(|l| {
            let (a, b) = l.split_once('=')?;
            (a.trim().eq_ignore_ascii_case(k)).then(|| clean_value(b).to_string())
        })
    };
    let names = split_list(&raw_value("names").unwrap_or_default());
    if names.is_empty() || names.iter().any(|n| n.is_empty()) {
        return Err(FitError::BadFunction("the file lists no parameter names".into()));
    }
    let lowers = split_list(&raw_value("lower bounds").unwrap_or_default());
    let uppers = split_list(&raw_value("upper bounds").unwrap_or_default());
    let inits = split_list(&raw_value("initial values").unwrap_or_default());
    let params: Vec<FdfParam> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let (initial, flags) = inits.get(i).map(|e| split_element(e)).unwrap_or((None, vec![]));
            FdfParam {
                name: n.clone(),
                initial,
                fixed: initial.is_some() && flags.iter().any(|f| f == "f"),
                lower: lowers.get(i).and_then(|e| parse_bound(e)),
                upper: uppers.get(i).and_then(|e| parse_bound(e)),
            }
        })
        .collect();

    let dep = dependent.first().cloned().unwrap_or_else(|| "y".to_string());
    let (formula, formula_note) = match translate_body(&sec.text(&["formula"]), &dep) {
        Ok(f) => (Some(f), None),
        Err(why) => (None, Some(why)),
    };
    let mut derived = Vec::new();
    let dtext = strip_comments(&sec.text(&["derived parameters", "derivedparameters"]));
    for stmt in dtext.split(';') {
        if let Some((lhs, rhs)) = split_assignment(stmt)
            && let Ok(e) = translate_expression(rhs, "log10")
        {
            derived.push((lhs.to_string(), e));
        }
    }
    Ok(FdfFunction { name, independent, dependent, params, formula, formula_note, derived })
}

impl FdfFunction {
    /// A fit function ready to use. Fails if the file has several independent variables or the
    /// formula could not be translated.
    pub fn to_fit_function(&self) -> Result<FitFunction, FitError> {
        let Some(formula) = &self.formula else {
            return Err(FitError::BadFunction(format!(
                "{}: {}",
                self.name,
                self.formula_note.clone().unwrap_or_else(|| "no formula".into())
            )));
        };
        if self.dependent.len() > 1 {
            return Err(FitError::BadFunction(format!(
                "{}: {} dependent variables ({}); several dependent variables (a global fit) are not supported",
                self.name,
                self.dependent.len(),
                self.dependent.join(", ")
            )));
        }
        if self.independent.len() != 1 {
            return Err(FitError::BadFunction(format!(
                "{}: {} independent variables; only one is supported",
                self.name,
                self.independent.len()
            )));
        }
        let x = &self.independent[0];
        crate::nlsf::check_param_names(x, self.params.iter().map(|p| p.name.as_str()))
            .map_err(|why| FitError::BadFunction(format!("{}: {why}", self.name)))?;
        // The position would point into the translated text, not into the file, so leave it out.
        let expr =
            Expr::parse(formula).map_err(|e| FitError::BadFunction(format!("{}: {}", self.name, e.message())))?;
        for v in expr.variables() {
            if &v != x && v != "i" && !self.params.iter().any(|p| p.name == v) {
                return Err(FitError::BadFunction(format!(
                    "{}: the formula uses '{v}', which is neither a parameter nor the variable {x}",
                    self.name
                )));
            }
        }
        Ok(FitFunction {
            name: self.name.clone(),
            equation: format!("{} = {formula}", self.dependent.first().map(String::as_str).unwrap_or("y")),
            x_name: x.clone(),
            params: self
                .params
                .iter()
                .map(|p| ParamSpec {
                    name: p.name.clone(),
                    initial: p.initial.unwrap_or(f64::NAN),
                    lower: p.lower.filter(|b| b.enabled).map(|b| b.value),
                    upper: p.upper.filter(|b| b.enabled).map(|b| b.value),
                    fixed: p.fixed,
                })
                .collect(),
            model: Model::Formula(expr),
            init: None,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Translating the formula body
// ---------------------------------------------------------------------------------------------

/// Removes `//` and `/* */` comments and lines starting with `#`.
fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else if b[i] == '/' && b.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out.lines().filter(|l| !l.trim_start().starts_with('#')).collect::<Vec<_>>().join("\n")
}

fn split_assignment(stmt: &str) -> Option<(&str, &str)> {
    let s = stmt.trim();
    let i = s.find('=')?;
    let after = s.as_bytes().get(i + 1);
    let before = if i > 0 { s.as_bytes().get(i - 1) } else { None };
    if after == Some(&b'=') || matches!(before, Some(b'<' | b'>' | b'!' | b'=')) {
        return None;
    }
    let lhs = s[..i].trim();
    // Drop a type word: "double t" -> "t".
    let lhs = lhs.rsplit(char::is_whitespace).next().unwrap_or(lhs);
    if lhs.is_empty() || !lhs.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some((lhs, s[i + 1..].trim()))
}

/// C dialect to expression language, for one expression. `log_as` is what a call to `log` becomes:
/// the formula section is Origin C (natural logarithm, `ln`), the derived-parameter section is
/// LabTalk (base 10, `log10`).
fn translate_expression(s: &str, log_as: &str) -> Result<String, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let id: String = chars[start..i].iter().collect();
            let next = chars[i..].iter().find(|c| !c.is_whitespace());
            let is_call = next == Some(&'(');
            let low = id.to_ascii_lowercase();
            if is_call && low == "log" {
                out.push_str(log_as);
            } else if is_call && low == "fabs" {
                out.push_str("abs");
            } else if !is_call && low == "pi" {
                out.push_str("pi");
            } else if !is_call && id == "NANUM" {
                out.push_str("na()");
            } else {
                out.push_str(&id);
            }
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            // A number, with an optional exponent; a C suffix such as f is not expected.
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            out.extend(&chars[start..i]);
        } else {
            out.push(c);
            i += 1;
        }
    }
    let out = out.trim().to_string();
    Expr::parse(&out).map_err(|e| e.message().to_string())?;
    Ok(out)
}

/// Replaces whole-word occurrences of `name` in `s` by `(value)`.
fn substitute(s: &str, name: &str, value: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let id: String = chars[start..i].iter().collect();
            if id == name {
                out.push_str(&format!("({value})"));
            } else {
                out.push_str(&id);
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Tokens of the statement language: words and single punctuation characters kept as raw text
/// pieces, so that expressions can be cut out of the source as they were written.
struct Parser<'a> {
    s: &'a [char],
    pos: usize,
    dep: &'a str,
    /// Temporaries already defined, innermost definitions applied by substitution.
    temps: Vec<(String, String)>,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.pos < self.s.len() && self.s[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek_word(&mut self) -> Option<String> {
        self.skip_ws();
        let mut j = self.pos;
        while j < self.s.len() && (self.s[j].is_alphanumeric() || self.s[j] == '_') {
            j += 1;
        }
        (j > self.pos).then(|| self.s[self.pos..j].iter().collect())
    }

    fn eat_char(&mut self, c: char) -> bool {
        self.skip_ws();
        if self.s.get(self.pos) == Some(&c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Text up to the matching close of a parenthesis (the opening one already consumed).
    fn paren_text(&mut self) -> Result<String, String> {
        let start = self.pos;
        let mut depth = 1;
        while self.pos < self.s.len() {
            match self.s[self.pos] {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        let t: String = self.s[start..self.pos].iter().collect();
                        self.pos += 1;
                        return Ok(t);
                    }
                }
                _ => {}
            }
            self.pos += 1;
        }
        Err("unbalanced parenthesis in the formula".into())
    }

    fn until_semicolon(&mut self) -> Result<String, String> {
        let start = self.pos;
        while self.pos < self.s.len() && self.s[self.pos] != ';' {
            self.pos += 1;
        }
        if self.pos >= self.s.len() {
            // A last statement without a semicolon is allowed.
            return Ok(self.s[start..].iter().collect());
        }
        let t: String = self.s[start..self.pos].iter().collect();
        self.pos += 1;
        Ok(t)
    }

    fn resolve(&self, text: &str) -> Result<String, String> {
        let mut t = translate_expression(text, "ln")?;
        for (n, v) in self.temps.iter().rev() {
            t = substitute(&t, n, v);
        }
        Ok(t)
    }

    /// Parses statements until the end of the block (`}` or end of text). `cur` is the value
    /// assigned to the dependent variable so far on this path.
    fn statements(&mut self, mut cur: Option<String>) -> Result<Option<String>, String> {
        loop {
            self.skip_ws();
            if self.pos >= self.s.len() || self.s[self.pos] == '}' {
                return Ok(cur);
            }
            cur = self.statement(cur)?;
        }
    }

    fn statement(&mut self, cur: Option<String>) -> Result<Option<String>, String> {
        self.skip_ws();
        if self.eat_char('{') {
            let r = self.statements(cur)?;
            if !self.eat_char('}') {
                return Err("missing } in the formula".into());
            }
            return Ok(r);
        }
        if self.eat_char(';') {
            return Ok(cur);
        }
        match self.peek_word().as_deref() {
            Some("if") => {
                self.pos += 2;
                if !self.eat_char('(') {
                    return Err("expected ( after if".into());
                }
                let cond = self.paren_text()?;
                let cond = self.resolve(&cond)?;
                let then_v = self.statement(cur.clone())?;
                let else_v = if self.peek_word().as_deref() == Some("else") {
                    self.pos += 4;
                    self.statement(cur)?
                } else {
                    cur
                };
                match (then_v, else_v) {
                    (Some(a), Some(b)) => Ok(Some(format!("(({cond}) ? ({a}) : ({b}))"))),
                    _ => Err("a branch leaves the result unassigned".into()),
                }
            }
            Some("for" | "while" | "do" | "switch" | "return") => Err("loops and returns are not supported".into()),
            _ => {
                let stmt = self.until_semicolon()?;
                let first = stmt.split_whitespace().next().unwrap_or("");
                if !stmt.contains('=') && matches!(first, "double" | "int" | "float") {
                    // A declaration without a value.
                    return Ok(cur);
                }
                let Some((lhs, rhs)) = split_assignment(&stmt) else {
                    return Err(format!("cannot read the statement '{}'", stmt.trim()));
                };
                let value = self.resolve(rhs)?;
                if lhs == self.dep {
                    Ok(Some(value))
                } else {
                    // A temporary (declared or not): later statements see its value.
                    self.temps.push((lhs.to_string(), value));
                    Ok(cur)
                }
            }
        }
    }
}

/// Turns the formula section into one expression for the dependent variable `dep`.
fn translate_body(body: &str, dep: &str) -> Result<String, String> {
    let text = strip_comments(body);
    if text.trim().is_empty() {
        return Err("the formula section is empty".into());
    }
    let chars: Vec<char> = text.chars().collect();
    let mut p = Parser { s: &chars, pos: 0, dep, temps: Vec::new() };
    let r = p.statements(None)?;
    p.skip_ws();
    if p.pos < chars.len() {
        return Err("unexpected text after the formula".into());
    }
    let out = r.ok_or_else(|| format!("the formula never assigns {dep}"))?;
    // Strip the outermost redundant parentheses added around a plain substituted value.
    Expr::parse(&out).map_err(|e| e.message().to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlsf::{FitOptions, fit};

    const EXPDEC: &str = "[General Information]\r\nFunction Name=MyDecay\r\nFunction Form=Expression\r\n\r\n\
[Fitting Parameters]\r\nNaming Method=User-Defined\r\nNames=y0,A1,t1\r\nMeanings=offset,amp,time\r\n\
Lower Bounds=--(X,Off),--(X,Off),0.0(X,ON)\r\nUpper Bounds=--,--,--\r\nInitial Values=0(V),--(V),1(V)\r\n\r\n\
[Formula]\r\n// a decay\r\ny = y0 + A1*exp(-x/t1);\r\n\r\n\
[Independent Variables]\r\nx=\r\n[Dependent Variables]\r\ny=\r\n\
[Derived Parameters]\r\nk = 1/t1;\r\ntau = t1*log(2); // half life\r\nB = 10^(log(0.25)/t1);\r\n";

    #[test]
    fn simple_function_parses() {
        let f = parse_fdf(EXPDEC).unwrap();
        assert_eq!(f.name, "MyDecay");
        assert_eq!(f.params.len(), 3);
        assert_eq!(f.params[2].lower, Some(FdfBound { value: 0.0, enabled: true }));
        assert_eq!(f.params[0].initial, Some(0.0));
        assert_eq!(f.params[1].initial, None);
        assert_eq!(f.formula.as_deref(), Some("y0 + A1*exp(-x/t1)"));
        // In the derived-parameter section log is LabTalk's base-10 logarithm.
        assert_eq!(f.derived[1], ("tau".to_string(), "t1*log10(2)".to_string()));
        assert_eq!(f.derived[2].1, "10^(log10(0.25)/t1)");
        let ff = f.to_fit_function().unwrap();
        assert_eq!(ff.params[2].lower, Some(0.0));
        assert_eq!(ff.params[0].lower, None);
        let v = ff.eval(&[1.0, 2.0, 3.0], 0.0);
        assert!((v - 3.0).abs() < 1e-12);
    }

    #[test]
    fn imported_function_can_be_fitted() {
        let ff = parse_fdf(EXPDEC).unwrap().to_fit_function().unwrap();
        let x: Vec<f64> = (0..40).map(|i| i as f64 * 0.25).collect();
        let y: Vec<f64> = x.iter().map(|x| 0.5 + 2.0 * (-x / 3.0).exp()).collect();
        let ff = ff.with_initial("A1", 1.0);
        let r = fit(&ff, &x, &y, &FitOptions::default()).unwrap();
        assert!((r.param("t1").unwrap().value - 3.0).abs() < 1e-6);
    }

    #[test]
    fn temporaries_and_branches_are_translated() {
        let body = "double t = (x - xc)/w;\nif (x < xc) { y = y0 + A*exp(-t*t); }\nelse if (x == xc) y = y0 + A;\nelse { y = y0 + A*pow(0.5, t); }";
        let e = translate_body(body, "y").unwrap();
        let ff = FitFunction::from_formula("t", &e, &["y0", "xc", "w", "A"]).unwrap();
        let p = [1.0, 2.0, 0.5, 3.0];
        assert!((ff.eval(&p, 1.0) - (1.0 + 3.0 * (-4.0f64).exp())).abs() < 1e-12);
        assert!((ff.eval(&p, 2.0) - 4.0).abs() < 1e-12);
        assert!((ff.eval(&p, 3.0) - (1.0 + 3.0 * 0.5f64.powf(2.0))).abs() < 1e-12);
    }

    #[test]
    fn untranslatable_bodies_are_reported() {
        assert!(translate_body("y = nlf_voigt(x, y0);", "y").is_err());
        let f = parse_fdf(&EXPDEC.replace("y = y0 + A1*exp(-x/t1);", "for(;;) y = 1;")).unwrap();
        assert!(f.formula.is_none() && f.to_fit_function().is_err());
        // An unknown function is caught when the fit function is built.
        let g = parse_fdf(&EXPDEC.replace("exp(-x/t1)", "nlf_voigt(x)")).unwrap();
        assert!(g.to_fit_function().is_err());
        assert!(parse_fdf("[General Information]\nFunction Name=\n").is_err());
    }

    fn small_fdf(names: &str, formula: &str, dependent: &str) -> String {
        format!(
            "[General Information]\nFunction Name=T\n[Fitting Parameters]\nNames={names}\n[Formula]\n{formula}\n[Independent Variables]\nx=\n[Dependent Variables]\n{dependent}\n"
        )
    }

    #[test]
    fn several_dependent_variables_are_refused() {
        let t = small_fdf("a, b", "y1 = a*x;\ny2 = b*x;", "y1=\ny2=");
        let e = parse_fdf(&t).unwrap().to_fit_function().unwrap_err().to_string();
        assert!(e.contains("several dependent variables"), "{e}");
    }

    #[test]
    fn bad_parameter_names_are_refused() {
        for names in ["a, x", "a, a", "a, E", "a, pi"] {
            let t = small_fdf(names, "y = a*x;", "y=");
            assert!(parse_fdf(&t).unwrap().to_fit_function().is_err(), "{names}");
        }
        assert!(parse_fdf(&small_fdf("a, b", "y = a*x + b;", "y=")).unwrap().to_fit_function().is_ok());
    }

    #[test]
    fn formula_errors_name_no_position_in_translated_text() {
        let e = parse_fdf(&small_fdf("a, b", "y = a*(x+b;", "y=")).unwrap().to_fit_function().unwrap_err();
        assert!(!e.to_string().contains("position"), "{e}");
        let e = parse_fdf(&small_fdf("a", "y = gamcdf(x, a);", "y=")).unwrap().to_fit_function().unwrap_err();
        assert!(!e.to_string().contains("position"), "{e}");
        assert!(e.to_string().contains("gamcdf"), "{e}");
    }

    #[test]
    fn case_and_spacing_of_keys_do_not_matter() {
        let text = "[GENERAL INFORMATION]\nfunction name = Foo\n[FITTING PARAMETERS]\nNames = a, b\nLower Bounds = 0(I,Off), --\n[Formula]\ny = a + b*x\n[INDEPENDENT VARIABLES]\nx=\n[DEPENDENT VARIABLES]\ny=\n";
        let f = parse_fdf(text).unwrap();
        assert_eq!(f.params[1].name, "b");
        assert_eq!(f.params[0].lower, Some(FdfBound { value: 0.0, enabled: false }));
        assert!(f.to_fit_function().unwrap().params[0].lower.is_none());
    }

    #[test]
    fn installed_files_report() {
        let dir = std::path::Path::new("C:/Program Files/OriginLab/Origin2016/FitFunc");
        let Ok(rd) = std::fs::read_dir(dir) else {
            eprintln!("skipped: no Origin install");
            return;
        };
        let (mut files, mut parsed, mut built, mut evaluated) = (0, 0, 0, 0);
        let mut failures = Vec::new();
        for e in rd.flatten() {
            let path = e.path();
            if !path.extension().is_some_and(|x| x.eq_ignore_ascii_case("fdf")) {
                continue;
            }
            files += 1;
            let bytes = std::fs::read(&path).unwrap();
            let text: String = bytes.iter().map(|&b| b as char).collect();
            let Ok(f) = parse_fdf(&text) else {
                failures.push(format!("{}: parse", path.display()));
                continue;
            };
            parsed += 1;
            match f.to_fit_function() {
                Ok(ff) => {
                    built += 1;
                    let p = vec![1.3; ff.params.len()];
                    if ff.eval(&p, 0.7).is_finite() {
                        evaluated += 1;
                    }
                }
                Err(e) => failures.push(e.to_string()),
            }
        }
        eprintln!(
            "installed FDF files: {files} read, {parsed} parsed, {built} built, {evaluated} evaluated to a finite number"
        );
        for f in failures.iter().take(80) {
            eprintln!("  not built: {f}");
        }
        assert!(files == 0 || parsed == files);
    }
}
