//! The detected Steam client and its libraries.

use std::path::PathBuf;

use super::libraries;
use crate::platform;

pub struct SteamClient {
    libraries: Vec<PathBuf>,
}

impl SteamClient {
    /// Detects the Steam install, requiring a readable library index.
    pub fn detect() -> Option<Self> {
        let install = platform::locate_steam().ok()??;

        let libraries: Vec<PathBuf> = libraries::load(&install.root)
            .into_iter()
            .map(|path| platform::translate_windows_path(&path))
            .collect();

        if libraries.is_empty() {
            return None;
        }

        Some(Self { libraries })
    }

    pub fn game_install_location_candidates(&self) -> &[PathBuf] {
        &self.libraries
    }
}
