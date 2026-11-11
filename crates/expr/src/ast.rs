//! The parsed form of an expression and the public [`Expr`] type.

use crate::error::ExprError;
use crate::funcs::{BinOp, Func};
use crate::parser;

/// Deepest allowed nesting of the tree (keeps evaluation and drop from overflowing the stack).
pub(crate) const MAX_NODE_DEPTH: u32 = 1000;

/// A reference to a worksheet column inside a formula.
///
/// How names are matched is up to the [`Host`](crate::Host); the usual choice is to ignore case
/// for short names and to compare long names exactly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColumnRef {
    /// `col(A)`: the column's short name, exactly as typed.
    Short(String),
    /// `col("Long Name")`: the column's long name.
    Long(String),
    /// `col(3)` or `wcol(3)`: 1-based position in the sheet.
    Index(usize),
}

impl std::fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnRef::Short(s) => write!(f, "col({s})"),
            ColumnRef::Long(s) => write!(f, "col(\"{s}\")"),
            ColumnRef::Index(n) => write!(f, "col({n})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnOp {
    Neg,
    Plus,
    Not,
}

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub kind: Kind,
    depth: u32,
}

#[derive(Debug, Clone)]
pub(crate) enum Kind {
    Num(f64),
    Var {
        name: String,
        pos: usize,
    },
    Col {
        r: ColumnRef,
        pos: usize,
    },
    Unary(UnOp, Box<Node>),
    Binary(BinOp, Box<Node>, Box<Node>),
    Cond(Box<Node>, Box<Node>, Box<Node>),
    Call {
        func: Func,
        name: &'static str,
        args: Vec<Node>,
        pos: usize,
    },
    Index {
        base: Box<Node>,
        idx: Box<Node>,
    },
}

impl Node {
    /// Builds a node, rejecting trees that are nested too deeply.
    pub(crate) fn new(kind: Kind, pos: usize) -> Result<Node, ExprError> {
        let child_depth = match &kind {
            Kind::Num(_) | Kind::Var { .. } | Kind::Col { .. } => 0,
            Kind::Unary(_, a) => a.depth,
            Kind::Binary(_, a, b) | Kind::Index { base: a, idx: b } => a.depth.max(b.depth),
            Kind::Cond(a, b, c) => a.depth.max(b.depth).max(c.depth),
            Kind::Call { args, .. } => args.iter().map(|n| n.depth).max().unwrap_or(0),
        };
        if child_depth >= MAX_NODE_DEPTH {
            return Err(ExprError::at(pos, "expression is too long or too deeply nested"));
        }
        Ok(Node {
            kind,
            depth: child_depth + 1,
        })
    }

    /// Visits this node and all descendants, parents first.
    pub(crate) fn walk(&self, f: &mut impl FnMut(&Node)) {
        f(self);
        match &self.kind {
            Kind::Num(_) | Kind::Var { .. } | Kind::Col { .. } => {}
            Kind::Unary(_, a) => a.walk(f),
            Kind::Binary(_, a, b) | Kind::Index { base: a, idx: b } => {
                a.walk(f);
                b.walk(f);
            }
            Kind::Cond(a, b, c) => {
                a.walk(f);
                b.walk(f);
                c.walk(f);
            }
            Kind::Call { args, .. } => args.iter().for_each(|n| n.walk(f)),
        }
    }
}

/// A parsed expression. Create one with [`parse`](crate::parse).
///
/// An `Expr` is immutable, cheap to keep around and can be shared between threads.
#[derive(Debug, Clone)]
pub struct Expr {
    pub(crate) source: String,
    pub(crate) root: Node,
}

impl Expr {
    pub(crate) fn parse(src: &str) -> Result<Expr, ExprError> {
        let root = parser::parse_root(src)?;
        Ok(Expr {
            source: src.to_string(),
            root,
        })
    }

    /// The text this expression was parsed from.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The free identifiers of the expression, in order of first use, without duplicates.
    ///
    /// These are the names the host must supply as variables (fit parameters, `x`, the row
    /// number `i`, ...) or, where the host allows it, as bare column names. Function names,
    /// constants such as `pi` and the arguments of `col()` are not included.
    pub fn variables(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        self.root.walk(&mut |n| {
            if let Kind::Var { name, .. } = &n.kind
                && !out.contains(name)
            {
                out.push(name.clone());
            }
        });
        out
    }

    /// Every column the expression reads through `col()` or `wcol()`, in order of first use,
    /// without duplicates. Use this to know which columns a formula column depends on.
    pub fn columns(&self) -> Vec<ColumnRef> {
        let mut out: Vec<ColumnRef> = Vec::new();
        self.root.walk(&mut |n| {
            if let Kind::Col { r, .. } = &n.kind
                && !out.contains(r)
            {
                out.push(r.clone());
            }
        });
        out
    }
}
