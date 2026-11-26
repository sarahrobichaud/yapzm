//! Instances in the store: a stored game version paired with its own profile.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Store, StoreError, is_safe_folder_name};
use crate::zomboid::Install;

/// The file inside an instance recording what it is built from.
const MANIFEST_FILE: &str = "instance.json";

/// The directory inside an instance holding its game configuration.
const PROFILE_DIR: &str = "profile";

/// An instance held in the store, whose game files were proven usable when it was loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    name: String,
    root: PathBuf,
    game_version: String,
    game: Install,
}

/// An instance found while listing the store, whether or not it can run.
#[derive(Debug)]
pub enum ListedInstance {
    Usable(Instance),
    Unusable {
        name: String,
        game_version: String,
        reason: StoreError,
    },
}

/// What an instance records about itself on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Manifest {
    game_version: String,
}

impl Instance {
    /// The instance's name, which is also its folder name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The stored game version the instance runs.
    pub fn game_version(&self) -> &str {
        &self.game_version
    }

    /// The stored game files the instance runs.
    pub fn game(&self) -> &Install {
        &self.game
    }

    /// The instance's directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory holding the instance's game configuration.
    pub fn profile_dir(&self) -> PathBuf {
        self.root.join(PROFILE_DIR)
    }
}

impl ListedInstance {
    /// The instance's name, which is also its folder name.
    pub fn name(&self) -> &str {
        match self {
            Self::Usable(instance) => instance.name(),
            Self::Unusable { name, .. } => name,
        }
    }

    /// The stored game version the instance records.
    pub fn game_version(&self) -> &str {
        match self {
            Self::Usable(instance) => instance.game_version(),
            Self::Unusable { game_version, .. } => game_version,
        }
    }
}

/// Reads the manifest in an instance directory, or `None` when the directory holds none.
fn read_manifest(root: &Path) -> Result<Option<Manifest>, StoreError> {
    let manifest_path = root.join(MANIFEST_FILE);
    let text = match fs::read(&manifest_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(StoreError::Read {
                path: manifest_path,
                source,
            });
        }
    };
    let manifest = serde_json::from_slice(&text).map_err(|source| StoreError::Parse {
        path: manifest_path,
        source,
    })?;
    Ok(Some(manifest))
}

impl Store {
    /// The directory of one instance.
    pub fn instance_root(&self, name: &str) -> PathBuf {
        self.instance_dir().join(name)
    }

    /// Checks that a new instance could take a name, before any choices are made for it.
    pub fn check_new_instance_name(&self, name: &str) -> Result<(), StoreError> {
        if !is_safe_folder_name(name) {
            return Err(StoreError::InvalidInstanceName(name.to_owned()));
        }
        if self.instance_root(name).exists() {
            return Err(StoreError::InstanceExists(name.to_owned()));
        }
        Ok(())
    }

    /// Creates an instance running a usable stored game version, with an empty profile.
    pub fn create_instance(&self, name: &str, game_version: &str) -> Result<Instance, StoreError> {
        self.check_new_instance_name(name)?;
        let game = self.game_install(game_version)?;

        let target = self.instance_root(name);
        let staging = target.with_file_name(format!(".staging-{name}"));
        // A leftover staging folder is an interrupted creation, never a usable instance.
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(write_error(&staging))?;
        }
        let profile = staging.join(PROFILE_DIR);
        fs::create_dir_all(&profile).map_err(write_error(&profile))?;

        let manifest = Manifest {
            game_version: game_version.to_owned(),
        };
        let manifest_path = staging.join(MANIFEST_FILE);
        let text =
            serde_json::to_vec_pretty(&manifest).expect("an instance manifest always serializes");
        fs::write(&manifest_path, text).map_err(write_error(&manifest_path))?;

        fs::rename(&staging, &target).map_err(write_error(&target))?;
        Ok(Instance {
            name: name.to_owned(),
            root: target,
            game_version: manifest.game_version,
            game,
        })
    }

    /// The instance with a name, failing unless its game version is still usable.
    pub fn instance(&self, name: &str) -> Result<Instance, StoreError> {
        if !is_safe_folder_name(name) {
            return Err(StoreError::InvalidInstanceName(name.to_owned()));
        }
        let root = self.instance_root(name);
        let manifest =
            read_manifest(&root)?.ok_or_else(|| StoreError::UnknownInstance(name.to_owned()))?;
        self.resolve_instance(name.to_owned(), root, manifest)
    }

    /// The instances the store holds, sorted by name, each usable or with the reason it is not.
    pub fn instances(&self) -> Result<Vec<ListedInstance>, StoreError> {
        let dir = self.instance_dir();
        let read_error = |source| StoreError::Read {
            path: dir.clone(),
            source,
        };

        let mut instances = Vec::new();
        for entry in fs::read_dir(&dir).map_err(read_error)? {
            let entry = entry.map_err(read_error)?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if !is_safe_folder_name(&name) || !entry.path().is_dir() {
                continue;
            }
            let Some(manifest) = read_manifest(&entry.path())? else {
                continue;
            };
            let game_version = manifest.game_version.clone();
            let listed = match self.resolve_instance(name.clone(), entry.path(), manifest) {
                Ok(instance) => ListedInstance::Usable(instance),
                Err(reason) => ListedInstance::Unusable {
                    name,
                    game_version,
                    reason,
                },
            };
            instances.push(listed);
        }
        instances.sort_by(|a, b| a.name().cmp(b.name()));
        Ok(instances)
    }

    /// Pairs a manifest with the stored game version it names.
    fn resolve_instance(
        &self,
        name: String,
        root: PathBuf,
        manifest: Manifest,
    ) -> Result<Instance, StoreError> {
        let game = self.game_install(&manifest.game_version)?;
        Ok(Instance {
            name,
            root,
            game_version: manifest.game_version,
            game,
        })
    }
}

