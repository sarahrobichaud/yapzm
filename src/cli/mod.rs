//! Command-line parsing and dispatch.

mod health;
mod mods;
mod output;
mod progress;
mod steamcmd;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};

use crate::components::{Component, Provision, Provisioned};
use crate::config::{Config, ConfigFile, ConfigState, DataDir, Origin};
use crate::store::Store;

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
    /// Manage mods and the local store
    Mods {
        #[command(subcommand)]
        command: mods::ModsCommand,
    },
}

/// Parses the command line and runs the requested command.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.setup {
        return setup();
    }

    let config_state = Config::resolve().context("failed to resolve config")?;
    let ready = match config_state {
        ConfigState::Ready(config) => Store::detect(config.data_dir().path())
            .ready()
            .map(|store| (config, store)),
        ConfigState::SetupRequired => None,
    };
    let Some((config, store)) = ready else {
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
        Commands::Mods { command } => mods::run(&config, &store, command),
    }
}

/// Creates whatever the app needs to run that does not exist yet.
fn setup() -> Result<()> {
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
        Provisioned::AlreadyPresent(file) => println!(
            "[1] - Configuration file already present: {}",
            file.location().path().display()
        ),
    }

    // The store's location comes from the config, so it is read back once the file exists.
    let ConfigState::Ready(config) = Config::resolve().context("failed to resolve config")? else {
        anyhow::bail!("the configuration file is missing right after creating it");
    };
    let data_dir = config.data_dir();

    match Store::provision(data_dir.path()).context("failed to create the store")? {
        Provisioned::Created(store) => println!(
            "[2] - Created store in {}: {}",
            describe_data_dir(data_dir),
            store.root().display()
        ),
        Provisioned::AlreadyPresent(store) => {
            println!("[2] - Store already present: {}", store.root().display());
        }
    }
    Ok(())
}

/// Names where the data directory was chosen, for setup output.
fn describe_data_dir(data_dir: &DataDir) -> String {
    match data_dir.origin() {
        None => "default location".to_owned(),
        Some(Origin::File) => "location set in the config file".to_owned(),
        Some(Origin::Env { key }) => format!("location set by {key}"),
    }
}
