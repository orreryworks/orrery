mod elaborate;
mod parser;

pub use elaborate::ElaborationDiagnosticError;
use miette::Diagnostic;
pub use parser::{ParseDiagnosticError, SlimParserError};
use std::io;
use thiserror::Error;

/// The main error type for Filament operations
#[derive(Debug, Error)]
pub enum FilamentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// For rich diagnostic parsing errors
    #[error(transparent)]
    ParseDiagnostic(#[from] ParseDiagnosticError),

    /// For rich diagnostic elaboration errors - holds the source code too.
    #[error("{err}")] // Display the inner error's message
    ElaborationDiagnostic {
        #[source] // The actual error
        err: ElaborationDiagnosticError,
        src: String, // The source code for this error
    },

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Export error: {0}")]
    Export(Box<dyn std::error::Error>),
}

// Manual implementation of Diagnostic for FilamentError
impl Diagnostic for FilamentError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        match self {
            FilamentError::Io(_) => Some(Box::new("filament::error::io")),
            FilamentError::ParseDiagnostic(_) => {
                Some(Box::new("filament::error::parser_diagnostic"))
            }
            FilamentError::ElaborationDiagnostic { .. } => {
                Some(Box::new("filament::error::elaboration_diagnostic"))
            }
            FilamentError::Graph(_) => Some(Box::new("filament::error::graph")),
            FilamentError::Export(_) => Some(Box::new("filament::error::export")),
        }
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        match self {
            FilamentError::ElaborationDiagnostic { err: source, .. } => source.help(),
            FilamentError::ParseDiagnostic(e) => e.help(),
            _ => None, // Other errors don't have specific help
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match self {
            FilamentError::ParseDiagnostic(e) => Some(&e.src),
            FilamentError::ElaborationDiagnostic { src, .. } => Some(src),
            _ => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        match self {
            FilamentError::ParseDiagnostic(e) => e.labels(),
            FilamentError::ElaborationDiagnostic { err: source, .. } => source.labels(),
            _ => None,
        }
    }

    // You can add overrides for severity(), url(), related() if needed
}

impl From<crate::export::Error> for FilamentError {
    fn from(error: crate::export::Error) -> Self {
        FilamentError::Export(Box::new(error))
    }
}

impl FilamentError {
    /// Create a new ElaborationDiagnostic error with the associated source code.
    /// This provides a cleaner API than directly constructing the variant.
    pub fn new_elaboration_error(
        error: ElaborationDiagnosticError,
        source_code: impl Into<String>,
    ) -> Self {
        FilamentError::ElaborationDiagnostic {
            err: error,
            src: source_code.into(),
        }
    }
}