/// Builds the error for a failed write to a path.
fn write_error(path: &Path) -> impl FnOnce(std::io::Error) -> StoreError {
    let path = path.to_path_buf();
    move |source| StoreError::Write { path, source }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Scratch, provisioned_store, stored_game_version};

    /// A store holding one runnable game version.
    fn store_with_game_version(scratch: &Scratch, version: &str) -> Store {
        let store = provisioned_store(scratch);
        stored_game_version(&store, version);
        store
    }

    #[test]
    fn a_new_instance_has_an_empty_profile_and_is_listed_with_its_game_version() {
        let scratch = Scratch::new();
        let store = store_with_game_version(&scratch, "42.22.2");

        let created = store.create_instance("survival", "42.22.2").unwrap();

        assert!(created.profile_dir().is_dir());
        assert_eq!(fs::read_dir(created.profile_dir()).unwrap().count(), 0);
        let listed = store.instances().unwrap();
        let [ListedInstance::Usable(listed)] = listed.as_slice() else {
            panic!("the new instance is listed as usable: {listed:?}");
        };
        assert_eq!(listed, &created);
        assert!(!store.instance_dir().join(".staging-survival").exists());
    }

    #[test]
    fn an_instance_needs_a_game_version_the_store_holds() {
        let scratch = Scratch::new();
        let store = store_with_game_version(&scratch, "42.22.2");

        let result = store.create_instance("survival", "41.78.16");

        assert!(matches!(result, Err(StoreError::UnknownGameVersion(_))));
        assert!(!store.instance_root("survival").exists());
    }

    #[test]
    fn an_existing_instance_is_never_overwritten() {
        let scratch = Scratch::new();
        let store = store_with_game_version(&scratch, "42.22.2");
        let existing = store.create_instance("survival", "42.22.2").unwrap();
        fs::write(existing.profile_dir().join("options.ini"), "kept").unwrap();

        let result = store.create_instance("survival", "42.22.2");

        assert!(matches!(result, Err(StoreError::InstanceExists(_))));
        assert_eq!(
            fs::read_to_string(existing.profile_dir().join("options.ini")).unwrap(),
            "kept"
        );
    }

    #[test]
    fn instance_names_that_could_escape_the_store_are_rejected() {
        let scratch = Scratch::new();
        let store = store_with_game_version(&scratch, "42.22.2");

        for name in ["", "..", "../evil", "a/b", ".staging-x"] {
            assert!(matches!(
                store.create_instance(name, "42.22.2"),
                Err(StoreError::InvalidInstanceName(_))
            ));
        }
    }

    #[test]
    fn an_instance_is_found_by_name_only_once_it_is_finished() {
        let scratch = Scratch::new();
        let store = store_with_game_version(&scratch, "42.22.2");
        let created = store.create_instance("survival", "42.22.2").unwrap();
        fs::create_dir_all(store.instance_dir().join("no-manifest")).unwrap();

        assert_eq!(store.instance("survival").unwrap(), created);
        assert!(matches!(
            store.instance("no-manifest"),
            Err(StoreError::UnknownInstance(_))
        ));
        assert!(matches!(
            store.instance("../survival"),
            Err(StoreError::InvalidInstanceName(_))
        ));
    }

    #[test]
    fn an_instance_whose_game_files_broke_cannot_be_loaded_but_is_still_listed() {
        let scratch = Scratch::new();
        let store = store_with_game_version(&scratch, "42.22.2");
        let created = store.create_instance("survival", "42.22.2").unwrap();
        fs::remove_file(created.game().java()).unwrap();

        let loaded = store.instance("survival");
        let listed = store.instances().unwrap();

        assert!(matches!(
            loaded,
            Err(StoreError::UnusableGameVersion { .. })
        ));
        let [ListedInstance::Unusable { name, reason, .. }] = listed.as_slice() else {
            panic!("the broken instance is listed as unusable: {listed:?}");
        };
        assert_eq!(name, "survival");
        assert!(matches!(reason, StoreError::UnusableGameVersion { .. }));
    }

    #[test]
    fn listing_skips_folders_that_are_not_finished_instances() {
        let scratch = Scratch::new();
        let store = store_with_game_version(&scratch, "42.22.2");
        store.create_instance("zeta", "42.22.2").unwrap();
        store.create_instance("alpha", "42.22.2").unwrap();
        fs::create_dir_all(store.instance_dir().join(".staging-beta/profile")).unwrap();
        fs::create_dir_all(store.instance_dir().join("no-manifest")).unwrap();

        let names: Vec<String> = store
            .instances()
            .unwrap()
            .iter()
            .map(|instance| instance.name().to_owned())
            .collect();

        assert_eq!(names, ["alpha", "zeta"]);
    }
}
