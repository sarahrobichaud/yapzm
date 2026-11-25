//! Where the config file lives.

use std::path::{Path, PathBuf};

use super::ConfigFileError;
use crate::config::env;
use crate::platform;

/// The config file's name inside its directory.
const FILE_NAME: &str = "config.json";

/// The config file's path, and whether a custom directory chose it.
#[derive(Clone, Debug)]
pub struct ConfigPath {
    path: PathBuf,
    is_custom: bool,
}

impl ConfigPath {
    /// The file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the path comes from a custom directory rather than the platform default.
    pub fn is_custom(&self) -> bool {
        self.is_custom
    }
}

/// Resolves the config file path, honoring a custom directory from the environment.
pub(super) fn resolve() -> Result<ConfigPath, ConfigFileError> {
    if let Some((_, dir)) = env::path(env::CONFIG_DIR) {
        let dir = std::path::absolute(dir)
            .map_err(|source| ConfigFileError::InvalidCustomDir { source })?;
        return Ok(ConfigPath {
            path: dir.join(FILE_NAME),
            is_custom: true,
        });
    }

    let config_dir = platform::get_config_dir().ok_or(ConfigFileError::NoConfigDir)?;
    Ok(ConfigPath {
        path: config_dir.join("yapzm").join(FILE_NAME),
        is_custom: false,
    })
}
