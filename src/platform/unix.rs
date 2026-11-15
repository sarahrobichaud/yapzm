use std::io;
use std::path::{Path, PathBuf};

use super::{SteamInstall, SteamKind, validate};

/// Locates Steam via the configured override, then the paths Steam occupies by default.
pub fn locate_steam() -> io::Result<Option<SteamInstall>> {
    if let Some(root) = super::steam_override_root() {
        return Ok(validate(&root, classify(&root)));
    }
    for root in candidate_roots() {
        if let Some(install) = validate(&root, classify(&root)) {
            return Ok(Some(install));
        }
    }
    wsl_fallback()
}

/// Returns the install roots Steam uses across Linux packaging formats and macOS.
fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Linux: the symlink Steam maintains itself, then the default data dir
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".local/share/Steam"));
        // Sandboxed installs keep their data inside their own home
        roots.push(home.join(".var/app/com.valvesoftware.Steam/data/Steam"));
        roots.push(home.join("snap/steam/common/.steam/steam"));
        // macOS: user data lives here regardless of where the bundle was dragged
        roots.push(home.join("Library/Application Support/Steam"));
    }
    roots
}

/// Classifies the packaging format an install root belongs to.
fn classify(root: &Path) -> SteamKind {
    let Some(home) = dirs::home_dir() else {
        return SteamKind::Native;
    };
    if root.starts_with(home.join(".var/app")) {
        return SteamKind::Flatpak;
    }
    if root.starts_with(home.join("snap")) {
        return SteamKind::Snap;
    }
    SteamKind::Native
}

/// Falls back to the Windows install when running inside WSL.
#[cfg(target_os = "linux")]
fn wsl_fallback() -> io::Result<Option<SteamInstall>> {
    if !super::wsl::is_wsl() {
        return Ok(None);
    }
    Ok(super::wsl::locate_windows_steam())
}

/// WSL detection does not apply outside Linux.
#[cfg(not(target_os = "linux"))]
fn wsl_fallback() -> io::Result<Option<SteamInstall>> {
    Ok(None)
}

/// Probes the locations steamcmd occupies, or the Windows host's under WSL.
pub fn steamcmd_candidates() -> Vec<PathBuf> {
    if super::wsl::is_wsl() {
        return wsl_steamcmd_candidates();
    }
    let mut candidates = Vec::new();
    if let Some(path) = super::path_lookup("steamcmd") {
        candidates.push(path);
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("steamcmd/steamcmd"));
    }
    candidates
}

/// Probes the Windows host's steamcmd through interop and the drive mounts.
fn wsl_steamcmd_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = super::path_lookup("steamcmd.exe") {
        candidates.push(path);
    }
    candidates.extend(super::wsl::drive_candidates());
    candidates
}
