//! Shared fixtures that keep unit tests short and readable.

use std::io::{Cursor, Write as _};
use std::path::{Path, PathBuf};

use crate::components::{Component, Location, Provision, Provisioned, Source};
use crate::store::{CopyProgress, Store};
use crate::zomboid::Install;

/// A temporary directory tree, deleted when dropped.
pub struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    /// Creates an empty scratch directory.
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("creating a scratch directory"),
        }
    }

    /// The scratch directory itself.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// The path of an entry inside the scratch directory, which need not exist.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    /// Creates a directory and its parents, returning its path.
    pub fn dir(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(&path).expect("creating a scratch subdirectory");
        path
    }

    /// Writes a file, creating its parents, and returns its path.
    pub fn file(&self, relative: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("creating a scratch file's parent");
        }
        std::fs::write(&path, contents).expect("writing a scratch file");
        path
    }

    /// The names of the entries directly inside the scratch directory, sorted.
    pub fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.root())
            .expect("listing the scratch directory")
            .map(|entry| entry.expect("reading a scratch entry").file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

/// A location found by searching, as component validation receives it.
pub fn searched_location(path: impl Into<PathBuf>) -> Location<()> {
    Location {
        path: path.into(),
        source: Source::Search,
        metadata: (),
    }
}

/// An in-memory zip archive holding the given name and content pairs.
pub fn zip_archive(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
    for (name, contents) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .expect("starting a zip entry");
        writer
            .write_all(contents.as_bytes())
            .expect("writing a zip entry");
    }
    writer.finish().expect("finishing the zip archive");
    bytes
}

/// A freshly provisioned store inside the scratch directory.
pub fn provisioned_store(scratch: &Scratch) -> Store {
    match Store::provision(&scratch.dir("data")).expect("provisioning a scratch store") {
        Provisioned::Created(store) | Provisioned::AlreadyPresent(store) => store,
    }
}

/// Writes the files a runnable game needs into a directory, with a given JVM executable name, and returns the validated install.
pub fn game_install(dir: &Path, java_executable: &str) -> Install {
    let java = dir.join("jre64/bin").join(java_executable);
    std::fs::create_dir_all(java.parent().unwrap()).expect("creating the bundled JVM directory");
    std::fs::write(&java, "").expect("writing the bundled JVM");
    std::fs::write(dir.join("projectzomboid.jar"), "").expect("writing the game jar");
    Install::validate(&searched_location(dir)).expect("the written game files are a valid install")
}

/// Stores a runnable game version in a store.
pub fn stored_game_version(store: &Store, version: &str) -> Install {
    game_install(&store.game_version_dir(version), "java")
}

/// Copy progress that records what a copy reported.
#[derive(Default)]
pub struct RecordedProgress {
    pub total_bytes: u64,
    pub advanced_bytes: u64,
    pub items_started: Vec<String>,
}

impl CopyProgress for RecordedProgress {
    fn started(&mut self, total_bytes: u64) {
        self.total_bytes = total_bytes;
    }

    fn item_started(&mut self, label: &str) {
        self.items_started.push(label.to_owned());
    }

    fn advanced(&mut self, bytes: u64) {
        self.advanced_bytes += bytes;
    }

    fn finished(&mut self) {}
}
