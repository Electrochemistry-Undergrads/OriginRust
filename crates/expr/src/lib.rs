//! The expression language of Sheaf: worksheet column formulas (`col(B)*1000`,
//! `sin(col(A)) + i/10`) and user-defined fit functions (`y0 + A*exp(-x/t)`).
//!
//! Parse once with [`Expr::parse`], then evaluate against a [`Host`] that supplies variable
//! values and column cells:
//!
//! ```
//! use sheaf_expr::{Expr, SimpleHost};
//!
//! let a = [1.0, 2.0, 3.0];
//! let host = SimpleHost::new().short("A", &a);
//! let formula = Expr::parse("col(A) * 1000 + i").unwrap();
//! let values = formula.eval_rows(&host, a.len()).unwrap();
//! assert_eq!(values, [1001.0, 2002.0, 3003.0]);
//! ```
//!
//! The language itself is described in `docs/spec/expression-language.md`.

mod ast;
mod error;
mod eval;
mod funcs;
mod lexer;
mod parser;
mod text;

pub use ast::{CellReads, ColumnRef, Expr};
pub use error::ExprError;
pub use eval::{Compiled, Host, SimpleHost};
pub use text::Value;

#[cfg(test)]
mod tests;
