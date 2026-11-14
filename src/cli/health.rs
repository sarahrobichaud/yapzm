//! Health check command reporting detected game clients.

use anyhow::Result;

use crate::{cli::{plural, print_header}, steam::SteamClient};

/// Clients detected on this machine, absent when not installed.
struct Clients {
    steam: Option<SteamClient>,
}

/// Runs the health check, reporting which game clients were detected.
pub fn run(is_verbose: bool) -> Result<()> {
    let clients = Clients {
        steam: SteamClient::detect(),
    };

    print_header("Health Check");

    println!("Detected Clients:");
    println!("");

    if let Some(steam) = &clients.steam {
        print_client(steam, is_verbose);
    }

    Ok(())
}

/// Prints the client's status, with root and library paths when verbose.
fn print_client(client: &SteamClient, is_verbose: bool) {
    let paths = &client.paths;
    if is_verbose {
        println!("[x] Steam:");
        println!("    Root: {}", paths.root.display());

        if !paths.libraries.is_empty() {
            println!("    Libraries:");
            for lib in &paths.libraries {
                println!("  - {}", lib.display());
            }
        }
    } else {
        println!("[x] Steam - {} {}", paths.libraries.len(), plural(paths.libraries.len(), "library", "libraries"))
    }
}
