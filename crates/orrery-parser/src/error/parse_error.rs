//! The ParseError type for wrapping parsing diagnostics.
//!
//! [`ParseError`] wraps one or more [`Diagnostic`]s that occurred during
//! the parsing lifecycle along with the [`SourceMap`] that maps virtual
//! spans to file locations.

use std::fmt;

use crate::{error::Diagnostic, source_map::SourceMap};

/// A type alias for `Result<T, Diagnostic>`.
pub type Result<T> = std::result::Result<T, Diagnostic>;

/// Error type for the parsing lifecycle.
///
/// Wraps one or more diagnostics together with the [`SourceMap`].
#[derive(Debug, thiserror::Error)]
pub struct ParseError<'a> {
    diagnostics: Vec<Diagnostic>,
    source_map: SourceMap<'a>,
}

impl fmt::Display for ParseError<'_> {
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

impl<'a> ParseError<'a> {
    /// Create a new parse error from diagnostics and a source map.
    pub fn new(diagnostics: Vec<Diagnostic>, source_map: SourceMap<'a>) -> Self {
        Self {
            diagnostics,
            source_map,
        }
    }

    /// Create a parse error from a single diagnostic and a source map.
    pub fn from_diagnostic(diagnostic: Diagnostic, source_map: SourceMap<'a>) -> Self {
        Self {
            diagnostics: vec![diagnostic],
            source_map,
        }
    }

    /// Get the source map associated with this error.
    pub fn source_map(&self) -> &SourceMap<'a> {
        &self.source_map
    }

    /// Get all diagnostics in this error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn test_parse_error_new() {
        let diag = Diagnostic::error("test error").with_code(ErrorCode::E300);
        let err = ParseError::new(vec![diag], SourceMap::new());

        assert_eq!(err.diagnostics().len(), 1);
        assert_eq!(err.diagnostics()[0].message(), "test error");
    }

    #[test]
    fn test_parse_error_new_multi_diagnostics() {
        let diags = vec![Diagnostic::error("error 1"), Diagnostic::error("error 2")];
        let err = ParseError::new(diags, SourceMap::new());

        assert_eq!(err.diagnostics().len(), 2);
    }

    #[test]
    fn test_parse_error_from_diagnostic() {
        let diag = Diagnostic::error("test error").with_code(ErrorCode::E300);
        let err = ParseError::from_diagnostic(diag, SourceMap::new());

        assert_eq!(err.diagnostics().len(), 1);
        assert_eq!(err.diagnostics()[0].message(), "test error");
    }

    #[test]
    fn test_parse_error_source_map_accessor() {
        let mut sm = SourceMap::new();
        sm.add_file("test.orr", "hello", None);
        let err = ParseError::new(vec![Diagnostic::error("err")], sm);

        assert_eq!(err.source_map().file_count(), 1);
    }

    #[test]
    fn test_parse_error_display_single() {
        let diag = Diagnostic::error("undefined type");
        let err = ParseError::from_diagnostic(diag, SourceMap::new());

        assert_eq!(err.to_string(), "error: undefined type");
    }

    #[test]
    fn test_parse_error_display_multiple() {
        let diags = vec![
            Diagnostic::error("first error"),
            Diagnostic::error("second error"),
            Diagnostic::error("third error"),
        ];
        let err = ParseError::new(diags, SourceMap::new());

        assert_eq!(err.to_string(), "error: first error (+2 more)");
    }
}
