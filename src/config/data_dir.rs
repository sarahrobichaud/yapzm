//! Where the app's local data directory lives.

use std::path::{Path, PathBuf};

use super::{ConfigError, Origin, env, setting};
use crate::platform;

/// The data directory's path, and where it was configured when not the platform default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataDir {
    path: PathBuf,
    origin: Option<Origin>,
}

impl DataDir {
    /// The directory's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where the directory was configured, or `None` for the platform default.
    pub fn origin(&self) -> Option<Origin> {
        self.origin
    }
}

/// Resolves the data directory from the environment, then the config file, then the platform default.
pub(super) fn resolve(file_path: Option<PathBuf>) -> Result<DataDir, ConfigError> {
    // The file is shared with Windows, so its paths use Windows notation even under WSL.
    let file_path = file_path.map(|path| platform::from_windows_notation(&path));

    if let Some(configured) = setting::layer(file_path, env::path(env::DATA_DIR)) {
        let origin = configured.origin();
        let path = std::path::absolute(configured.as_path())
            .map_err(|source| ConfigError::InvalidDataDir { source })?;
        return Ok(DataDir {
            path,
            origin: Some(origin),
        });
    }

    let data_dir = platform::get_data_dir().ok_or(ConfigError::NoDataDir)?;
    Ok(DataDir {
        path: data_dir.join("yapzm"),
        origin: None,
    })
}
