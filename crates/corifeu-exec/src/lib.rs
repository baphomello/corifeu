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
        output: stdout + &stderr,
        elapsed: started.elapsed(),
        timed_out,
    })
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
