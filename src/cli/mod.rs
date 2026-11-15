mod health;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::cmp;

#[derive(Parser)]
#[command(
    name = "yapzm",
    version,
    about = "Yet Another Project Zomboid Manager",
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the health of system components
    Health {
        /// Display more details
        #[arg(short, long)]
        verbose: bool,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Health { verbose } => health::run(verbose),
    }
}

/// The width of the separator lines dividing output sections.
const SEPARATOR_WIDTH: usize = 60;

pub(crate) fn print_header(title: &str) {
    println!();
    print_separator();
    println!("YAPZM - {}", title);
    print_separator();
    println!();
}

/// Prints a horizontal separator line dividing output sections.
pub(crate) fn print_separator() {
    println!("{}", "-".repeat(SEPARATOR_WIDTH));
}

pub(crate) fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    match count.cmp(&1) {
        cmp::Ordering::Equal => singular,
        _ => plural,
    }
}
