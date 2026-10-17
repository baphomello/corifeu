//! Runs a command on an agent's behalf.
//!
//! A command is a program and its arguments, never a line of shell (ADR-0060).
//! That one restriction removes three separate defects the measurement found,
//! rather than guarding against them:
//!
//! - a pipeline cannot mask an exit status, because there is no pipeline;
//! - a heredoc cannot go around a checkpoint, because there is no heredoc;
//! - a metacharacter cannot evade the allowlist, because nothing interprets one.
//!
//! What comes back is a [`corifeu_check::Observed`], which `corifeu-check`
//! turns into a verdict. Running and judging are kept apart because the
//! measurement's worst defects were in the judging, and a judge that cannot be
//! tested without spawning a process does not get tested.
//!
//! # Killing does not reach grandchildren
//!
//! At the deadline this kills the process it started. It does not kill what
//! that process started. `cargo test` spawning a test binary that never returns
//! — the exact case the timeout exists for — leaves the test binary running
//! after cargo is gone. Reaping a whole process group needs `libc` on Unix and
//! a job object on Windows, which is a dependency decision, not an oversight.
//! Until then the deadline frees the runtime, not the machine.

pub mod allowlist;

pub use allowlist::Allowlist;
pub use corifeu_check::Observed;

use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

/// How often the deadline is checked while a command runs. Short enough that a
/// timeout is punctual, long enough that waiting costs nothing measurable.
const POLL: Duration = Duration::from_millis(10);

/// A command as Corifeu runs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    program: String,
    args: Vec<String>,
}

impl Command {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Build from a configured oracle's `command`, which is already an argv
    /// (`corifeu_config::model::Oracle`). An empty vector has no program and is
    /// not a command.
    pub fn from_argv(argv: &[String]) -> Option<Self> {
        let (program, args) = argv.split_first()?;
        Some(Self {
            program: program.clone(),
            args: args.to_vec(),
        })
    }

    pub fn argv(&self) -> Vec<String> {
        std::iter::once(self.program.clone())
            .chain(self.args.iter().cloned())
            .collect()
    }

    /// For a message to a human or an agent. Never parsed, never executed.
    pub fn display(&self) -> String {
        self.argv().join(" ")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// ADR-0037: the agent is told what was refused and what it may run
    /// instead. An agent that cannot tell a denial from a bug will retry it.
    #[error("{command} is not permitted.{}", permitted_hint(.allowed))]
    Denied {
        command: String,
        allowed: Vec<String>,
    },

    #[error("could not start {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
}

fn permitted_hint(allowed: &[String]) -> String {
    if allowed.is_empty() {
        " No commands are configured as permitted.".to_string()
    } else {
        format!(" Permitted: {}.", allowed.join(", "))
    }
}

/// Run `command` in `working_dir`, giving up after `timeout`.
///
/// A command that outlives its deadline comes back with `timed_out` set rather
/// than as an error: non-termination is evidence about the code under test, and
/// ADR-0049 requires it to reach the agent as a verdict rather than as a
/// runtime failure.
pub fn run(
    command: &Command,
    working_dir: &Path,
    timeout: Duration,
    allowlist: &Allowlist,
) -> Result<Observed, Error> {
    if !allowlist.permits(command) {
        return Err(Error::Denied {
            command: command.display(),
            allowed: allowlist.entries(),
        });
    }

    let started = Instant::now();
    let mut child = std::process::Command::new(&command.program)
        .args(&command.args)
        .current_dir(working_dir)
        // Without this a command that reads input waits for a terminal that is
        // not there, and the deadline becomes the only thing that ends it.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: command.program.clone(),
            source,
        })?;

    // Drained on their own threads. A pipe that fills blocks the child, which
    // would turn every verbose command into a timeout.
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());

    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Err(_) => break None,
            Ok(None) => {}
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            let _ = child.kill();
            break child.wait().ok();
        }
        std::thread::sleep(POLL);
    };

    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();

    Ok(Observed {
        // `None` means killed rather than exited, which is also what a signal
        // produces. `timed_out` says which.
        exit_status: status.and_then(|s| s.code()),
        output: steady(&(stdout + &stderr)),
        elapsed: started.elapsed(),
        timed_out,
    })
}

