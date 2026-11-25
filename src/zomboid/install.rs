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
#[derive(Debug)]
pub struct Install {
    path: PathBuf,
}

impl Install {
    /// The install's top-level directory.
    pub fn path(&self) -> &Path {
        &self.path
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
        if java_path(&location.path).is_none() {
            unmet.push(Requirement {
                name: "bundled JVM (jre64/bin)",
            });
        }

        if !unmet.is_empty() {
            return Err(unmet);
        }
        Ok(Self {
            path: location.path.clone(),
        })
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

    /// A search location for a directory, as validation receives it.
    fn location(path: &Path) -> Location<()> {
        Location {
            path: path.to_path_buf(),
            source: Source::Search,
            metadata: (),
        }
    }

    #[test]
    fn locate_reads_the_install_dir_from_the_manifest() {
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

        let location = locate_in_libraries(std::slice::from_ref(&library)).unwrap();
        assert_eq!(
            location.path,
            steamapps.join("common").join("ProjectZomboid")
        );

        let _ = std::fs::remove_dir_all(&library);
    }

    #[test]
    fn locate_reports_the_manifests_it_searched() {
        let library =
            std::env::temp_dir().join(format!("yapzm-install-missing-{}", std::process::id()));

        let missing = locate_in_libraries(std::slice::from_ref(&library)).unwrap_err();
        assert_eq!(
            missing.searched,
            vec![library.join(format!("steamapps/appmanifest_{APP_ID}.acf"))]
        );
    }

    #[test]
    fn validate_reports_every_unmet_requirement_in_an_empty_directory() {
        let dir =
            std::env::temp_dir().join(format!("yapzm-install-req-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let unmet = Install::validate(&location(&dir)).unwrap_err();
        assert_eq!(unmet.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_accepts_an_install_with_its_files() {
        let dir =
            std::env::temp_dir().join(format!("yapzm-install-req-full-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("jre64/bin")).unwrap();
        std::fs::write(dir.join("projectzomboid.jar"), "").unwrap();
        std::fs::write(dir.join("jre64/bin/java"), "").unwrap();

        let install = Install::validate(&location(&dir)).unwrap();
        assert_eq!(install.path(), dir);

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
