//! Configuration file loading for the CLI
//!
//! This module handles finding and loading TOML configuration files
//! from various locations (explicit path, local directory, system directory).

use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use log::{debug, info};
use thiserror::Error;

use filament::{FilamentError, config::AppConfig};

/// Configuration-related errors for CLI
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to parse TOML configuration: {0}")]
    Parse(String),

    #[error("Missing configuration file: {0}")]
    MissingFile(PathBuf),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<ConfigError> for FilamentError {
    fn from(err: ConfigError) -> Self {
        FilamentError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        ))
    }
}

/// Find and load configuration from various locations
///
/// Search order:
/// 1. Explicit path if provided
/// 2. Local project directory (filament/config.toml)
/// 3. Platform-specific config directory
/// 4. Default config if none found
///
/// # Arguments
///
/// * `explicit_path` - Optional explicit path to config file
///
/// # Errors
///
/// Returns error if:
/// - Explicit path is provided but file doesn't exist
/// - Config file exists but cannot be parsed
pub fn load_config(explicit_path: Option<impl AsRef<Path>>) -> Result<AppConfig, FilamentError> {
    // 1. Try the explicitly provided path first if available
    if let Some(path) = explicit_path {
        let path = path.as_ref();
        info!(path = path.display().to_string(); "Loading configuration from explicit path");
        return load_config_file(path);
    }

    // 2. Try the local project directory
    let local_config = Path::new("filament/config.toml");
    if local_config.exists() {
        info!(path = local_config.display().to_string(); "Loading configuration from local path");
        return load_config_file(local_config);
    }

    // 3. Try the platform-specific config directory
    if let Some(proj_dirs) = ProjectDirs::from("com", "filament", "filament") {
        let config_dir = proj_dirs.config_dir();
        let system_config = config_dir.join("config.toml");

        if system_config.exists() {
            info!(path = system_config.display().to_string(); "Loading configuration from system path");
            return load_config_file(system_config);
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
///
/// # Arguments
///
/// * `path` - Path to the TOML configuration file
///
/// # Errors
///
/// Returns error if:
/// - File doesn't exist
/// - File cannot be read
/// - TOML parsing fails
fn load_config_file(path: impl AsRef<Path>) -> Result<AppConfig, FilamentError> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(ConfigError::MissingFile(path.to_path_buf()).into());
    }

    // Read file content
    let content = fs::read_to_string(path)?;

    // Parse TOML content
    let config: AppConfig =
        toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;

    Ok(config)
}
