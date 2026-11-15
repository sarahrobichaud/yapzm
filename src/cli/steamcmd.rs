//! steamcmd subcommands.

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

use crate::steamcmd;

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
        SteamcmdCommand::Install { to } => {
            let exe = steamcmd::install(to.as_deref())?;
            println!("steamcmd installed: {}", exe.display());
            Ok(())
        }
    }
}
