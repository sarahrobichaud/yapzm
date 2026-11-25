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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh library directory, unique per test name.
    fn scratch_library(name: &str) -> PathBuf {
        let library =
            std::env::temp_dir().join(format!("yapzm-workshop-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&library);
        library
    }

    #[test]
    fn locate_finds_the_content_directory() {
        let library = scratch_library("locate");
        let content = content_dir(&library);
        std::fs::create_dir_all(&content).unwrap();

        let location = locate_in_libraries(std::slice::from_ref(&library)).unwrap();
        assert_eq!(location.path, content);

        let _ = std::fs::remove_dir_all(&library);
    }

    #[test]
    fn locate_skips_libraries_without_workshop_content() {
        let library = scratch_library("empty");
        std::fs::create_dir_all(&library).unwrap();

        assert!(locate_in_libraries(std::slice::from_ref(&library)).is_err());

        let _ = std::fs::remove_dir_all(&library);
    }

    #[test]
    fn validate_requires_the_workshop_record() {
        let library = scratch_library("record");
        let content = content_dir(&library);
        std::fs::create_dir_all(&content).unwrap();
        let location = locate_in_libraries(std::slice::from_ref(&library)).unwrap();

        assert!(Workshop::validate(&location).is_err());

        let record = library.join(format!("steamapps/workshop/appworkshop_{APP_ID}.acf"));
        std::fs::write(record, "").unwrap();
        assert_eq!(Workshop::validate(&location).unwrap().path(), content);

        let _ = std::fs::remove_dir_all(&library);
    }
}
