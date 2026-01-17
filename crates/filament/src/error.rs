//! Error types for Filament operations.
//!
//! This module provides the main error type [`FilamentError`] which wraps
//! various error conditions that can occur during diagram processing.

use std::io;

use thiserror::Error;

use filament_parser::DiagnosticError;

/// The main error type for Filament operations.
///
/// # Diagnostic Variants
///
/// The `LexerDiagnostic`, `ParseDiagnostic`, `ElaborationDiagnostic`, and
/// `ValidationDiagnostic` variants contain structured error information with
/// source code spans. These provide detailed error information that can be
/// used for rich error reporting.
#[derive(Debug, Error)]
pub enum FilamentError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("{err}")]
    ParseDiagnostic { err: DiagnosticError, src: String },

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Layout error: {0}")]
    Layout(String),

    #[error("Export error: {0}")]
    Export(Box<dyn std::error::Error>),
}

impl From<crate::export::Error> for FilamentError {
    fn from(error: crate::export::Error) -> Self {
        Self::Export(Box::new(error))
    }
}

impl FilamentError {
    /// Create a new `ParseDiagnostic` error with the associated source code.
    pub fn new_parse_error(err: DiagnosticError, src: impl Into<String>) -> Self {
        Self::ParseDiagnostic {
            err,
            src: src.into(),
        }
    }
}
