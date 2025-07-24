use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// A rich diagnostic error for parsing
#[derive(Debug, Error, Diagnostic)]
#[error("Parse error: {message}")]
pub struct ParseDiagnosticError {
    /// The source code being parsed
    #[source_code]
    pub src: String,

    /// Error message to display
    pub message: String,

    /// The error span in the source
    #[label("here")]
    pub span: Option<SourceSpan>,

    /// Optional help text
    #[help]
    pub help: Option<String>,
}
