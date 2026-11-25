//! Where steamcmd executables live.

use std::path::PathBuf;

use super::path_lookup;

/// The steamcmd executables to probe, or only the Windows host's under WSL.
pub fn steamcmd_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    if super::is_wsl() {
        return wsl_host_candidates();
    }

    native_candidates()
}

/// The executable on `PATH`, then the conventional install in the home directory.
#[cfg(unix)]
fn native_candidates() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = path_lookup("steamcmd").into_iter().collect();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("steamcmd/steamcmd"));
    }
    candidates
}

/// The executable on `PATH`, then a manual install at the root of each local fixed drive.
#[cfg(windows)]
fn native_candidates() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = ["steamcmd.exe", "steamcmd"]
        .into_iter()
        .filter_map(path_lookup)
        .collect();
    candidates.extend(
        local_fixed_drives()
            .into_iter()
            .map(|root| root.join("steamcmd").join("steamcmd.exe")),
    );
    candidates
}

/// The roots of the drives that exist and are local fixed disks.
#[cfg(windows)]
fn local_fixed_drives() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;

    // SAFETY: takes no arguments and returns a bitmask of present drive letters.
    let present_drives = unsafe { GetLogicalDrives() };
    (b'A'..=b'Z')
        .enumerate()
        .filter(|(index, _)| present_drives & (1 << index) != 0)
        .map(|(_, letter)| format!("{}:\\", letter as char))
        .filter(|root| {
            let wide_root: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: the pointer refers to a NUL-terminated UTF-16 buffer that outlives the call.
            unsafe { GetDriveTypeW(wide_root.as_ptr()) == DRIVE_FIXED }
        })
        .map(PathBuf::from)
        .collect()
}

/// The host's executable through interop, then a manual install on each mounted drive.
#[cfg(target_os = "linux")]
fn wsl_host_candidates() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = path_lookup("steamcmd.exe").into_iter().collect();
    candidates.extend(
        super::wsl::drive_mounts()
            .into_iter()
            .map(|mount| mount.join("steamcmd/steamcmd.exe")),
    );
    candidates
}

/// Where steamcmd installs by default, on the platforms that support installing it.
pub fn steamcmd_install_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        Some(PathBuf::from(r"C:\steamcmd"))
    }
    #[cfg(target_os = "linux")]
    {
        super::is_wsl().then(|| super::wsl::mount_root().join("c").join("steamcmd"))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}
