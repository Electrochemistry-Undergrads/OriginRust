//! Recursive-descent parser.
//!
//! ```text
//! ternary  := or ( '?' ternary ':' ternary )?
//! or       := and ( '||' and )*
//! and      := compare ( '&&' compare )*
//! compare  := additive ( ('=='|'!='|'<'|'<='|'>'|'>=') additive )*
//! additive := multiplicative ( ('+'|'-') multiplicative )*
//! multiplicative := unary ( ('*'|'/'|'%') unary )*
//! unary    := ('-'|'+'|'!') unary | power
//! power    := postfix ( '^' unary )?          -- right associative, exponent may be signed
//! postfix  := primary ( '[' ternary ( ':' ternary )? ']' )? '$'?
//!             -- subscripts only on columns; '$' on a column/cell or text-returning function
//! primary  := number | "text" | name | name '(' args ')' | col-reference | '(' ternary ')'
//! ```

use crate::ast::{ColumnLabel, ColumnRef, Kind, Node, UnOp, WorksheetRef};
use crate::error::ExprError;
use crate::funcs::{Agg, BinOp, Func, Resolved, resolve};
use crate::lexer::{Tok, Token, lex};
use crate::text::StrFn;

/// How deeply parentheses, signs and `?:` may nest before the parser gives up.
const MAX_NESTING: u32 = 64;

struct Parser {
    source: String,
    toks: Vec<Token>,
    at: usize,
    nesting: u32,
}

