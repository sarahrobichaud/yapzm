//! The config file: its format, reading it, creating it, and updating it.

mod location;

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::components::{Provision, Provisioned};

pub use location::ConfigPath;

/// The values the config file can set.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) steam_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) steamcmd_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) data_path: Option<PathBuf>,
}

/// The ways locating, reading, or creating the config file can fail.
#[derive(Debug, thiserror::Error)]
pub enum ConfigFileError {
    /// This system reports no directory for configuration.
    #[error("this system has no configuration directory")]
    NoConfigDir,

    /// The custom configuration directory cannot be made absolute.
    #[error("the custom configuration directory is not a valid path")]
    InvalidCustomDir {
        #[source]
        source: io::Error,
    },

    /// The file exists but could not be read.
    #[error("reading {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The file is not a valid configuration.
    #[error("parsing {path}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    /// The configuration could not be encoded as JSON.
    #[error("encoding the configuration for {path}")]
    Encode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    /// The file or its directory could not be written.
    #[error("writing {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Reads and parses the config file, or `None` when it does not exist yet.
pub(super) fn read() -> Result<Option<FileConfig>, ConfigFileError> {
    let location = location::resolve()?;
    let path = location.path();

    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigFileError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    serde_json::from_str(&text)
        .map(Some)
        .map_err(|source| ConfigFileError::Parse {
            path: path.to_path_buf(),
            source,
        })
}

/// Applies a change to the config file and writes it back atomically, creating it when missing.
pub(super) fn update(change: impl FnOnce(&mut FileConfig)) -> Result<ConfigPath, ConfigFileError> {
    let mut config = read()?.unwrap_or_default();
    change(&mut config);

    let location = location::resolve()?;
    let path = location.path();
    let encoded = serde_json::to_vec_pretty(&config).map_err(|source| ConfigFileError::Encode {
        path: path.to_path_buf(),
        source,
    })?;
    let write_error = |source: io::Error| ConfigFileError::Write {
        path: path.to_path_buf(),
        source,
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(write_error)?;
    }
    crate::json::write_atomic(path, &encoded).map_err(write_error)?;
    Ok(location)
}

/// The config file on disk.
#[derive(Debug)]
pub struct ConfigFile {
    location: ConfigPath,
}

impl ConfigFile {
    /// Where the file lives.
    pub fn location(&self) -> &ConfigPath {
        &self.location
    }
}

impl Provision for ConfigFile {
    type Options<'a> = ();
    type Error = ConfigFileError;

    /// Creates an empty config file, leaving an existing one untouched.
    fn provision(_: ()) -> Result<Provisioned<Self>, ConfigFileError> {
        let location = location::resolve()?;
        let path = location.path();
        let write_error = |source: io::Error| ConfigFileError::Write {
            path: path.to_path_buf(),
            source,
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(write_error)?;
        }

        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => {
                file.write_all(b"{}").map_err(write_error)?;
                Ok(Provisioned::Created(Self { location }))
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Ok(Provisioned::AlreadyPresent(Self { location }))
            }
            Err(error) => Err(write_error(error)),
        }
    }
}
