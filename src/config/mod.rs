//! User configuration, merged from the config file and environment variables.

mod data_dir;
mod env;
mod file;
mod setting;

use std::io;
use std::path::{Path, PathBuf};

use crate::platform;

pub use data_dir::DataDir;
pub use file::{ConfigFile, ConfigFileError, ConfigPath};
pub use setting::{Origin, Setting};

/// The merged configuration, where environment variables override the config file.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    steam_path: Option<Setting<PathBuf>>,
    steamcmd_path: Option<Setting<PathBuf>>,
    data_dir: DataDir,
}

/// Whether the app is set up to run.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigState {
    /// No config file exists yet.
    SetupRequired,
    /// The config file exists and has been merged with the environment.
    Ready(Config),
}

/// The ways resolving the configuration can fail.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The config file could not be located or read.
    #[error(transparent)]
    File(#[from] ConfigFileError),

    /// This system reports no directory for local application data.
    #[error("this system has no local data directory")]
    NoDataDir,

    /// The configured data directory cannot be made absolute.
    #[error("the configured data directory is not a valid path")]
    InvalidDataDir {
        #[source]
        source: io::Error,
    },
}

impl Config {
    /// Reads the config file and merges the environment over it.
    pub fn resolve() -> Result<ConfigState, ConfigError> {
        let Some(file) = file::read()? else {
            return Ok(ConfigState::SetupRequired);
        };

        Ok(ConfigState::Ready(Config {
            steam_path: path_setting(file.steam_path, env::STEAM_DIR),
            steamcmd_path: path_setting(file.steamcmd_path, env::STEAMCMD),
            data_dir: data_dir::resolve(file.data_path)?,
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

    /// The directory holding the app's local data, such as the store.
    pub fn data_dir(&self) -> &DataDir {
        &self.data_dir
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
