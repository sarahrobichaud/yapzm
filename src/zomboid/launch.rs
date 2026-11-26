//! Launching the game client from an install with a private profile.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use super::Install;
use crate::platform;

/// The client's main class: arguments before it go to the JVM, arguments after it to the game.
const CLIENT_MAIN_CLASS: &str = "zombie.gameStates.MainScreenState";

/// Everything needed to spawn one game process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// The JVM flavor a game directory ships, which decides how paths and lists are spelled for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JavaFlavor {
    Windows,
    Native,
}

impl JavaFlavor {
    /// Reads the flavor off the JVM executable's name.
    fn of(program: &Path) -> Self {
        let is_exe = program
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
        if is_exe { Self::Windows } else { Self::Native }
    }

    /// The separator between classpath and library path entries.
    fn path_list_separator(self) -> &'static str {
        match self {
            Self::Windows => ";",
            Self::Native => ":",
        }
    }

    /// Spells a path the way this JVM reads it.
    fn host_path(self, path: &Path) -> String {
        match self {
            Self::Windows => platform::to_windows_notation(path).display().to_string(),
            Self::Native => path.display().to_string(),
        }
    }
}

impl LaunchPlan {
    /// Plans a client launch running an install's files in place, with a profile directory as its user data.
    pub fn client(game: &Install, profile_dir: &Path) -> io::Result<Self> {
        let program = game.java().to_path_buf();
        // The game rejects files whose canonical path falls outside its working directory.
        let cwd = platform::canonical_path(game.path())?;
        let flavor = JavaFlavor::of(&program);
        let args = client_args(flavor, &flavor.host_path(profile_dir));
        Ok(Self { program, args, cwd })
    }

    /// Runs the game attached to this console with its output suppressed, waiting for it to exit.
    pub fn run(&self) -> io::Result<ExitStatus> {
        Command::new(&self.program)
            .args(&self.args)
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    }
}

/// The full client launch line: JVM options, then the main class, then game arguments.
fn client_args(flavor: JavaFlavor, cachedir: &str) -> Vec<String> {
    let mut args = Vec::new();
    args.extend(jvm_runtime_args(flavor));
    args.extend(jvm_tuning_args());
    args.push(CLIENT_MAIN_CLASS.to_owned());
    args.extend(isolation_args(cachedir));
    args
}

/// JVM options the game cannot start without: module access, native libraries, and the classpath.
fn jvm_runtime_args(flavor: JavaFlavor) -> Vec<String> {
    let separator = flavor.path_list_separator();
    vec![
        // The game's windowing is native, so Java's own desktop toolkit must stay out of the way.
        "-Djava.awt.headless=true".to_owned(),
        // The native libraries are loaded through JNI, which the JVM otherwise warns about or restricts.
        "--enable-native-access=ALL-UNNAMED".to_owned(),
        // The game reaches into JDK internals that are sealed off by default.
        "--add-exports=java.base/jdk.internal.misc=ALL-UNNAMED".to_owned(),
        format!("-Djava.library.path=./win64/{separator}./"),
        "-cp".to_owned(),
        format!(".{separator}projectzomboid.jar"),
    ]
}

/// JVM and game options that shape how the game runs, matching the defaults of the game's own launcher.
fn jvm_tuning_args() -> Vec<String> {
    vec![
        // Steam integration: required for multiplayer and the workshop.
        "-Dzomboid.steam=1".to_owned(),
        // Maximum heap size.
        "-Xmx3072m".to_owned(),
        // A low-pause garbage collector, avoiding stutter from long collections.
        "-XX:+UseZGC".to_owned(),
        // A crash would otherwise dump the whole heap into the shared game folder.
        "-XX:-CreateCoredumpOnCrash".to_owned(),
    ]
}

/// Game arguments that confine the game to the instance's profile.
fn isolation_args(cachedir: &str) -> Vec<String> {
    vec![
        // Where the game keeps its user data: settings, saves, logs, and mods.
        format!("-cachedir={cachedir}"),
        // Only the profile's own mods load, so workshop copies never shadow them.
        "-modfolders".to_owned(),
        "mods".to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Scratch, game_install};

    /// The value following a flag in a launch line.
    fn value_after<'a>(args: &'a [String], flag: &str) -> &'a str {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .unwrap_or_else(|| panic!("{flag} is in the launch line"));
        &args[index + 1]
    }

    #[test]
    fn a_client_runs_the_game_files_in_place_with_the_profile_as_its_user_data() {
        let scratch = Scratch::new();
        let game = game_install(&scratch.dir("game"), "java");
        let profile = scratch.dir("profile");

        let plan = LaunchPlan::client(&game, &profile).unwrap();

        assert_eq!(plan.program, game.java());
        assert_eq!(
            plan.cwd,
            platform::canonical_path(&scratch.path("game")).unwrap()
        );
        let cachedir = format!("-cachedir={}", profile.display());
        let main_class = plan.args.iter().position(|arg| arg == CLIENT_MAIN_CLASS);
        let cachedir_index = plan.args.iter().position(|arg| *arg == cachedir);
        assert!(
            main_class < cachedir_index,
            "the profile is a game argument"
        );
        assert_eq!(value_after(&plan.args, "-cp"), ".:projectzomboid.jar");
        assert_eq!(value_after(&plan.args, "-modfolders"), "mods");
    }

    #[test]
    fn a_windows_jvm_gets_windows_path_lists() {
        let scratch = Scratch::new();
        let game = game_install(&scratch.dir("game"), "java.exe");

        let plan = LaunchPlan::client(&game, &scratch.dir("profile")).unwrap();

        assert_eq!(value_after(&plan.args, "-cp"), ".;projectzomboid.jar");
        assert!(
            plan.args
                .contains(&"-Djava.library.path=./win64/;./".to_owned())
        );
    }
}
