//! A Project Zomboid game install and its layout.

use std::path::{Path, PathBuf};

use crate::components::{Component, Location, Missing, Requirement, Source};
use crate::steam::{SteamClient, vdf};

/// Project Zomboid's Steam app ID.
pub const APP_ID: &str = "108600";

/// The game jar every install ships.
const GAME_JAR: &str = "projectzomboid.jar";

/// The bundled JVM executables across install layouts.
const JAVA_EXECUTABLES: &[&str] = &["jre64/bin/java.exe", "jre64/bin/java"];

/// A runnable Project Zomboid game install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    path: PathBuf,
    java: PathBuf,
}

impl Install {
    /// The install's top-level directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The bundled JVM executable the game runs on.
    pub fn java(&self) -> &Path {
        &self.java
    }
}

impl Component for Install {
    const NAME: &'static str = "Project Zomboid";

    type Hint<'a> = &'a SteamClient;
    type Metadata = ();

    /// Finds the first Steam library whose app manifest points at an existing install.
    fn locate(steam: &SteamClient) -> Result<Location<()>, Missing> {
        locate_in_libraries(steam.libraries())
    }

    /// Requires the game jar and the bundled JVM.
    fn validate(location: &Location<()>) -> Result<Self, Vec<Requirement>> {
        let mut unmet = Vec::new();
        if !location.path.join(GAME_JAR).is_file() {
            unmet.push(Requirement { name: GAME_JAR });
        }

        let java = java_path(&location.path);

        if java.is_none() {
            unmet.push(Requirement {
                name: "bundled JVM (jre64/bin)",
            });
        }

        match java {
            Some(java) if unmet.is_empty() => Ok(Self {
                path: location.path.clone(),
                java,
            }),
            _ => Err(unmet),
        }
    }
}

/// Finds the first library whose app manifest points at an existing install.
fn locate_in_libraries(libraries: &[PathBuf]) -> Result<Location<()>, Missing> {
    let mut searched = Vec::new();
    for library in libraries {
        let manifest = library
            .join("steamapps")
            .join(format!("appmanifest_{APP_ID}.acf"));
        if let Some(path) = install_dir_from_manifest(library, &manifest)
            && path.exists()
        {
            return Ok(Location {
                path,
                source: Source::Search,
                metadata: (),
            });
        }
        searched.push(manifest);
    }
    Err(Missing { searched })
}

/// The install directory a library's app manifest declares.
fn install_dir_from_manifest(library: &Path, manifest: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let index = vdf::parse(&text).ok()?;
    let install_dir = index.get("AppState")?.get("installdir")?.as_str()?;
    Some(library.join("steamapps").join("common").join(install_dir))
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
    use crate::testing::{Scratch, searched_location};

    #[test]
    fn the_install_is_found_where_the_app_manifest_points() {
        let library = Scratch::new();
        library.file(
            &format!("steamapps/appmanifest_{APP_ID}.acf"),
            r#""AppState" { "installdir" "RenamedZomboid" }"#,
        );
        let install_dir = library.dir("steamapps/common/RenamedZomboid");

        let location = locate_in_libraries(&[library.root().to_path_buf()]).unwrap();

        assert_eq!(location.path, install_dir);
    }

    #[test]
    fn an_install_needs_the_game_jar_and_the_bundled_jvm() {
        let game = Scratch::new();
        let unmet = Install::validate(&searched_location(game.root())).unwrap_err();
        assert_eq!(unmet.len(), 2);

        game.file("projectzomboid.jar", "");
        game.file("jre64/bin/java", "");
        assert!(Install::validate(&searched_location(game.root())).is_ok());
    }
}
