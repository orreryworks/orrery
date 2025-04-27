use miette::{Diagnostic, SourceSpan};
use std::io;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum FilamentError {
    #[error("I/O error: {0}")]
    #[diagnostic(code(filament::error::io))]
    Io(#[from] io::Error),

    // TODO: Ideally, this should be deprecated.
    #[error("Parse error: {0}")]
    #[diagnostic(code(filament::error::parser))]
    Parse(String),

    #[error(transparent)]
    #[diagnostic(code(filament::error::parser_diagnostic))]
    ParseDiagnostic(#[from] ParseDiagnosticError),

    #[error("Elaboration error: {0}")]
    #[diagnostic(code(filament::error::elaboration))]
    Elaboration(String),

    #[error("Graph error: {0}")]
    #[diagnostic(code(filament::error::graph))]
    Graph(String),

    #[error("Export error: {0}")]
    #[diagnostic(code(filament::error::export))]
    Export(Box<dyn std::error::Error>),
}

#[derive(Debug, Error, Diagnostic)]
#[error("Parse error: {message}")]
pub struct ParseDiagnosticError {
    #[source_code]
    pub src: String,

    pub message: String,

    #[label("here")]
    pub span: Option<SourceSpan>,

    #[help]
    pub help: Option<String>,
}

impl From<String> for FilamentError {
    fn from(error: String) -> Self {
        FilamentError::Parse(error)
    }
}

impl From<crate::export::Error> for FilamentError {
    fn from(error: crate::export::Error) -> Self {
        FilamentError::Export(Box::new(error))
    }
}
