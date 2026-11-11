//! The error type shared by parsing, compiling and evaluating.

use thiserror::Error;

/// Something went wrong while parsing, compiling or evaluating an expression.
///
/// The message is written for the person who typed the formula. When the problem can be tied to
/// a place in the formula text, the message ends with `at position N` and [`ExprError::position`]
/// returns the same `N`: a zero-based byte offset into the source text (for plain ASCII formulas
/// this is the character index).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{text}")]
pub struct ExprError {
    text: String,
    position: Option<usize>,
}

impl ExprError {
    /// An error tied to a byte offset in the source.
    pub(crate) fn at(position: usize, message: impl AsRef<str>) -> Self {
        Self { text: format!("{} at position {position}", message.as_ref()), position: Some(position) }
    }

    /// An error that has no useful source position.
    pub(crate) fn general(message: impl Into<String>) -> Self {
        Self { text: message.into(), position: None }
    }

    /// The zero-based byte offset in the formula text where the problem was found, if any.
    pub fn position(&self) -> Option<usize> {
        self.position
    }
}
