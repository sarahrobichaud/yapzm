//! Helpers for reading and resolving paths on this system.

use std::io;
use std::path::{Path, PathBuf};

/// Maps a stored path in Windows notation, such as from Steam or the config file, onto this system; never for user input.
pub fn from_windows_notation(path: &Path) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        super::wsl::from_windows_notation(path)
    }
    #[cfg(not(target_os = "linux"))]
    {
        path.to_path_buf()
    }
}

/// Maps a path on this system into Windows notation for storing alongside Windows-notation paths; never for display.
pub fn to_windows_notation(path: &Path) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        super::wsl::to_windows_notation(path)
    }
    #[cfg(not(target_os = "linux"))]
    {
        path.to_path_buf()
    }
}

/// Resolves an existing path to its canonical form, in a shape other tools accept.
pub fn canonical_path(path: &Path) -> io::Result<PathBuf> {
    let canonical = path.canonicalize()?;
    Ok(strip_verbatim(&canonical))
}

/// Removes the extended-length prefix canonicalize adds on Windows drive paths.
fn strip_verbatim(path: &Path) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path.to_path_buf();
    };
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.chars().nth(1) == Some(':') => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Searches `PATH` for an executable name, returning the first file match.
pub(crate) fn path_lookup(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}
