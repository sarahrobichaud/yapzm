//! The Steam Workshop content directory for Project Zomboid.

use std::path::{Path, PathBuf};

use super::install::APP_ID;
use crate::components::{Component, Location, Missing, Requirement, Source};
use crate::steam::SteamClient;

/// A Steam library's Project Zomboid workshop content that Steam keeps a record of.
#[derive(Debug)]
pub struct Workshop {
    path: PathBuf,
}

impl Workshop {
    /// The directory holding one folder per downloaded workshop item.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Component for Workshop {
    const NAME: &'static str = "Steam Workshop";

    type Hint<'a> = &'a SteamClient;
    type Metadata = ();

    /// Finds the first Steam library holding Project Zomboid workshop content.
    fn locate(steam: &SteamClient) -> Result<Location<()>, Missing> {
        locate_in_libraries(steam.libraries())
    }

    /// Requires Steam's record of the installed workshop items next to the content.
    fn validate(location: &Location<()>) -> Result<Self, Vec<Requirement>> {
        let has_record = workshop_dir(&location.path)
            .is_some_and(|dir| dir.join(format!("appworkshop_{APP_ID}.acf")).is_file());

        if !has_record {
            return Err(vec![Requirement {
                name: "Steam's workshop record (appworkshop.acf)",
            }]);
        }
        Ok(Self {
            path: location.path.clone(),
        })
    }
}

/// Finds the first library holding a Project Zomboid workshop content directory.
fn locate_in_libraries(libraries: &[PathBuf]) -> Result<Location<()>, Missing> {
    let candidates: Vec<PathBuf> = libraries
        .iter()
        .map(|library| content_dir(library))
        .collect();
    match candidates.iter().find(|path| path.is_dir()) {
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

/// The Project Zomboid workshop content directory inside a library.
fn content_dir(library: &Path) -> PathBuf {
    library
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join(APP_ID)
}

/// The workshop directory a content directory sits two levels under.
fn workshop_dir(content_dir: &Path) -> Option<&Path> {
    content_dir.parent()?.parent()
}
