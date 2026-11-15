use std::io;
use std::path::PathBuf;

use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use super::{SteamInstall, SteamKind, validate};

/// Locates Steam via the configured override, its registry key, then default paths.
pub fn locate_steam() -> io::Result<Option<SteamInstall>> {
    if let Some(root) = super::steam_override_root() {
        return Ok(validate(&root, SteamKind::Native));
    }
    if let Some(root) = registry_root() {
        if let Some(install) = validate(&root, SteamKind::Native) {
            return Ok(Some(install));
        }
    }
    for root in candidate_roots() {
        if let Some(install) = validate(&root, SteamKind::Native) {
            return Ok(Some(install));
        }
    }
    Ok(None)
}

/// Reads the install path Steam publishes under its own registry key.
fn registry_root() -> Option<PathBuf> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path.replace('/', "\\")))
}

/// Probes the default install directories for a Steam folder.
fn candidate_roots() -> Vec<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(|var| std::env::var_os(var))
        .map(|dir| PathBuf::from(dir).join("Steam"))
        .collect()
}

/// Probes the locations steamcmd occupies on a Windows system.
pub fn steamcmd_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for name in ["steamcmd.exe", "steamcmd"] {
        if let Some(path) = super::path_lookup(name) {
            candidates.push(path);
        }
    }
    for letter in b'A'..=b'Z' {
        candidates.push(PathBuf::from(format!(
            "{}:\\steamcmd\\steamcmd.exe",
            letter as char
        )));
    }
    candidates
}
