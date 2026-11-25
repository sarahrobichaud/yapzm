//! Reads Steam's library index from an install root.

use std::path::{Path, PathBuf};

use super::vdf;

/// Location of the library index within an install root.
const LIBRARY_FOLDERS_VDF: &str = "config/libraryfolders.vdf";

/// Returns the library directories listed in the install's index, empty when unreadable.
pub fn load(root: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(root.join(LIBRARY_FOLDERS_VDF)) else {
        return Vec::new();
    };
    let Ok(index) = vdf::parse(&text) else {
        return Vec::new();
    };
    library_paths(&index)
}

/// Extracts each folder's `path` entry from a parsed library index.
fn library_paths(index: &vdf::Value) -> Vec<PathBuf> {
    let Some(folders) = index.get("libraryfolders") else {
        return Vec::new();
    };
    let Some(entries) = folders.as_map() else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|(_, folder)| folder.get("path"))
        .filter_map(|path| path.as_str())
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;

    #[test]
    fn every_library_in_the_index_is_listed() {
        let steam = Scratch::new();
        steam.file(
            LIBRARY_FOLDERS_VDF,
            r#"
"libraryfolders"
{
    "0" { "path" "C:\\Program Files (x86)\\Steam" }
    "1" { "path" "D:\\SteamLibrary" }
}
"#,
        );

        assert_eq!(
            load(steam.root()),
            [
                PathBuf::from(r"C:\Program Files (x86)\Steam"),
                PathBuf::from(r"D:\SteamLibrary"),
            ]
        );
    }
}
