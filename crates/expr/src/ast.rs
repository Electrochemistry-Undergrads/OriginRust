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
    /// `col(j-1)`, `wcol(j+2)`: the column at the position of the column being calculated plus
    /// this offset (`col(j)` is offset 0). The host that knows `j` resolves it; without a `j` no
    /// column matches. A bare lowercase `col(j)` is the name `j` ([`ColumnRef::Short`]); the host
    /// decides whether that means the position.
    Relative(i64),
    /// `[Book]Sheet!col(A)` or `Sheet!col(A)`. Without a book, use the formula's workbook.
    Scoped { book: Option<String>, sheet: WorksheetRef, column: Box<ColumnRef> },
}

/// A worksheet selected by its name or its 1-based position within a workbook.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorksheetRef {
    Name(String),
    Index(usize),
}

/// A built-in column header row, using Origin's label-row characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnLabel {
    ShortName,
    LongName,
    Units,
    Comments,
    Formula,
}

impl ColumnLabel {
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code.to_ascii_uppercase().as_str() {
            "G" => Some(Self::ShortName),
            "L" => Some(Self::LongName),
            "U" => Some(Self::Units),
            "C" => Some(Self::Comments),
            "O" => Some(Self::Formula),
            _ => None,
        }
    }
}

impl std::fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnRef::Short(s) => write!(f, "col({s})"),
            ColumnRef::Long(s) => write!(f, "col(\"{s}\")"),
            ColumnRef::Index(n) => write!(f, "col({n})"),
            ColumnRef::Relative(0) => write!(f, "col(j)"),
            ColumnRef::Relative(d) => write!(f, "col(j{d:+})"),
            ColumnRef::Scoped { book, sheet, column } => {
                if let Some(book) = book {
                    write!(f, "[{book}]")?;
                }
                match sheet {
                    WorksheetRef::Name(name) => write!(f, "{name}!{column}"),
                    WorksheetRef::Index(index) => write!(f, "{index}!{column}"),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnOp {
    Neg,
    Plus,
    Not,
}

#[derive(Debug)]
pub(crate) struct Node {
    pub kind: Kind,
    depth: u32,
    pos: usize,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        crate::eval::grow_stack(|| Self { kind: self.kind.clone(), depth: self.depth, pos: self.pos })
    }
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
    /// A header read, e.g. `col(A)[L]$` (text) or `col(A)[L]` (numeric).
    Label {
        r: ColumnRef,
        label: ColumnLabel,
        text: bool,
    },
    /// A user parameter row. Numeric reads are missing; use `value(...$)` to convert text.
    Parameter {
        r: ColumnRef,
        name: String,
        text: bool,
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
            Kind::Num(_)
            | Kind::Str(_)
            | Kind::Var { .. }
            | Kind::Col { .. }
            | Kind::Label { .. }
            | Kind::Parameter { .. } => 0,
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
            Kind::Num(_)
            | Kind::Str(_)
            | Kind::Var { .. }
            | Kind::Col { .. }
            | Kind::Label { .. }
            | Kind::Parameter { .. } => {}
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

    /// Replaces each variable named in `vars` by a copy of its expression (T307).
    fn substitute(&mut self, vars: &[(String, Node)]) {
        if vars.is_empty() {
            return;
        }
        if let Kind::Var { name, .. } = &self.kind
            && let Some((_, value)) = vars.iter().find(|(n, _)| n == name)
        {
            *self = value.clone();
            return;
        }
        match &mut self.kind {
            Kind::Unary(_, a) | Kind::Text(a) => a.substitute(vars),
            Kind::Binary(_, a, b) | Kind::Index { base: a, idx: b } => {
                a.substitute(vars);
                b.substitute(vars);
            }
            Kind::Cond(a, b, c) | Kind::Range { base: a, from: b, to: c, .. } => {
                a.substitute(vars);
                b.substitute(vars);
                c.substitute(vars);
            }
            Kind::Call { args, .. } => args.iter_mut().for_each(|a| a.substitute(vars)),
            _ => {}
        }
    }

    fn resolve_parameters(&mut self, host: &dyn crate::Host) {
        crate::eval::grow_stack(|| self.resolve_parameters_inner(host));
    }

    fn resolve_parameters_inner(&mut self, host: &dyn crate::Host) {
        match &mut self.kind {
            Kind::Unary(_, a) | Kind::Text(a) => a.resolve_parameters(host),
            Kind::Binary(_, a, b) | Kind::Index { base: a, idx: b } => {
                a.resolve_parameters(host);
                b.resolve_parameters(host);
            }
            Kind::Cond(a, b, c) | Kind::Range { base: a, from: b, to: c, .. } => {
                a.resolve_parameters(host);
                b.resolve_parameters(host);
                c.resolve_parameters(host);
            }
            Kind::Call { args, .. } => args.iter_mut().for_each(|a| a.resolve_parameters(host)),
            _ => {}
        }
        let replacement = match &self.kind {
            Kind::Index { base, idx }
                if let Kind::Col { r, .. } = &base.kind
                    && let Kind::Var { name, .. } = &idx.kind
                    && (host.column_parameter(r, name).is_some() || (name != "i" && host.variable(name).is_none())) =>
            {
                Some(Kind::Parameter { r: r.clone(), name: name.clone(), text: false })
            }
            Kind::Text(a) if let Kind::Parameter { r, name, .. } = &a.kind => {
                Some(Kind::Parameter { r: r.clone(), name: name.clone(), text: true })
            }
            _ => None,
        };
        if let Some(kind) = replacement {
            self.kind = kind;
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

    /// Parses `formula` after Set Column Values' Before Formula Scripts (T307): `script` holds
    /// statements `name = expression` separated by `;` or new lines (an optional `double`, `int`,
    /// `const` or `range` in front is ignored, `//` starts a comment line). Every use of a name in
    /// the formula, and in the later statements, stands for its expression, so the result reads the
    /// same columns as if the expressions had been typed in place: `m = mean(col(B))` with
    /// `col(A) - m` is `col(A) - (mean(col(B)))`. An empty script parses the formula alone.
    pub fn parse_with_script(script: &str, formula: &str) -> Result<Expr, ExprError> {
        let mut vars: Vec<(String, Node)> = Vec::new();
        for statement in script.split([';', '\n', '\r']).map(str::trim) {
            if statement.is_empty() || statement.starts_with("//") {
                continue;
            }
            let body = ["double ", "int ", "const ", "range "]
                .iter()
                .find_map(|k| statement.strip_prefix(k))
                .unwrap_or(statement)
                .trim();
            let bytes = body.as_bytes();
            let at = (0..bytes.len()).find(|&i| {
                bytes[i] == b'='
                    && bytes.get(i + 1) != Some(&b'=')
                    && (i == 0 || !matches!(bytes[i - 1], b'<' | b'>' | b'!' | b'='))
            });
            let Some(at) = at else {
                return Err(ExprError::general(format!(
                    "the before-formula script line '{statement}' is not of the form name = expression"
                )));
            };
            let name = body[..at].trim();
            let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !valid {
                return Err(ExprError::general(format!(
                    "'{name}' in the before-formula script is not a variable name"
                )));
            }
            let mut value = Expr::parse(&body[at + 1..]).map_err(|e| {
                ExprError::general(format!("in the before-formula script, {name} = ...: {}", e.message()))
            })?;
            value.root.substitute(&vars);
            vars.retain(|(n, _)| n != name);
            vars.push((name.to_string(), value.root));
        }
        let mut expr = Expr::parse(formula)?;
        expr.root.substitute(&vars);
        Ok(expr)
    }

    /// The text this expression was parsed from.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Checks column references without evaluating functions or allocating output rows.
    /// Useful when a caller must report a missing source before deciding how many rows to fill.
    pub fn validate_columns(&self, host: &dyn crate::Host) -> Result<(), ExprError> {
        let mut error = None;
        self.root.walk(&mut |node| {
            let column = match &node.kind {
                Kind::Col { r, .. } | Kind::Label { r, .. } | Kind::Parameter { r, .. } => Some(r),
                _ => None,
            };
            if error.is_none()
                && let Some(r) = column
                && !host.has_column(r)
            {
                error = Some(ExprError::at(node.pos(), format!("unknown column {r}")));
            }
        });
        error.map_or(Ok(()), Err)
    }

    /// Resolves named subscripts against a host before inspecting dependencies. A parameter row
    /// takes precedence over a variable; otherwise a known variable (including `i`) still indexes
    /// data. An unknown name is an empty parameter row, as in Origin. Evaluation does this too.
    pub fn resolve_parameters(&self, host: &dyn crate::Host) -> Self {
        let mut resolved = self.clone();
        resolved.root.resolve_parameters(host);
        resolved
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
        self.column_refs(true, true)
    }

    /// Columns whose cell data is read, excluding header-only reads. These edges determine
    /// evaluation order and circular data dependencies; a header is independent of its cells.
    pub fn data_columns(&self) -> Vec<ColumnRef> {
        self.column_refs(true, false)
    }

    /// Columns whose headers are read. Changes to their metadata invalidate the result without
    /// creating a data dependency (a formula can read its own column's units, for example).
    pub fn label_columns(&self) -> Vec<ColumnRef> {
        self.column_refs(false, true)
    }

    /// Whether this resolved expression reads user parameter rows.
    pub fn reads_parameters(&self) -> bool {
        let mut found = false;
        self.root.walk(&mut |n| found |= matches!(n.kind, Kind::Parameter { .. }));
        found
    }

    /// Columns whose user parameter rows are read, after resolving named subscripts.
    pub fn parameter_columns(&self) -> Vec<ColumnRef> {
        let mut out = Vec::new();
        self.root.walk(&mut |n| {
            if let Kind::Parameter { r, .. } = &n.kind
                && !out.contains(r)
            {
                out.push(r.clone());
            }
        });
        out
    }

    fn column_refs(&self, data: bool, labels: bool) -> Vec<ColumnRef> {
        let mut out: Vec<ColumnRef> = Vec::new();
        self.root.walk(&mut |n| {
            let r = match &n.kind {
                Kind::Col { r, .. } if data => Some(r),
                Kind::Label { r, .. } | Kind::Parameter { r, .. } if labels => Some(r),
                _ => None,
            };
            if let Some(r) = r
                && !out.contains(r)
            {
                out.push(r.clone());
            }
        });
        out
    }
}
