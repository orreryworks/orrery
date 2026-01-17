use thiserror::Error;

use crate::span::Span;

/// A rich diagnostic error for compiler issues in the Filament language.
///
/// This error provides detailed diagnostic information including:
/// - A precise location ([`Span`]) in the source code
/// - A descriptive message and label
/// - Optional help text with suggestions to fix the error
///
/// This is a shared diagnostic type used across all parsing phases
/// (lexing, parsing, validation, elaboration, etc.). Phase context is provided
/// by the container error type.
///
/// The source code itself is expected to be provided by the container error
/// type (e.g., `FilamentError`).
#[derive(Debug, Error)]
#[error("{message}")]
pub struct DiagnosticError {
    /// Error message to display
    message: String,

    /// The error span in the source
    span: Span,

    /// Label for the error span
    label: String,

    /// Optional help text
    help: Option<String>,
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
            span,
            label: label.into(),
            help,
        }
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the error span in the source code.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Get the label describing the error.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Get the help text, if any
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }
}

/// A type alias for `Result<T, DiagnosticError>`
pub type Result<T> = std::result::Result<T, DiagnosticError>;
