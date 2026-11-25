//! Copying workshop items into the store, one folder per item version.

use super::copy::{CopyProgress, copy_into_place, tree_size};
use super::{Store, StoreError};
use crate::zomboid::WorkshopItem;

/// What copying one workshop item did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkshopCopyOutcome {
    /// The item's version was copied into the store.
    Copied { id: u64, time_updated: u64 },
    /// The store already held the item's version.
    AlreadyStored { id: u64, time_updated: u64 },
}

impl Store {
    /// Copies each item's version into the store, skipping versions it already holds.
    pub fn copy_workshop_items(
        &self,
        items: &[WorkshopItem],
        progress: &mut dyn CopyProgress,
    ) -> Result<Vec<WorkshopCopyOutcome>, StoreError> {
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
    ) -> Result<WorkshopCopyOutcome, StoreError> {
        let id = item.id;
        let time_updated = item.time_updated;
        if self.holds_version(item) {
            return Ok(WorkshopCopyOutcome::AlreadyStored { id, time_updated });
        }

        progress.item_started(&id.to_string());
        let target = self.workshop_version_dir(id, time_updated);
        let staging = target.with_file_name(format!(".staging-{time_updated}"));
        copy_into_place(&item.path, &target, &staging, progress)?;
        Ok(WorkshopCopyOutcome::Copied { id, time_updated })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::testing::{RecordedProgress, Scratch, provisioned_store};

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
            [WorkshopCopyOutcome::Copied {
                id: 111,
                time_updated: 1_700_000_000
            }]
        );
        assert_eq!(
            copy(&first),
            [WorkshopCopyOutcome::AlreadyStored {
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

        assert_eq!(progress.items_started, ["222"]);
        assert_eq!(progress.total_bytes, "new".len() as u64);
        assert_eq!(progress.advanced_bytes, progress.total_bytes);
    }
}
