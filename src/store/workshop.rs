//! Copying workshop items into the store, one folder per item version.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use super::{Store, StoreError};
use crate::zomboid::WorkshopItem;

/// How many bytes each read moves, so progress keeps moving through large files.
const COPY_CHUNK_SIZE: usize = 1024 * 1024;

/// What copying one workshop item did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyOutcome {
    /// The item's version was copied into the store.
    Copied { id: u64, time_updated: u64 },
    /// The store already held the item's version.
    AlreadyStored { id: u64, time_updated: u64 },
}

/// Receives progress while workshop items are copied into the store.
pub trait CopyProgress {
    /// Copying begins, with the total bytes of the versions the store does not hold yet.
    fn started(&mut self, total_bytes: u64);

    /// An item's content starts being copied.
    fn item_started(&mut self, item: &WorkshopItem);

    /// More bytes were written into the store.
    fn advanced(&mut self, bytes: u64);

    /// Every item has been handled.
    fn finished(&mut self);
}

impl Store {
    /// Copies each item's version into the store, skipping versions it already holds.
    pub fn copy_workshop_items(
        &self,
        items: &[WorkshopItem],
        progress: &mut dyn CopyProgress,
    ) -> Result<Vec<CopyOutcome>, StoreError> {
        let mut total_bytes = 0;
        for item in items.iter().filter(|item| !self.holds_version(item)) {
            total_bytes += tree_size(&item.path)?;
        }
        progress.started(total_bytes);

        let outcomes = items
            .iter()
            .map(|item| self.copy_workshop_item(item, progress))
            .collect::<Result<Vec<_>, _>>()?;

        progress.finished();
        Ok(outcomes)
    }

    /// Whether the store already holds an item's version.
    fn holds_version(&self, item: &WorkshopItem) -> bool {
        self.workshop_version_dir(item.id, item.time_updated)
            .is_dir()
    }

    /// Copies one item's version into the store unless it is already held.
    fn copy_workshop_item(
        &self,
        item: &WorkshopItem,
        progress: &mut dyn CopyProgress,
    ) -> Result<CopyOutcome, StoreError> {
        let id = item.id;
        let time_updated = item.time_updated;
        if self.holds_version(item) {
            return Ok(CopyOutcome::AlreadyStored { id, time_updated });
        }

        progress.item_started(item);
        let target = self.workshop_version_dir(id, time_updated);
        // Copying into a staging folder keeps an interrupted copy from passing as a stored version.
        let staging = target.with_file_name(format!(".staging-{time_updated}"));
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|source| StoreError::Write {
                path: staging.clone(),
                source,
            })?;
        }
        copy_tree(&item.path, &staging, progress)?;
        fs::rename(&staging, &target).map_err(|source| StoreError::Write {
            path: target,
            source,
        })?;
        Ok(CopyOutcome::Copied { id, time_updated })
    }
}

/// The total size of the regular files in a directory tree.
fn tree_size(dir: &Path) -> Result<u64, StoreError> {
    let read_error = |source| StoreError::Read {
        path: dir.to_path_buf(),
        source,
    };

    let mut total = 0;
    for entry in fs::read_dir(dir).map_err(read_error)? {
        let entry = entry.map_err(read_error)?;
        let file_type = entry.file_type().map_err(read_error)?;
        if file_type.is_dir() {
            total += tree_size(&entry.path())?;
        } else if file_type.is_file() {
            total += entry.metadata().map_err(read_error)?.len();
        }
    }
    Ok(total)
}

/// Copies a directory tree into a new location, skipping symbolic links.
fn copy_tree(
    source: &Path,
    target: &Path,
    progress: &mut dyn CopyProgress,
) -> Result<(), StoreError> {
    let read_error = |source_error| StoreError::Read {
        path: source.to_path_buf(),
        source: source_error,
    };

    fs::create_dir_all(target).map_err(|source_error| StoreError::Write {
        path: target.to_path_buf(),
        source: source_error,
    })?;

    for entry in fs::read_dir(source).map_err(read_error)? {
        let entry = entry.map_err(read_error)?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source_error| StoreError::Read {
            path: from.clone(),
            source: source_error,
        })?;

        if file_type.is_dir() {
            copy_tree(&from, &to, progress)?;
        } else if file_type.is_file() {
            copy_file(&from, &to, progress)?;
        }
    }
    Ok(())
}

