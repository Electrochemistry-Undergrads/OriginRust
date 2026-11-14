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

use crate::ast::{ColumnLabel, ColumnRef, Kind, Node, UnOp};
use crate::error::ExprError;
use crate::funcs::{Agg, BinOp, Func, Resolved, resolve};
use crate::lexer::{Tok, Token, lex};

/// How deeply parentheses, signs and `?:` may nest before the parser gives up.
const MAX_NESTING: u32 = 64;

struct Parser {
    toks: Vec<Token>,
    at: usize,
    nesting: u32,
}

pub(crate) fn parse_root(src: &str) -> Result<Node, ExprError> {
    let toks = lex(src)?;
    if toks.len() == 1 {
        return Err(ExprError::at(0, "the expression is empty"));
    }
    let mut p = Parser { toks, at: 0, nesting: 0 };
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
                if matches!(func, Func::Agg(_)) && !args.first().is_some_and(is_col) {
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
                if matches!(func, Func::Vec(_)) && !args.first().is_some_and(is_col) {
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

    /// Parses after `col (`: a short name, a quoted long name or a 1-based number.
    fn column_ref(&mut self, pos: usize) -> Result<Node, ExprError> {
        let t = self.advance();
        let r = match t.tok {
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
