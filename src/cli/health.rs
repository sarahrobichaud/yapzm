//! Health check command reporting detected clients, steamcmd, installs, and workshop content.

use std::path::PathBuf;

use anyhow::Result;

use crate::{
    cli::{
        CommandInput,
        output::{plural, print_header, print_section},
    },
    components::{Component, Detection, Location, Missing, Requirement},
    config::{Origin, Setting},
    steam::SteamClient,
    steamcmd::Steamcmd,
    zomboid::{self, GameVersion, Install, VersionError, Workshop},
};

/// Runs the health check, reporting each component in its own section.
pub fn run(input: &CommandInput) -> Result<()> {
    if input.invocation.is_verbose {
        print_header("Component Health Check (detailed)");
    } else {
        print_header("Component Health Check");
    }

    report_clients(input);
    report_steamcmd(input);
    report_workshop(input);
    report_install(input);

    Ok(())
}

// Sections, in the order they print.

/// Reports each game client, or why it is unusable.
fn report_clients(input: &CommandInput) {
    let config = &input.context.config;
    let is_verbose = input.invocation.is_verbose;

    println!("Detected Clients:");
    println!();

    match input.context.components.steam_detection() {
        Detection::Ready(steam) => print_steam_client(steam, is_verbose),
        Detection::Invalid { location, unmet } => {
            print_invalid::<SteamClient, _>(location, unmet);
        }
        Detection::Missing(missing) => {
            let reason = not_found_reason(config.steam_path());
            print_missing::<SteamClient>(&reason, missing, is_verbose);
        }
    }
    println!();
}

/// Reports where steamcmd was found, or why it is unusable.
fn report_steamcmd(input: &CommandInput) {
    let config = &input.context.config;
    let is_verbose = input.invocation.is_verbose;

    print_section(&format!("{} Status", Steamcmd::NAME));

    match input.context.components.steamcmd_detection() {
        Detection::Ready(steamcmd) => {
            println!("[x] {} - detected", Steamcmd::NAME);
            if is_verbose {
                println!("    Location: {}", steamcmd.path().display());
            }
        }
        Detection::Invalid { location, unmet } => print_invalid::<Steamcmd, _>(location, unmet),
        Detection::Missing(missing) => {
            let reason = not_found_reason(config.steamcmd_path());
            print_missing::<Steamcmd>(&reason, missing, is_verbose);
        }
    }
    println!();
}

/// Reports the workshop content, with guidance when it cannot be detected.
fn report_workshop(input: &CommandInput) {
    let is_verbose = input.invocation.is_verbose;

    print_section(&format!("{} Integration Status", Workshop::NAME));

    let Some(detection) = input.context.components.workshop_detection() else {
        println!("[ ] {} - install {}", Workshop::NAME, SteamClient::NAME);
        println!();
        return;
    };

    match detection {
        Detection::Ready(workshop) => {
            println!("[x] {} - Enabled", Workshop::NAME);
            if is_verbose {
                println!("    Location: {}", workshop.path().display());
            }
        }
        Detection::Invalid { location, unmet } => print_invalid::<Workshop, _>(location, unmet),
        Detection::Missing(missing) => print_missing::<Workshop>(
            "install the game and subscribe to a workshop item to enable workshop integration",
            missing,
            is_verbose,
        ),
    }
    println!();
}

/// Reports the game install with its version, or why it is unusable.
fn report_install(input: &CommandInput) {
    let components = &input.context.components;
    let is_verbose = input.invocation.is_verbose;

    // Probed before printing so the output never pauses midway.
    let version = components
        .install()
        .map(|install| zomboid::version::detect(install.path()));

    print_section("Detected External Game Installs");

    match (components.install_detection(), version) {
        (Some(Detection::Ready(install)), Some(version)) => {
            print_install(install, &version, is_verbose);
        }
        (Some(Detection::Invalid { location, unmet }), _) => {
            print_invalid::<Install, _>(location, unmet);
        }
        (Some(Detection::Missing(missing)), _) => {
            print_missing::<Install>("not detected", missing, is_verbose);
        }
        _ => println!("[ ] {} - not detected", Install::NAME),
    }
    println!();
}

// Formatting.

/// Prints the Steam client's library count, or the libraries themselves in verbose mode.
fn print_steam_client(steam: &SteamClient, is_verbose: bool) {
    let libraries = steam.libraries();
    if !is_verbose {
        println!(
            "[x] {} - {} {}",
            SteamClient::NAME,
            libraries.len(),
            plural(libraries.len(), "library", "libraries")
        );
        return;
    }

    println!("[x] {}:", SteamClient::NAME);
    println!("    Location: {}", steam.root().display());
    println!("    Library locations:");
    for library in libraries {
        println!("  - {}", library.display());
    }
}

/// Prints a valid install's location and game version.
fn print_install(install: &Install, version: &Result<GameVersion, VersionError>, is_verbose: bool) {
    println!("[x] {} - from {}", Install::NAME, SteamClient::NAME);
    if is_verbose {
        println!("    Location: {}", install.path().display());
    }
    println!("    Status:  Valid");
    println!("    Version: {}", describe_version(version, is_verbose));
}

/// Prints a component that was found but fails its requirements.
fn print_invalid<T: Component, M>(location: &Location<M>, unmet: &[Requirement]) {
    println!(
        "[ ] {} - found but missing {}",
        T::NAME,
        describe_unmet(unmet)
    );
    println!("    Location: {}", location.path.display());
}

/// Prints a component that was not found, and in verbose mode where detection looked.
fn print_missing<T: Component>(reason: &str, missing: &Missing, is_verbose: bool) {
    println!("[ ] {} - {reason}", T::NAME);
    if is_verbose && !missing.searched.is_empty() {
        println!("    Searched:");
        for path in &missing.searched {
            println!("  - {}", path.display());
        }
    }
}

/// Why a component was not found, naming the override that pointed elsewhere.
fn not_found_reason(setting: Option<&Setting<PathBuf>>) -> String {
    match setting.map(Setting::origin) {
        None => "not found in the usual locations".to_owned(),
        Some(Origin::File) => "not found at the path set in the config file".to_owned(),
        Some(Origin::Env { key }) => format!("not found at the path set by {key}"),
    }
}

/// The unmet requirements joined into one phrase.
fn describe_unmet(unmet: &[Requirement]) -> String {
    let names: Vec<&str> = unmet.iter().map(|requirement| requirement.name).collect();
    names.join(" and ")
}

/// The detected game version, with the revision included in verbose mode.
fn describe_version(version: &Result<GameVersion, VersionError>, is_verbose: bool) -> String {
    let game_version = match version {
        Ok(game_version) => game_version,
        Err(error) => return format!("unknown ({error})"),
    };
    if is_verbose {
        format!(
            "{} (revision {})",
            game_version.version, game_version.revision
        )
    } else {
        game_version.version.clone()
    }
}
