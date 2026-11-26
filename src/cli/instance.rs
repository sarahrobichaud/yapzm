//! Instance subcommands.

use anyhow::{Context as _, Result};
use clap::Subcommand;

use crate::cli::CommandInput;
use crate::cli::output::{plural, print_header};
use crate::cli::prompt::pick;
use crate::store::Store;

#[derive(Subcommand)]
pub enum InstanceCommand {
    /// Create an instance running a game version from the local store
    New {
        /// The instance's name
        name: String,
        /// The stored game version to use, instead of picking one
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },
    /// List the instances held in the local store
    List,
}

/// Runs an instance subcommand.
pub fn run(input: &CommandInput, command: InstanceCommand) -> Result<()> {
    match command {
        InstanceCommand::New { name, version } => new_instance(input, &name, version),
        InstanceCommand::List => list_instances(input),
    }
}

/// Creates an instance from a stored game version, asking for the version when none was given.
fn new_instance(input: &CommandInput, name: &str, game_version: Option<String>) -> Result<()> {
    let store = &input.context.store;
    store.check_new_instance_name(name)?;

    print_header("New Instance");

    let game_version = match game_version {
        Some(version) => version,
        None => {
            let Some(version) = pick_game_version(store)? else {
                println!(
                    "No game versions stored, run \"yapzm version steam --copy\" to store one"
                );
                return Ok(());
            };
            version
        }
    };

    let instance = store
        .create_instance(name, &game_version)
        .context("failed to create the instance")?;

    println!(
        "Created instance {} on version {}",
        instance.name(),
        instance.game_version()
    );
    if input.invocation.is_verbose {
        println!("    Location: {}", instance.root().display());
    }
    Ok(())
}

/// Asks the user to pick a stored game version, newest first, or `None` when the store holds none.
fn pick_game_version(store: &Store) -> Result<Option<String>> {
    let mut versions = store
        .game_versions()
        .context("failed to read the game versions in the store")?;
    if versions.is_empty() {
        return Ok(None);
    }
    versions.reverse();
    let index = pick("Game version", &versions, 0)?;
    Ok(Some(versions.swap_remove(index)))
}

/// Lists the instances held in the store.
fn list_instances(input: &CommandInput) -> Result<()> {
    let instances = input
        .context
        .store
        .instances()
        .context("failed to read the instances in the store")?;

    print_header("Instances");

    if instances.is_empty() {
        println!("No instances, run \"yapzm instance new <name>\" to create one");
        return Ok(());
    }

    for instance in &instances {
        println!(
            "[x] {} - version {}",
            instance.name(),
            instance.game_version()
        );
        if input.invocation.is_verbose {
            println!("    Location: {}", instance.root().display());
        }
    }

    println!();
    println!(
        "{} {}",
        instances.len(),
        plural(instances.len(), "instance", "instances")
    );
    Ok(())
}
