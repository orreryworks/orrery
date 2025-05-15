use crate::{
    ast::LayoutEngine,
    color::Color,
    error::{ConfigError, FilamentError},
};
use directories::ProjectDirs;
use log::{debug, info};
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

    /// Style configuration section
    #[serde(default)]
    pub style: StyleConfig,

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

/// Style configuration section
#[derive(Debug, Default, Clone, Deserialize)]
pub struct StyleConfig {
    /// Default background color for diagrams
    #[serde(default)]
    background_color: Option<String>,
}

impl AppConfig {
    pub fn find_and_load(explicit_path: Option<impl AsRef<Path>>) -> Result<Self, FilamentError> {
        // 1. Try the explicitly provided path first if available
        if let Some(path) = explicit_path {
            let path = path.as_ref();
            info!(path = path.display().to_string(); "Loading configuration from explicit path");
            return Self::load(path);
        }

        // 2. Try the local project directory
        let local_config = Path::new("filament/config.toml");
        if local_config.exists() {
            info!(path = local_config.display().to_string(); "Loading configuration from local path");
            return Self::load(local_config);
        }

        // 3. Try the platform-specific config directory
        if let Some(proj_dirs) = ProjectDirs::from("com", "filament", "filament") {
            let config_dir = proj_dirs.config_dir();
            let system_config = config_dir.join("config.toml");

            if system_config.exists() {
                info!(path = system_config.display().to_string(); "Loading configuration from system path");
                return Self::load(system_config);
            }

            debug!(path = system_config.display().to_string(); "System configuration file not found");
        } else {
            debug!("Could not determine platform-specific config directory");
        }

        // 4. If no config is found, return default config
        debug!("No configuration file found, using default configuration");
        Ok(AppConfig::default())
    }

    /// Load configuration from a TOML file
    fn load(path: impl AsRef<Path>) -> Result<Self, FilamentError> {
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

impl StyleConfig {
    /// Get the background color from configuration
    /// Returns None if no background color is configured
    pub fn background_color(&self) -> Result<Option<Color>, ConfigError> {
        self.background_color
            .as_ref()
            .map(|color| Color::new(color))
            .transpose()
            .map_err(|err| {
                ConfigError::Validation(format!("Invalid background color in config: {err}"))
            })
    }
}
