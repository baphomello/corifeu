//! Ask the runtime's own counter what it makes of a check's output.
//!
//! `bench/run.sh` proves a task's oracle can be counted before it spends a
//! round on it. It used to do that with a regular expression of its own, which
//! was a lookalike of the parser rather than the parser — and the first task in
//! a fourth language caught it out: vitest prints `Tests  26 failed (26)` and
//! the preflight was looking for the `in 0.07s` that cargo and pytest end with.
//! An all-red suite, which is exactly what a scaffolded task should produce,
//! was read as an oracle that could not run.
//!
//! A preflight that disagrees with the thing it is protecting is worse than no
//! preflight, so it now asks the same question the doer asks.
//!
//! Reads the output on stdin. Prints `ran/passed`, or `unknown` and exits 1.

use corifeu_check::Cases;
use std::io::Read;

fn main() -> std::process::ExitCode {
    let mut output = String::new();
    if std::io::stdin().read_to_string(&mut output).is_err() {
        eprintln!("count: could not read stdin");
        return std::process::ExitCode::FAILURE;
    }

    match corifeu_check::cases::count(&output) {
        Cases::Counted { ran, passed } => {
            println!("{ran}/{passed}");
            std::process::ExitCode::SUCCESS
        }
        Cases::Unknown => {
            println!("unknown");
            std::process::ExitCode::FAILURE
        }
    }
}
