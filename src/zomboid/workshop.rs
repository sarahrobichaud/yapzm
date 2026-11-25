//! The Steam Workshop content directory for Project Zomboid.

use std::io;
use std::path::{Path, PathBuf};

use super::install::APP_ID;
use crate::components::{Component, Location, Missing, Requirement, Source};
use crate::steam::{SteamClient, vdf};

/// A Steam library's Project Zomboid workshop content that Steam keeps a record of.
#[derive(Debug)]
pub struct Workshop {
    path: PathBuf,
}

/// One installed workshop item, at the version Steam last downloaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkshopItem {
    pub id: u64,
    /// When the item was last updated on the workshop, which identifies its version.
    pub time_updated: u64,
    /// The folder holding the item's content.
    pub path: PathBuf,
}

/// The ways reading Steam's workshop record can fail.
#[derive(Debug, thiserror::Error)]
pub enum WorkshopRecordError {
    /// The record could not be read.
    #[error("reading {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The record is not valid VDF.
    #[error("parsing {path}")]
    Parse {
        path: PathBuf,
        #[source]
        source: vdf::ParseError,
    },
}

impl Workshop {
    /// The directory holding one folder per downloaded workshop item.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The installed items Steam's record lists that have content on disk.
    pub fn items(&self) -> Result<Vec<WorkshopItem>, WorkshopRecordError> {
        let path = record_path(&self.path).unwrap_or_default();
        let text = std::fs::read_to_string(&path).map_err(|source| WorkshopRecordError::Read {
            path: path.clone(),
            source,
        })?;
        let record =
            vdf::parse(&text).map_err(|source| WorkshopRecordError::Parse { path, source })?;
        Ok(installed_items(&record, &self.path))
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
        let has_record = record_path(&location.path).is_some_and(|path| path.is_file());

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

/// Steam's record of the installed workshop items for a content directory.
fn record_path(content_dir: &Path) -> Option<PathBuf> {
    workshop_dir(content_dir).map(|dir| dir.join(format!("appworkshop_{APP_ID}.acf")))
}

/// The record's installed items, skipping entries that are malformed or have no content folder.
fn installed_items(record: &vdf::Value, content_dir: &Path) -> Vec<WorkshopItem> {
    let Some(entries) = record
        .as_map()
        .and_then(|pairs| pairs.first())
        .and_then(|(_, root)| root.get("WorkshopItemsInstalled"))
        .and_then(vdf::Value::as_map)
    else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|(id, entry)| {
            let id: u64 = id.parse().ok()?;
            let time_updated = entry.get("timeupdated")?.as_str()?.parse().ok()?;
            let path = content_dir.join(id.to_string());
            path.is_dir().then_some(WorkshopItem {
                id,
                time_updated,
                path,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;

    #[test]
    fn installed_items_with_content_and_a_version_are_listed() {
        let scratch = Scratch::new();
        let content = scratch.dir("steamapps/workshop/content/108600");
        scratch.dir("steamapps/workshop/content/108600/111/mods");
        scratch.dir("steamapps/workshop/content/108600/333");
        scratch.file(
            "steamapps/workshop/appworkshop_108600.acf",
            r#"
"AppWorkshop"
{
    "WorkshopItemsInstalled"
    {
        "111" { "timeupdated" "1700000000" }
        "222" { "timeupdated" "1700000001" }
        "333" { "size" "10" }
    }
}
"#,
        );
        let workshop = Workshop { path: content };

        let items = workshop.items().unwrap();

        assert_eq!(
            items,
            [WorkshopItem {
                id: 111,
                time_updated: 1_700_000_000,
                path: scratch.path("steamapps/workshop/content/108600/111"),
            }]
        );
    }
}
