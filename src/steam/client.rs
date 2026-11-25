//! The Steam client and the libraries it manages.

use std::path::{Path, PathBuf};

use super::libraries;
use crate::components::{Component, Location, Missing, Requirement, Source};
use crate::platform::{self, SteamKind};

/// A Steam install with at least one library.
#[derive(Debug)]
pub struct SteamClient {
    root: PathBuf,
    kind: SteamKind,
    libraries: Vec<PathBuf>,
}

impl SteamClient {
    /// The install root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The packaging format the install came from.
    pub fn kind(&self) -> SteamKind {
        self.kind
    }

    /// The library roots to search for game content.
    pub fn libraries(&self) -> &[PathBuf] {
        &self.libraries
    }
}

impl Component for SteamClient {
    const NAME: &'static str = "Steam";

    type Hint<'a> = Option<&'a Path>;
    type Metadata = SteamKind;

    /// Uses only the configured override when it is set, otherwise the first candidate that looks like a Steam root.
    fn locate(steam_dir: Option<&Path>) -> Result<Location<SteamKind>, Missing> {
        if let Some(dir) = steam_dir {
            let path = dir.to_path_buf();
            if !path.exists() {
                return Err(Missing {
                    searched: vec![path],
                });
            }
            return Ok(Location {
                metadata: platform::steam_kind(&path),
                path,
                source: Source::Override,
            });
        }

        let candidates = platform::steam_candidates();
        match candidates.iter().find(|root| looks_like_steam_root(root)) {
            Some(root) => Ok(Location {
                path: root.to_owned(),
                source: Source::Search,
                metadata: platform::steam_kind(root),
            }),
            None => Err(Missing {
                searched: candidates,
            }),
        }
    }

    /// Requires the root to list at least one library.
    fn validate(location: &Location<SteamKind>) -> Result<Self, Vec<Requirement>> {
        let unmet = || {
            vec![Requirement {
                name: "a Steam library (libraryfolders.vdf)",
            }]
        };

        let root = platform::canonical_path(&location.path).map_err(|_| unmet())?;
        let libraries: Vec<PathBuf> = libraries::load(&root)
            .into_iter()
            .map(|path| platform::from_windows_notation(&path))
            .collect();

        if libraries.is_empty() {
            return Err(unmet());
        }

        Ok(Self {
            root,
            kind: location.metadata,
            libraries,
        })
    }
}

/// Whether a candidate holds Steam's app directory, so empty leftovers are skipped.
fn looks_like_steam_root(root: &Path) -> bool {
    root.join("steamapps").is_dir()
}
