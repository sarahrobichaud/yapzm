//! Installs steamcmd when detection finds none.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::Steamcmd;
use crate::components::{Component, Location, Provision, Provisioned, Source};
use crate::{platform, registry};

/// How long the whole download may take; the zip is small but links can be slow.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// The ways a steamcmd installation can fail.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// This platform has no automatic steamcmd installation.
    #[error(
        "steamcmd cannot be installed automatically on this platform; on Linux, \
         follow https://developer.valvesoftware.com/wiki/SteamCMD#_Linux and \
         install yapzm alongside steamcmd"
    )]
    UnsupportedPlatform,

    /// The target path exists and is not an empty directory.
    #[error("{path} already exists and is not an empty directory")]
    TargetOccupied { path: PathBuf },

    /// The registry could not be loaded.
    #[error("could not load the registry")]
    Registry {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// The registry lists no steamcmd package.
    #[error("the registry lists no steamcmd package")]
    PackageNotListed,

    /// The download failed.
    #[error("downloading {url}")]
    Download {
        url: String,
        #[source]
        source: ureq::Error,
    },

    /// The downloaded archive does not match the registry's checksum.
    #[error(
        "the downloaded steamcmd.zip does not match the registry's checksum \
         (got {got}, expected {expected})"
    )]
    ChecksumMismatch { got: String, expected: String },

    /// The archive could not be read.
    #[error("reading steamcmd.zip")]
    Archive {
        #[source]
        source: zip::result::ZipError,
    },

    /// A file or directory could not be written under the target.
    #[error("writing {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An archive entry path escapes the target directory.
    #[error("the archive contains an unsafe path: {name}")]
    UnsafeEntryPath { name: String },

    /// The unpacked archive did not contain the executable.
    #[error("the installed archive has no executable at {path}")]
    ExecutableMissing { path: PathBuf },
}

/// What a steamcmd installation needs.
pub struct InstallOptions<'a> {
    /// The directory to install into, replacing the default and skipping detection.
    pub to: Option<&'a Path>,
    /// The configured steamcmd override, used to find an existing install.
    pub steamcmd: Option<&'a Path>,
}

impl Provision for Steamcmd {
    type Options<'a> = InstallOptions<'a>;
    type Error = InstallError;

    /// Installs steamcmd, or returns the detected one when no target directory is given.
    fn provision(options: InstallOptions<'_>) -> Result<Provisioned<Self>, InstallError> {
        let default_dir =
            platform::steamcmd_install_dir().ok_or(InstallError::UnsupportedPlatform)?;

        if options.to.is_none()
            && let Some(existing) = Steamcmd::detect(options.steamcmd).ready()
        {
            return Ok(Provisioned::AlreadyPresent(existing));
        }

        let package = registry::load()
            .map_err(|error| InstallError::Registry {
                source: error.into(),
            })?
            .steamcmd
            .ok_or(InstallError::PackageNotListed)?;

        let target = match options.to {
            Some(dir) => dir.to_path_buf(),
            None => default_dir,
        };

        ensure_empty(&target)?;

        let archive = download(&package.url)?;

        verify(&archive, &package.sha256)?;
        extract(&archive, &target)?;

        let location = Location {
            path: target.join("steamcmd.exe"),
            source: Source::Created,
            metadata: (),
        };
        Steamcmd::validate(&location)
            .map(Provisioned::Created)
            .map_err(|_| InstallError::ExecutableMissing {
                path: location.path,
            })
    }
}

/// Refuses a target that is not an absent or empty directory.
fn ensure_empty(target: &Path) -> Result<(), InstallError> {
    if !target.exists() {
        return Ok(());
    }
    let empty = std::fs::read_dir(target)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false);
    if empty {
        Ok(())
    } else {
        Err(InstallError::TargetOccupied {
            path: target.to_path_buf(),
        })
    }
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
    use crate::testing::{Scratch, zip_archive};

    #[test]
    fn extraction_reproduces_the_archive_layout() {
        let scratch = Scratch::new();
        let archive = zip_archive(&[
            ("steamcmd.exe", "executable"),
            ("bin/steamclient.dll", "library"),
        ]);

        extract(&archive, scratch.root()).unwrap();

        assert_eq!(scratch.entries(), ["bin", "steamcmd.exe"]);
        assert_eq!(
            std::fs::read_to_string(scratch.path("bin/steamclient.dll")).unwrap(),
            "library"
        );
    }

    #[test]
    fn extraction_refuses_entries_that_escape_the_target() {
        let scratch = Scratch::new();
        let target = scratch.path("steamcmd");
        let archive = zip_archive(&[("../escaped.txt", "payload")]);

        let result = extract(&archive, &target);

        assert!(matches!(result, Err(InstallError::UnsafeEntryPath { .. })));
        assert!(!scratch.path("escaped.txt").exists());
    }

    #[test]
    fn an_archive_must_match_the_registry_checksum() {
        let archive = zip_archive(&[("steamcmd.exe", "executable")]);
        let expected = sha256_hex(&archive);

        assert!(verify(&archive, &expected).is_ok());
        assert!(verify(&archive, &expected.to_uppercase()).is_ok());

        let tampered = zip_archive(&[("steamcmd.exe", "tampered")]);
        assert!(matches!(
            verify(&tampered, &expected),
            Err(InstallError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn installing_never_overwrites_existing_files() {
        let scratch = Scratch::new();
        let occupied = scratch.dir("occupied");
        scratch.file("occupied/steamcmd.exe", "user's own copy");

        assert!(ensure_empty(&scratch.path("absent")).is_ok());
        assert!(ensure_empty(&scratch.dir("empty")).is_ok());
        assert!(matches!(
            ensure_empty(&occupied),
            Err(InstallError::TargetOccupied { .. })
        ));
    }
}
