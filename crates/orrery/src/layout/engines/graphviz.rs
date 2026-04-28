//! Graphviz-backed layout engines.
//!
//! This module hosts layout engines that delegate spatial positioning to
//! Graphviz. It is only compiled when the `graphviz` Cargo feature is
//! enabled; with the feature disabled these engines are not available and
//! the build does not pull in any Graphviz-related dependencies.
//!
//! # Overview
//!
//! - [`Component`] - Graphviz-based layout engine for component diagrams.

mod component;

pub use component::Engine as Component;
