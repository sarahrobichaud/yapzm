//! Command-line parsing and dispatch.

mod health;
mod instance;
mod launch;
mod mods;
mod output;
mod progress;
mod prompt;
mod steamcmd;
mod version;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};

use crate::components::{Component, Provision, Provisioned};
use crate::config::{Config, ConfigFile, ConfigState, DataDir, Origin};
use crate::context::Context as AppContext;
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

    /// Show more detail in command output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

/// Everything a command receives: the app's state and how it was invoked.
pub(crate) struct CommandInput {
    pub(crate) context: AppContext,
    pub(crate) invocation: Invocation,
}

/// How the command line asked a command to behave, apart from the command's own arguments.
pub(crate) struct Invocation {
    pub(crate) is_verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Report which components are detected and usable
    Health,
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
    /// List stored game versions, detect them and copy them into the local store
    Version {
        #[command(subcommand)]
        command: version::VersionCommand,
    },
    /// Create and list instances built from stored game versions
    Instance {
        #[command(subcommand)]
        command: instance::InstanceCommand,
    },
    /// Launch the game for an instance, using its own profile
    Launch {
        /// The instance to launch
        #[arg(value_name = "INSTANCE")]
        name: String,
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

    let input = CommandInput {
        context: AppContext::detect(config, store),
        invocation: Invocation {
            is_verbose: cli.verbose,
        },
    };
    match command {
        Commands::Health => health::run(&input),
        Commands::Steamcmd { command } => steamcmd::run(&input, command),
        Commands::Mods { command } => mods::run(&input, command),
        Commands::Version { command } => version::run(&input, command),
        Commands::Instance { command } => instance::run(&input, command),
        Commands::Launch { name } => launch::run(&input, &name),
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
