//! The SteamCMD executable used for downloads and account management.

use std::path::{Path, PathBuf};

use crate::components::{Component, Location, Missing, Requirement, Source};
use crate::platform;

mod install;

pub use install::{InstallError, InstallOptions};

/// A steamcmd executable known to exist.
#[derive(Debug)]
pub struct Steamcmd {
    path: PathBuf,
}

impl Steamcmd {
    /// The path to the executable.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Component for Steamcmd {
    const NAME: &'static str = "SteamCMD";

    type Hint<'a> = Option<&'a Path>;
    type Metadata = ();

    /// Uses only the configured override when it is set, otherwise the first candidate that exists.
    fn locate(steamcmd: Option<&Path>) -> Result<Location<()>, Missing> {
        if let Some(path) = steamcmd {
            let path = path.to_path_buf();
            if !path.exists() {
                return Err(Missing {
                    searched: vec![path],
                });
            }
            return Ok(Location {
                path,
                source: Source::Override,
                metadata: (),
            });
        }

        let candidates = platform::steamcmd_candidates();
        match candidates.iter().find(|path| path.exists()) {
            Some(path) => Ok(Location {
                path: path.to_owned(),
                source: Source::Search,
                metadata: (),
            }),
            None => Err(Missing {
                searched: candidates,
            }),
        }
    }

    /// Requires the location to be a file.
    fn validate(location: &Location<()>) -> Result<Self, Vec<Requirement>> {
        if !location.path.is_file() {
            return Err(vec![Requirement {
                name: "steamcmd executable",
            }]);
        }
        Ok(Self {
            path: location.path.clone(),
        })
    }
}
