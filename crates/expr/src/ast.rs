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
    pos: usize,
}

#[derive(Debug, Clone)]
pub(crate) enum Kind {
    Num(f64),
    /// A text string `"abc"`.
    Str(String),
    /// `col(A)$` or `col(A)[i]$`: the text of a cell; the child is a `Col` or `Index` node.
    Text(Box<Node>),
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
        args: Vec<Node>,
    },
    /// `col(A)[row]`; `base` is always a `Col` node.
    Index {
        base: Box<Node>,
        idx: Box<Node>,
    },
    /// `col(A)[from:to]`; `base` is always a `Col` node. Only valid as a statistic's argument.
    Range {
        base: Box<Node>,
        from: Box<Node>,
        to: Box<Node>,
        pos: usize,
    },
}

impl Node {
    /// Zero-based byte offset the node was built at (for error messages).
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// Builds a node, rejecting trees that are nested too deeply.
    pub(crate) fn new(kind: Kind, pos: usize) -> Result<Node, ExprError> {
        let child_depth = match &kind {
            Kind::Num(_) | Kind::Str(_) | Kind::Var { .. } | Kind::Col { .. } => 0,
            Kind::Unary(_, a) | Kind::Text(a) => a.depth,
            Kind::Binary(_, a, b) | Kind::Index { base: a, idx: b } => a.depth.max(b.depth),
            Kind::Cond(a, b, c) | Kind::Range { base: a, from: b, to: c, .. } => a.depth.max(b.depth).max(c.depth),
            Kind::Call { args, .. } => args.iter().map(|n| n.depth).max().unwrap_or(0),
        };
        if child_depth >= MAX_NODE_DEPTH {
            return Err(ExprError::at(pos, "expression is too long or too deeply nested"));
        }
        Ok(Node { kind, depth: child_depth + 1, pos })
    }

    /// Visits this node and all descendants, parents first.
    pub(crate) fn walk(&self, f: &mut impl FnMut(&Node)) {
        f(self);
        match &self.kind {
            Kind::Num(_) | Kind::Str(_) | Kind::Var { .. } | Kind::Col { .. } => {}
            Kind::Unary(_, a) | Kind::Text(a) => a.walk(f),
            Kind::Binary(_, a, b) | Kind::Index { base: a, idx: b } => {
                a.walk(f);
                b.walk(f);
            }
            Kind::Cond(a, b, c) | Kind::Range { base: a, from: b, to: c, .. } => {
                a.walk(f);
                b.walk(f);
                c.walk(f);
            }
            Kind::Call { args, .. } => args.iter().for_each(|n| n.walk(f)),
        }
    }
}

/// Which columns a formula reads through `cell(row, column)`, whose column is a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellReads {
    /// No `cell()` in the formula.
    None,
    /// The 1-based positions of the columns, when every `cell()` names its column with a number.
    Columns(Vec<usize>),
    /// A `cell()` whose column is computed: it may read any column.
    Any,
}

/// A parsed expression. Create one with [`Expr::parse`].
///
/// An `Expr` is immutable, cheap to keep around and can be shared between threads.
#[derive(Debug, Clone)]
pub struct Expr {
    pub(crate) source: String,
    pub(crate) root: Node,
}

impl Expr {
    /// Parses formula text. Function names, argument counts and `col()` syntax are checked
    /// here; variables and column names are checked when the expression is evaluated against a
    /// [`Host`](crate::Host).
    ///
    /// ```
    /// let e = sheaf_expr::Expr::parse("sin(col(A)) + i/10").unwrap();
    /// assert_eq!(e.variables(), ["i"]);
    /// ```
    pub fn parse(src: &str) -> Result<Expr, ExprError> {
        let root = parser::parse_root(src)?;
        Ok(Expr { source: src.to_string(), root })
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

    /// The columns the formula reads through `cell(row, column)`, for dependency tracking. A
    /// column given as a plain number is named; a computed one means any column.
    pub fn cell_reads(&self) -> CellReads {
        let mut positions: Vec<usize> = Vec::new();
        let mut any = false;
        let mut found = false;
        self.root.walk(&mut |n| {
            if let Kind::Call { func: Func::Cell, args } = &n.kind {
                found = true;
                match &args[1].kind {
                    Kind::Num(v) => {
                        let p = v.round();
                        if (1.0..1.0e9).contains(&p) && !positions.contains(&(p as usize)) {
                            positions.push(p as usize);
                        }
                    }
                    _ => any = true,
                }
            }
        });
        match (found, any) {
            (false, _) => CellReads::None,
            (true, true) => CellReads::Any,
            (true, false) => CellReads::Columns(positions),
        }
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
