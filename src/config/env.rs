//! Configuration read from environment variables.

use std::ffi::OsString;
use std::path::PathBuf;

/// Overrides the Steam install root.
pub(super) const STEAM_DIR: &str = "YAPZM_STEAM_DIR";

/// Overrides the steamcmd executable.
pub(super) const STEAMCMD: &str = "YAPZM_STEAMCMD";

/// Overrides the directory holding the config file.
pub(super) const CONFIG_DIR: &str = "YAPZM_CONFIG_DIR";

/// A path from an environment variable, paired with its key; empty values count as unset.
pub(super) fn path(key: &'static str) -> Option<(&'static str, PathBuf)> {
    non_empty(std::env::var_os(key)).map(|value| (key, PathBuf::from(value)))
}

/// Treats an empty value as unset.
fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_values_count_as_unset() {
        assert_eq!(non_empty(Some(OsString::new())), None);
        assert_eq!(non_empty(None), None);
        assert_eq!(
            non_empty(Some(OsString::from("/x"))),
            Some(OsString::from("/x"))
        );
    }
}
