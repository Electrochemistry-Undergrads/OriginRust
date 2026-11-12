//! The error type shared by parsing, compiling and evaluating.

/// Something went wrong while parsing, compiling or evaluating an expression.
///
/// The message is written for the person who typed the formula. When the problem can be tied to
/// a place in the formula text, the displayed text ends with `at position N` and
/// [`ExprError::position`] returns the same `N`: a zero-based byte offset into the source text
/// (for plain ASCII formulas this is the character index). [`ExprError::message`] is the text
/// without that ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprError {
    message: String,
    position: Option<usize>,
}

impl ExprError {
    /// An error tied to a byte offset in the source.
    pub(crate) fn at(position: usize, message: impl AsRef<str>) -> Self {
        Self { message: message.as_ref().to_string(), position: Some(position) }
    }

    /// An error that has no useful source position.
    pub(crate) fn general(message: impl Into<String>) -> Self {
        Self { message: message.into(), position: None }
    }

    /// The zero-based byte offset in the formula text where the problem was found, if any.
    pub fn position(&self) -> Option<usize> {
        self.position
    }

    /// The message without the trailing `at position N`, for callers that report the place
    /// themselves or whose text is not the text the person typed.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.position {
            Some(p) => write!(f, "{} at position {p}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for ExprError {}
