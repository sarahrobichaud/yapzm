//! Where Steam installs live and which packaging format they came from.

use std::path::{Path, PathBuf};

/// The packaging format a Steam installation came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SteamKind {
    /// Installed by the Steam installer or a system package manager.
    Native,
    /// Installed as a Flatpak sandbox.
    Flatpak,
    /// Installed as a Snap sandbox.
    Snap,
}

/// The Steam install roots to probe, most likely first, or only the Windows host's under WSL.
pub fn steam_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    if super::is_wsl() {
        return wsl_host_roots();
    }

    native_roots()
}

/// Classifies the packaging format a Steam install root belongs to.
pub fn steam_kind(root: &Path) -> SteamKind {
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

/// The install roots Steam uses across Linux packaging formats and macOS.
#[cfg(unix)]
fn native_roots() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    vec![
        // Linux: the symlink Steam maintains itself, then the default data dir
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        // Sandboxed installs keep their data inside their own home
        home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
        home.join("snap/steam/common/.steam/steam"),
        // macOS: user data lives here regardless of where the bundle was dragged
        home.join("Library/Application Support/Steam"),
    ]
}

/// The registry's install path, then the default install directories.
#[cfg(windows)]
fn native_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = windows_registry_root().into_iter().collect();
    roots.extend(
        ["ProgramFiles(x86)", "ProgramFiles"]
            .iter()
            .filter_map(|var| std::env::var_os(var))
            .map(|dir| PathBuf::from(dir).join("Steam")),
    );
    roots
}

/// The install path Steam publishes under its own registry key.
#[cfg(windows)]
fn windows_registry_root() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path.replace('/', "\\")))
}

/// The Windows host's registry install path, then its default install directories.
#[cfg(target_os = "linux")]
fn wsl_host_roots() -> Vec<PathBuf> {
    use super::wsl;

    let mut roots: Vec<PathBuf> = wsl::query_hkcu(r"HKCU\Software\Valve\Steam", "SteamPath")
        .and_then(|path| wsl::mounted_path(&path))
        .into_iter()
        .collect();

    let c_drive = wsl::mount_root().join("c");
    roots.extend(
        ["Program Files (x86)", "Program Files"]
            .iter()
            .map(|dir| c_drive.join(dir).join("Steam")),
    );
    roots
}
