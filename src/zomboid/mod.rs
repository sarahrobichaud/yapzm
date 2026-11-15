//! Project Zomboid installs and game versions.

pub mod install;
pub mod version;

pub use install::Install;
pub use version::{GameVersion, VersionError};
