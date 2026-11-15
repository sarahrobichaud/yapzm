//! Health check command reporting detected game clients and install details.

use anyhow::Result;

use crate::{
    cli::{plural, print_header},
    steam::SteamClient,
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

/// Runs the health check, reporting clients and the details of each install.
pub fn run(is_verbose: bool) -> Result<()> {
    let clients = Clients {
        steam: SteamClient::detect(),
    };

    print_header("Health Check");

    println!("Detected Clients:");
    println!();

    let mut detected: Vec<(String, Install)> = Vec::new();

    match &clients.steam {
        Some(steam) => {
            print_client(steam, is_verbose);

            if let Some(install) =
                zomboid::Install::detect_steam(steam.game_install_location_candidates())
            {
                detected.push(("Steam".to_owned(), install));
            }
        }
        None => println!("[ ] Steam - not detected"),
    }

    // Every version is probed up front so the install section prints without pauses.
    let installs: Vec<InstallSource> = detected
        .into_iter()
        .map(|(source, install)| {
            let version = zomboid::version::detect(&install.path);
            InstallSource {
                source,
                details: install,
                version,
            }
        })
        .collect();

    println!();
    println!("------------------");
    println!();
    println!("Detected Game Installs:");
    println!();

    if installs.is_empty() {
        println!("[ ] Project Zomboid - not detected");
    } else {
        for install in &installs {
            println!("[x] Project Zomboid - from {}", install.source);
            if is_verbose {
                println!("    Location: {}", install.details.path.display());
            }
            print_details(install, is_verbose);
        }
    }
    println!();

    Ok(())
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

/// Prints the client's status
fn print_client(client: &SteamClient, is_verbose: bool) {
    let candidates = client.game_install_location_candidates();
    if is_verbose {
        println!("[x] Steam:");
        if !candidates.is_empty() {
            println!("    Looking for install inside:");
            for lib in candidates {
                println!("  - {}", lib.display());
            }
        }
    } else {
        println!(
            "[x] Steam - {} {}",
            candidates.len(),
            plural(candidates.len(), "library", "libraries")
        )
    }
}
