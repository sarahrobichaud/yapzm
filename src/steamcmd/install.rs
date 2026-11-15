//! Installs steamcmd when detection finds none.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};

use super::locate;
use crate::platform;

/// How long the whole download may take; the zip is small but links can be slow.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// The official Windows steamcmd distribution, the build Windows and WSL both run.
const STEAMCMD_WINDOWS_URL: &str = "https://client-update.steamstatic.com/installer/steamcmd.zip";

/// The SteamCMD guide, which covers manual installation on other platforms.
const STEAMCMD_DOCS: &str = "https://developer.valvesoftware.com/wiki/SteamCMD#_Linux";

/// Installs steamcmd and returns the path of the installed executable.
pub fn install(to: Option<&Path>) -> Result<PathBuf> {
    if to.is_none()
        && let Some(existing) = locate()
    {
        bail!(
            "steamcmd is already installed at {}; nothing to do",
            existing.display()
        );
    }
    if !cfg!(windows) && !platform::is_wsl() {
        bail!(
            "steamcmd installation is only supported on Windows; on Linux, \
             follow {STEAMCMD_DOCS} and install yapzm alongside steamcmd"
        );
    }

    let target = match to {
        Some(dir) => dir.to_path_buf(),
        None => default_target()?,
    };

    let archive = download()?;
    extract(&archive, &target)?;
    Ok(target.join("steamcmd.exe"))
}

/// Picks the default install directory; only reached on Windows or WSL.
fn default_target() -> Result<PathBuf> {
    if platform::is_wsl() {
        return Ok(PathBuf::from("/mnt/c/steamcmd"));
    }
    Ok(PathBuf::from(r"C:\steamcmd"))
}

/// Downloads the steamcmd distribution to install.
fn download() -> Result<Vec<u8>> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .new_agent();
    let mut response = agent
        .get(STEAMCMD_WINDOWS_URL)
        // A stale CDN entry can serve an old archive at the same path, so pin identity.
        .header("accept-encoding", "identity")
        .call()
        .with_context(|| format!("downloading {STEAMCMD_WINDOWS_URL}"))?;
    response
        .body_mut()
        .read_to_vec()
        .with_context(|| format!("reading the response from {STEAMCMD_WINDOWS_URL}"))
}

/// Unpacks the downloaded archive into the target directory.
fn extract(archive: &[u8], target: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).context("reading steamcmd.zip")?;
    std::fs::create_dir_all(target).with_context(|| format!("creating {}", target.display()))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().context("reading an archive entry name")?;
        let out = entry_target(target, &name)?;
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut file)?;
        // The zip carries no Unix mode bits; WSL needs the execute bit to
        // launch the exe through interop.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

/// Resolves an archive entry name against the target, refusing paths that escape it.
fn entry_target(target: &Path, name: &str) -> Result<PathBuf> {
    let escapes = Path::new(name).components().any(|component| {
        !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    });
    if escapes {
        bail!("the archive contains an unsafe path: {name}");
    }
    Ok(target.join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an in-memory zip archive from name and content pairs.
    fn zip_of(entries: &[(&str, &[u8])]) -> Result<Vec<u8>> {
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
    fn unpacks_every_entry_under_the_target() -> Result<()> {
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
}
