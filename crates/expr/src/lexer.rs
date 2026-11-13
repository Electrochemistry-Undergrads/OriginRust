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
            let value: f64 = text.parse().map_err(|_| ExprError::at(pos, format!("malformed number '{text}'")))?;
            if !value.is_finite() {
                return Err(ExprError::at(pos, format!("number '{text}' is too large")));
            }
            while chars.peek().is_some_and(|&(i, _)| i < end) {
                chars.next();
            }
            out.push(Token { tok: Tok::Num(value), pos });
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
            out.push(Token { tok: Tok::Ident(src[pos..end].to_string()), pos });
            continue;
        }
        if c == '"' {
            chars.next();
            let mut text = String::new();
            let mut closed = false;
            for (_, ch) in chars.by_ref() {
                if ch == '"' {
                    closed = true;
                    break;
                }
                text.push(ch);
            }
            if !closed {
                return Err(ExprError::at(pos, "text string is missing its closing '\"'"));
            }
            out.push(Token { tok: Tok::Str(text), pos });
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
            _ => single(c, pos)?,
        };
        if matches!(tok, Tok::Le | Tok::Ge | Tok::EqEq | Tok::Ne | Tok::AndAnd | Tok::OrOr) {
            chars.next();
        }
        out.push(Token { tok, pos });
    }
    out.push(Token { tok: Tok::End, pos: src.len() });
    Ok(out)
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
