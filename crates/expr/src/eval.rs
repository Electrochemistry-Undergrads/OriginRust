//! Evaluation: the [`Host`] trait, a ready-made host, and the compiled form of an expression.
//!
//! Evaluating against a host first *compiles* the parsed tree: variables are looked up once and
//! become constants, column references are resolved to a slice when the host can hand one out,
//! and constant sub-expressions are folded. The compiled tree is then run once per row without
//! any heap allocation (except for statistics over a moving row range on a host that cannot
//! lend column slices).

use crate::ast::{ColumnLabel, ColumnRef, Expr, Kind, Node, UnOp};
use crate::error::ExprError;
use crate::funcs::{self, Agg, BinOp, F1, F2, FnN, Func, NA, VecFn, apply_bin, max2, min2, truthy};
use crate::text::{self, StrFn, Value};
use std::cell::{Cell, OnceCell};

/// Where an expression gets the values of variables and worksheet cells.
///
/// Only [`variable`](Host::variable) and [`cell`](Host::cell) are required. The other methods
/// have defaults and let a host be stricter (reject unknown columns) or faster (lend whole
/// columns as slices).
pub trait Host {
    /// The value of a named variable (a fit parameter, `x`, ...), or `None` if there is none.
    /// Names are matched exactly as typed in the formula. The value is read once per
    /// evaluation call, not once per row.
    fn variable(&self, name: &str) -> Option<f64>;

    /// The value of a cell, with `row` counted from 0. `None` means "no such cell" (past the
    /// end of the column) and is treated as a missing value (NaN); a stored missing value
    /// should be returned as `Some(f64::NAN)`.
    fn cell(&self, col: &ColumnRef, row: usize) -> Option<f64>;

    /// Whether the column exists. If this returns `false`, evaluation fails with an
    /// "unknown column" error that points at the reference. The default accepts every column.
    fn has_column(&self, _col: &ColumnRef) -> bool {
        true
    }

    /// The column's values as one slice, if the host stores them that way. This is the fast
    /// path for column formulas; return `None` to have [`cell`](Host::cell) called per value.
    fn column_data(&self, _col: &ColumnRef) -> Option<&[f64]> {
        None
    }

    /// Number of rows in the column. The default uses [`column_data`](Host::column_data) or
    /// counts rows until [`cell`](Host::cell) returns `None`.
    fn column_len(&self, col: &ColumnRef) -> usize {
        if let Some(d) = self.column_data(col) {
            return d.len();
        }
        let mut n = 0;
        while n < (1 << 31) && self.cell(col, n).is_some() {
            n += 1;
        }
        n
    }

    /// The current date and time as a date number (days since the Julian epoch, 2440587 is
    /// 1970-01-01), for `now()` and `today()`. The default is the system clock in UTC; a host that
    /// knows the local time zone should override it.
    fn now(&self) -> f64 {
        funcs::system_now()
    }

    /// The text of the cell if it is a text cell (a text column, or a text cell of a Text &
    /// Numeric column); `None` for a number or a missing value. Numeric reads
    /// ([`cell`](Host::cell)) see a text cell as missing. The default has no text.
    fn text_cell(&self, _col: &ColumnRef, _row: usize) -> Option<String> {
        None
    }

    /// Whether the column holds any text cell. Formulas that read such a column bare, as in
    /// `col(A)`, are evaluated with text values (see [`Expr::eval_values`]).
    fn column_has_text(&self, _col: &ColumnRef) -> bool {
        false
    }

    /// The text of a built-in column header. A host without metadata returns `None`, read as
    /// empty text (or missing in a numeric header read). Column existence is checked separately.
    fn column_label(&self, _col: &ColumnRef, _label: ColumnLabel) -> Option<String> {
        None
    }

    /// The 0-based row that [`Expr::eval`] evaluates. The variable `i` is this row plus one.
    fn row(&self) -> usize {
        0
    }
}

/// A host built from plain data: named variables and columns that are borrowed slices.
///
/// Meant for fit functions (variables only), tests and simple callers. Variable names match
/// exactly; short column names match ignoring case, long names match exactly.
///
/// ```
/// use sheaf_expr::{Expr, SimpleHost};
/// let host = SimpleHost::new().var("A", 2.0).var("x", 3.0);
/// let e = Expr::parse("A * x^2").unwrap();
/// assert_eq!(e.eval(&host).unwrap(), 18.0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct SimpleHost<'a> {
    vars: Vec<(String, f64)>,
    cols: Vec<(ColumnRef, &'a [f64])>,
    row: usize,
}

