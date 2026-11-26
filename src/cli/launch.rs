//! The launch command.

use anyhow::{Context as _, Result};

use crate::cli::CommandInput;
use crate::cli::output::print_header;
use crate::zomboid::LaunchPlan;

/// Launches an instance's stored game version in place with the instance's profile, waiting for the game to exit.
pub fn run(input: &CommandInput, name: &str) -> Result<()> {
    let store = &input.context.store;
    let instance = store
        .instance(name)
        .context("failed to load the instance")?;
    let game_version = instance.game_version();

    let plan = LaunchPlan::client(instance.game(), &instance.profile_dir())
        .context("failed to prepare the launch")?;

    print_header("Launch");
    println!(
        "Launched instance {} on version {game_version}",
        instance.name()
    );
    if input.invocation.is_verbose {
        println!("    Game files: {}", plan.cwd.display());
        println!("    Profile: {}", instance.profile_dir().display());
    }
    println!("Ctrl+C or closing the terminal stops the game");

    let status = plan
        .run()
        .with_context(|| format!("failed to run {}", plan.program.display()))?;

    println!();
    match status.code() {
        Some(0) => println!("The game exited"),
        Some(code) => println!("The game exited with code {code}"),
        None => println!("The game was stopped"),
    }
    Ok(())
}
