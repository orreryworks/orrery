use serde::Deserialize;

use crate::{ast::LayoutEngine, color::Color};

/// Application configuration loaded from TOML file
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    /// Layout configuration section
    #[serde(default)]
    pub layout: LayoutConfig,

    /// Style configuration section
    #[serde(default)]
    pub style: StyleConfig,
}

/// Layout configuration section
#[derive(Debug, Default, Clone, Deserialize)]
pub struct LayoutConfig {
    /// Default layout engine for component diagrams
    #[serde(default)]
    pub component: LayoutEngine,

    /// Default layout engine for sequence diagrams
    #[serde(default)]
    pub sequence: LayoutEngine,
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