impl<'a> SimpleHost<'a> {
    /// An empty host.
    pub fn new() -> Self {
        Self::default()
    }

    /// A host with the given variables.
    pub fn from_vars<S: AsRef<str>>(vars: impl IntoIterator<Item = (S, f64)>) -> Self {
        let mut h = Self::new();
        for (n, v) in vars {
            h.set_var(n.as_ref(), v);
        }
        h
    }

    /// Adds or replaces a variable.
    #[must_use]
    pub fn var(mut self, name: &str, value: f64) -> Self {
        self.set_var(name, value);
        self
    }

    /// Adds or replaces a variable in place (handy inside a fitting loop).
    pub fn set_var(&mut self, name: &str, value: f64) {
        if let Some(slot) = self.vars.iter_mut().find(|(n, _)| n == name) {
            slot.1 = value;
        } else {
            self.vars.push((name.to_string(), value));
        }
    }

    /// Adds a column under any kind of reference.
    #[must_use]
    pub fn column(mut self, col: ColumnRef, data: &'a [f64]) -> Self {
        self.cols.push((col, data));
        self
    }

    /// Adds a column that `col(name)` finds by its short name.
    #[must_use]
    pub fn short(self, name: &str, data: &'a [f64]) -> Self {
        self.column(ColumnRef::Short(name.to_string()), data)
    }

    /// Adds a column that `col("name")` finds by its long name.
    #[must_use]
    pub fn long(self, name: &str, data: &'a [f64]) -> Self {
        self.column(ColumnRef::Long(name.to_string()), data)
    }

    /// Sets the 0-based row used by [`Expr::eval`].
    #[must_use]
    pub fn at_row(mut self, row: usize) -> Self {
        self.row = row;
        self
    }

    fn find(&self, col: &ColumnRef) -> Option<&'a [f64]> {
        self.cols.iter().find_map(|(k, d)| {
            let same = match (k, col) {
                (ColumnRef::Short(a), ColumnRef::Short(b)) => a.eq_ignore_ascii_case(b),
                (a, b) => a == b,
            };
            same.then_some(*d)
        })
    }
}

impl Host for SimpleHost<'_> {
    fn variable(&self, name: &str) -> Option<f64> {
        self.vars.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }

    fn cell(&self, col: &ColumnRef, row: usize) -> Option<f64> {
        self.find(col)?.get(row).copied()
    }

    fn has_column(&self, col: &ColumnRef) -> bool {
        self.find(col).is_some()
    }

    fn column_data(&self, col: &ColumnRef) -> Option<&[f64]> {
        self.find(col)
    }

    fn row(&self) -> usize {
        self.row
    }
}

enum Col<'h> {
    Slice(&'h [f64]),
    Cell(ColumnRef),
}

impl Col<'_> {
    #[inline]
    fn get(&self, host: &dyn Host, row: usize) -> f64 {
        match self {
            Col::Slice(s) => s.get(row).copied().unwrap_or(NA),
            Col::Cell(r) => host.cell(r, row).unwrap_or(NA),
        }
    }
}

impl Col<'_> {
    /// The length up to the last value that is not missing: Origin does not store trailing
    /// missing cells, so they are not rows.
    fn used_len(&self, host: &dyn Host) -> usize {
        let mut n = self.len(host);
        while n > 0 && self.get(host, n - 1).is_nan() {
            n -= 1;
        }
        n
    }

    fn len(&self, host: &dyn Host) -> usize {
        match self {
            Col::Slice(s) => s.len(),
            Col::Cell(r) => host.column_len(r),
        }
    }

    /// Rows `lo..=hi` (1-based, already clamped) as a vector.
    fn rows(&self, host: &dyn Host, lo: usize, hi: usize) -> Vec<f64> {
        if lo > hi {
            return Vec::new();
        }
        match self {
            Col::Slice(s) => s[lo - 1..hi].to_vec(),
            Col::Cell(r) => (lo - 1..hi).map(|k| host.cell(r, k).unwrap_or(NA)).collect(),
        }
    }
}

