//! Game version detection by briefly running the bundled headless server, which prints its version during startup.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use thiserror::Error;

use super::install::java_path;

/// How long the probe waits for the server to report its version.
const PROBE_TIMEOUT: Duration = Duration::from_secs(90);

/// The failures of detecting a game version.
#[derive(Debug, Error)]
pub enum VersionError {
    /// The directory holds no bundled JVM.
    #[error("{} is not a Project Zomboid game directory", .0.display())]
    NotAGameDirectory(PathBuf),

    /// The bundled server could not be started.
    #[error("could not spawn {}: {source}", .path.display())]
    Spawn {
        path: PathBuf,
        source: std::io::Error,
    },

    /// The server ran but never printed its version.
    #[error("the headless server did not report a version")]
    NotReported,
}

/// The game version an install reports when its server starts up.
#[derive(Debug)]
pub struct GameVersion {
    /// The full version string, e.g. `42.22.2`.
    pub version: String,
    /// The build revision fingerprint, e.g. `3d39148846`.
    pub revision: String,
}

/// Detects the version of the game files in a directory.
pub fn detect(game_dir: &Path) -> Result<GameVersion, VersionError> {
    let Some(java) = java_path(game_dir) else {
        return Err(VersionError::NotAGameDirectory(game_dir.to_path_buf()));
    };
    run_probe(game_dir, &java)?.ok_or(VersionError::NotReported)
}

/// Spawns the headless server and waits for its version line.
fn run_probe(game_dir: &Path, java: &Path) -> Result<Option<GameVersion>, VersionError> {
    let mut child = server_command(game_dir, java)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| VersionError::Spawn {
            path: java.to_path_buf(),
            source,
        })?;

    let stdout = child.stdout.take().expect("the server's stdout is piped");
    let stderr = child.stderr.take().expect("the server's stderr is piped");
    let (sender, receiver) = mpsc::channel();
    let stdout_sender = sender.clone();
    let stdout_reader = std::thread::spawn(move || forward_lines(stdout, stdout_sender));
    let stderr_reader = std::thread::spawn(move || forward_lines(stderr, sender));

    let found = wait_for_version(receiver);

    let _ = child.kill();
    let _ = child.wait();
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    Ok(found)
}

/// Builds the server command, tuned to start up as fast as possible.
fn server_command(game_dir: &Path, java: &Path) -> Command {
    let windows_layout = is_windows_layout(java);
    let separator = if windows_layout { ";" } else { ":" };
    let natives = if windows_layout { "win64" } else { "linux64" };

    let mut command = Command::new(java);
    command
        .args([
            "--enable-native-access=ALL-UNNAMED",
            "--add-exports=java.base/jdk.internal.misc=ALL-UNNAMED",
            "-XX:+UseSerialGC",
            "-XX:TieredStopAtLevel=1",
            "-XX:-UsePerfData",
            "-XX:-CreateCoredumpOnCrash",
            "-XX:-OmitStackTraceInFastThrow",
            "-Xmx512m",
        ])
        .arg(format!(
            "-Djava.library.path=./natives/{separator}./natives/{natives}{separator}./"
        ))
        .args(["-cp", &format!(".{separator}projectzomboid.jar")])
        .arg("zombie.network.GameServer")
        .current_dir(game_dir);
    command
}

/// Whether the JVM executable follows Windows command-line conventions.
fn is_windows_layout(java: &Path) -> bool {
    java.file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("java.exe"))
}

/// Forwards stream lines to the channel until the stream ends.
fn forward_lines(stream: impl Read, sender: mpsc::Sender<String>) {
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        if sender.send(line).is_err() {
            return;
        }
    }
}

/// Waits for the first version line, giving up at the timeout.
fn wait_for_version(receiver: mpsc::Receiver<String>) -> Option<GameVersion> {
    let deadline = Instant::now() + PROBE_TIMEOUT;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(remaining) {
            Ok(line) => {
                if let Some((version, revision)) = parse_version_line(&line) {
                    return Some(GameVersion { version, revision });
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    None
}

/// Parses `version=42.22.2 3d39148846` out of a server log line.
fn parse_version_line(line: &str) -> Option<(String, String)> {
    let mut tokens = line.split_whitespace();
    while let Some(token) = tokens.next() {
        if let Some(version) = token.strip_prefix("version=") {
            let revision = tokens.next()?;
            if is_revision(revision) {
                return Some((version.to_owned(), revision.to_owned()));
            }
        }
    }
    None
}

/// Whether a token looks like a build revision fingerprint.
fn is_revision(text: &str) -> bool {
    (8..=40).contains(&text.len())
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zomboid::Install;

    #[test]
    fn parses_server_version_line() {
        let line = "LOG  : General      f:0 st:104> \
                    version=42.22.2 3d39148846 integrity=pending demo=false";
        assert_eq!(
            parse_version_line(line),
            Some(("42.22.2".to_owned(), "3d39148846".to_owned()))
        );
        assert_eq!(parse_version_line("fileversion=1.0 other"), None);
    }

    #[test]
    fn detect_rejects_a_directory_without_a_jvm() {
        let dir = std::env::temp_dir().join(format!("yapzm-probe-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let error = detect(&dir).unwrap_err().to_string();
        assert!(error.contains("not a Project Zomboid game directory"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_smoke() {
        let Some(install) = Install::detect() else {
            eprintln!("no Project Zomboid install visible; skipping");
            return;
        };
        let game_version = detect(&install.path).unwrap();
        assert!(!game_version.version.is_empty());
        assert!(!game_version.revision.is_empty());
    }
}
