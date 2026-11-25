//! Where the app's local, non-roaming data directory lives.

use std::path::PathBuf;

/// The directory the app's local data lives under.
pub fn get_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    if super::is_wsl() {
        return wsl_host_data_dir();
    }

    dirs::data_local_dir()
}

/// The Windows host user's local application data directory.
#[cfg(target_os = "linux")]
fn wsl_host_data_dir() -> Option<PathBuf> {
    use super::wsl;

    let path = wsl::query_hkcu(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders",
        "Local AppData",
    )?;
    wsl::mounted_path(&path)
}
