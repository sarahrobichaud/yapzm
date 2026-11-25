//! Mod management subcommands.

use anyhow::{Context, Result};
use clap::Subcommand;

use crate::cli::output::{plural, print_header};
use crate::cli::progress::CopyBar;
use crate::components::{Component, Detection};
use crate::config::{Config, Setting};
use crate::platform;
use crate::steam::SteamClient;
use crate::store::{CopyOutcome, Store};
use crate::zomboid::Workshop;

#[derive(Subcommand)]
pub enum ModsCommand {
    /// Work with mods from the Steam Workshop
    #[command(arg_required_else_help = true)]
    Workshop {
        /// Copy every installed workshop item into the local store
        #[arg(long)]
        copy: bool,
    },
}

/// Runs a mods subcommand.
pub fn run(config: &Config, store: &Store, command: ModsCommand) -> Result<()> {
    match command {
        ModsCommand::Workshop { copy: true } => copy_workshop(config, store),
        ModsCommand::Workshop { copy: false } => Ok(()),
    }
}

/// Copies the installed workshop items into the store and reports what changed.
fn copy_workshop(config: &Config, store: &Store) -> Result<()> {
    let steam = SteamClient::detect(config.steam_path().map(Setting::as_path));
    let Some(Detection::Ready(workshop)) = steam.as_ready().map(Workshop::detect) else {
        println!(
            "{} integration is not enabled, run \"yapzm health\" for details",
            Workshop::NAME
        );
        return Ok(());
    };

    let items = workshop
        .items()
        .context("failed to read the installed workshop items")?;

    print_header("Copy Workshop Items");
    if platform::is_wsl() {
        println!("Note: copying from WSL reads through the Windows drive mounts and is slow.");
        println!("      Run yapzm.exe from Windows for much faster copies.");
        println!();
    }

    let mut progress = CopyBar::new();
    let outcomes = store
        .copy_workshop_items(&items, &mut progress)
        .context("failed to copy workshop items into the store")?;

    let mut copied = 0;
    for outcome in &outcomes {
        if let CopyOutcome::Copied { id, time_updated } = outcome {
            println!("[x] {id} - stored version {time_updated}");
            copied += 1;
        }
    }
    let already_stored = outcomes.len() - copied;

    if copied > 0 {
        println!();
    }
    println!(
        "Copied {copied} {}, {already_stored} already stored",
        plural(copied, "item", "items")
    );
    Ok(())
}
