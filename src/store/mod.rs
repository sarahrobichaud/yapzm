//! The local store holding the app's own copies of mods and game versions.

mod copy;
mod game;
mod workshop;

use std::io;
use std::path::{Path, PathBuf};

use crate::components::{
    Component, Location, Missing, Provision, Provisioned, Requirement, Source,
};

pub use copy::CopyProgress;
pub use game::GameCopyOutcome;
pub use workshop::WorkshopCopyOutcome;

/// The store directory's name inside the data directory.
const STORE_DIR: &str = ".store";

/// The directory holding workshop items inside the store.
const WORKSHOP_DIR: &str = "workshop";

/// The directory holding game versions inside the store.
const GAME_DIR: &str = "game";

/// The directories every store holds.
const LAYOUT: [(&str, &str); 2] = [
    (WORKSHOP_DIR, "workshop directory"),
    (GAME_DIR, "game directory"),
];

/// A store on disk with its expected layout.
#[derive(Debug)]
pub struct Store {
    root: PathBuf,
}

/// The ways creating or filling the store can fail.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A file or directory could not be read.
    #[error("reading {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A file or directory could not be written.
    #[error("writing {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A game version cannot safely name a folder in the store.
    #[error("the game version {0:?} cannot name a store folder")]
    InvalidVersion(String),
}

impl Store {
    /// The store directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory holding one folder per stored workshop item.
    pub fn workshop_dir(&self) -> PathBuf {
        self.root.join(WORKSHOP_DIR)
    }

    /// The directory holding one stored version of a workshop item.
    pub fn workshop_version_dir(&self, id: u64, time_updated: u64) -> PathBuf {
        self.workshop_dir()
            .join(id.to_string())
            .join(time_updated.to_string())
    }

    /// The directory holding one folder per stored game version.
    pub fn game_dir(&self) -> PathBuf {
        self.root.join(GAME_DIR)
    }

    /// The directory holding the files of one stored game version.
    pub fn game_version_dir(&self, version: &str) -> PathBuf {
        self.game_dir().join(version)
    }
}

impl Component for Store {
    const NAME: &'static str = "Store";

    type Hint<'a> = &'a Path;
    type Metadata = ();

    /// Finds the store directory inside the data directory.
    fn locate(data_dir: &Path) -> Result<Location<()>, Missing> {
        let path = data_dir.join(STORE_DIR);
        if !path.is_dir() {
            return Err(Missing {
                searched: vec![path],
            });
        }
        Ok(Location {
            path,
            source: Source::Search,
            metadata: (),
        })
    }

    /// Requires every directory of the store layout.
    fn validate(location: &Location<()>) -> Result<Self, Vec<Requirement>> {
        let unmet: Vec<Requirement> = LAYOUT
            .iter()
            .filter(|(dir, _)| !location.path.join(dir).is_dir())
            .map(|&(_, name)| Requirement { name })
            .collect();
        if !unmet.is_empty() {
            return Err(unmet);
        }
        Ok(Self {
            root: location.path.clone(),
        })
    }
}

impl Provision for Store {
    type Options<'a> = &'a Path;
    type Error = StoreError;

    /// Creates the store layout inside the data directory, leaving an existing store untouched.
    fn provision(data_dir: &Path) -> Result<Provisioned<Self>, StoreError> {
        if let Some(existing) = Store::detect(data_dir).ready() {
            return Ok(Provisioned::AlreadyPresent(existing));
        }

        let root = data_dir.join(STORE_DIR);
        for (dir, _) in LAYOUT {
            let path = root.join(dir);
            std::fs::create_dir_all(&path).map_err(|source| StoreError::Write { path, source })?;
        }
        Ok(Provisioned::Created(Self { root }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;

    #[test]
    fn provisioning_creates_the_store_once() {
        let scratch = Scratch::new();

        assert!(matches!(
            Store::detect(scratch.root()),
            crate::components::Detection::Missing(_)
        ));
        assert!(matches!(
            Store::provision(scratch.root()),
            Ok(Provisioned::Created(_))
        ));
        assert!(scratch.path(".store/workshop").is_dir());
        assert!(scratch.path(".store/game").is_dir());
        assert!(matches!(
            Store::provision(scratch.root()),
            Ok(Provisioned::AlreadyPresent(_))
        ));
    }
}