enum C<'h> {
    Num(f64),
    Row,
    Col(Col<'h>),
    Index(Col<'h>, Box<C<'h>>),
    Unary(UnOp, Box<C<'h>>),
    Bin(BinOp, Box<C<'h>>, Box<C<'h>>),
    Cond(Box<C<'h>>, Box<C<'h>>, Box<C<'h>>),
    U(F1, Box<C<'h>>),
    B(F2, Box<C<'h>>, Box<C<'h>>),
    MinMax(bool, Vec<C<'h>>),
    Agg {
        kind: Agg,
        col: Col<'h>,
        range: Option<(Box<C<'h>>, Box<C<'h>>)>,
    },
    /// Column-in, column-out; `cache` holds the whole result when nothing depends on the row.
    Vector {
        kind: VecFn,
        col: Col<'h>,
        range: Option<(Box<C<'h>>, Box<C<'h>>)>,
        arg: Option<Box<C<'h>>>,
        cache: OnceCell<Vec<f64>>,
    },
    N(FnN, Vec<C<'h>>),
    Rand {
        gauss: bool,
        state: Cell<u64>,
    },
    Lookup(Box<C<'h>>, Col<'h>, Col<'h>),
    CellAt(Box<C<'h>>, Box<C<'h>>),
}

/// An expression bound to a host: variables and columns are resolved, ready to run row by row.
///
/// Create one with [`Expr::compile`]. It borrows the host, so the host's variable values are
/// fixed for as long as it lives.
pub struct Compiled<'h> {
    host: &'h dyn Host,
    root: C<'h>,
}

impl Compiled<'_> {
    /// Value for the 0-based `row` (the variable `i` is `row + 1`). Never fails: anything
    /// undefined comes out as NaN.
    pub fn eval_row(&self, row: usize) -> f64 {
        ev(self.host, &self.root, row)
    }
}

impl Expr {
    /// Resolves variables and columns against `host` and returns a form that is fast to
    /// evaluate repeatedly. Fails if a variable or column is unknown.
    pub fn compile<'h>(&self, host: &'h dyn Host) -> Result<Compiled<'h>, ExprError> {
        self.compile_seeded(host, funcs::system_seed())
    }

    /// Like [`compile`](Expr::compile), with the seed of the random numbers (`rnd()`, `grnd()`)
    /// fixed, so that the same seed gives the same numbers.
    pub fn compile_seeded<'h>(&self, host: &'h dyn Host, seed: u64) -> Result<Compiled<'h>, ExprError> {
        let seeds = Cell::new(seed);
        Ok(Compiled { host, root: build(host, &seeds, &self.root)? })
    }

    /// Evaluates the expression once, at the host's [`row`](Host::row).
    ///
    /// Division by zero, `sqrt` of a negative number and similar give NaN, not an error. Errors
    /// are only unknown variables or columns.
    pub fn eval(&self, host: &dyn Host) -> Result<f64, ExprError> {
        if is_dynamic(host, &self.root) {
            let seeds = Cell::new(funcs::system_seed());
            let v = build_v(host, &seeds, &self.root, true)?;
            return Ok(ev_v(host, &v, host.row()).number());
        }
        Ok(self.compile(host)?.eval_row(host.row()))
    }

    /// Evaluates the expression for rows `0..n` (the column-formula case) and returns the
    /// values in row order. The expression is compiled once; the loop does no parsing and no
    /// per-row allocation.
    pub fn eval_rows(&self, host: &dyn Host, n: usize) -> Result<Vec<f64>, ExprError> {
        self.eval_rows_seeded(host, n, funcs::system_seed())
    }

    /// [`eval_rows`](Expr::eval_rows) with a fixed seed for `rnd()` and `grnd()`.
    ///
    /// A formula with text in it gives its numbers, and a missing value where the result is text.
    pub fn eval_rows_seeded(&self, host: &dyn Host, n: usize, seed: u64) -> Result<Vec<f64>, ExprError> {
        if is_dynamic(host, &self.root) {
            let values = self.eval_values_seeded(host, n, seed)?;
            return Ok(values.iter().map(Value::number).collect());
        }
        let c = self.compile_seeded(host, seed)?;
        Ok((0..n).map(|r| c.eval_row(r)).collect())
    }

    /// Evaluates the expression for rows `0..n` like [`eval_rows`](Expr::eval_rows), but a row
    /// may come out as text: a string, `col(A)$`, a string function such as `left(col(A)$, 3)`,
    /// or a plain `col(A)` on a column with text cells. Text takes part in calculations as a
    /// missing value; `+` joins two texts.
    pub fn eval_values(&self, host: &dyn Host, n: usize) -> Result<Vec<Value>, ExprError> {
        self.eval_values_seeded(host, n, funcs::system_seed())
    }

    /// [`eval_values`](Expr::eval_values) with a fixed seed for `rnd()` and `grnd()`.
    pub fn eval_values_seeded(&self, host: &dyn Host, n: usize, seed: u64) -> Result<Vec<Value>, ExprError> {
        let seeds = Cell::new(seed);
        let v = build_v(host, &seeds, &self.root, true)?;
        Ok((0..n).map(|r| ev_v(host, &v, r)).collect())
    }

    /// Whether the formula involves text (a string, `col(A)$`, a string function, or a column
    /// with text cells), so that its result may be text.
    pub fn involves_text(&self, host: &dyn Host) -> bool {
        is_dynamic(host, &self.root)
    }
}

