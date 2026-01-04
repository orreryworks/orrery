use serde::Deserialize;

use crate::{color::Color, semantic::LayoutEngine};

/// Application configuration loaded from TOML file
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    /// Layout configuration section
    #[serde(default)]
    layout: LayoutConfig,

    /// Style configuration section
    #[serde(default)]
    style: StyleConfig,
}

impl AppConfig {
    /// Create a new AppConfig with the specified layout and style configurations
    pub fn new(layout: LayoutConfig, style: StyleConfig) -> Self {
        Self { layout, style }
    }

    /// Get the layout configuration
    pub fn layout(&self) -> &LayoutConfig {
        &self.layout
    }

    /// Get the style configuration
    pub fn style(&self) -> &StyleConfig {
        &self.style
    }
}

/// Layout configuration section
#[derive(Debug, Default, Clone, Deserialize)]
pub struct LayoutConfig {
    /// Default layout engine for component diagrams
    #[serde(default)]
    component: LayoutEngine,

    /// Default layout engine for sequence diagrams
    #[serde(default)]
    sequence: LayoutEngine,
}

impl LayoutConfig {
    /// Create a new LayoutConfig with the specified layout engines
    pub fn new(component: LayoutEngine, sequence: LayoutEngine) -> Self {
        Self {
            component,
            sequence,
        }
    }

    /// Get the layout engine for component diagrams
    pub fn component(&self) -> LayoutEngine {
        self.component
    }

    /// Get the layout engine for sequence diagrams
    pub fn sequence(&self) -> LayoutEngine {
        self.sequence
    }
}

/// Style configuration section
#[derive(Debug, Default, Clone, Deserialize)]
pub struct StyleConfig {
    /// Default background color for diagrams
    #[serde(default)]
    background_color: Option<String>,
}

impl StyleConfig {
    /// Get the background color from configuration
    /// Returns None if no background color is configured
    pub fn background_color(&self) -> Result<Option<Color>, String> {
        self.background_color
            .as_ref()
            .map(|color| Color::new(color))
            .transpose()
            .map_err(|err| format!("Invalid background color in config: {err}"))
    }
}
