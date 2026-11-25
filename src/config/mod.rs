//! User configuration, merged from the config file and environment variables.

mod env;
mod file;
mod setting;

use std::path::{Path, PathBuf};

use crate::platform;

pub use file::{ConfigFile, ConfigFileError, ConfigPath};
pub use setting::{Origin, Setting};

/// The merged configuration, where environment variables override the config file.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    steam_path: Option<Setting<PathBuf>>,
    steamcmd_path: Option<Setting<PathBuf>>,
}

/// Whether the app is set up to run.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigState {
    /// No config file exists yet.
    SetupRequired,
    /// The config file exists and has been merged with the environment.
    Ready(Config),
}

impl Config {
    /// Reads the config file and merges the environment over it.
    pub fn resolve() -> Result<ConfigState, ConfigFileError> {
        let Some(file) = file::read()? else {
            return Ok(ConfigState::SetupRequired);
        };

        Ok(ConfigState::Ready(Config {
            steam_path: path_setting(file.steam_path, env::STEAM_DIR),
            steamcmd_path: path_setting(file.steamcmd_path, env::STEAMCMD),
        }))
    }

    /// The configured Steam install root.
    pub fn steam_path(&self) -> Option<&Setting<PathBuf>> {
        self.steam_path.as_ref()
    }

    /// The configured steamcmd executable.
    pub fn steamcmd_path(&self) -> Option<&Setting<PathBuf>> {
        self.steamcmd_path.as_ref()
    }
}

/// Records the steamcmd executable in the config file, returning where the file lives.
pub fn save_steamcmd_path(path: &Path) -> Result<ConfigPath, ConfigFileError> {
    let stored = platform::to_windows_notation(path);
    file::update(|file| file.steamcmd_path = Some(stored))
}

/// Merges a path from the config file with its environment variable.
fn path_setting(file_path: Option<PathBuf>, env_key: &'static str) -> Option<Setting<PathBuf>> {
    // The file is shared with Windows, so its paths use Windows notation even under WSL.
    let file_path = file_path.map(|path| platform::from_windows_notation(&path));
    setting::layer(file_path, env::path(env_key))
}
