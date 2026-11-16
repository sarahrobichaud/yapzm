//! User configuration, resolved from the environment.

use std::path::PathBuf;

/// The registry source used when no override is configured.
const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/sarahrobichaud/yapzm/main/registry.json";

/// Steam install root to use in place of automatic discovery, when set.
pub fn steam_dir() -> Option<PathBuf> {
    std::env::var_os("YAPZM_STEAM_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// SteamCMD executable to use in place of automatic discovery, when set.
pub fn steamcmd() -> Option<PathBuf> {
    std::env::var_os("YAPZM_STEAMCMD")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// URL of the version registry.
pub fn registry_url() -> String {
    std::env::var("YAPZM_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_owned())
}