pub(crate) fn parse_root(src: &str) -> Result<Node, ExprError> {
    let toks = lex(src)?;
    if toks.len() == 1 {
        return Err(ExprError::at(0, "the expression is empty"));
    }
    let mut p = Parser { source: src.to_string(), toks, at: 0, nesting: 0 };
    let node = p.ternary()?;
    let t = p.peek();
    if t.tok != Tok::End {
        let msg = match t.tok {
            Tok::RParen => "unmatched ')'".to_string(),
            _ => format!("expected an operator or the end of the expression, found {}", t.tok.describe()),
        };
        return Err(ExprError::at(t.pos, msg));
    }
    Ok(node)
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.at.min(self.toks.len() - 1)]
    }

    fn peek_tok(&self) -> &Tok {
        &self.peek().tok
    }

    fn advance(&mut self) -> Token {
        let t = self.peek().clone();
        if self.at < self.toks.len() - 1 {
            self.at += 1;
        }
        t
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek_tok() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, tok: Tok, what: &str) -> Result<(), ExprError> {
        if self.eat(&tok) {
            Ok(())
        } else {
            Err(ExprError::at(self.peek().pos, format!("expected {} {what}", tok.describe())))
        }
    }

    fn enter(&mut self) -> Result<(), ExprError> {
        self.nesting += 1;
        if self.nesting > MAX_NESTING {
            return Err(ExprError::at(self.peek().pos, "expression is nested too deeply"));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.nesting -= 1;
    }

    fn ternary(&mut self) -> Result<Node, ExprError> {
        self.enter()?;
        // Each nesting level passes through about ten parsing functions; in debug builds their
        // frames are large, so grow the stack on demand instead of overflowing a 1 MB thread.
        let r = crate::eval::grow_stack(|| self.ternary_inner());
        self.leave();
        r
    }

    fn ternary_inner(&mut self) -> Result<Node, ExprError> {
        let cond = self.or()?;
        let q = self.peek().pos;
        if self.eat(&Tok::Question) {
            let a = self.ternary()?;
            self.expect(Tok::Colon, "in the '?' conditional expression")?;
            let b = self.ternary()?;
            return Node::new(Kind::Cond(Box::new(cond), Box::new(a), Box::new(b)), q);
        }
        Ok(cond)
    }

    fn binary_level(
        &mut self,
        next: fn(&mut Parser) -> Result<Node, ExprError>,
        ops: &[(Tok, BinOp)],
    ) -> Result<Node, ExprError> {
        let mut lhs = next(self)?;
        loop {
            let Some(&(_, op)) = ops.iter().find(|(t, _)| t == self.peek_tok()) else {
                return Ok(lhs);
            };
            let pos = self.advance().pos;
            let rhs = next(self)?;
            lhs = Node::new(Kind::Binary(op, Box::new(lhs), Box::new(rhs)), pos)?;
        }
    }

    fn or(&mut self) -> Result<Node, ExprError> {
        self.binary_level(Parser::and, &[(Tok::OrOr, BinOp::Or)])
    }

    fn and(&mut self) -> Result<Node, ExprError> {
        self.binary_level(Parser::compare, &[(Tok::AndAnd, BinOp::And)])
    }

    fn compare(&mut self) -> Result<Node, ExprError> {
        self.binary_level(
            Parser::additive,
            &[
                (Tok::EqEq, BinOp::Eq),
                (Tok::Ne, BinOp::Ne),
                (Tok::Lt, BinOp::Lt),
                (Tok::Le, BinOp::Le),
                (Tok::Gt, BinOp::Gt),
                (Tok::Ge, BinOp::Ge),
            ],
        )
    }

    fn additive(&mut self) -> Result<Node, ExprError> {
        self.binary_level(Parser::multiplicative, &[(Tok::Plus, BinOp::Add), (Tok::Minus, BinOp::Sub)])
    }

    fn multiplicative(&mut self) -> Result<Node, ExprError> {
        self.binary_level(
            Parser::unary,
            &[(Tok::Star, BinOp::Mul), (Tok::Slash, BinOp::Div), (Tok::Percent, BinOp::Rem)],
        )
    }

    fn unary(&mut self) -> Result<Node, ExprError> {
        let op = match self.peek_tok() {
            Tok::Minus => Some(UnOp::Neg),
            Tok::Plus => Some(UnOp::Plus),
            Tok::Not => Some(UnOp::Not),
            _ => None,
        };
        let Some(op) = op else {
            return self.power();
        };
        let pos = self.advance().pos;
        self.enter()?;
        let operand = self.unary();
        self.leave();
        Node::new(Kind::Unary(op, Box::new(operand?)), pos)
    }

    fn power(&mut self) -> Result<Node, ExprError> {
        let base = self.postfix()?;
        if matches!(self.peek_tok(), Tok::Caret) {
            let pos = self.advance().pos;
            self.enter()?;
            let exp = self.unary();
            self.leave();
            return Node::new(Kind::Binary(BinOp::Pow, Box::new(base), Box::new(exp?)), pos);
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<Node, ExprError> {
        let node = self.primary()?;
        if matches!(node.kind, Kind::Col { .. }) && matches!(self.peek_tok(), Tok::Dollar) {
            let pos = self.advance().pos;
            return Node::new(Kind::Text(Box::new(node)), pos);
        }
        if matches!(&node.kind, Kind::Call { func: Func::Str(f), .. } if f.returns_text()) && self.eat(&Tok::Dollar) {
            // The call already produces text; accept Origin's string-result suffix without
            // converting a numeric function or changing the existing unsuffixed syntax.
            return Ok(node);
        }
        if !matches!(self.peek_tok(), Tok::LBracket) {
            return Ok(node);
        }
        let pos = self.advance().pos;
        if !matches!(node.kind, Kind::Col { .. }) {
            return Err(ExprError::at(
                pos,
                "only a column reference such as col(A) can be followed by a [row] subscript",
            ));
        }
        if let Tok::Ident(code) = self.peek_tok()
            && let Some(label) = ColumnLabel::from_code(code)
            && self.toks.get(self.at + 1).is_some_and(|t| t.tok == Tok::RBracket)
        {
            // A standalone label-row character takes precedence over a variable of that name.
            self.advance();
            self.advance();
            let text = self.eat(&Tok::Dollar);
            let Kind::Col { r, .. } = node.kind else { unreachable!("checked above") };
            return Node::new(Kind::Label { r, label, text }, pos);
        }
        // Origin permits bare parameter names containing spaces, e.g. [Bath Temp]$.
        // A single identifier remains an index until the host resolves label/variable precedence.
        let start = self.at;
        let mut end = start;
        while self.toks.get(end).is_some_and(|t| matches!(t.tok, Tok::Ident(_) | Tok::Num(_))) {
            end += 1;
        }
        if matches!(self.peek_tok(), Tok::Ident(_))
            && end > start + 1
            && self.toks.get(end).is_some_and(|t| t.tok == Tok::RBracket)
        {
            let name = self.source[self.toks[start].pos..self.toks[end].pos].trim_end().to_string();
            self.at = end + 1;
            let text = self.eat(&Tok::Dollar);
            let Kind::Col { r, .. } = node.kind else { unreachable!("checked above") };
            return Node::new(Kind::Parameter { r, name, text }, pos);
        }
        let first = self.ternary()?;
        if self.eat(&Tok::Colon) {
            let last = self.ternary()?;
            self.expect(Tok::RBracket, "after the row range")?;
            Node::new(Kind::Range { base: Box::new(node), from: Box::new(first), to: Box::new(last), pos }, pos)
        } else {
            self.expect(Tok::RBracket, "after the row index")?;
            let cell = Node::new(Kind::Index { base: Box::new(node), idx: Box::new(first) }, pos)?;
            if matches!(self.peek_tok(), Tok::Dollar) {
                let pos = self.advance().pos;
                return Node::new(Kind::Text(Box::new(cell)), pos);
            }
            Ok(cell)
        }
    }

    fn primary(&mut self) -> Result<Node, ExprError> {
        let t = self.advance();
        match t.tok {
            Tok::Percent => self.substitution(t.pos, false),
            Tok::QuotedSubstitution => self.substitution(t.pos, true),
            Tok::LBracket => {
                let book = self.reference_name("workbook")?;
                self.expect(Tok::RBracket, "after the workbook name")?;
                let sheet = self.worksheet_ref()?;
                self.scoped_column(Some(book), sheet, t.pos)
            }
            Tok::Ident(name) | Tok::Str(name) if matches!(self.peek_tok(), Tok::Not) => {
                self.scoped_column(None, WorksheetRef::Name(name), t.pos)
            }
            Tok::Num(index) if matches!(self.peek_tok(), Tok::Not) => {
                let sheet = Self::worksheet_index(index, t.pos)?;
                self.scoped_column(None, sheet, t.pos)
            }
            Tok::Num(v) => Node::new(Kind::Num(v), t.pos),
            Tok::Ident(name) => {
                if matches!(self.peek_tok(), Tok::LParen) {
                    self.advance();
                    self.call(name, t.pos)
                } else if name.eq_ignore_ascii_case("pi") {
                    Node::new(Kind::Num(std::f64::consts::PI), t.pos)
                } else if name.eq_ignore_ascii_case("e") {
                    Node::new(Kind::Num(std::f64::consts::E), t.pos)
                } else {
                    Node::new(Kind::Var { name, pos: t.pos }, t.pos)
                }
            }
            Tok::LParen => {
                let inner = self.ternary()?;
                self.expect(Tok::RParen, &format!("to close the '(' opened at offset {}", t.pos))?;
                Ok(inner)
            }
            Tok::Str(s) => Node::new(Kind::Str(s), t.pos),
            Tok::End => Err(ExprError::at(t.pos, "unexpected end of expression; expected a value")),
            other => Err(ExprError::at(t.pos, format!("expected a value, found {}", other.describe()))),
        }
    }

    fn reference_name(&mut self, what: &str) -> Result<String, ExprError> {
        let token = self.advance();
        match token.tok {
            Tok::Ident(name) | Tok::Str(name) if !name.is_empty() => Ok(name),
            _ => Err(ExprError::at(token.pos, format!("expected a {what} name or a quoted name"))),
        }
    }

    fn substitution(&mut self, pos: usize, quoted: bool) -> Result<Node, ExprError> {
        self.expect(Tok::LParen, "after '%' string substitution")?;
        // A static [book]sheet,column prefix distinguishes the legacy cell form from a
        // single expression such as %([Book]Sheet!col(A)$). Names remain real dependencies.
        let worksheet =
            matches!(self.peek_tok(), Tok::LBracket) && self.toks.get(self.at + 4).is_some_and(|t| t.tok == Tok::Comma);
        let value = if worksheet { self.worksheet_substitution(pos)? } else { self.ternary()? };
        self.expect(Tok::RParen, "after the substituted expression")?;
        if quoted { Ok(value) } else { Node::new(Kind::Call { func: Func::Str(StrFn::Value), args: vec![value] }, pos) }
    }

    fn worksheet_substitution(&mut self, pos: usize) -> Result<Node, ExprError> {
        let column_pos = self.advance().pos; // '['
        let book = self.reference_name("workbook")?;
        self.expect(Tok::RBracket, "after the workbook name")?;
        let sheet = self.worksheet_ref()?;
        self.expect(Tok::Comma, "after the worksheet")?;
        let column = self.advance();
        let column = match column.tok {
            Tok::Ident(name) => ColumnRef::Short(name),
            Tok::Str(name) => ColumnRef::Long(name),
            Tok::Num(n) if n >= 1.0 && n.fract() == 0.0 && n < 1e9 => ColumnRef::Index(n as usize),
            _ => {
                return Err(ExprError::at(
                    column.pos,
                    "expected a column name, quoted long name or positive column number",
                ));
            }
        };
        let r = ColumnRef::Scoped { book: Some(book), sheet, column: Box::new(column) };
        let column = Node::new(Kind::Col { r, pos: column_pos }, column_pos)?;
        self.expect(Tok::Comma, "before the worksheet cell row")?;
        // Keep the row separate from Kind::Index: a variable matching a parameter-row name
        // is still a numeric row expression here, never a metadata read.
        let mut args = vec![column, self.ternary()?];
        if self.eat(&Tok::Comma) {
            let start = self.at;
            let format = if let Tok::Str(s) = self.peek_tok() {
                let s = s.clone();
                self.advance();
                s
            } else {
                while matches!(self.peek_tok(), Tok::Num(_) | Tok::Ident(_) | Tok::Star) {
                    self.advance();
                }
                if self.at == start {
                    return Err(ExprError::at(self.toks[start].pos, "expected a static worksheet cell format"));
                }
                self.source[self.toks[start].pos..self.toks[self.at].pos].trim().to_string()
            };
            args.push(Node::new(Kind::Str(format), self.toks[start].pos)?);
        }
        Node::new(Kind::Call { func: Func::Str(StrFn::WorksheetCell), args }, pos)
    }

    fn worksheet_index(value: f64, pos: usize) -> Result<WorksheetRef, ExprError> {
        if value >= 1.0 && value.fract() == 0.0 && value < 1e9 {
            Ok(WorksheetRef::Index(value as usize))
        } else {
            Err(ExprError::at(pos, "a worksheet number must be a whole number, 1 or more"))
        }
    }

    fn worksheet_ref(&mut self) -> Result<WorksheetRef, ExprError> {
        if let Tok::Num(value) = self.peek_tok() {
            let value = *value;
            let pos = self.advance().pos;
            Self::worksheet_index(value, pos)
        } else {
            self.reference_name("worksheet").map(WorksheetRef::Name)
        }
    }

    fn scoped_column(&mut self, book: Option<String>, sheet: WorksheetRef, pos: usize) -> Result<Node, ExprError> {
        self.expect(Tok::Not, "between the worksheet and column")?;
        let token = self.advance();
        if !matches!(&token.tok, Tok::Ident(name) if name.eq_ignore_ascii_case("col") || name.eq_ignore_ascii_case("wcol"))
        {
            return Err(ExprError::at(token.pos, "expected col(...) or wcol(...) after the worksheet"));
        }
        self.expect(Tok::LParen, "after col or wcol")?;
        let node = self.column_ref(pos)?;
        let Kind::Col { r, .. } = node.kind else { unreachable!("column_ref creates a column") };
        Node::new(Kind::Col { r: ColumnRef::Scoped { book, sheet, column: Box::new(r) }, pos }, pos)
    }

    /// Parses after `name (`.
    fn call(&mut self, name: String, pos: usize) -> Result<Node, ExprError> {
        if name.eq_ignore_ascii_case("col") || name.eq_ignore_ascii_case("wcol") {
            return self.column_ref(pos);
        }
        let mut args = Vec::new();
        if !self.eat(&Tok::RParen) {
            loop {
                args.push(self.ternary()?);
                if self.eat(&Tok::Comma) {
                    continue;
                }
                self.expect(Tok::RParen, "after argument list")?;
                break;
            }
        }
        match resolve(&name, args.len()) {
            Resolved::Found(mut func, canonical) => {
                let is_col = |n: &Node| matches!(n.kind, Kind::Col { .. } | Kind::Range { .. });
                // `min(col(A))` and `max(col(A))` are column statistics, not the identity.
                if let Func::MinMax { max } = func
                    && args.len() == 1
                    && is_col(&args[0])
                {
                    func = Func::Agg(if max { Agg::Max } else { Agg::Min });
                }
                // A bare name may be a named range (T400); the host says so when the formula is bound.
                let is_col_or_name = |n: &Node| is_col(n) || matches!(n.kind, Kind::Var { .. });
                if matches!(func, Func::Agg(_)) && !args.first().is_some_and(is_col_or_name) {
                    return Err(ExprError::at(
                        pos,
                        format!("function '{canonical}' needs a column such as col(A) or col(A)[1:10] as its argument"),
                    ));
                }
                let needs_col = |what: &str| {
                    ExprError::at(
                        pos,
                        format!("function '{canonical}' needs a column such as col(A) as its {what} argument"),
                    )
                };
                if matches!(func, Func::Vec(_)) && !args.first().is_some_and(is_col_or_name) {
                    return Err(needs_col("first"));
                }
                if matches!(func, Func::Lookup) {
                    let plain = |n: &Node| matches!(n.kind, Kind::Col { .. });
                    if !plain(&args[1]) {
                        return Err(needs_col("second"));
                    }
                    if !plain(&args[2]) {
                        return Err(needs_col("third"));
                    }
                }
                Node::new(Kind::Call { func, args }, pos)
            }
            Resolved::Unknown => Err(ExprError::at(pos, format!("unknown function '{name}'"))),
            Resolved::BadArity { name, expected } => Err(ExprError::at(
                pos,
                format!(
                    "function '{name}' takes {expected}, but {} {} given",
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" }
                ),
            )),
        }
    }

    /// Parses after `col (`: a short name, a quoted long name, a 1-based number or `j` with an offset.
    fn column_ref(&mut self, pos: usize) -> Result<Node, ExprError> {
        let t = self.advance();
        let r = match t.tok {
            Tok::Ident(s) if s == "j" && matches!(self.peek_tok(), Tok::Plus | Tok::Minus) => {
                let sign = if self.advance().tok == Tok::Minus { -1 } else { 1 };
                let offset = self.advance();
                match offset.tok {
                    Tok::Num(v) if v.fract() == 0.0 && (0.0..1e9).contains(&v) => ColumnRef::Relative(sign * v as i64),
                    _ => return Err(ExprError::at(offset.pos, "expected a whole number after j+ or j- inside col()")),
                }
            }
            Tok::Ident(s) => ColumnRef::Short(s),
            Tok::Str(s) => ColumnRef::Long(s),
            Tok::Num(v) => {
                if v >= 1.0 && v.fract() == 0.0 && v < 1e9 {
                    ColumnRef::Index(v as usize)
                } else {
                    return Err(ExprError::at(t.pos, "a column number must be a whole number, 1 or more"));
                }
            }
            other => {
                return Err(ExprError::at(
                    t.pos,
                    format!("expected a column name, \"long name\" or number inside col(), found {}", other.describe()),
                ));
            }
        };
        self.expect(Tok::RParen, "after the column name")?;
        Node::new(Kind::Col { r, pos }, pos)
    }
}
