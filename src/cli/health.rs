//! Health check command reporting detected clients, steamcmd, installs, and workshop content.

use std::path::Path;

use anyhow::Result;

use crate::{
    cli::{plural, print_header, print_separator},
    config,
    steam::SteamClient,
    steamcmd,
    zomboid::{self, GameVersion, Install, VersionError},
};

/// Clients detected on this machine, absent when not installed.
struct Clients {
    steam: Option<SteamClient>,
}

/// An install detected through a game client, with its detected version.
struct InstallSource {
    source: String,
    details: Install,
    version: Result<GameVersion, VersionError>,
}

/// Runs the health check, reporting clients, installs, and workshop content.
pub fn run(is_verbose: bool) -> Result<()> {
    let clients = Clients {
        steam: SteamClient::detect(),
    };

    if is_verbose {
        print_header("Component Health Check (detailed)");
    } else {
        print_header("Component Health Check");
    }
    print_clients(&clients, is_verbose);
    print_steamcmd(is_verbose);
    print_workshop(&clients, is_verbose);
    print_installs(&detect_installs(&clients), is_verbose);

    Ok(())
}

/// Prints each detected game client, or its absence.
fn print_clients(clients: &Clients, is_verbose: bool) {
    println!("Detected Clients:");
    println!();

    match &clients.steam {
        Some(steam) => print_client(steam, is_verbose),
        None => {
            let reason = config::steam_dir()
                .map(|root| override_reason("YAPZM_STEAM_DIR", &root, "a Steam install"));
            print_not_detected("Steam", reason.as_deref());
        }
    }
    println!();
}

/// Detects installs through each client, probing versions up front so printing never pauses.
fn detect_installs(clients: &Clients) -> Vec<InstallSource> {
    let mut detected: Vec<(String, Install)> = Vec::new();

    if let Some(steam) = &clients.steam {
        if let Some(install) = zomboid::Install::detect_steam(steam.libraries()) {
            detected.push(("Steam".to_owned(), install));
        }
    }

    detected
        .into_iter()
        .map(|(source, install)| {
            let version = zomboid::version::detect(&install.path);
            InstallSource {
                source,
                details: install,
                version,
            }
        })
        .collect()
}

/// Prints each detected install with its status and version.
fn print_installs(installs: &[InstallSource], is_verbose: bool) {
    print_separator();
    println!();
    println!("Detected External Game Installs:");
    println!();

    if installs.is_empty() {
        println!("[ ] Project Zomboid - not detected");
    } else {
        for install in installs {
            println!("[x] Project Zomboid - from {}", install.source);
            if is_verbose {
                println!("    Location: {}", install.details.path.display());
            }
            print_details(install, is_verbose);
        }
    }
    println!();
}

/// Prints the steamcmd detection status, with guidance when it is missing.
fn print_steamcmd(is_verbose: bool) {
    print_separator();
    println!();
    println!("SteamCMD Status:");
    println!();

    match steamcmd::locate() {
        Some(program) => {
            println!("[x] SteamCMD - detected");
            if is_verbose {
                println!("    Location: {}", program.display());
            }
        }
        None => {
            let reason = config::steamcmd()
                .map(|path| override_reason("YAPZM_STEAMCMD", &path, "a steamcmd executable"));
            let fallback = "install it or set YAPZM_STEAMCMD";
            print_not_detected("SteamCMD", reason.as_deref().or(Some(fallback)));
        }
    }
    println!();
}

/// Prints the workshop content, with guidance when it cannot be detected.
fn print_workshop(clients: &Clients, is_verbose: bool) {
    let workshop = clients
        .steam
        .as_ref()
        .and_then(|steam| zomboid::Workshop::detect_steam(steam.libraries()));

    print_separator();
    println!();
    println!("Steam Workshop Integration Status:");
    println!();

    match (&clients.steam, &workshop) {
        (Some(_), Some(workshop)) => {
            println!("[x] Steam Workshop - Enabled");
            if is_verbose {
                println!("    Location: {}", workshop.path.display());
            }
        }
        (None, _) => println!("[ ] Steam Workshop - install Steam"),
        (Some(_), None) => println!(
            "[ ] Steam Workshop - install the game and subscribe to a workshop item to enable workshop integration"
        ),
    }
    println!();
}

/// Prints the client's status and, in verbose mode, its library roots.
fn print_client(client: &SteamClient, is_verbose: bool) {
    let libraries = client.libraries();
    if is_verbose {
        println!("[x] Steam:");
        if !libraries.is_empty() {
            println!("    Looking for install inside:");
            for lib in libraries {
                println!("  - {}", lib.display());
            }
        }
    } else {
        println!(
            "[x] Steam - {} {}",
            libraries.len(),
            plural(libraries.len(), "library", "libraries")
        )
    }
}

/// Prints the file status and, when the install is valid, its game version.
fn print_details(install: &InstallSource, is_verbose: bool) {
    println!("    Status:  {}", describe_status(&install.details));
    if install.details.valid() {
        println!(
            "    Version: {}",
            describe_version(&install.version, is_verbose)
        );
    }
}

/// The install status: valid, or the requirements that are unmet.
fn describe_status(install: &Install) -> String {
    let unmet: Vec<&str> = install
        .requirements()
        .iter()
        .filter(|requirement| !requirement.met)
        .map(|requirement| requirement.name)
        .collect();
    if unmet.is_empty() {
        "Valid".to_owned()
    } else {
        format!("missing {}", unmet.join(" and "))
    }
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

/// Prints a component's not-detected line, with the reason when one is known.
fn print_not_detected(component: &str, reason: Option<&str>) {
    match reason {
        Some(reason) => println!("[ ] {component} - not detected; {reason}"),
        None => println!("[ ] {component} - not detected"),
    }
}

/// The reason a configured override failed to yield a detection.
fn override_reason(flag: &str, path: &Path, expectation: &str) -> String {
    format!(
        "{flag} points to {}, which is not {expectation}",
        path.display()
    )
}
