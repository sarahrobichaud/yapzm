//! Locates the SteamCMD executable for downloads and account management.

use std::path::PathBuf;

use crate::{config, platform};

/// Locates steamcmd, using only the configured override when it is set.
pub fn locate() -> Option<PathBuf> {
    if let Some(path) = config::steamcmd() {
        let path = platform::translate_windows_path(&path);
        return path.is_file().then_some(path);
    }
    platform::steamcmd_candidates()
        .into_iter()
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_an_installed_steamcmd() {
        let Some(program) = locate() else {
            eprintln!("no steamcmd install visible; skipping");
            return;
        };
        assert!(program.is_file());
    }
}
