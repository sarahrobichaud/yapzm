//! Mod management subcommands.

use anyhow::{Context as _, Result};
use clap::Subcommand;

use crate::cli::CommandInput;
use crate::cli::output::{plural, print_header, print_wsl_copy_note};
use crate::cli::progress::CopyBar;
use crate::components::Component;
use crate::context::Context;
use crate::store::WorkshopCopyOutcome;
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
pub fn run(input: &CommandInput, command: ModsCommand) -> Result<()> {
    match command {
        ModsCommand::Workshop { copy: true } => copy_workshop(&input.context),
        ModsCommand::Workshop { copy: false } => Ok(()),
    }
}

/// Copies the installed workshop items into the store and reports what changed.
fn copy_workshop(context: &Context) -> Result<()> {
    let Some(workshop) = context.components.workshop() else {
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
    print_wsl_copy_note();

    let mut progress = CopyBar::new("Measuring workshop items...");
    let outcomes = context
        .store
        .copy_workshop_items(&items, &mut progress)
        .context("failed to copy workshop items into the store")?;

    let mut copied = 0;
    for outcome in &outcomes {
        if let WorkshopCopyOutcome::Copied { id, time_updated } = outcome {
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
