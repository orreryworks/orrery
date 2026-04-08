//! Export functionality for Orrery diagrams.
//!
//! This module provides the [`Exporter`] trait that defines the interface for
//! converting laid-out diagrams into output formats. It is the final stage in
//! the Orrery processing pipeline.
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
//! errors. [`Error`] converts into [`RenderError::Export`] at the crate
//! boundary.
//!
//! [`RenderError::Export`]: crate::RenderError::Export

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
/// This type is converted into [`RenderError::Export`] at the crate
/// boundary via the [`From`] implementation in [`crate::error`].
///
/// [`RenderError::Export`]: crate::RenderError::Export
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A rendering or conversion failure described by `message`.
    #[error("Render error: {0}")]
    Render(String),
    /// An I/O error encountered while writing output.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