fn build_col<'h>(host: &'h dyn Host, n: &Node) -> Result<Col<'h>, ExprError> {
    let Kind::Col { r, pos } = &n.kind else {
        return Err(ExprError::general("internal error: expected a column reference"));
    };
    if !host.has_column(r) {
        return Err(ExprError::at(*pos, format!("unknown column {r}")));
    }
    Ok(match host.column_data(r) {
        Some(d) => Col::Slice(d),
        None => Col::Cell(r.clone()),
    })
}

/// Runs `f`, first moving to a newly allocated stack segment if less than 64 KiB of the current
/// stack is left. The tree of a long operator chain (`a+b+c+...`, up to about 1000 terms) is as
/// deep as the chain is long, and building or evaluating it recurses once per level; without
/// this a debug build overflows the 1 MB main thread at about 200 terms.
#[inline]
pub(crate) fn grow_stack<R>(f: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(64 * 1024, 1024 * 1024, f)
}

fn build<'h>(host: &'h dyn Host, seeds: &Cell<u64>, n: &Node) -> Result<C<'h>, ExprError> {
    grow_stack(|| build_inner(host, seeds, n))
}

fn build_inner<'h>(host: &'h dyn Host, seeds: &Cell<u64>, n: &Node) -> Result<C<'h>, ExprError> {
    let b = |n: &Node| build(host, seeds, n).map(Box::new);
    let c = match &n.kind {
        Kind::Num(v) => C::Num(*v),
        Kind::Str(_) | Kind::Text(_) | Kind::Label { text: true, .. } => {
            return Err(ExprError::at(
                n.pos(),
                "text values are not available here (only as the result of a column formula)",
            ));
        }
        Kind::Var { name, pos } => match host.variable(name) {
            Some(v) => C::Num(v),
            None if name == "i" => C::Row,
            None => return Err(ExprError::at(*pos, format!("unknown variable '{name}'"))),
        },
        Kind::Col { .. } => C::Col(build_col(host, n)?),
        Kind::Label { r, label, text: false } => {
            let s = read_label(host, r, *label, n.pos())?;
            C::Num(text::apply(StrFn::Value, vec![Value::Text(s)]).number())
        }
        Kind::Index { base, idx } => C::Index(build_col(host, base)?, b(idx)?),
        Kind::Range { pos, .. } => {
            return Err(ExprError::at(
                *pos,
                "a row range such as [1:10] can only be used inside a statistics function like mean()",
            ));
        }
        Kind::Unary(op, a) => C::Unary(*op, b(a)?),
        Kind::Binary(op, a, c) => C::Bin(*op, b(a)?, b(c)?),
        Kind::Cond(a, x, y) => C::Cond(b(a)?, b(x)?, b(y)?),
        Kind::Call { func, args, .. } => match *func {
            Func::U(f) => C::U(f, b(&args[0])?),
            Func::B(f) => C::B(f, b(&args[0])?, b(&args[1])?),
            Func::MinMax { max } => {
                let list = args.iter().map(|a| build(host, seeds, a)).collect::<Result<Vec<_>, _>>()?;
                if list.len() == 1 {
                    // A single scalar argument: min(x) is just x.
                    list.into_iter().next().unwrap_or(C::Num(NA))
                } else {
                    C::MinMax(max, list)
                }
            }
            Func::IfElse => C::Cond(b(&args[0])?, b(&args[1])?, b(&args[2])?),
            Func::Na => C::Num(NA),
            Func::N(f, _) => C::N(f, args.iter().map(|a| build(host, seeds, a)).collect::<Result<Vec<_>, _>>()?),
            Func::Clock { date_only } => {
                let now = host.now();
                C::Num(if date_only { now.floor() } else { now })
            }
            Func::Rand { gauss } => C::Rand { gauss, state: Cell::new(funcs::next_u64(seeds)) },
            Func::Lookup => C::Lookup(b(&args[0])?, build_col(host, &args[1])?, build_col(host, &args[2])?),
            Func::Cell => C::CellAt(b(&args[0])?, b(&args[1])?),
            Func::Str(_) => {
                return Err(ExprError::at(n.pos(), "text functions are not available here (only in a column formula)"));
            }
            Func::Vec(kind) => {
                let arg = args.get(1).map(b).transpose()?;
                let cache = OnceCell::new();
                match &args[0].kind {
                    Kind::Range { base, from, to, .. } => {
                        C::Vector { kind, col: build_col(host, base)?, range: Some((b(from)?, b(to)?)), arg, cache }
                    }
                    _ => C::Vector { kind, col: build_col(host, &args[0])?, range: None, arg, cache },
                }
            }
            Func::Agg(kind) => match &args[0].kind {
                Kind::Range { base, from, to, .. } => {
                    C::Agg { kind, col: build_col(host, base)?, range: Some((b(from)?, b(to)?)) }
                }
                _ => C::Agg { kind, col: build_col(host, &args[0])?, range: None },
            },
        },
    };
    Ok(fold(host, c))
}

