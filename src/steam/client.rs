//! The detected Steam client and its libraries.

use super::libraries;
use crate::{ClientPaths, platform};

/// The Steam game client.
pub struct SteamClient {
    pub paths: ClientPaths,
}

impl SteamClient {
    /// Detects the Steam install, requiring a readable library index.
    pub fn detect() -> Option<Self> {
        let install = platform::locate_steam().ok()??;
        let libraries = libraries::load(&install.root);

        if libraries.is_empty() {
            return None;
        }

        Some(Self {
            paths: ClientPaths {
                root: install.root,
                libraries,
            },
        })
    }
}