/// Copies one file in chunks, reporting each chunk as it is written.
fn copy_file(from: &Path, to: &Path, progress: &mut dyn CopyProgress) -> Result<(), StoreError> {
    let read_error = |source| StoreError::Read {
        path: from.to_path_buf(),
        source,
    };
    let write_error = |source| StoreError::Write {
        path: to.to_path_buf(),
        source,
    };

    let mut reader = File::open(from).map_err(read_error)?;
    let mut writer = File::create(to).map_err(write_error)?;
    let mut buffer = vec![0; COPY_CHUNK_SIZE];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(read_error(error)),
        };
        writer.write_all(&buffer[..read]).map_err(write_error)?;
        progress.advanced(read as u64);
    }
    writer.flush().map_err(write_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Provision, Provisioned};
    use crate::testing::Scratch;

    /// Records the totals a copy reports.
    #[derive(Default)]
    struct RecordedProgress {
        total_bytes: u64,
        advanced_bytes: u64,
        items_started: Vec<u64>,
    }

    impl CopyProgress for RecordedProgress {
        fn started(&mut self, total_bytes: u64) {
            self.total_bytes = total_bytes;
        }

        fn item_started(&mut self, item: &WorkshopItem) {
            self.items_started.push(item.id);
        }

        fn advanced(&mut self, bytes: u64) {
            self.advanced_bytes += bytes;
        }

        fn finished(&mut self) {}
    }

    fn provisioned_store(scratch: &Scratch) -> Store {
        match Store::provision(&scratch.dir("data")).unwrap() {
            Provisioned::Created(store) | Provisioned::AlreadyPresent(store) => store,
        }
    }

    fn workshop_item(
        scratch: &Scratch,
        id: u64,
        time_updated: u64,
        contents: &str,
    ) -> WorkshopItem {
        let relative = format!("content/{id}");
        scratch.file(&format!("{relative}/mods/Example/mod.info"), contents);
        WorkshopItem {
            id,
            time_updated,
            path: scratch.path(&relative),
        }
    }

    #[test]
    fn each_version_is_copied_once_and_older_versions_are_kept() {
        let scratch = Scratch::new();
        let store = provisioned_store(&scratch);

        let first = workshop_item(&scratch, 111, 1_700_000_000, "first");
        let copy = |item: &WorkshopItem| {
            store
                .copy_workshop_items(std::slice::from_ref(item), &mut RecordedProgress::default())
                .unwrap()
        };
        assert_eq!(
            copy(&first),
            [CopyOutcome::Copied {
                id: 111,
                time_updated: 1_700_000_000
            }]
        );
        assert_eq!(
            copy(&first),
            [CopyOutcome::AlreadyStored {
                id: 111,
                time_updated: 1_700_000_000
            }]
        );

        let updated = workshop_item(&scratch, 111, 1_700_000_500, "updated");
        copy(&updated);

        let stored = |time: u64| {
            fs::read_to_string(
                store
                    .workshop_version_dir(111, time)
                    .join("mods/Example/mod.info"),
            )
            .unwrap()
        };
        assert_eq!(stored(1_700_000_000), "first");
        assert_eq!(stored(1_700_000_500), "updated");
        assert!(
            !store
                .workshop_dir()
                .join("111/.staging-1700000500")
                .exists()
        );
    }

    #[test]
    fn progress_covers_only_the_versions_being_copied() {
        let scratch = Scratch::new();
        let store = provisioned_store(&scratch);
        let stored = workshop_item(&scratch, 111, 1, "already here");
        let fresh = workshop_item(&scratch, 222, 1, "new");
        store
            .copy_workshop_items(
                std::slice::from_ref(&stored),
                &mut RecordedProgress::default(),
            )
            .unwrap();

        let mut progress = RecordedProgress::default();
        store
            .copy_workshop_items(&[stored, fresh], &mut progress)
            .unwrap();

        assert_eq!(progress.items_started, [222]);
        assert_eq!(progress.total_bytes, "new".len() as u64);
        assert_eq!(progress.advanced_bytes, progress.total_bytes);
    }
}
