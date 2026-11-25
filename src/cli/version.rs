//! Game version subcommands.

use anyhow::{Context as _, Result};
use clap::Subcommand;

use crate::cli::CommandInput;
use crate::cli::output::{print_header, print_wsl_copy_note};
use crate::cli::progress::{CopyBar, spinner};
use crate::components::Component;
use crate::store::GameCopyOutcome;
use crate::zomboid::{self, Install};

#[derive(Subcommand)]
pub enum VersionCommand {
    /// Detect the version of the game installed through Steam
    Steam {
        /// Copy the Steam game install into the local store under its version
        #[arg(long)]
        copy: bool,
    },
}

/// Runs a version subcommand.
pub fn run(input: &CommandInput, command: VersionCommand) -> Result<()> {
    match command {
        VersionCommand::Steam { copy } => steam_version(input, copy),
    }
}

/// Reports the Steam install's game version and whether the store holds it, copying it when asked.
fn steam_version(input: &CommandInput, should_copy: bool) -> Result<()> {
    let context = &input.context;
    let Some(install) = context.components.install() else {
        println!(
            "No usable {} Steam install, run \"yapzm health\" for details",
            Install::NAME
        );
        return Ok(());
    };

    if should_copy {
        print_header("Copy Steam Game Install");
    } else {
        print_header("Steam Game Version");
    }

    let probing = spinner("Detecting the game version...");
    let detected = zomboid::version::detect(install.path());
    probing.finish_and_clear();
    let detected = detected.context("failed to detect the version of the Steam install")?;
    let version = detected.version.as_str();

    println!("[x] {} {version} ({})", Install::NAME, detected.revision);
    if input.invocation.is_verbose {
        println!("    Location: {}", install.path().display());
    }

    let is_stored = context.store.holds_game_version(version);
    if !should_copy {
        if is_stored {
            println!("    Stored in the local store");
        } else {
            println!("    Not stored, run \"yapzm version steam --copy\" to store it");
        }
        return Ok(());
    }

    println!();
    if is_stored {
        println!("Version {version} already stored");
        return Ok(());
    }

    print_wsl_copy_note();
    let mut progress = CopyBar::new("Measuring the game install...");
    let outcome = context
        .store
        .copy_game_install(install.path(), version, &mut progress)
        .context("failed to copy the game install into the store")?;

    match outcome {
        GameCopyOutcome::Copied { version } => println!("Stored version {version}"),
        GameCopyOutcome::AlreadyStored { version } => println!("Version {version} already stored"),
    }
    Ok(())
}
