use miette::{Diagnostic, SourceSpan};
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FilamentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Parse error")]
    ParseDiagnostic(#[from] ParseDiagnosticError),

    #[error("Elaboration error: {0}")]
    Elaboration(String),

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Export error: {0}")]
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
