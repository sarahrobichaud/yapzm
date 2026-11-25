//! Command-line parsing and dispatch.

mod health;
mod output;
mod steamcmd;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};

use crate::components::{Provision, Provisioned};
use crate::config::{Config, ConfigFile, ConfigState};

#[derive(Parser)]
#[command(
    name = "yapzm",
    version,
    about = "Yet Another Project Zomboid Manager",
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    /// Create the configuration file
    #[arg(short, long)]
    setup: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Report which components are detected and usable
    Health {
        #[arg(short, long)]
        verbose: bool,
    },
    /// Manage steamcmd
    Steamcmd {
        #[command(subcommand)]
        command: steamcmd::SteamcmdCommand,
    },
}

/// Parses the command line and runs the requested command.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let config_state = Config::resolve().context("failed to resolve config")?;

    if cli.setup {
        return setup(config_state);
    }

    let ConfigState::Ready(config) = config_state else {
        println!("Setup required, run \"yapzm --setup\"");
        return Ok(());
    };

    let Some(command) = cli.command else {
        Cli::command()
            .print_long_help()
            .context("failed to print help")?;
        return Ok(());
    };

    match command {
        Commands::Health { verbose } => health::run(&config, verbose),
        Commands::Steamcmd { command } => steamcmd::run(&config, command),
    }
}

/// Creates the configuration file when it does not exist yet.
fn setup(config_state: ConfigState) -> Result<()> {
    if let ConfigState::Ready(_) = config_state {
        println!("Tool is already setup");
        return Ok(());
    }

    println!();
    println!("Running setup");
    println!();

    match ConfigFile::provision(()).context("failed to create the configuration file")? {
        Provisioned::Created(file) => {
            let location = file.location();
            let kind = if location.is_custom() {
                "custom"
            } else {
                "default"
            };
            println!(
                "[1] - Created configuration file in {kind} location: {}",
                location.path().display()
            );
        }
        Provisioned::AlreadyPresent(_) => println!("configuration already initialized, skipping"),
    }
    Ok(())
}
