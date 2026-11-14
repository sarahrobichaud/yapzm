//! Steam client implementation of the game client interface.

use super::libraries;
use crate::{ClientPaths, GameClient, platform};

/// The Steam game client
pub struct SteamClient {
    pub paths: ClientPaths,
}

impl GameClient for SteamClient {
    /// Detects the Steam install, requiring a readable library index.
    fn detect() -> Option<Self> {
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

    /// The client's display name.
    fn name(&self) -> &str {
        "Steam"
    }

    /// The paths belonging to this client.
    fn paths(&self) -> &ClientPaths {
        &self.paths
    }
}
