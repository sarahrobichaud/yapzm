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
    fn the_environment_overrides_the_config_file() {
        let from_env = layer(Some("file"), Some(("KEY", "env"))).unwrap();
        assert_eq!(*from_env.value(), "env");
        assert_eq!(from_env.origin(), Origin::Env { key: "KEY" });

        let from_file = layer(Some("file"), None).unwrap();
        assert_eq!(from_file.origin(), Origin::File);
    }
}
