//! The bridge to the Windows host under WSL: detection, registry reads, and drive mounts.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Reports whether this process runs inside WSL.
pub(super) fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|version| version.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

/// Reads a REG_SZ value from HKCU through WSL interop.
pub(super) fn query_hkcu(key: &str, value: &str) -> Option<String> {
    let output = Command::new("reg.exe")
        .args(["query", key, "/v", value])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find(|line| line.contains(value) && line.contains("REG_SZ"))
        .and_then(|line| line.split_once("REG_SZ"))
        .map(|(_, value)| value.trim().to_owned())
}

/// Maps a Windows path onto the drive mounts, passing through paths without a drive letter.
pub(super) fn from_windows_notation(path: &Path) -> PathBuf {
    if !is_wsl() {
        return path.to_path_buf();
    }
    mounted_path(&path.to_string_lossy()).unwrap_or_else(|| path.to_path_buf())
}

/// Maps a Windows path onto the drive mounts, rejecting paths without a drive.
pub(super) fn mounted_path(windows_path: &str) -> Option<PathBuf> {
    mount_windows_path(windows_path, &mount_root())
}

/// Maps a Windows path under a given mount root, rejecting paths without a drive.
fn mount_windows_path(path: &str, mount_root: &Path) -> Option<PathBuf> {
    let normalized = path.replace('/', "\\");
    let mut parts = normalized.split('\\').filter(|part| !part.is_empty());
    let drive = parts.next()?;
    let mut chars = drive.chars();
    let letter = chars.next()?.to_ascii_lowercase();
    if chars.next() != Some(':') || !letter.is_ascii_lowercase() {
        return None;
    }
    let mut mounted = mount_root.to_path_buf();
    mounted.push(letter.to_string());
    for part in parts {
        mounted.push(part);
    }
    Some(mounted)
}

/// The directory the Windows drives are mounted under.
pub(super) fn mount_root() -> PathBuf {
    match std::fs::read_to_string("/etc/wsl.conf") {
        Ok(conf) => parse_automount_root(&conf),
        Err(_) => PathBuf::from("/mnt"),
    }
}

/// Parses the automount root from WSL configuration contents, defaulting when undeclared.
fn parse_automount_root(conf: &str) -> PathBuf {
    let mut in_automount = false;
    for line in conf.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_automount = line.eq_ignore_ascii_case("[automount]");
        } else if in_automount
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "root"
        {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
    }
    PathBuf::from("/mnt")
}

/// The mount points of Windows drive letters currently mounted into WSL.
pub(super) fn drive_mounts() -> Vec<PathBuf> {
    std::fs::read_to_string("/proc/mounts")
        .map(|mounts| parse_drive_mounts(&mounts))
        .unwrap_or_default()
}

/// Parses mount table contents into the mount points backed by a whole Windows drive.
fn parse_drive_mounts(mounts: &str) -> Vec<PathBuf> {
    mounts
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let source = fields.next()?;
            let mount_point = fields.next()?;
            let filesystem = fields.next()?;
            let options = fields.next()?;
            let is_drvfs = filesystem == "drvfs" || options.contains("aname=drvfs");
            (is_drvfs && is_drive_root(&unescape_mount_field(source)))
                .then(|| PathBuf::from(unescape_mount_field(mount_point)))
        })
        .collect()
}

/// Reports whether a mount source names a drive letter root rather than a share or subdirectory.
fn is_drive_root(source: &str) -> bool {
    let mut chars = source.chars();
    matches!(
        (chars.next(), chars.next(), chars.next(), chars.next()),
        (Some(letter), Some(':'), Some('\\') | None, None) if letter.is_ascii_alphabetic()
    )
}

/// Decodes the octal escapes the kernel uses for special characters in mount table fields.
fn unescape_mount_field(field: &str) -> String {
    let mut decoded = String::with_capacity(field.len());
    let mut rest = field;
    while let Some(index) = rest.find('\\') {
        decoded.push_str(&rest[..index]);
        let escape = rest.get(index + 1..index + 4);
        match escape.and_then(|digits| u8::from_str_radix(digits, 8).ok()) {
            Some(byte) => {
                decoded.push(byte as char);
                rest = &rest[index + 4..];
            }
            None => {
                decoded.push('\\');
                rest = &rest[index + 1..];
            }
        }
    }
    decoded.push_str(rest);
    decoded
}

/// Maps a path under the drive mounts to Windows notation, passing through other paths.
pub(super) fn to_windows_notation(path: &Path) -> PathBuf {
    if !is_wsl() {
        return path.to_path_buf();
    }
    unmount_path(path, &mount_root()).unwrap_or_else(|| path.to_path_buf())
}

/// Maps a path under a given mount root to Windows notation, rejecting paths outside a drive mount.
fn unmount_path(path: &Path, mount_root: &Path) -> Option<PathBuf> {
    let mut components = path.strip_prefix(mount_root).ok()?.components();
    let drive = components.next()?.as_os_str().to_str()?;
    let mut chars = drive.chars();
    let letter = chars.next()?;
    if chars.next().is_some() || !letter.is_ascii_alphabetic() {
        return None;
    }

    let mut windows_path = format!("{}:\\", letter.to_ascii_uppercase());
    let parts: Vec<String> = components
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    windows_path.push_str(&parts.join("\\"));
    Some(PathBuf::from(windows_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNT_ROOT: &str = "/mnt";

    #[test]
    fn drive_paths_translate_to_windows_notation_and_back() {
        let mounted = Path::new("/mnt/c/Tools/steamcmd/steamcmd.exe");
        let windows = PathBuf::from(r"C:\Tools\steamcmd\steamcmd.exe");

        assert_eq!(
            unmount_path(mounted, Path::new(MOUNT_ROOT)),
            Some(windows.clone())
        );
        assert_eq!(
            mount_windows_path(&windows.to_string_lossy(), Path::new(MOUNT_ROOT)),
            Some(mounted.to_path_buf())
        );
    }

    #[test]
    fn paths_outside_a_drive_mount_have_no_windows_notation() {
        for linux_only in ["/home/user/steamcmd", "/mnt/wsl/state"] {
            assert_eq!(
                unmount_path(Path::new(linux_only), Path::new(MOUNT_ROOT)),
                None,
                "{linux_only}"
            );
        }
    }

    #[test]
    fn only_whole_drive_mounts_are_reported() {
        let mount_table = r"
none /mnt/wsl tmpfs rw,relatime 0 0
C:\134 /mnt/c 9p rw,aname=drvfs;path=C:\;uid=1000 0 0
\134\134server\134share /mnt/share 9p rw,aname=drvfs;path=UNC\server\share 0 0
E: /mnt/e\040drive drvfs rw,noatime 0 0
";

        assert_eq!(
            parse_drive_mounts(mount_table),
            [PathBuf::from("/mnt/c"), PathBuf::from("/mnt/e drive")]
        );
    }
}
