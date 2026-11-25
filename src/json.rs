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

    /// Rewriting an existing file succeeds and leaves no stray temp files behind.
    #[test]
    fn write_replaces_contents_and_leaves_no_temp() -> Result<()> {
        let base = std::env::temp_dir().join(format!("yapzm-json-{}", std::process::id()));
        fs::create_dir_all(&base)?;
        let path = base.join("data.json");

        write(&path, &serde_json::json!({ "n": 1 }))?;
        let value: serde_json::Value = read(&path)?;
        assert_eq!(value, serde_json::json!({ "n": 1 }));

        write(&path, &serde_json::json!({ "n": 3 }))?;
        let value: serde_json::Value = read(&path)?;
        assert_eq!(value, serde_json::json!({ "n": 3 }));

        let strays = fs::read_dir(&base)?
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().ends_with(".tmp"))
            .collect::<Vec<_>>();
        assert!(strays.is_empty(), "stray temp files: {strays:?}");

        fs::remove_dir_all(&base)?;
        Ok(())
    }
}
