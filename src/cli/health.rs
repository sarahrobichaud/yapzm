//! Health check command reporting detected game clients.

use anyhow::Result;

use crate::{
    cli::{plural, print_header},
    steam::SteamClient,
    zomboid::{self, Install},
};

/// Clients detected on this machine, absent when not installed.
struct Clients {
    steam: Option<SteamClient>,
}

struct InstallSource {
    source: String,
    details: Install,
}

/// Runs the health check, reporting which game clients were detected.
pub fn run(is_verbose: bool) -> Result<()> {
    let clients = Clients {
        steam: SteamClient::detect(),
    };

    print_header("Health Check");

    println!("Detected Clients:");
    println!("");

    let mut installs: Vec<InstallSource> = Vec::new();

    if let Some(steam) = &clients.steam {
        print_client(steam, is_verbose);

        if let Some(install) = zomboid::Install::detect_steam(steam.game_install_location_candidates()) {
            installs.push(InstallSource {
                source: "Steam".to_owned(),
                details: install,
            });
        }
    }

    println!("");
    println!("------------------");
    println!("");
    println!("Detected Game Installs:");
    println!("");

    for install in &installs {
        println!("[x] Project Zomboid - from {}", install.source);
        if is_verbose {
            println!("    Location: {}", install.details.path.display());
        }
    }
    println!("");

    Ok(())
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
