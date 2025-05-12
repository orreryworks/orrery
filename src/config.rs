use crate::{
    ast::LayoutEngine,
    error::{ConfigError, FilamentError},
};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Application configuration loaded from TOML file
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    /// Layout configuration section
    #[serde(default)]
    pub layout: LayoutConfig,

    #[serde(skip)]
    config_file_path: Option<PathBuf>,
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

impl AppConfig {
    /// Load configuration from a TOML file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, FilamentError> {
        let path = path.as_ref();

        // Check if file exists
        if !path.exists() {
            return Err(FilamentError::Config(ConfigError::MissingFile(
                path.to_path_buf(),
            )));
        }

        // Read file content
        let content = fs::read_to_string(path)?;

        // Parse TOML content directly using serde
        let mut config: AppConfig = toml::from_str(&content)
            .map_err(ConfigError::from)
            .map_err(FilamentError::Config)?;

        // Store the config file path for potential future use
        config.config_file_path = Some(path.to_path_buf());

        // No need to validate layout engines as they're now enum values
        Ok(config)
    }
}
