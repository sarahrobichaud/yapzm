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

    #[test]
    fn extracts_paths_from_a_library_index() {
        let index = vdf::parse(
            r#"
"libraryfolders"
{
	"0" { "path" "C:\\Program Files (x86)\\Steam" }
	"1" { "path" "D:\\SteamLibrary" }
}
"#,
        )
        .unwrap();

        let paths = library_paths(&index);

        assert_eq!(
            paths,
            vec![
                PathBuf::from(r"C:\Program Files (x86)\Steam"),
                PathBuf::from(r"D:\SteamLibrary"),
            ]
        );
    }

    #[test]
    fn returns_nothing_for_an_unrelated_document() {
        let index = vdf::parse("\"something\"\n{\n\"else\"\t\"here\"\n}").unwrap();
        assert!(library_paths(&index).is_empty());
    }
}
