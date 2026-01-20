//! Collector for accumulating diagnostics during a processing phase.
//!
//! The [`DiagnosticCollector`] allows phases to report multiple errors
//! and warnings instead of failing on the first error encountered.

use crate::error::{Diagnostic, ParseError};

/// A collector for accumulating diagnostics during a processing phase.
///
/// # Example
///
/// ```text
/// # use filament_parser::error::{Diagnostic, ErrorCode};
/// # use filament_parser::Span;
///
/// let mut collector = DiagnosticCollector::new();
///
/// let span1 = Span::new(0..10);
/// let span2 = Span::new(20..30);
///
/// // Emit multiple errors
/// collector.emit(
///     Diagnostic::error("undefined component `foo`")
///         .with_code(ErrorCode::E200)
///         .with_label(span1, "not found")
/// );
///
/// collector.emit(
///     Diagnostic::error("undefined component `bar`")
///         .with_code(ErrorCode::E200)
///         .with_label(span2, "not found")
/// );
///
/// // Finish and convert to Result
/// let result = collector.finish();
/// ```
#[derive(Debug, Default)]
pub struct DiagnosticCollector {
    diagnostics: Vec<Diagnostic>,
    has_errors: bool,
}

impl DiagnosticCollector {
    /// Create a new empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Emit a diagnostic to this collector.
    ///
    /// The diagnostic is added to the collection and if it's an error,
    /// the collector is marked as having errors.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        if diagnostic.severity().is_error() {
            self.has_errors = true;
        }
        self.diagnostics.push(diagnostic);
    }

    /// Finish collection and return a result.
    ///
    /// - If there are errors, returns `Err(ParseError)` with all diagnostics.
    /// - If there are no errors, returns `Ok(())`.
    ///
    /// Note: Warnings are currently discarded in the success case.
    pub fn finish(self) -> Result<(), ParseError> {
        if self.has_errors {
            Err(ParseError::new(self.diagnostics))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{error::ErrorCode, span::Span};

    #[test]
    fn test_collector_new_finish_ok() {
        let collector = DiagnosticCollector::new();
        assert!(collector.finish().is_ok());
    }

    #[test]
    fn test_collector_emit_error_finish_err() {
        let mut collector = DiagnosticCollector::new();

        collector.emit(Diagnostic::error("test error"));

        assert!(collector.finish().is_err());
    }

    #[test]
    fn test_collector_emit_warning_finish_ok() {
        let mut collector = DiagnosticCollector::new();

        collector.emit(Diagnostic::warning("test warning"));

        assert!(collector.finish().is_ok());
    }

    #[test]
    fn test_collector_emit_multiple_finish_err() {
        let mut collector = DiagnosticCollector::new();

        collector.emit(Diagnostic::error("error 1"));
        collector.emit(Diagnostic::warning("warning 1"));
        collector.emit(Diagnostic::error("error 2"));

        assert!(collector.finish().is_err());
    }

    #[test]
    fn test_collector_finish_with_errors() {
        let mut collector = DiagnosticCollector::new();

        collector.emit(
            Diagnostic::error("test error")
                .with_code(ErrorCode::E300)
                .with_label(Span::new(10..20), "here"),
        );
        collector.emit(Diagnostic::warning("test warning"));

        let result = collector.finish();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.diagnostics().len(), 2);
        assert_eq!(err.diagnostics()[0].message(), "test error");
    }

    #[test]
    fn test_collector_finish_warnings_only() {
        let mut collector = DiagnosticCollector::new();

        collector.emit(Diagnostic::warning("warning 1"));
        collector.emit(Diagnostic::warning("warning 2"));

        let result = collector.finish();
        assert!(result.is_ok());
    }
}
