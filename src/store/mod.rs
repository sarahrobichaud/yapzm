//! The local store holding the app's own copies of mods.

mod workshop;

use std::io;
use std::path::{Path, PathBuf};

use crate::components::{
    Component, Location, Missing, Provision, Provisioned, Requirement, Source,
};

pub use workshop::{CopyOutcome, CopyProgress};

/// The store directory's name inside the data directory.
const STORE_DIR: &str = ".store";

/// The directory holding workshop items inside the store.
const WORKSHOP_DIR: &str = "workshop";

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

    /// Requires the directory for workshop items.
    fn validate(location: &Location<()>) -> Result<Self, Vec<Requirement>> {
        if !location.path.join(WORKSHOP_DIR).is_dir() {
            return Err(vec![Requirement {
                name: "workshop directory",
            }]);
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
        let workshop_dir = root.join(WORKSHOP_DIR);
        std::fs::create_dir_all(&workshop_dir).map_err(|source| StoreError::Write {
            path: workshop_dir,
            source,
        })?;
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
        assert!(matches!(
            Store::provision(scratch.root()),
            Ok(Provisioned::AlreadyPresent(_))
        ));
    }
}
