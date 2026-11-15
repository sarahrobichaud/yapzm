//! The Steam Workshop content directory for Project Zomboid.

use std::path::{Path, PathBuf};

use super::install::APP_ID;

/// A Steam library directory holding Project Zomboid workshop content.
pub struct Workshop {
    pub path: PathBuf,
}

impl Workshop {
    /// Finds the first Steam library holding Project Zomboid workshop content.
    pub fn detect_steam(libraries: &[PathBuf]) -> Option<Self> {
        libraries
            .iter()
            .find_map(|candidate| Self::in_steam_library(candidate))
    }

    /// Reads the workshop out of a Steam library's content directory.
    fn in_steam_library(library: &Path) -> Option<Self> {
        let path = library
            .join("steamapps")
            .join("workshop")
            .join("content")
            .join(APP_ID);

        if !path.is_dir() {
            return None;
        }

        Some(Self { path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_steam_finds_the_content_directory() {
        let library =
            std::env::temp_dir().join(format!("yapzm-workshop-detect-{}", std::process::id()));
        let content = library.join("steamapps/workshop/content").join(APP_ID);
        std::fs::create_dir_all(&content).unwrap();

        let workshop = Workshop::detect_steam(std::slice::from_ref(&library)).unwrap();
        assert_eq!(workshop.path, content);

        let _ = std::fs::remove_dir_all(&library);
    }

    #[test]
    fn detect_steam_skips_libraries_without_workshop_content() {
        let library =
            std::env::temp_dir().join(format!("yapzm-workshop-empty-{}", std::process::id()));
        std::fs::create_dir_all(&library).unwrap();

        assert!(Workshop::detect_steam(std::slice::from_ref(&library)).is_none());

        let _ = std::fs::remove_dir_all(&library);
    }
}
