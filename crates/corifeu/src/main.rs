//! Corifeu — a runtime that orchestrates AI agents and models to build software.
//!
//! This binary is a placeholder. The design it will implement is recorded in `docs/`,
//! decision by decision, in `docs/adr/`.

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "corifeu",
    version,
    about = "Orchestrates AI agents and models to build software",
    long_about = None,
)]
struct Cli {}

fn main() {
    Cli::parse();

    println!("corifeu {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Not implemented yet. This release reserves the name while the");
    println!("runtime is built. The design is at {}", env!("CARGO_PKG_REPOSITORY"));
}
