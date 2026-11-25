//! Everything a command can use, gathered once at startup.

use crate::components::{Component, Detection};
use crate::config::{Config, Setting};
use crate::steam::SteamClient;
use crate::steamcmd::Steamcmd;
use crate::store::Store;
use crate::zomboid::{Install, Workshop};

/// The configuration, the store, and the detected components a command works with.
pub struct Context {
    pub config: Config,
    /// Always usable, since commands only run once setup has created it.
    pub store: Store,
    pub components: Components,
}

impl Context {
    /// Detects every component the configuration points to.
    pub fn detect(config: Config, store: Store) -> Self {
        let components = Components::detect(&config);
        Self {
            config,
            store,
            components,
        }
    }
}

/// The external components found on this machine.
pub struct Components {
    steam: Detection<SteamClient>,
    steamcmd: Detection<Steamcmd>,
    /// Found through Steam, so `None` when Steam is not ready.
    workshop: Option<Detection<Workshop>>,
    /// Found through Steam, so `None` when Steam is not ready.
    install: Option<Detection<Install>>,
}

impl Components {
    /// Detects each component, starting from the configured overrides.
    pub fn detect(config: &Config) -> Self {
        let steam = SteamClient::detect(config.steam_path().map(Setting::as_path));
        let steamcmd = Steamcmd::detect(config.steamcmd_path().map(Setting::as_path));
        let workshop = steam.as_ready().map(Workshop::detect);
        let install = steam.as_ready().map(Install::detect);
        Self {
            steam,
            steamcmd,
            workshop,
            install,
        }
    }

    // Ready components, for commands that only need what is usable.

    /// The Steam client, when it is ready to use.
    pub fn steam(&self) -> Option<&SteamClient> {
        self.steam.as_ready()
    }

    /// The steamcmd executable, when it is ready to use.
    pub fn steamcmd(&self) -> Option<&Steamcmd> {
        self.steamcmd.as_ready()
    }

    /// The workshop content, when it is ready to use.
    pub fn workshop(&self) -> Option<&Workshop> {
        self.workshop.as_ref().and_then(Detection::as_ready)
    }

    /// The game install, when it is ready to use.
    pub fn install(&self) -> Option<&Install> {
        self.install.as_ref().and_then(Detection::as_ready)
    }

    // Full detections, for commands that explain why a component is unusable.

    /// How detecting the Steam client went.
    pub fn steam_detection(&self) -> &Detection<SteamClient> {
        &self.steam
    }

    /// How detecting steamcmd went.
    pub fn steamcmd_detection(&self) -> &Detection<Steamcmd> {
        &self.steamcmd
    }

    /// How detecting the workshop content went, or `None` when Steam is not ready.
    pub fn workshop_detection(&self) -> Option<&Detection<Workshop>> {
        self.workshop.as_ref()
    }

    /// How detecting the game install went, or `None` when Steam is not ready.
    pub fn install_detection(&self) -> Option<&Detection<Install>> {
        self.install.as_ref()
    }
}
