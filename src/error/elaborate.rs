use crate::ast::span::Spanned;
use miette::{Diagnostic, SourceSpan};
use nom_locate::LocatedSpan;
use thiserror::Error;

/// A rich diagnostic error for elaboration issues in the Filament language.
///
/// This error provides detailed diagnostic information including:
/// - The source code where the error occurred
/// - A precise location (span) in the source
/// - A descriptive message and label
/// - Optional help text with suggestions to fix the error
///
/// These rich errors are displayed using miette's pretty error formatting.
#[derive(Debug, Error, Diagnostic)]
#[error("Elaboration error: {message}")]
pub struct ElaborationDiagnosticError {
    /// The source code being elaborated
    #[source_code]
    src: String,

    /// Error message to display
    message: String,

    /// The error span in the source
    #[label("{label}")]
    span: SourceSpan,

    /// Label for the error span
    label: String,

    /// Optional help text
    #[help]
    help: Option<String>,
}

impl ElaborationDiagnosticError {
    /// Create a new elaboration error from a nom_locate::LocatedSpan and source.
    ///
    /// This is the legacy method that works with the original parsing infrastructure.
    pub fn new(
        message: String,
        span: LocatedSpan<&str>,
        src: &str,
        label: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        let offset = span.location_offset();
        let length = span.fragment().len();

        ElaborationDiagnosticError {
            src: src.to_string(),
            message,
            span: (offset, length).into(),
            label: label.into(),
            help,
        }
    }

    /// Create a new elaboration error from a Spanned value.
    ///
    /// This method works with the newer Spanned<T> type that can wrap any AST element
    /// while preserving source location information.
    pub fn from_spanned<T>(
        message: String,
        spanned: &Spanned<T>,
        src: &str,
        label: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        ElaborationDiagnosticError {
            src: src.to_string(),
            message,
            span: (spanned.offset(), spanned.length()).into(),
            label: label.into(),
            help,
        }
    }

    /// Create a new elaboration error with manually provided position information.
    ///
    /// This is useful when you need to construct an error for a specific location
    /// that isn't associated with a Spanned or LocatedSpan value.
    pub fn with_position(
        message: String,
        offset: usize,
        length: usize,
        src: &str,
        label: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        ElaborationDiagnosticError {
            src: src.to_string(),
            message,
            span: (offset, length).into(),
            label: label.into(),
            help,
        }
    }
}
