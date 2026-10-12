//! Corifeu — a runtime that orchestrates AI agents and models to build software.
//!
//! The design this implements is recorded in `docs/`, decision by decision, in
//! `docs/adr/`.

mod config_cmd;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "corifeu",
    version,
    about = "Orchestrates AI agents and models to build software",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect the effective configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the effective configuration.
    Show {
        /// Show the source of every value: which layer set it, or `default`.
        #[arg(long)]
        explain: bool,
        /// Read this file as the project layer instead of discovering it.
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Config {
            command: ConfigCommand::Show { explain, file },
        }) => config_cmd::show(explain, file.as_deref()),
        None => {
            banner();
            ExitCode::SUCCESS
        }
    }
}

fn banner() {
    println!("corifeu {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Not implemented yet. The runtime is being built; the design is at");
    println!("{}", env!("CARGO_PKG_REPOSITORY"));
    println!();
    println!("Try `corifeu config show --explain`.");
}
