//! steamcmd subcommands.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;

use crate::components::{Provision, Provisioned};
use crate::config::{self, Config, Origin, Setting};
use crate::steamcmd::{InstallOptions, Steamcmd};

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
pub fn run(config: &Config, command: SteamcmdCommand) -> Result<()> {
    match command {
        SteamcmdCommand::Install { to } => install(config, to),
    }
}

/// Installs steamcmd and reports where it is, saving a custom location to the config file.
fn install(config: &Config, to: Option<PathBuf>) -> Result<()> {
    // The saved path must stay valid regardless of the directory yapzm later runs from.
    let to = to
        .map(std::path::absolute)
        .transpose()
        .context("the install directory is not a valid path")?;
    let options = InstallOptions {
        to: to.as_deref(),
        steamcmd: config.steamcmd_path().map(Setting::as_path),
    };

    match Steamcmd::provision(options)? {
        Provisioned::Created(steamcmd) => {
            println!("steamcmd installed: {}", steamcmd.path().display());
            if to.is_some() {
                remember_location(config, &steamcmd)?;
            }
        }
        Provisioned::AlreadyPresent(steamcmd) => {
            println!(
                "steamcmd is already installed at {}; nothing to do",
                steamcmd.path().display()
            );
        }
    }
    Ok(())
}

/// Saves a freshly installed steamcmd to the config file so later commands find it.
fn remember_location(config: &Config, steamcmd: &Steamcmd) -> Result<()> {
    let config_path = config::save_steamcmd_path(steamcmd.path())
        .context("steamcmd was installed, but saving its location to the configuration failed")?;
    println!(
        "Saved steamcmd location to {}",
        config_path.path().display()
    );

    if let Some(Origin::Env { key }) = config.steamcmd_path().map(Setting::origin) {
        println!("Note: {key} is set and still takes precedence over the saved location");
    }
    Ok(())
}
