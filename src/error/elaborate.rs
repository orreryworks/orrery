use crate::ast::span::SpanImpl;
use miette::{Diagnostic, SourceSpan};
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
/// The source code itself is expected to be provided by the container error type (e.g., `FilamentError`).
#[derive(Debug, Error)]
#[error("{message}")]
pub struct ElaborationDiagnosticError {
    /// Error message to display
    message: String,

    /// The error span in the source
    span: SourceSpan,

    /// Label for the error span
    label: String,

    /// Optional help text
    help: Option<String>,
}

// We implement Diagnostic manually or via the containing error type,
// as #[source_code] is no longer here.
impl Diagnostic for ElaborationDiagnosticError {
    // We only define the parts miette can't get from the container
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

    // code(), severity(), url(), related() can use defaults or be customized if needed
    // code() will now come from the FilamentError wrapper
}

impl ElaborationDiagnosticError {
    /// Create a new elaboration error from a Span value.
    pub fn from_span(
        message: String,
        span: SpanImpl,
        label: impl Into<String>,
        help: Option<String>,
    ) -> Self {
        Self {
            message,
            span: (span.start, span.end - span.start).into(),
            label: label.into(),
            help,
        }
    }
}
