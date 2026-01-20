//! The core diagnostic type for the Filament error system.
//!
//! A [`Diagnostic`] represents a single error or warning with optional
//! error code, multiple labeled source spans, and help text.

use std::fmt;

use crate::{
    error::{Severity, error_code::ErrorCode, label::Label},
    span::Span,
};

/// A rich diagnostic message with source location information.
///
/// Diagnostics provide detailed information about errors and warnings,
/// including:
/// - A severity level
/// - An optional error code for documentation and searchability
/// - A primary message describing the issue
/// - One or more labeled source spans
/// - Optional help text with suggestions
///
/// # Example
///
/// ```text
/// error[E301]: type `User` is defined multiple times
///   --> src/main.fil:10:1
///    |
/// 10 | type User = Rectangle;
///    | ^^^^^^^^^^^^^^^^^^^^^^ duplicate definition
///    |
///   --> src/main.fil:5:1
///    |
///  5 | type User = Circle;
///    | ------------------- first defined here
///    |
///    = help: remove the duplicate or use a different name
/// ```
#[derive(Debug, Clone)]
pub struct Diagnostic {
    severity: Severity,
    code: Option<ErrorCode>,
    message: String,
    labels: Vec<Label>,
    help: Option<String>,
}

impl Diagnostic {
    /// Create an error diagnostic.
    ///
    /// # Example
    ///
    /// ```
    /// # use filament_parser::error::{Diagnostic, ErrorCode};
    /// # use filament_parser::Span;
    ///
    /// let span = Span::new(0..10);
    /// let diag = Diagnostic::error("undefined type `Foo`")
    ///     .with_code(ErrorCode::E300)
    ///     .with_label(span, "not found")
    ///     .with_help("did you mean `Bar`?");
    /// ```
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    /// Create a warning diagnostic.
    ///
    /// # Example
    ///
    /// ```
    /// # use filament_parser::error::Diagnostic;
    /// # use filament_parser::Span;
    ///
    /// let span = Span::new(0..10);
    /// let diag = Diagnostic::warning("unused component")
    ///     .with_label(span, "this component is never referenced")
    ///     .with_help("consider removing it");
    /// ```
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    /// Get the severity of this diagnostic.
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Get the error code, if any.
    pub fn code(&self) -> Option<ErrorCode> {
        self.code
    }

    /// Get the primary message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get all labels attached to this diagnostic.
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Get the help text, if any.
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Set the error code.
    pub fn with_code(mut self, code: ErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    /// Add a primary label to this diagnostic.
    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::primary(span, message));
        self
    }

    /// Add a secondary label to this diagnostic.
    pub fn with_secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::secondary(span, message));
        self
    }

    /// Set the help text.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Create a new diagnostic with the given severity and message.
    fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            help: None,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: "error[E001]: message" or "error: message"
        write!(f, "{}", self.severity)?;
        if let Some(code) = self.code {
            write!(f, "[{}]", code)?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for Diagnostic {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_new() {
        let diag = Diagnostic::new(Severity::Error, "test error");

        assert!(diag.severity().is_error());
        assert!(!diag.severity().is_warning());
        assert_eq!(diag.message(), "test error");
        assert!(diag.code().is_none());
        assert!(diag.labels().is_empty());
        assert!(diag.help().is_none());
    }

    #[test]
    fn test_diagnostic_with_code() {
        let diag = Diagnostic::new(Severity::Error, "undefined type").with_code(ErrorCode::E300);

        assert_eq!(diag.code(), Some(ErrorCode::E300));
    }

    #[test]
    fn test_diagnostic_with_label() {
        let diag = Diagnostic::new(Severity::Error, "test error")
            .with_label(Span::new(10..20), "error here");

        assert_eq!(diag.labels().len(), 1);
        assert!(diag.labels()[0].is_primary());
        assert_eq!(diag.labels()[0].message(), "error here");
    }

    #[test]
    fn test_diagnostic_with_secondary_label() {
        let diag = Diagnostic::new(Severity::Error, "duplicate definition")
            .with_label(Span::new(10..20), "duplicate here")
            .with_secondary_label(Span::new(5..15), "first defined here");

        assert_eq!(diag.labels().len(), 2);
        assert!(diag.labels()[0].is_primary());
        assert!(diag.labels()[1].is_secondary());
    }

    #[test]
    fn test_diagnostic_with_help() {
        let diag = Diagnostic::new(Severity::Warning, "unused variable")
            .with_help("consider removing or prefixing with underscore");

        assert_eq!(
            diag.help(),
            Some("consider removing or prefixing with underscore")
        );
    }

    #[test]
    fn test_diagnostic_display_with_code() {
        let diag =
            Diagnostic::new(Severity::Error, "undefined type `Foo`").with_code(ErrorCode::E300);

        assert_eq!(diag.to_string(), "error[E300]: undefined type `Foo`");
    }

    #[test]
    fn test_diagnostic_display_without_code() {
        let diag = Diagnostic::new(Severity::Warning, "unused import");

        assert_eq!(diag.to_string(), "warning: unused import");
    }

    #[test]
    fn test_diagnostic_builder_chain() {
        let diag = Diagnostic::new(Severity::Error, "type `User` is defined multiple times")
            .with_code(ErrorCode::E301)
            .with_label(Span::new(100..120), "duplicate definition")
            .with_secondary_label(Span::new(50..70), "first defined here")
            .with_help("remove the duplicate or use a different name");

        assert!(diag.severity().is_error());
        assert_eq!(diag.code(), Some(ErrorCode::E301));
        assert_eq!(diag.message(), "type `User` is defined multiple times");
        assert_eq!(diag.labels().len(), 2);
        assert_eq!(
            diag.help(),
            Some("remove the duplicate or use a different name")
        );
    }
}
