//! Copying clean game installs into the store, one folder per game version.

use std::cmp::Ordering;
use std::path::Path;

use super::copy::{CopyProgress, copy_into_place, tree_size};
use super::{Store, StoreError};

/// What copying a game install did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCopyOutcome {
    /// The install was copied into the store under its version.
    Copied { version: String },
    /// The store already held this version.
    AlreadyStored { version: String },
}

impl Store {
    /// Whether the store already holds a game version.
    pub fn holds_game_version(&self, version: &str) -> bool {
        is_safe_version_name(version) && self.game_version_dir(version).is_dir()
    }

    /// The game versions the store holds, oldest first.
    pub fn game_versions(&self) -> Result<Vec<String>, StoreError> {
        let dir = self.game_dir();
        let read_error = |source| StoreError::Read {
            path: dir.clone(),
            source,
        };

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(read_error)? {
            let entry = entry.map_err(read_error)?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if is_safe_version_name(&name) && entry.path().is_dir() {
                versions.push(name);
            }
        }
        versions.sort_by(|a, b| compare_versions(a, b));
        Ok(versions)
    }

    /// Copies a game install into the store under its version, unless that version is already held.
    pub fn copy_game_install(
        &self,
        install_dir: &Path,
        version: &str,
        progress: &mut dyn CopyProgress,
    ) -> Result<GameCopyOutcome, StoreError> {
        if !is_safe_version_name(version) {
            return Err(StoreError::InvalidVersion(version.to_owned()));
        }
        if self.holds_game_version(version) {
            progress.finished();
            return Ok(GameCopyOutcome::AlreadyStored {
                version: version.to_owned(),
            });
        }

        progress.started(tree_size(install_dir)?);
        progress.item_started(version);
        let target = self.game_version_dir(version);
        let staging = target.with_file_name(format!(".staging-{version}"));
        copy_into_place(install_dir, &target, &staging, progress)?;
        progress.finished();
        Ok(GameCopyOutcome::Copied {
            version: version.to_owned(),
        })
    }
}

/// Whether a reported version can name a store folder without escaping it.
fn is_safe_version_name(version: &str) -> bool {
    !version.is_empty()
        && !version.starts_with('.')
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Orders versions by their dot-separated parts, comparing numeric parts by value.
fn compare_versions(a: &str, b: &str) -> Ordering {
    let mut a_parts = a.split('.');
    let mut b_parts = b.split('.');
    loop {
        let ordering = match (a_parts.next(), b_parts.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(a_part), Some(b_part)) => compare_version_parts(a_part, b_part),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
}

/// Compares two version parts by value when both are numbers, and as text otherwise.
fn compare_version_parts(a: &str, b: &str) -> Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(a_number), Ok(b_number)) => a_number.cmp(&b_number).then_with(|| a.cmp(b)),
        _ => a.cmp(b),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::testing::{RecordedProgress, Scratch, provisioned_store};

    #[test]
    fn a_game_version_is_stored_with_the_install_files_and_only_copied_once() {
        let scratch = Scratch::new();
        let store = provisioned_store(&scratch);
        scratch.file("install/media/scripts/items.txt", "items");
        let install = scratch.path("install");
        let copy = || {
            store
                .copy_game_install(&install, "42.22.2", &mut RecordedProgress::default())
                .unwrap()
        };

        let first = copy();
        let second = copy();

        let version = "42.22.2".to_owned();
        assert_eq!(
            first,
            GameCopyOutcome::Copied {
                version: version.clone()
            }
        );
        assert_eq!(second, GameCopyOutcome::AlreadyStored { version });
        let stored = store.game_version_dir("42.22.2");
        assert_eq!(
            fs::read_to_string(stored.join("media/scripts/items.txt")).unwrap(),
            "items"
        );
        assert!(!store.game_dir().join(".staging-42.22.2").exists());
    }

    #[test]
    fn versions_that_could_escape_the_store_are_rejected() {
        let scratch = Scratch::new();
        let store = provisioned_store(&scratch);
        let install = scratch.dir("install");

        for version in ["", "..", "../evil", "42/1", ".staging-42"] {
            assert!(matches!(
                store.copy_game_install(&install, version, &mut RecordedProgress::default()),
                Err(StoreError::InvalidVersion(_))
            ));
        }
    }

    #[test]
    fn stored_versions_are_listed_in_numeric_order_without_staging_folders_or_files() {
        let scratch = Scratch::new();
        let store = provisioned_store(&scratch);
        for version in ["42.10.0", "41.78.16", "42.9.1", "42.10.0-unstable"] {
            fs::create_dir_all(store.game_version_dir(version)).unwrap();
        }
        fs::create_dir_all(store.game_dir().join(".staging-42.11.0")).unwrap();
        fs::write(store.game_dir().join("notes.txt"), "not a version").unwrap();

        let versions = store.game_versions().unwrap();

        assert_eq!(
            versions,
            ["41.78.16", "42.9.1", "42.10.0", "42.10.0-unstable"]
        );
    }
}
