//! Copying directory trees into the store with progress reporting.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use super::StoreError;

/// How many bytes each read moves, so progress keeps moving through large files.
const COPY_CHUNK_SIZE: usize = 1024 * 1024;

/// Receives progress while content is copied into the store.
pub trait CopyProgress {
    /// Copying begins, with the total bytes of the content the store does not hold yet.
    fn started(&mut self, total_bytes: u64);

    /// One piece of content, named by its label, starts being copied.
    fn item_started(&mut self, label: &str);

    /// More bytes were written into the store.
    fn advanced(&mut self, bytes: u64);

    /// Every piece of content has been handled.
    fn finished(&mut self);
}

/// Copies a directory tree to a target that only appears once the copy is complete.
pub(super) fn copy_into_place(
    source: &Path,
    target: &Path,
    staging: &Path,
    progress: &mut dyn CopyProgress,
) -> Result<(), StoreError> {
    // A leftover staging folder is an interrupted copy, never a usable one.
    if staging.exists() {
        fs::remove_dir_all(staging).map_err(|error| StoreError::Write {
            path: staging.to_path_buf(),
            source: error,
        })?;
    }
    copy_tree(source, staging, progress)?;
    fs::rename(staging, target).map_err(|error| StoreError::Write {
        path: target.to_path_buf(),
        source: error,
    })
}

/// The total size of the regular files in a directory tree.
pub(super) fn tree_size(dir: &Path) -> Result<u64, StoreError> {
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
