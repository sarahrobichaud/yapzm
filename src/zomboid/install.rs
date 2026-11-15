//! A Project Zomboid game install and its layout.

use std::path::{Path, PathBuf};

use crate::steam::{SteamClient, vdf};

/// Project Zomboid's Steam app ID.
pub const APP_ID: &str = "108600";

/// The game jar every install ships.
const GAME_JAR: &str = "projectzomboid.jar";

/// The bundled JVM executables across install layouts.
const JAVA_EXECUTABLES: &[&str] = &["jre64/bin/java.exe", "jre64/bin/java"];

/// One requirement of a runnable install, and whether it is met.
pub struct Requirement {
    /// The name of the required file or component.
    pub name: &'static str,
    /// Whether the requirement is satisfied.
    pub met: bool,
}

/// A Project Zomboid game install rooted at its top-level directory.
pub struct Install {
    pub path: PathBuf,
}

impl Install {
    /// Detects the install on this machine through the local Steam client.
    pub fn detect() -> Option<Self> {
        let steam = SteamClient::detect()?;
        Self::detect_steam(steam.game_install_location_candidates())
    }

    /// Finds the first Steam library holding a Project Zomboid install.
    pub fn detect_steam(libraries: &[PathBuf]) -> Option<Self> {
        libraries
            .iter()
            .find_map(|candidate| Self::in_steam_library(candidate))
    }

    /// The requirements of a runnable install, with their state.
    pub fn requirements(&self) -> Vec<Requirement> {
        vec![
            Requirement {
                name: GAME_JAR,
                met: self.path.join(GAME_JAR).is_file(),
            },
            Requirement {
                name: "bundled JVM (jre64/bin)",
                met: java_path(&self.path).is_some(),
            },
        ]
    }

    /// Whether every requirement of a runnable install is met.
    pub fn valid(&self) -> bool {
        self.requirements()
            .iter()
            .all(|requirement| requirement.met)
    }

    /// Reads the install out of a Steam library's app manifest.
    fn in_steam_library(library: &Path) -> Option<Self> {
        let manifest_path = library
            .join("steamapps")
            .join(format!("appmanifest_{APP_ID}.acf"));

        let manifest = std::fs::read_to_string(manifest_path).ok()?;

        let index = vdf::parse(&manifest).ok()?;
        let state = index.get("AppState")?;

        let install_dir = state.get("installdir")?.as_str()?;
        let path = library.join("steamapps").join("common").join(install_dir);

        if !path.exists() {
            return None;
        }

        Some(Self { path })
    }
}

/// The bundled JVM executable inside a game directory, if present.
pub(crate) fn java_path(game_dir: &Path) -> Option<PathBuf> {
    JAVA_EXECUTABLES
        .iter()
        .map(|executable| game_dir.join(executable))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_steam_reads_the_install_dir_from_the_manifest() {
        let library =
            std::env::temp_dir().join(format!("yapzm-install-detect-{}", std::process::id()));
        let steamapps = library.join("steamapps");
        std::fs::create_dir_all(steamapps.join("common").join("ProjectZomboid")).unwrap();
        std::fs::write(
            steamapps.join(format!("appmanifest_{APP_ID}.acf")),
            r#""AppState"
{
	"installdir"		"ProjectZomboid"
}
"#,
        )
        .unwrap();

        let install = Install::detect_steam(&[library.clone()]).unwrap();
        assert_eq!(
            install.path,
            steamapps.join("common").join("ProjectZomboid")
        );

        let _ = std::fs::remove_dir_all(&library);
    }

    #[test]
    fn requirements_fail_in_an_empty_directory() {
        let dir =
            std::env::temp_dir().join(format!("yapzm-install-req-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let install = Install { path: dir.clone() };
        assert!(
            install
                .requirements()
                .iter()
                .all(|requirement| !requirement.met)
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn requirements_pass_when_the_files_exist() {
        let dir =
            std::env::temp_dir().join(format!("yapzm-install-req-full-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("jre64/bin")).unwrap();
        std::fs::write(dir.join("projectzomboid.jar"), "").unwrap();
        std::fs::write(dir.join("jre64/bin/java"), "").unwrap();

        let install = Install { path: dir.clone() };
        assert!(
            install
                .requirements()
                .iter()
                .all(|requirement| requirement.met)
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn java_path_finds_the_bundled_executable() {
        let dir = std::env::temp_dir().join(format!("yapzm-install-java-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("jre64/bin")).unwrap();
        std::fs::write(dir.join("jre64/bin/java"), "").unwrap();

        assert_eq!(java_path(&dir), Some(dir.join("jre64/bin/java")));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
