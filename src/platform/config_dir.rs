//! Where the app's configuration directory lives.

use std::path::PathBuf;

/// The directory the app's configuration lives under.
pub fn get_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    if super::is_wsl() {
        return wsl_host_config_dir();
    }

    dirs::config_dir()
}

/// The Windows host user's roaming application data directory.
#[cfg(target_os = "linux")]
fn wsl_host_config_dir() -> Option<PathBuf> {
    use super::wsl;

    let path = wsl::query_hkcu(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders",
        "AppData",
    )?;
    wsl::mounted_path(&path)
}