/// The same output for the same code, so a seeded round can be replayed.
///
/// A check's output goes into the next step's prompt, and every test runner
/// prints how long it took. Two runs of one unchanged crate differ by
/// `Finished ... in 0.22s` against `in 0.13s`, which is enough to change a
/// token under greedy decoding and every token after it. Round eight measured
/// the consequence: two passes of one configuration at one seed agreed on 6
/// slice runs out of 10, with speculative decoding off.
///
/// A duration is never something the model needs. The wall clock a round
/// reports comes from `elapsed`, which is measured here and not parsed back
/// out of the text.
pub fn steady(output: &str) -> String {
    let mut out = String::with_capacity(output.len());
    let mut rest = output;
    // `in <number>s`, `in <number>ms` and the bare `<number>ms` vitest prints,
    // matched by hand rather than by a regex crate: this runs on every check of
    // every step and the grammar is three shapes wide.
    while let Some(at) = rest.find(|c: char| c.is_ascii_digit()) {
        let (before, from) = rest.split_at(at);
        let digits = from
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(from.len());
        let (number, after) = from.split_at(digits);
        // A unit the number is glued to, and the unit has to end the word: it
        // keeps `189ms` and `0.22s` and leaves a test named `takes_1step`
        // alone.
        let unit = ["ms", "s"].into_iter().find(|u| {
            after.strip_prefix(u).is_some_and(|tail| {
                tail.chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric() && c != '_')
            })
        });
        match unit {
            Some(unit) => {
                out.push_str(before);
                out.push_str("Xs");
                rest = &after[unit.len()..];
            }
            _ => {
                out.push_str(before);
                out.push_str(number);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn drain<R: Read + Send + 'static>(stream: Option<R>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut stream) = stream {
            let _ = stream.read_to_end(&mut buffer);
        }
        String::from_utf8_lossy(&buffer).into_owned()
    })
}

#[cfg(test)]
mod steady_tests {
    use super::steady;

    /// The line that made a seeded round unreplayable. Two runs of one
    /// unchanged crate print different compile times, and the text goes into
    /// the next step's prompt.
    #[test]
    fn two_cargo_runs_of_one_crate_read_the_same() {
        let first = "    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s";
        let second = "    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s";
        assert_eq!(steady(first), steady(second));
    }

    #[test]
    fn a_test_summary_keeps_every_count_it_carries() {
        let line = "test result: ok. 22 passed; 0 failed; 3 ignored; 0 measured; \
                    0 filtered out; finished in 0.16s";
        let steadied = steady(line);
        assert!(steadied.contains("22 passed"), "{steadied}");
        assert!(steadied.contains("3 ignored"), "{steadied}");
        assert!(steadied.contains("0 filtered out"), "{steadied}");
        assert!(
            !steadied.contains("0.16"),
            "the duration is gone: {steadied}"
        );
    }

    /// vitest prints milliseconds, and prints them without `in` in front.
    #[test]
    fn vitest_durations_go_too() {
        let a = "   Duration  189ms (transform 20ms, collect 20ms, tests 3ms)";
        let b = "   Duration  205ms (transform 24ms, collect 18ms, tests 4ms)";
        assert_eq!(steady(a), steady(b));
    }

    #[test]
    fn a_vitest_summary_keeps_its_counts() {
        let steadied = steady("Tests  1 failed | 2 passed | 1 skipped (5)");
        assert_eq!(steadied, "Tests  1 failed | 2 passed | 1 skipped (5)");
    }

    /// A number that is not a duration is information. Case counts, byte
    /// counts, line numbers and versions all have to survive.
    #[test]
    fn numbers_that_are_not_durations_survive() {
        for line in [
            "pytest: 26 passed",
            "wrote 8001 bytes to src/pathclass.ts",
            "error[E0308]: mismatched types at src/lib.rs:42:17",
            "Compiling f-expr v0.0.0",
            "0 passed; 26 failed",
        ] {
            assert_eq!(steady(line), line, "changed: {line}");
        }
    }

    #[test]
    fn output_with_no_numbers_at_all_is_untouched() {
        let text = "thread 'main' panicked at src/lib.rs\nnot done yet\n";
        assert_eq!(steady(text), text);
    }
}
