use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::ast::span::Span;

/// A rich diagnostic error for compiler issues in the Filament language.
///
/// This error provides detailed diagnostic information including:
/// - The source code where the error occurred
/// - A precise location (span) in the source
/// - A descriptive message and label
/// - Optional help text with suggestions to fix the error
///
/// This is a shared diagnostic type used across all compilation phases
/// (parsing, validation, elaboration, etc.). Phase context is provided
/// by the `FilamentError` variant that wraps this diagnostic.
///
/// These rich errors are displayed using miette's pretty error formatting.
/// The source code itself is expected to be provided by the container error
/// type (e.g., `FilamentError::ValidationDiagnostic`).
#[derive(Debug, Error)]
#[error("{message}")]
pub struct DiagnosticError {
    /// Error message to display
    message: String,

    /// The error span in the source
    span: SourceSpan,

    /// Label for the error span
    label: String,

    /// Optional help text
    help: Option<String>,
}

impl Diagnostic for DiagnosticError {
    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(
            miette::LabeledSpan::new_with_span(Some(self.label.clone()), self.span),
        )))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn std::fmt::Display + 'a>)
    }

    // code(), severity(), url(), related() use defaults
    // code() will come from the FilamentError wrapper variant
}

impl DiagnosticError {
    /// Create a new diagnostic error from a Span value.
    ///
    /// # Arguments
    /// * `message` - The main error message
    /// * `span` - The source location where the error occurred
    /// * `label` - A label describing the error location
    /// * `help` - Optional help text with suggestions to fix the error
    pub fn from_span(
        message: String,
        span: Span,
        label: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        Self {
            message,
            span: span.into(),
            label: label.into(),
            help,
        }
    }
}

/// A type alias for `Result<T, DiagnosticError>`
///
/// # Usage
///
/// ```text
/// fn process() -> Result<ElaboratedElement> {
///     // Function that may return DiagnosticError
///     Ok(ElaboratedElement)
/// }
/// ```
pub type Result<T> = std::result::Result<T, DiagnosticError>;
