mod elaborate;
mod parser;

pub use elaborate::ElaborationDiagnosticError;
use miette::Diagnostic;
pub use parser::ParseDiagnosticError;
use std::{io, path::PathBuf};
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
    #[error("{err}")] // Empty message to avoid duplication with inner error
    ElaborationDiagnostic {
        err: ElaborationDiagnosticError,
        src: String, // The source code for this error
    },

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Layout error: {0}")]
    Layout(String),

    #[error("Export error: {0}")]
    Export(Box<dyn std::error::Error>),

    #[error(transparent)]
    Config(#[from] ConfigError),
}

// Manual implementation of Diagnostic for FilamentError
impl Diagnostic for FilamentError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        match self {
            Self::Io(_) => Some(Box::new("filament::error::io")),
            Self::ParseDiagnostic(_) => Some(Box::new("filament::error::parser_diagnostic")),
            Self::ElaborationDiagnostic { .. } => {
                // Return None to suppress the error code in output
                None
            }
            Self::Graph(_) => Some(Box::new("filament::error::graph")),
            Self::Layout(_) => Some(Box::new("filament::error::layout")),
            Self::Export(_) => Some(Box::new("filament::error::export")),
            Self::Config(_) => Some(Box::new("filament::error::config")),
        }
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        match self {
            Self::ElaborationDiagnostic { err: source, .. } => source.help(),
            Self::ParseDiagnostic(e) => e.help(),
            _ => None, // Other errors don't have specific help
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match self {
            Self::ParseDiagnostic(e) => Some(&e.src),
            Self::ElaborationDiagnostic { src, .. } => Some(src),
            _ => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        match self {
            Self::ParseDiagnostic(e) => e.labels(),
            Self::ElaborationDiagnostic { err: source, .. } => source.labels(),
            _ => None,
        }
    }

    // You can add overrides for severity(), url(), related() if needed
}

impl From<crate::export::Error> for FilamentError {
    fn from(error: crate::export::Error) -> Self {
        Self::Export(Box::new(error))
    }
}

/// Specific configuration-related error types
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to parse TOML configuration: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Missing configuration file: {0}")]
    MissingFile(PathBuf),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl FilamentError {
    /// Create a new `ElaborationDiagnostic` error with the associated source code.
    /// This provides a cleaner API than directly constructing the variant.
    pub fn new_elaboration_error(
        error: ElaborationDiagnosticError,
        source_code: impl Into<String>,
    ) -> Self {
        Self::ElaborationDiagnostic {
            err: error,
            src: source_code.into(),
        }
    }
}
