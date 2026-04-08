//! Error types for the render pipeline.
//!
//! This module provides the error type [`RenderError`] for the render pipeline
//! (graph construction, layout, and export).

use std::io;

use thiserror::Error;

/// The main error type for Orrery runtime operations.
///
/// This covers I/O, graph construction, layout, and export errors.
#[derive(Debug, Error)]
pub enum RenderError {
    /// An I/O error from file operations.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// A graph construction error.
    #[error("Graph error: {0}")]
    Graph(String),

    /// A layout calculation error.
    #[error("Layout error: {0}")]
    Layout(String),

    /// An export/rendering error from the output backend.
    #[error("Export error: {0}")]
    Export(Box<dyn std::error::Error>),
}

impl From<crate::export::Error> for RenderError {
    fn from(error: crate::export::Error) -> Self {
        Self::Export(Box::new(error))
    }
}
