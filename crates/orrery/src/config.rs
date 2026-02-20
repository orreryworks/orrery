//! Configuration types for Orrery diagram rendering.
//!
//! This module provides configuration structures that control how diagrams
//! are laid out and styled. All types implement [`serde::Deserialize`] for
//! flexible loading from external sources.
//!
//! # Overview
//!
//! - [`AppConfig`] - Top-level application configuration combining layout and style settings.
//! - [`LayoutConfig`] - Controls which [`LayoutEngine`] is used for each diagram type.
//! - [`StyleConfig`] - Controls visual styling options such as background color.
//!
//! # Example
//!
//! ```
//! # use orrery::config::AppConfig;
//! // Use default configuration
//! let config = AppConfig::default();
//! assert!(config.style().background_color().is_ok());
//! ```

use serde::Deserialize;

use orrery_core::{color::Color, semantic::LayoutEngine};

/// Top-level application configuration combining layout and style settings.
///
/// Groups [`LayoutConfig`] and [`StyleConfig`] into a single configuration
/// root.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    /// Layout configuration section.
    #[serde(default)]
    layout: LayoutConfig,

    /// Style configuration section.
    #[serde(default)]
    style: StyleConfig,
}

impl AppConfig {
    /// Creates a new [`AppConfig`] with the specified layout and style configurations.
    ///
    /// # Arguments
    ///
    /// * `layout` - Layout engine settings for diagram types.
    /// * `style` - Visual styling options.
    pub fn new(layout: LayoutConfig, style: StyleConfig) -> Self {
        Self { layout, style }
    }

    /// Returns the layout configuration.
    pub fn layout(&self) -> &LayoutConfig {
        &self.layout
    }

    /// Returns the style configuration.
    pub fn style(&self) -> &StyleConfig {
        &self.style
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

/// Visual styling configuration for rendered diagrams.
///
/// Controls appearance options such as background color. Fields that are
/// not set fall back to renderer defaults.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct StyleConfig {
    /// Default background [`Color`] for diagrams, as a color string.
    #[serde(default)]
    background_color: Option<String>,
}

impl StyleConfig {
    /// Returns the parsed background [`Color`], or `None` if no color is configured.
    ///
    /// # Errors
    ///
    /// Returns an error if the configured color string cannot be parsed
    /// into a valid [`Color`].
    pub fn background_color(&self) -> Result<Option<Color>, String> {
        self.background_color
            .as_ref()
            .map(|color| Color::new(color))
            .transpose()
            .map_err(|err| format!("Invalid background color in config: {err}"))
    }
}
