//! Reading and writing JSON files.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};

/// Deserialize a `T` from the JSON file at `path`.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Serialize `value` to `path` atomically.
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let buf = serde_json::to_vec_pretty(value)
        .with_context(|| format!("serializing for {}", path.display()))?;
    write_atomic(path, &buf).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Write `buf` to `path` atomically.
pub(crate) fn write_atomic(path: &Path, buf: &[u8]) -> Result<usize, io::Error> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;

    let temp = path.with_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));

    let persist = || -> Result<usize, io::Error> {
        let mut file = fs::File::create(&temp)?;
        file.write_all(buf)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        Ok(buf.len())
    };

    match persist() {
        Ok(written) => Ok(written),
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;

    #[test]
    fn rewriting_a_file_replaces_it_without_leaving_temp_files() {
        let scratch = Scratch::new();
        let path = scratch.path("data.json");

        write(&path, &serde_json::json!({ "version": 1 })).unwrap();
        write(&path, &serde_json::json!({ "version": 2 })).unwrap();

        let stored: serde_json::Value = read(&path).unwrap();
        assert_eq!(stored, serde_json::json!({ "version": 2 }));
        assert_eq!(scratch.entries(), ["data.json"]);
    }
}
