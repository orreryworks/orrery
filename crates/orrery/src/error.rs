//! Error types for Orrery operations.
//!
//! This module provides the main error type [`OrreryError`] which wraps
//! various error conditions that can occur during diagram processing.

use std::io;

use thiserror::Error;

use orrery_parser::error::ParseError;

/// The main error type for Orrery operations.
///
/// # Diagnostic Variants
///
/// The `Parse` variant contains structured error information with source code
/// spans. This provides detailed error information that can be used for rich
/// error reporting.
#[derive(Debug, Error)]
pub enum OrreryError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("{err}")]
    Parse { err: ParseError, src: String },

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Layout error: {0}")]
    Layout(String),

    #[error("Export error: {0}")]
    Export(Box<dyn std::error::Error>),
}

impl From<crate::export::Error> for OrreryError {
    fn from(error: crate::export::Error) -> Self {
        Self::Export(Box::new(error))
    }
}

impl OrreryError {
    /// Create a new `Parse` error with the associated source code.
    pub fn new_parse_error(err: ParseError, src: impl Into<String>) -> Self {
        Self::Parse {
            err,
            src: src.into(),
        }
    }
}
