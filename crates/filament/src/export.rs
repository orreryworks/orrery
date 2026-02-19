//! Export functionality for Filament diagrams.
//!
//! This module provides the [`Exporter`] trait that defines the interface for
//! converting laid-out diagrams into output formats. It is the final stage in
//! the Filament processing pipeline.
//!
//! # Pipeline Position
//!
//! ```text
//! Source Text
//!     ↓ parse
//! Semantic Model
//!     ↓ structure
//! Hierarchy Graph
//!     ↓ layout
//! Positioned Elements (LayeredLayout)
//!     ↓ export (this module)
//! Output File
//! ```
//!
//! # Available Backends
//!
//! - [`svg`] — SVG output via [`svg::SvgBuilder`] and [`svg::Svg`]
//!
//! # Error Handling
//!
//! Export operations return [`Error`], covering rendering failures and I/O
//! errors. [`Error`] converts into [`FilamentError::Export`] at the crate
//! boundary.
//!
//! [`FilamentError::Export`]: crate::FilamentError::Export

/// SVG export backend.
pub mod svg;

use crate::layout::layer::LayeredLayout;

/// Abstraction for diagram export backends.
///
/// Implementors convert a [`LayeredLayout`] into a specific output format
/// (e.g., SVG).
///
/// See the [`svg`] module for the built-in SVG implementation.
pub trait Exporter {
    /// Exports a layered layout to the backend's output format.
    ///
    /// A [`LayeredLayout`] contains positioned diagram elements organized into
    /// rendering layers. Each layer holds either component or sequence diagram
    /// content with absolute coordinates ready for output.
    ///
    /// # Arguments
    ///
    /// * `layout` - The positioned diagram layers to export.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Render`] if the layout cannot be converted to the
    /// target format, or [`Error::Io`] if writing the output fails.
    fn export_layered_layout(&mut self, layout: &LayeredLayout) -> Result<(), Error>;
}

/// Errors that can occur during diagram export.
///
/// This type is converted into [`FilamentError::Export`] at the crate
/// boundary via the [`From`] implementation in [`crate::error`].
///
/// [`FilamentError::Export`]: crate::FilamentError::Export
#[derive(Debug)]
pub enum Error {
    /// A rendering or conversion failure described by `message`.
    Render(String),
    /// An I/O error encountered while writing output.
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Render(msg) => write!(f, "Render error: {msg}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Render(_) => None,
            Self::Io(err) => Some(err),
        }
    }
}
