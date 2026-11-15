//! Turns formula text into tokens.

use crate::error::ExprError;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    Num(f64),
    Ident(String),
    Str(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    /// A substitution inside a quoted string retains its text result.
    QuotedSubstitution,
    Caret,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Question,
    Colon,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    Ne,
    Not,
    Dollar,
    AndAnd,
    OrOr,
    End,
}

impl Tok {
    /// Short description for error messages.
    pub(crate) fn describe(&self) -> String {
        match self {
            Tok::Num(_) => "a number".into(),
            Tok::Ident(s) => format!("the name '{s}'"),
            Tok::Str(_) => "a text string".into(),
            Tok::Plus => "'+'".into(),
            Tok::Minus => "'-'".into(),
            Tok::Star => "'*'".into(),
            Tok::Slash => "'/'".into(),
            Tok::Percent => "'%'".into(),
            Tok::QuotedSubstitution => "a string substitution".into(),
            Tok::Caret => "'^'".into(),
            Tok::LParen => "'('".into(),
            Tok::RParen => "')'".into(),
            Tok::LBracket => "'['".into(),
            Tok::RBracket => "']'".into(),
            Tok::Comma => "','".into(),
            Tok::Question => "'?'".into(),
            Tok::Colon => "':'".into(),
            Tok::Lt => "'<'".into(),
            Tok::Le => "'<='".into(),
            Tok::Gt => "'>'".into(),
            Tok::Ge => "'>='".into(),
            Tok::EqEq => "'=='".into(),
            Tok::Ne => "'!='".into(),
            Tok::Dollar => "'$'".into(),
            Tok::Not => "'!'".into(),
            Tok::AndAnd => "'&&'".into(),
            Tok::OrOr => "'||'".into(),
            Tok::End => "the end of the expression".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub tok: Tok,
    pub pos: usize,
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_part(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// Splits `src` into tokens; the last token is always [`Tok::End`].
pub(crate) fn lex(src: &str) -> Result<Vec<Token>, ExprError> {
    lex_at(src, 0, 0)
}

const MAX_SUBSTITUTION_NESTING: usize = 64;

fn lex_at(src: &str, base: usize, depth: usize) -> Result<Vec<Token>, ExprError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut chars = src.char_indices().peekable();
    while let Some(&(pos, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && bytes.get(pos + 1).is_some_and(u8::is_ascii_digit)) {
            // Number: digits, optional fraction, optional exponent (only when digits follow).
            let mut p = pos;
            while bytes.get(p).is_some_and(u8::is_ascii_digit) {
                p += 1;
            }
            if bytes.get(p) == Some(&b'.') {
                p += 1;
                while bytes.get(p).is_some_and(u8::is_ascii_digit) {
                    p += 1;
                }
            }
            let mut end = p;
            if matches!(bytes.get(p), Some(b'e' | b'E')) {
                let mut q = p + 1;
                if matches!(bytes.get(q), Some(b'+' | b'-')) {
                    q += 1;
                }
                if bytes.get(q).is_some_and(u8::is_ascii_digit) {
                    while bytes.get(q).is_some_and(u8::is_ascii_digit) {
                        q += 1;
                    }
                    end = q;
                }
            }
            let text = &src[pos..end];
            let value: f64 =
                text.parse().map_err(|_| ExprError::at(base + pos, format!("malformed number '{text}'")))?;
            if !value.is_finite() {
                return Err(ExprError::at(base + pos, format!("number '{text}' is too large")));
            }
            while chars.peek().is_some_and(|&(i, _)| i < end) {
                chars.next();
            }
            out.push(Token { tok: Tok::Num(value), pos: base + pos });
            continue;
        }
        if is_ident_start(c) {
            let mut end = pos;
            while let Some(&(i, ch)) = chars.peek() {
                if is_ident_part(ch) {
                    end = i + ch.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            out.push(Token { tok: Tok::Ident(src[pos..end].to_string()), pos: base + pos });
            continue;
        }
        if c == '"' {
            let (tokens, end) = quoted_tokens(src, pos, base, depth)?;
            out.extend(tokens);
            while chars.peek().is_some_and(|&(i, _)| i < end) {
                chars.next();
            }
            continue;
        }
        chars.next();
        let next = chars.peek().map(|&(_, ch)| ch);
        let tok = match (c, next) {
            ('<', Some('=')) => Tok::Le,
            ('>', Some('=')) => Tok::Ge,
            ('=', Some('=')) => Tok::EqEq,
            ('!', Some('=')) => Tok::Ne,
            ('&', Some('&')) => Tok::AndAnd,
            ('|', Some('|')) => Tok::OrOr,
            _ => single(c, base + pos)?,
        };
        if matches!(tok, Tok::Le | Tok::Ge | Tok::EqEq | Tok::Ne | Tok::AndAnd | Tok::OrOr) {
            chars.next();
        }
        out.push(Token { tok, pos: base + pos });
    }
    out.push(Token { tok: Tok::End, pos: base + src.len() });
    Ok(out)
}

/// Finds a closing quote or substitution without building tokens. The mutual scans preserve
/// quotes inside substitutions and substitutions inside quoted arguments, with bounded depth.
fn quoted_end(src: &str, start: usize, base: usize, depth: usize) -> Result<usize, ExprError> {
    if depth >= MAX_SUBSTITUTION_NESTING {
        return Err(ExprError::at(base + start, "string substitution is nested too deeply"));
    }
    let mut at = start + 1;
    while at < src.len() {
        if src[at..].starts_with("%%") {
            at += 2;
        } else if src[at..].starts_with("%(") {
            at = substitution_end(src, at, base, depth + 1)? + 1;
        } else if src.as_bytes()[at] == b'"' {
            return Ok(at + 1);
        } else {
            at += src[at..].chars().next().expect("inside the string").len_utf8();
        }
    }
    Err(ExprError::at(base + start, "text string is missing its closing '\"'"))
}

fn substitution_end(src: &str, start: usize, base: usize, depth: usize) -> Result<usize, ExprError> {
    if depth >= MAX_SUBSTITUTION_NESTING {
        return Err(ExprError::at(base + start, "string substitution is nested too deeply"));
    }
    let mut at = start + 2;
    let mut parens = 1usize;
    while at < src.len() {
        match src.as_bytes()[at] {
            b'"' => {
                at = quoted_end(src, at, base, depth + 1)?;
                continue;
            }
            b'(' => parens += 1,
            b')' => {
                parens -= 1;
                if parens == 0 {
                    return Ok(at);
                }
            }
            _ => {}
        }
        at += src[at..].chars().next().expect("inside the substitution").len_utf8();
    }
    Err(ExprError::at(base + start, "string substitution is missing its closing ')'"))
}

/// Lowers interpolation to grouped string concatenation. Only formula text is tokenized;
/// text read from cells or metadata is never reparsed as an expression or command.
fn quoted_tokens(src: &str, start: usize, base: usize, depth: usize) -> Result<(Vec<Token>, usize), ExprError> {
    if depth >= MAX_SUBSTITUTION_NESTING {
        return Err(ExprError::at(base + start, "string substitution is nested too deeply"));
    }
    let mut out = Vec::new();
    let mut literal = String::new();
    let mut at = start + 1;
    let mut interpolated = false;
    while at < src.len() {
        if src.as_bytes()[at] == b'"' {
            out.push(Token { tok: Tok::Str(literal), pos: base + start });
            if interpolated {
                out.push(Token { tok: Tok::RParen, pos: base + at });
            }
            return Ok((out, at + 1));
        }
        if src[at..].starts_with("%%") {
            literal.push('%');
            at += 2;
            continue;
        }
        if src[at..].starts_with("%(") {
            let close = substitution_end(src, at, base, depth + 1)?;
            if !interpolated {
                out.push(Token { tok: Tok::LParen, pos: base + start });
                interpolated = true;
            }
            out.push(Token { tok: Tok::Str(std::mem::take(&mut literal)), pos: base + start });
            out.push(Token { tok: Tok::Plus, pos: base + at });
            out.push(Token { tok: Tok::QuotedSubstitution, pos: base + at });
            out.push(Token { tok: Tok::LParen, pos: base + at });
            let inner = &src[at + 2..close];
            let keyword = match inner.trim().to_ascii_uppercase().as_str() {
                "TAB" => Some("\t"),
                "CRLF" => Some("\r\n"),
                _ => None,
            };
            if let Some(keyword) = keyword {
                out.push(Token { tok: Tok::Str(keyword.into()), pos: base + at });
            } else {
                let mut tokens = lex_at(inner, base + at + 2, depth + 1)?;
                tokens.pop(); // The surrounding expression supplies its own end token.
                out.extend(tokens);
            }
            out.push(Token { tok: Tok::RParen, pos: base + close });
            out.push(Token { tok: Tok::Plus, pos: base + close });
            at = close + 1;
            continue;
        }
        let ch = src[at..].chars().next().expect("inside the string");
        literal.push(ch);
        at += ch.len_utf8();
    }
    Err(ExprError::at(base + start, "text string is missing its closing '\"'"))
}

fn single(c: char, pos: usize) -> Result<Tok, ExprError> {
    Ok(match c {
        '+' => Tok::Plus,
        '-' => Tok::Minus,
        '*' => Tok::Star,
        '/' => Tok::Slash,
        '%' => Tok::Percent,
        '^' => Tok::Caret,
        '(' => Tok::LParen,
        ')' => Tok::RParen,
        '[' => Tok::LBracket,
        ']' => Tok::RBracket,
        ',' => Tok::Comma,
        '?' => Tok::Question,
        ':' => Tok::Colon,
        '<' => Tok::Lt,
        '>' => Tok::Gt,
        '!' => Tok::Not,
        '$' => Tok::Dollar,
        '=' => {
            return Err(ExprError::at(pos, "a single '=' is not an operator here; use '==' to compare"));
        }
        '&' => {
            return Err(ExprError::at(pos, "'&' must be doubled ('&&') to mean logical and"));
        }
        '|' => {
            return Err(ExprError::at(pos, "'|' must be doubled ('||') to mean logical or"));
        }
        _ => return Err(ExprError::at(pos, format!("unexpected character '{c}'"))),
    })
}
