//! steamcmd subcommands.

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

use crate::steamcmd::{self, InstallOutcome};

#[derive(Subcommand)]
pub enum SteamcmdCommand {
    /// Install steamcmd when it was not detected
    Install {
        /// Directory to install into
        #[arg(long)]
        to: Option<PathBuf>,
    },
}

/// Runs a steamcmd subcommand.
pub fn run(command: SteamcmdCommand) -> Result<()> {
    match command {
        SteamcmdCommand::Install { to } => match steamcmd::install(to.as_deref())? {
            InstallOutcome::Installed { path } => {
                println!("steamcmd installed: {}", path.display());
            }
            InstallOutcome::AlreadyInstalled { path } => {
                println!(
                    "steamcmd is already installed at {}; nothing to do",
                    path.display()
                );
            }
        },
    }
    Ok(())
}
