//! Per-platform knowledge of where components live and how to read paths on this system.

mod config_dir;
mod data_dir;
mod paths;
mod steam;
mod steamcmd;

#[cfg(target_os = "linux")]
mod wsl;

pub use config_dir::get_config_dir;
pub use data_dir::get_data_dir;
pub(crate) use paths::path_lookup;
pub use paths::{canonical_path, from_windows_notation, to_windows_notation};
pub use steam::{SteamKind, steam_candidates, steam_kind};
pub use steamcmd::{steamcmd_candidates, steamcmd_install_dir};

/// Reports whether this process runs inside WSL.
pub fn is_wsl() -> bool {
    #[cfg(target_os = "linux")]
    {
        wsl::is_wsl()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
