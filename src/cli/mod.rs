
mod health;


use std::cmp;
use anyhow::Result;
use clap::{Parser, Subcommand};

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

pub(crate) fn print_header(title: &str) {
    println!("");
    println!("------------------------------");
    println!("YAPZM - {}", title);
    println!("------------------------------");
    println!("");
}

pub(crate) fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    match count.cmp(&1) {
        cmp::Ordering::Equal => singular,
        _ => plural,
    }
}
