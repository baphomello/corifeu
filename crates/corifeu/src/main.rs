//! Corifeu — a runtime that orchestrates AI agents and models to build software.
//!
//! The design this implements is recorded in `docs/`, decision by decision, in
//! `docs/adr/`.

mod build;
mod config_cmd;
mod doctor;
mod run;

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
    /// Work one slice of this project until its oracle passes.
    Run {
        /// What to do, in a sentence. This is what the role is told.
        #[arg(long, value_name = "SENTENCE")]
        task: String,
        /// A file the slice is about. Pinned into every step. Repeat it when a
        /// change to one file is wrong without another, such as a component
        /// and its test: they are then checkpointed and rolled back together.
        #[arg(long = "artefact", value_name = "FILE", required = true)]
        artefacts: Vec<String>,
        /// Which declared oracle decides when it is done.
        #[arg(long, value_name = "NAME")]
        oracle: String,
        /// Which role does the work.
        #[arg(long, default_value = "coder", value_name = "ROLE")]
        role: String,
        /// How many cases the oracle must run for its verdict to count
        /// (ADR-0057). Omitted, a shrinking suite is not detected.
        #[arg(long, value_name = "N")]
        expect_cases: Option<u32>,
        /// Read this file as the project layer instead of discovering it.
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
    },
    /// Plan a brief with one role and work every slice with another.
    Build {
        /// What to build, in plain language. This is what the planner is told.
        #[arg(long, value_name = "SENTENCE")]
        brief: String,
        /// What the planner needs in order to name real commands: the
        /// language, the test runner, the layout.
        #[arg(long, value_name = "TEXT", default_value = "")]
        context: String,
        /// Read the context from a file instead, for anything longer than a
        /// shell argument comfortably holds.
        #[arg(long, value_name = "FILE")]
        context_file: Option<std::path::PathBuf>,
        /// Which role cuts the brief into slices.
        #[arg(long, default_value = "lead", value_name = "ROLE")]
        planner: String,
        /// Which role works them.
        #[arg(long, default_value = "coder", value_name = "ROLE")]
        coder: String,
        /// Read this file as the project layer instead of discovering it.
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
    },
    /// Check what this machine will actually do, by doing it.
    Doctor {
        /// Read this file as the project layer instead of discovering it.
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
        /// Skip the per-model tool-call probe, which loads each model.
        #[arg(long)]
        quick: bool,
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
        Some(Command::Run {
            task,
            artefacts,
            oracle,
            role,
            expect_cases,
            file,
        }) => run_cmd(
            &task,
            &artefacts,
            &oracle,
            &role,
            expect_cases,
            file.as_deref(),
        ),
        Some(Command::Build {
            brief,
            context,
            context_file,
            planner,
            coder,
            file,
        }) => build_cmd(
            &brief,
            &context,
            context_file.as_deref(),
            &planner,
            &coder,
            file.as_deref(),
        ),
        Some(Command::Doctor { file, quick }) => doctor::run(file.as_deref(), quick),
        None => {
            banner();
            ExitCode::SUCCESS
        }
    }
}

fn run_cmd(
    task: &str,
    artefacts: &[String],
    oracle: &str,
    role: &str,
    expect_cases: Option<u32>,
    file: Option<&std::path::Path>,
) -> ExitCode {
    let Some(loaded) = load(file) else {
        return ExitCode::FAILURE;
    };
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    run::run(
        &loaded,
        run::Ask {
            task,
            artefacts,
            oracle,
            role,
            expect_cases,
            root: &root,
        },
    )
}

/// The effective configuration, with every layer's warnings already on
/// stderr. `None` means the reason has been reported and the caller only has
/// to fail.
fn load(file: Option<&std::path::Path>) -> Option<corifeu_config::model::Config> {
    // collect_layers has already said why, on stderr.
    let layers = config_cmd::collect_layers(file).ok()?;
    let loaded = match corifeu_config::load(layers) {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("{error}");
            return None;
        }
    };
    for problem in &loaded.warnings {
        eprintln!("warning: {}", problem.message);
    }
    Some(loaded.config)
}

fn build_cmd(
    brief: &str,
    context: &str,
    context_file: Option<&std::path::Path>,
    planner: &str,
    coder: &str,
    file: Option<&std::path::Path>,
) -> ExitCode {
    let context = match context_file {
        None => context.to_string(),
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("cannot read {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        },
    };
    let Some(loaded) = load(file) else {
        return ExitCode::FAILURE;
    };
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    build::build(
        &loaded,
        build::Ask {
            goal: brief,
            context: &context,
            planner,
            coder,
            root: &root,
        },
    )
}

fn banner() {
    println!("corifeu {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Not implemented yet. The runtime is being built; the design is at");
    println!("{}", env!("CARGO_PKG_REPOSITORY"));
    println!();
    println!("Try `corifeu doctor` or `corifeu config show --explain`.");
}
