//! The ParseError type for wrapping parsing diagnostics.
//!
//! [`ParseError`] wraps one or more [`Diagnostic`]s that occurred during
//! the parsing lifecycle (lexing, parsing, validation, or elaboration).

use std::fmt;

use crate::error::Diagnostic;

/// A type alias for `Result<T, Diagnostic>`.
pub type Result<T> = std::result::Result<T, Diagnostic>;

/// Error type for the parsing lifecycle.
///
/// Wraps one or more diagnostics.
#[derive(Debug)]
pub struct ParseError {
    diagnostics: Vec<Diagnostic>,
}

impl ParseError {
    /// Create a new parse error from diagnostics.
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    /// Get all diagnostics in this error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(first) = self.diagnostics.first() {
            write!(f, "{}", first)?;
            if self.diagnostics.len() > 1 {
                write!(f, " (+{} more)", self.diagnostics.len() - 1)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

impl From<Diagnostic> for ParseError {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }
}

impl From<Vec<Diagnostic>> for ParseError {
    fn from(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn test_parse_error_from_diagnostic() {
        let diag = Diagnostic::error("test error").with_code(ErrorCode::E300);
        let err: ParseError = diag.into();

        assert_eq!(err.diagnostics().len(), 1);
        assert_eq!(err.diagnostics()[0].message(), "test error");
    }

    #[test]
    fn test_parse_error_from_vec() {
        let diags = vec![Diagnostic::error("error 1"), Diagnostic::error("error 2")];
        let err: ParseError = diags.into();

        assert_eq!(err.diagnostics().len(), 2);
    }

    #[test]
    fn test_parse_error_display_single() {
        let diag = Diagnostic::error("undefined type");
        let err: ParseError = diag.into();

        assert_eq!(err.to_string(), "error: undefined type");
    }

    #[test]
    fn test_parse_error_display_multiple() {
        let diags = vec![
            Diagnostic::error("first error"),
            Diagnostic::error("second error"),
            Diagnostic::error("third error"),
        ];
        let err: ParseError = diags.into();

        assert_eq!(err.to_string(), "error: first error (+2 more)");
    }
}
