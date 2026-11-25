//! A configured value and where it came from.

use std::path::{Path, PathBuf};

/// A value the user configured, with its origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting<T> {
    value: T,
    origin: Origin,
}

/// Where a configured value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The config file.
    File,
    /// An environment variable.
    Env { key: &'static str },
}

impl<T> Setting<T> {
    /// The configured value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Where the value came from.
    pub fn origin(&self) -> Origin {
        self.origin
    }
}

impl Setting<PathBuf> {
    /// The configured path.
    pub fn as_path(&self) -> &Path {
        &self.value
    }
}

/// The environment value when set, otherwise the file value.
pub(super) fn layer<T>(file: Option<T>, env: Option<(&'static str, T)>) -> Option<Setting<T>> {
    if let Some((key, value)) = env {
        return Some(Setting {
            value,
            origin: Origin::Env { key },
        });
    }
    file.map(|value| Setting {
        value,
        origin: Origin::File,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_wins_over_the_file() {
        let setting = layer(Some("file"), Some(("KEY", "env"))).unwrap();
        assert_eq!(*setting.value(), "env");
        assert_eq!(setting.origin(), Origin::Env { key: "KEY" });
    }

    #[test]
    fn the_file_applies_without_an_environment_value() {
        let setting = layer(Some("file"), None).unwrap();
        assert_eq!(*setting.value(), "file");
        assert_eq!(setting.origin(), Origin::File);
    }

    #[test]
    fn nothing_configured_is_no_setting() {
        assert_eq!(layer::<&str>(None, None), None);
    }
}
