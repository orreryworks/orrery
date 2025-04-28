mod parser;
mod elaborate;

use miette::Diagnostic;
pub use parser::{ParseDiagnosticError, SlimParserError};
pub use elaborate::ElaborationDiagnosticError;
use std::io;
use thiserror::Error;

/// The main error type for Filament operations
#[derive(Debug, Error, Diagnostic)]
pub enum FilamentError {
    #[error("I/O error: {0}")]
    #[diagnostic(code(filament::error::io))]
    Io(#[from] io::Error),

    /// For simple parsing errors (will be deprecated)
    #[error("Parse error: {0}")]
    #[diagnostic(code(filament::error::parser))]
    Parse(String),

    /// For rich diagnostic parsing errors
    #[error(transparent)]
    #[diagnostic(code(filament::error::parser_diagnostic))]
    ParseDiagnostic(#[from] ParseDiagnosticError),
    
    /// For simple elaboration errors (will be deprecated)
    #[error("Elaboration error: {0}")]
    #[diagnostic(code(filament::error::elaboration))]
    Elaboration(String),
    
    /// For rich diagnostic elaboration errors
    #[error(transparent)]
    #[diagnostic(code(filament::error::elaboration_diagnostic))]
    ElaborationDiagnostic(#[from] ElaborationDiagnosticError),

    #[error("Graph error: {0}")]
    #[diagnostic(code(filament::error::graph))]
    Graph(String),

    #[error("Export error: {0}")]
    #[diagnostic(code(filament::error::export))]
    Export(Box<dyn std::error::Error>),
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
