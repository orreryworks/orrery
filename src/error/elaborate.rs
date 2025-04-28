use miette::{Diagnostic, SourceSpan};
use nom_locate::LocatedSpan;
use thiserror::Error;

/// A rich diagnostic error for elaboration issues
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
    /// Create a new elaboration error from a span and source
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

    /// Create a new elaboration error from a manual offset and length
    pub fn with_span(
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
