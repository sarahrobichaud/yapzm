//! Installs steamcmd when detection finds none.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::locate;
use crate::{platform, registry};

/// How long the whole download may take; the zip is small but links can be slow.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// The ways a steamcmd installation can fail.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// steamcmd was detected and no explicit target was requested.
    #[error("steamcmd is already installed at {path}; nothing to do")]
    AlreadyInstalled { path: PathBuf },

    /// This platform has no automatic steamcmd installation.
    #[error(
        "steamcmd installation is only supported on Windows; on Linux, follow \
         https://developer.valvesoftware.com/wiki/SteamCMD#_Linux and install \
         yapzm alongside steamcmd"
    )]
    UnsupportedPlatform,

    /// The registry could not be loaded.
    #[error("could not load the registry")]
    Registry {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// The registry lists no steamcmd package to trust.
    #[error("the registry does not list a steamcmd package")]
    PackageNotListed,

    /// The download failed.
    #[error("downloading {url}")]
    Download {
        url: String,
        #[source]
        source: ureq::Error,
    },

    /// The downloaded archive does not match the checksum the registry vouches for.
    #[error(
        "the downloaded steamcmd.zip does not match the checksum in the registry: \
         got {got}, expected {expected}; refusing to install it"
    )]
    ChecksumMismatch { got: String, expected: String },

    /// The archive could not be read.
    #[error("reading steamcmd.zip")]
    Archive {
        #[source]
        source: zip::result::ZipError,
    },

    /// A directory or file could not be written.
    #[error("writing {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An archive entry path escapes the target directory.
    #[error("the archive contains an unsafe path: {name}")]
    UnsafeEntryPath { name: String },
}

/// Installs steamcmd and returns the path of the installed executable.
pub fn install(to: Option<&Path>) -> Result<PathBuf, InstallError> {
    if to.is_none()
        && let Some(existing) = locate()
    {
        return Err(InstallError::AlreadyInstalled { path: existing });
    }
    if !cfg!(windows) && !platform::is_wsl() {
        return Err(InstallError::UnsupportedPlatform);
    }

    let package = registry::load()
        .map_err(|error| InstallError::Registry { source: error.into() })?
        .steamcmd
        .ok_or(InstallError::PackageNotListed)?;

    let target = match to {
        Some(dir) => dir.to_path_buf(),
        None => default_target(),
    };

    let archive = download(&package.url)?;
    verify(&archive, &package.sha256)?;
    extract(&archive, &target)?;
    Ok(target.join("steamcmd.exe"))
}

/// Picks the default install directory; only reached on Windows or WSL.
fn default_target() -> PathBuf {
    if platform::is_wsl() {
        return PathBuf::from("/mnt/c/steamcmd");
    }
    PathBuf::from(r"C:\steamcmd")
}

/// Downloads the steamcmd distribution to install.
fn download(url: &str) -> Result<Vec<u8>, InstallError> {
    let failed = |error: ureq::Error| InstallError::Download {
        url: url.to_owned(),
        source: error,
    };

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .new_agent();
    let mut response = agent
        .get(url)
        // A stale CDN entry can serve an old archive at the same path, so pin identity.
        .header("accept-encoding", "identity")
        .call()
        .map_err(failed)?;
    response.body_mut().read_to_vec().map_err(failed)
}

/// Refuses archives that do not match the checksum the registry vouches for.
fn verify(archive: &[u8], expected: &str) -> Result<(), InstallError> {
    let hash = sha256_hex(archive);
    if !hash.eq_ignore_ascii_case(expected) {
        return Err(InstallError::ChecksumMismatch {
            got: hash,
            expected: expected.to_owned(),
        });
    }
    Ok(())
}

/// Unpacks the downloaded archive into the target directory.
fn extract(archive: &[u8], target: &Path) -> Result<(), InstallError> {
    let write_error = |path: &Path, error: std::io::Error| InstallError::Write {
        path: path.to_path_buf(),
        source: error,
    };

    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .map_err(|error| InstallError::Archive { source: error })?;
    std::fs::create_dir_all(target).map_err(|error| write_error(target, error))?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| InstallError::Archive { source: error })?;
        let name = entry
            .name()
            .map_err(|error| InstallError::Archive { source: error })?;
        let out = entry_target(target, &name)?;
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|error| write_error(&out, error))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|error| write_error(parent, error))?;
        }
        let mut file = std::fs::File::create(&out).map_err(|error| write_error(&out, error))?;
        std::io::copy(&mut entry, &mut file).map_err(|error| write_error(&out, error))?;
        // The zip carries no Unix mode bits; WSL needs the execute bit to
        // launch the exe through interop.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755))
                .map_err(|error| write_error(&out, error))?;
        }
    }
    Ok(())
}

/// Resolves an archive entry name against the target, refusing paths that escape it.
fn entry_target(target: &Path, name: &str) -> Result<PathBuf, InstallError> {
    let escapes = Path::new(name).components().any(|component| {
        !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    });
    if escapes {
        return Err(InstallError::UnsafeEntryPath {
            name: name.to_owned(),
        });
    }
    Ok(target.join(name))
}

/// The lowercase hex SHA-256 of a byte slice.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an in-memory zip archive from name and content pairs.
    fn zip_of(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
            let options = zip::write::SimpleFileOptions::default();
            for (name, contents) in entries {
                writer.start_file(name.to_owned(), options)?;
                std::io::Write::write_all(&mut writer, contents)?;
            }
            writer.finish()?;
        }
        Ok(bytes)
    }

    /// An empty scratch directory, unique per test name.
    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yapzm-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn unpacks_every_entry_under_the_target() -> Result<(), Box<dyn std::error::Error>> {
        let archive = zip_of(&[
            ("steamcmd.exe", b"MZ fake exe".as_slice()),
            ("sub/steamclient.dll", b"dll bytes".as_slice()),
        ])?;
        let target = scratch_dir("unpack");

        extract(&archive, &target)?;

        assert_eq!(
            std::fs::read(target.join("steamcmd.exe"))?,
            b"MZ fake exe",
            "top-level entries land directly in the target"
        );
        assert_eq!(
            std::fs::read(target.join("sub/steamclient.dll"))?,
            b"dll bytes",
            "nested entries keep their directory structure"
        );

        std::fs::remove_dir_all(&target)?;
        Ok(())
    }

    #[test]
    fn resolves_entry_paths_under_the_target() {
        let target = Path::new("/store");

        assert_eq!(
            entry_target(target, "steamcmd.exe").unwrap(),
            PathBuf::from("/store/steamcmd.exe")
        );
        assert_eq!(
            entry_target(target, "sub/steamclient.dll").unwrap(),
            PathBuf::from("/store/sub/steamclient.dll")
        );
        assert!(
            entry_target(target, "./fine.txt").is_ok(),
            "current-directory components are harmless"
        );
    }

    #[test]
    fn rejects_entry_paths_that_escape_the_target() {
        let target = Path::new("/store");

        for name in ["../evil.txt", "a/../../evil.txt"] {
            assert!(
                entry_target(target, name).is_err(),
                "{name} must be refused"
            );
        }
    }

    #[test]
    fn hashes_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
