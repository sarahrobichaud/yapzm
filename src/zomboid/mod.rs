use std::path::{Path, PathBuf};

use crate::steam::vdf;

pub const APP_ID: &str = "108600";

pub struct Install {
    pub path: PathBuf,
}

impl Install {
    pub fn detect_steam(libraries: &[PathBuf]) -> Option<Self> {
        libraries
            .iter()
            .find_map(|candidate| Self::in_steam_library(candidate))
    }

    fn in_steam_library(library: &Path) -> Option<Self> {

        let manifest_path = library
            .join("steamapps")
            .join(format!("appmanifest_{APP_ID}.acf"));


        let manifest = std::fs::read_to_string(manifest_path).ok()?;


        let index = vdf::parse(&manifest).ok()?;
        let state = index.get("AppState")?;

        let install_dir = state.get("installdir")?.as_str()?;
        let path = library.join("steamapps").join("common").join(install_dir);

        if !path.exists() {
            return None;
        }

        Some(Self { path })
    }
}
