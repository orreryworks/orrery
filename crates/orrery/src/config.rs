//! Configuration controlling how diagrams are laid out.
//!
//! All types implement [`serde::Deserialize`] for loading from external sources.

use serde::Deserialize;

use orrery_core::semantic::LayoutEngine;

/// Top-level application configuration.
///
/// Wraps the [`LayoutConfig`] that controls layout-engine selection.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    layout: LayoutConfig,
}

impl AppConfig {
    /// Creates an [`AppConfig`] with the given layout configuration.
    pub fn new(layout: LayoutConfig) -> Self {
        Self { layout }
    }

    /// Returns the layout configuration.
    pub fn layout(&self) -> &LayoutConfig {
        &self.layout
    }
}

/// Layout engine configuration for different diagram types.
///
/// Controls which [`LayoutEngine`] variant is used for each diagram type.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct LayoutConfig {
    /// Default [`LayoutEngine`] for component diagrams.
    #[serde(default)]
    component: LayoutEngine,

    /// Default [`LayoutEngine`] for sequence diagrams.
    #[serde(default)]
    sequence: LayoutEngine,
}

impl LayoutConfig {
    /// Creates a new [`LayoutConfig`] with the specified layout engines.
    ///
    /// # Arguments
    ///
    /// * `component` - Layout engine for component diagrams.
    /// * `sequence` - Layout engine for sequence diagrams.
    pub fn new(component: LayoutEngine, sequence: LayoutEngine) -> Self {
        Self {
            component,
            sequence,
        }
    }

    /// Returns the [`LayoutEngine`] for component diagrams.
    pub fn component(&self) -> LayoutEngine {
        self.component
    }

    /// Returns the [`LayoutEngine`] for sequence diagrams.
    pub fn sequence(&self) -> LayoutEngine {
        self.sequence
    }
}