/// Replaces nodes whose inputs are all constants by their value.
fn fold<'h>(host: &'h dyn Host, c: C<'h>) -> C<'h> {
    let constant = match &c {
        C::Unary(_, a) | C::U(_, a) => matches!(**a, C::Num(_)),
        C::Bin(_, a, b) | C::B(_, a, b) => matches!((&**a, &**b), (C::Num(_), C::Num(_))),
        C::Agg { range: None, .. } => true,
        C::Agg { range: Some((a, b)), .. } => matches!((&**a, &**b), (C::Num(_), C::Num(_))),
        C::N(_, a) => a.iter().all(|x| matches!(x, C::Num(_))),
        _ => false,
    };
    if constant { C::Num(ev(host, &c, 0)) } else { c }
}

fn ev(host: &dyn Host, c: &C<'_>, row: usize) -> f64 {
    match c {
        // Leaves need no stack check; this keeps the common case fast.
        C::Num(v) => *v,
        C::Row => (row + 1) as f64,
        C::Col(col) => col.get(host, row),
        _ => grow_stack(|| ev_inner(host, c, row)),
    }
}

fn ev_inner(host: &dyn Host, c: &C<'_>, row: usize) -> f64 {
    match c {
        C::Num(v) => *v,
        C::Row => (row + 1) as f64,
        C::Col(col) => col.get(host, row),
        C::Index(col, idx) => {
            let v = ev(host, idx, row);
            if v.is_nan() {
                return NA;
            }
            let r = v.round();
            if !(1.0..=4.0e15).contains(&r) {
                return NA;
            }
            col.get(host, r as usize - 1)
        }
        C::Unary(op, a) => {
            let v = ev(host, a, row);
            match op {
                UnOp::Neg => -v,
                UnOp::Plus => v,
                UnOp::Not => f64::from(!truthy(v)),
            }
        }
        C::Bin(op, a, b) => apply_bin(*op, ev(host, a, row), ev(host, b, row)),
        C::Cond(t, a, b) => {
            if truthy(ev(host, t, row)) {
                ev(host, a, row)
            } else {
                ev(host, b, row)
            }
        }
        C::U(f, a) => f(ev(host, a, row)),
        C::B(f, a, b) => f(ev(host, a, row), ev(host, b, row)),
        C::MinMax(max, args) => {
            let mut it = args.iter().map(|a| ev(host, a, row));
            let first = it.next().unwrap_or(NA);
            if *max { it.fold(first, max2) } else { it.fold(first, min2) }
        }
        C::N(f, args) => {
            let mut buf = [0.0; 4];
            for (slot, a) in buf.iter_mut().zip(args) {
                *slot = ev(host, a, row);
            }
            f(&buf[..args.len().min(4)])
        }
        C::Rand { gauss, state } => {
            if *gauss {
                funcs::normal(state)
            } else {
                funcs::uniform(state)
            }
        }
        C::Lookup(x, vx, vy) => {
            let x = ev(host, x, row);
            if x.is_nan() {
                return NA;
            }
            // First exact match wins; no match gives missing.
            (0..vx.len(host)).find(|&k| vx.get(host, k) == x).map_or(NA, |k| vy.get(host, k))
        }
        C::CellAt(r, c) => {
            let (r, c) = (ev(host, r, row).round(), ev(host, c, row).round());
            if !(r.is_finite() && c.is_finite()) || r < 1.0 || c < 1.0 || r > 4.0e15 || c > 1.0e9 {
                return NA;
            }
            host.cell(&ColumnRef::Index(c as usize), r as usize - 1).unwrap_or(NA)
        }
        C::Vector { kind, col, range, arg, cache } => {
            let nv = arg.as_ref().map_or(0.0, |a| ev(host, a, row));
            let fixed = range.as_ref().is_none_or(|(a, b)| matches!((&**a, &**b), (C::Num(_), C::Num(_))))
                && arg.as_ref().is_none_or(|a| matches!(**a, C::Num(_)));
            let compute = || {
                let len = col.len(host);
                let (lo, hi) = match range {
                    None => (1, len),
                    Some((a, b)) => {
                        let (a, b) = (ev(host, a, row), ev(host, b, row));
                        if a.is_nan() || b.is_nan() {
                            return Vec::new();
                        }
                        (a.round().clamp(1.0, 4.0e15) as usize, b.round().clamp(0.0, len as f64) as usize)
                    }
                };
                funcs::vec_fn(*kind, &col.rows(host, lo, hi), nv)
            };
            if fixed {
                cache.get_or_init(compute).get(row).copied().unwrap_or(NA)
            } else {
                compute().get(row).copied().unwrap_or(NA)
            }
        }
        C::Agg { kind, col, range } => {
            let len = if *kind == Agg::Rows { col.used_len(host) } else { col.len(host) };
            let (lo, hi) = match range {
                None => (1, len),
                Some((a, b)) => {
                    let (a, b) = (ev(host, a, row), ev(host, b, row));
                    if a.is_nan() || b.is_nan() {
                        return NA;
                    }
                    let a = a.round().clamp(1.0, 4.0e15) as usize;
                    let b = b.round().clamp(0.0, len as f64) as usize;
                    (a, b)
                }
            };
            if lo > hi {
                return funcs::aggregate(*kind, &[]);
            }
            match col {
                Col::Slice(s) => funcs::aggregate(*kind, &s[lo - 1..hi]),
                Col::Cell(r) => {
                    let v: Vec<f64> = (lo - 1..hi).map(|k| host.cell(r, k).unwrap_or(NA)).collect();
                    funcs::aggregate(*kind, &v)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Formulas with text: a second, slower evaluator that works on `Value`s
// ---------------------------------------------------------------------------------------------

/// Whether a subtree contains a text construct: a string, `col(A)$` or a string function.
fn explicit_text(n: &Node) -> bool {
    let mut found = false;
    n.walk(&mut |m| {
        found |= matches!(
            m.kind,
            Kind::Str(_) | Kind::Text(_) | Kind::Label { text: true, .. } | Kind::Call { func: Func::Str(_), .. }
        );
    });
    found
}

/// Whether the value of a node is a text by its construction (a string, `col(A)$`, a function
/// that returns text, a join with `+`).
fn is_str(n: &Node) -> bool {
    match &n.kind {
        Kind::Str(_) | Kind::Text(_) | Kind::Label { text: true, .. } => true,
        Kind::Call { func: Func::Str(f), .. } => f.returns_text(),
        Kind::Binary(BinOp::Add, a, b) => is_str(a) || is_str(b),
        Kind::Cond(_, a, b) => is_str(a) || is_str(b),
        Kind::Call { func: Func::IfElse, args } => is_str(&args[1]) || is_str(&args[2]),
        _ => false,
    }
}

/// Whether the value of a node could be a bare read of a text column (the whole formula, or a
/// branch of `?:`), which copies the text as it is.
fn bare_text_read(host: &dyn Host, n: &Node) -> bool {
    match &n.kind {
        Kind::Col { r, .. } => host.column_has_text(r),
        Kind::Index { base, .. } => bare_text_read(host, base),
        Kind::Cond(_, a, b) => bare_text_read(host, a) || bare_text_read(host, b),
        Kind::Call { func: Func::IfElse, args } => bare_text_read(host, &args[1]) || bare_text_read(host, &args[2]),
        _ => false,
    }
}

/// Whether a subtree needs the text evaluator. `free` says that a bare column read may stay a
/// text (the whole formula, an argument of a string function, an operand next to a string).
/// Elsewhere a bare column is a number, and text in it is missing, as in Origin.
fn needs_v(host: &dyn Host, n: &Node, free: bool) -> bool {
    explicit_text(n) || (free && bare_text_read(host, n))
}

/// Whether the whole formula needs the text evaluator.
fn is_dynamic(host: &dyn Host, n: &Node) -> bool {
    needs_v(host, n, true)
}

enum V<'h> {
    Num(C<'h>),
    Str(String),
    /// A cell read bare: its text if it is a text cell, else its number.
    Cell(ColumnRef, Option<Box<V<'h>>>),
    /// `col(A)$`.
    TextOf(ColumnRef, Option<Box<V<'h>>>),
    Unary(UnOp, Box<V<'h>>),
    Bin(BinOp, Box<V<'h>>, Box<V<'h>>),
    Cond(Box<V<'h>>, Box<V<'h>>, Box<V<'h>>),
    U(F1, Box<V<'h>>),
    B(F2, Box<V<'h>>, Box<V<'h>>),
    N(FnN, Vec<V<'h>>),
    MinMax(bool, Vec<V<'h>>),
    Str1(StrFn, Vec<V<'h>>),
}

fn read_label(host: &dyn Host, r: &ColumnRef, label: ColumnLabel, pos: usize) -> Result<String, ExprError> {
    if !host.has_column(r) {
        return Err(ExprError::at(pos, format!("unknown column {r}")));
    }
    Ok(host.column_label(r, label).unwrap_or_default())
}

fn cell_ref(host: &dyn Host, n: &Node) -> Result<(ColumnRef, Option<Node>), ExprError> {
    let (col, idx) = match &n.kind {
        Kind::Index { base, idx } => (&**base, Some((**idx).clone())),
        _ => (n, None),
    };
    let Kind::Col { r, pos } = &col.kind else {
        return Err(ExprError::general("internal error: expected a column reference"));
    };
    if !host.has_column(r) {
        return Err(ExprError::at(*pos, format!("unknown column {r}")));
    }
    Ok((r.clone(), idx))
}

fn build_v<'h>(host: &'h dyn Host, seeds: &Cell<u64>, n: &Node, free: bool) -> Result<V<'h>, ExprError> {
    grow_stack(|| build_v_inner(host, seeds, n, free))
}

fn build_v_inner<'h>(host: &'h dyn Host, seeds: &Cell<u64>, n: &Node, free: bool) -> Result<V<'h>, ExprError> {
    if !needs_v(host, n, free) {
        return Ok(V::Num(build(host, seeds, n)?));
    }
    let b = |n: &Node, free: bool| build_v(host, seeds, n, free).map(Box::new);
    let list = |args: &[Node]| args.iter().map(|a| build_v(host, seeds, a, false)).collect::<Result<Vec<_>, _>>();
    Ok(match &n.kind {
        Kind::Str(s) => V::Str(s.clone()),
        Kind::Label { r, label, text: true } => V::Str(read_label(host, r, *label, n.pos())?),
        Kind::Text(inner) => {
            let (r, idx) = cell_ref(host, inner)?;
            V::TextOf(r, idx.map(|i| b(&i, false)).transpose()?)
        }
        Kind::Col { .. } | Kind::Index { .. } => {
            let (r, idx) = cell_ref(host, n)?;
            V::Cell(r, idx.map(|i| b(&i, false)).transpose()?)
        }
        Kind::Unary(op, a) => V::Unary(*op, b(a, false)?),
        Kind::Binary(op, x, y) => {
            let joined = matches!(op, BinOp::Add | BinOp::Eq | BinOp::Ne) && (is_str(x) || is_str(y));
            V::Bin(*op, b(x, joined)?, b(y, joined)?)
        }
        Kind::Cond(c, x, y) => V::Cond(b(c, false)?, b(x, free)?, b(y, free)?),
        Kind::Call { func, args } => match *func {
            Func::Str(f) => {
                let list = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| build_v(host, seeds, a, f.text_arg(i)))
                    .collect::<Result<Vec<_>, _>>()?;
                V::Str1(f, list)
            }
            Func::U(f) => V::U(f, b(&args[0], false)?),
            Func::B(f) => V::B(f, b(&args[0], false)?, b(&args[1], false)?),
            Func::N(f, _) => V::N(f, list(args)?),
            Func::MinMax { max } => V::MinMax(max, list(args)?),
            Func::IfElse => V::Cond(b(&args[0], false)?, b(&args[1], free)?, b(&args[2], free)?),
            _ => return Err(ExprError::at(n.pos(), "text cannot be used in this function")),
        },
        Kind::Num(_) | Kind::Var { .. } | Kind::Range { .. } | Kind::Label { text: false, .. } => {
            build(host, seeds, n).map(V::Num)?
        }
    })
}

/// The 0-based row an optional subscript points at; `None` when it is missing or out of range.
fn subscript(host: &dyn Host, idx: &Option<Box<V<'_>>>, row: usize) -> Option<usize> {
    let Some(i) = idx else {
        return Some(row);
    };
    let r = ev_v(host, i, row).number().round();
    (1.0..=4.0e15).contains(&r).then(|| r as usize - 1)
}

fn ev_v(host: &dyn Host, v: &V<'_>, row: usize) -> Value {
    grow_stack(|| ev_v_inner(host, v, row))
}

fn ev_v_inner(host: &dyn Host, v: &V<'_>, row: usize) -> Value {
    let num = |v: &V<'_>| ev_v(host, v, row).number();
    match v {
        V::Num(c) => Value::Num(ev(host, c, row)),
        V::Str(s) => Value::Text(s.clone()),
        V::Cell(r, idx) => {
            let Some(k) = subscript(host, idx, row) else {
                return Value::Num(NA);
            };
            match host.text_cell(r, k) {
                Some(t) => Value::Text(t),
                None => Value::Num(host.cell(r, k).unwrap_or(NA)),
            }
        }
        V::TextOf(r, idx) => {
            let Some(k) = subscript(host, idx, row) else {
                return Value::Text(String::new());
            };
            Value::Text(match host.text_cell(r, k) {
                Some(t) => t,
                None => host.cell(r, k).map_or_else(String::new, text::format_general),
            })
        }
        V::Unary(op, a) => {
            let x = num(a);
            Value::Num(match op {
                UnOp::Neg => -x,
                UnOp::Plus => x,
                UnOp::Not => f64::from(!truthy(x)),
            })
        }
        V::Bin(op, a, b) => bin_value(*op, ev_v(host, a, row), ev_v(host, b, row)),
        V::Cond(c, a, b) => {
            if truthy(num(c)) {
                ev_v(host, a, row)
            } else {
                ev_v(host, b, row)
            }
        }
        V::U(f, a) => Value::Num(f(num(a))),
        V::B(f, a, b) => Value::Num(f(num(a), num(b))),
        V::N(f, args) => {
            let mut buf = [0.0; 4];
            for (slot, a) in buf.iter_mut().zip(args) {
                *slot = num(a);
            }
            Value::Num(f(&buf[..args.len().min(4)]))
        }
        V::MinMax(max, args) => {
            let mut it = args.iter().map(num);
            let first = it.next().unwrap_or(NA);
            Value::Num(if *max { it.fold(first, max2) } else { it.fold(first, min2) })
        }
        V::Str1(f, args) => text::apply(*f, args.iter().map(|a| ev_v(host, a, row)).collect()),
    }
}

/// A binary operator on values. Text is missing in arithmetic and ordered comparisons; `==` and
/// `!=` compare two texts; `+` joins texts, and a number added to a text adds nothing (Origin).
fn bin_value(op: BinOp, a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => Value::Num(apply_bin(op, x, y)),
        (a, b) => match op {
            BinOp::Add => match (a, b) {
                (Value::Text(x), Value::Text(y)) => Value::Text(x + &y),
                (Value::Text(x), _) | (_, Value::Text(x)) => Value::Text(x),
                _ => Value::Num(NA),
            },
            BinOp::Eq | BinOp::Ne => {
                let same = matches!((&a, &b), (Value::Text(x), Value::Text(y)) if x == y);
                Value::Num(f64::from(same == (op == BinOp::Eq)))
            }
            BinOp::And | BinOp::Or => Value::Num(apply_bin(op, a.number(), b.number())),
            _ => Value::Num(NA),
        },
    }
}
