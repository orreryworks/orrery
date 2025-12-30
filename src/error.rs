use std::{io, path::PathBuf};

use miette::Diagnostic;
use thiserror::Error;

use diagnostic::DiagnosticError;

pub mod diagnostic;

/// The main error type for Filament operations
#[derive(Debug, Error)]
pub enum FilamentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("{err}")]
    LexerDiagnostic { err: DiagnosticError, src: String },

    #[error("{err}")]
    ParseDiagnostic { err: DiagnosticError, src: String },

    #[error("{err}")]
    ElaborationDiagnostic { err: DiagnosticError, src: String },

    #[error("{err}")]
    ValidationDiagnostic { err: DiagnosticError, src: String },

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
            Self::LexerDiagnostic { .. }
            | Self::ParseDiagnostic { .. }
            | Self::ElaborationDiagnostic { .. }
            | Self::ValidationDiagnostic { .. } => {
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
            Self::LexerDiagnostic { err: source, .. } => source.help(),
            Self::ParseDiagnostic { err: source, .. } => source.help(),
            Self::ElaborationDiagnostic { err: source, .. } => source.help(),
            Self::ValidationDiagnostic { err: source, .. } => source.help(),
            _ => None, // Other errors don't have specific help
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match self {
            Self::LexerDiagnostic { src, .. } => Some(src),
            Self::ParseDiagnostic { src, .. } => Some(src),
            Self::ElaborationDiagnostic { src, .. } => Some(src),
            Self::ValidationDiagnostic { src, .. } => Some(src),
            _ => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        match self {
            Self::LexerDiagnostic { err, .. } => err.labels(),
            Self::ParseDiagnostic { err, .. } => err.labels(),
            Self::ElaborationDiagnostic { err, .. } => err.labels(),
            Self::ValidationDiagnostic { err, .. } => err.labels(),
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
    /// Create a new `LexerDiagnostic` error with the associated source code.
    pub fn new_lexer_error(err: DiagnosticError, src: impl Into<String>) -> Self {
        Self::LexerDiagnostic {
            err,
            src: src.into(),
        }
    }

    /// Create a new `ElaborationDiagnostic` error with the associated source code.
    pub fn new_elaboration_error(err: DiagnosticError, src: impl Into<String>) -> Self {
        Self::ElaborationDiagnostic {
            err,
            src: src.into(),
        }
    }

    /// Create a new `ValidationDiagnostic` error with the associated source code.
    pub fn new_validation_error(err: DiagnosticError, src: impl Into<String>) -> Self {
        Self::ValidationDiagnostic {
            err,
            src: src.into(),
        }
    }

    /// Create a new `ParseDiagnostic` error with the associated source code.
    pub fn new_parse_error(err: DiagnosticError, src: impl Into<String>) -> Self {
        Self::ParseDiagnostic {
            err,
            src: src.into(),
        }
    }
}
