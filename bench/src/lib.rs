//! What every round shares: where it runs, what it is allowed to assume, and
//! how what it produced is judged.
//!
//! Nothing here is product code. It exists so that a round can be re-run from
//! the repository by someone who was not in the room, and so that a change to
//! the runtime that breaks the harness breaks the build rather than the next
//! round.

use corifeu_check::{Cases, Check, Verdict};
use corifeu_doer::Steps;
use corifeu_doer::{Oracle, Slice};
use corifeu_exec::{Allowlist, Command};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A task, as `task.json` describes it.
///
/// The scaffold copies this file into the workspace it builds, so a round reads
/// what it is working on rather than carrying a copy of it in Rust. Adding a
/// task is then a directory and a JSON file, which is the only way the number
/// of tasks is going to grow.
#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub name: String,
    /// The one file every slice edits.
    pub artefact: String,
    /// The per-slice check, with `{module}` standing in for the slice's module.
    pub oracle: Vec<String>,
    /// Every case the task has, for the row measured after a round is scored.
    pub whole_suite: Vec<String>,
    pub slices: Vec<TaskSlice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskSlice {
    pub module: String,
    pub cases: u32,
    /// What a doer is told when the tests already exist.
    pub implement: String,
    /// What a pair is told when they do not, so QA has a specification to read
    /// rather than a suite to satisfy.
    pub specify: String,
}

impl Task {
    pub fn read(work: &Path) -> Task {
        let text = std::fs::read_to_string(work.join("task.json"))
            .unwrap_or_else(|e| panic!("no task.json in {}: {e}", work.display()));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("task.json: {e}"))
    }

    pub fn at(&self, which: usize) -> &TaskSlice {
        self.slices
            .get(which - 1)
            .unwrap_or_else(|| panic!("{} has {} slices", self.name, self.slices.len()))
    }

    /// The slice as the runtime wants it. `expect_cases` is what the task
    /// declares for a doer working against tests that already exist, and None
    /// for a pair, where QA's red run supplies it instead (ADR-0057).
    pub fn slice(&self, at: usize, task: String, expect_cases: Option<u32>) -> Slice {
        let slice = self.at(at);
        Slice {
            task,
            artefact: self.artefact.clone(),
            oracle: Oracle {
                argv: self
                    .oracle
                    .iter()
                    .map(|part| part.replace("{module}", &slice.module))
                    .collect(),
                timeout: Duration::from_secs(240),
                expect_cases,
            },
        }
    }
}

/// The step allowance this round runs with (ADR-0062).
///
/// Read from the environment, because the fifth run's whole question is what
/// the floor should be, and sweeping it must not mean rebuilding the harness.
pub fn steps() -> Steps {
    let fallback = Steps::default();
    Steps {
        floor: number("BENCH_STEPS_FLOOR", fallback.floor),
        ceiling: number("BENCH_STEPS_CEILING", fallback.ceiling),
        patience: number("BENCH_STEPS_PATIENCE", fallback.patience),
        raise_by: number("BENCH_STEPS_RAISE_BY", fallback.raise_by),
    }
}

fn number(name: &str, fallback: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

/// What a round is judged on beyond "it went green".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldOut {
    /// The crate built and the hidden cases ran.
    Ran { passed: u32, of: u32 },
    /// The crate did not build with the hidden cases added, which is itself a
    /// result: the signature or the visibility is not what the task asked for.
    DidNotBuild,
    /// No `held-out.rs` beside the crate. This round does not measure quality.
    NotAsked,
}

impl std::fmt::Display for HeldOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeldOut::Ran { passed, of } => write!(f, "{passed}/{of}"),
            HeldOut::DidNotBuild => write!(f, "did-not-build"),
            HeldOut::NotAsked => write!(f, "-"),
        }
    }
}

/// Run cases the round never saw against the crate it produced.
///
/// ADR-0053 turns on whether work whose test was written by its own author is
/// worse than work where it was not. The tests the round wrote cannot answer
/// that — it passes those by construction — so the answer has to come from
/// outside the round.
///
/// Judged in a copy. Appending the hidden cases to the crate would change the
/// artefact the round produced, and that artefact is the evidence.
pub fn held_out(work: &Path, allow: &Allowlist) -> HeldOut {
    let Ok(cases) = std::fs::read_to_string(work.join("held-out.rs")) else {
        return HeldOut::NotAsked;
    };
    let Ok(produced) = std::fs::read_to_string(work.join("src/lib.rs")) else {
        return HeldOut::NotAsked;
    };

    let judged = judging_copy(work, &produced, &cases);
    let Some(command) = Command::from_argv(&argv()) else {
        return HeldOut::DidNotBuild;
    };
    let Ok(observed) = corifeu_exec::run(&command, &judged, Duration::from_secs(240), allow) else {
        return HeldOut::DidNotBuild;
    };

    let report = Check {
        expect_cases: None,
        output_budget: 8_000,
    }
    .judge(observed);

    match (report.verdict, report.cases) {
        (Verdict::DidNotTerminate { .. }, _) | (_, Cases::Unknown) => HeldOut::DidNotBuild,
        (_, Cases::Counted { ran, passed }) => HeldOut::Ran { passed, of: ran },
    }
}

fn argv() -> Vec<String> {
    ["cargo", "test", "--lib", "held_out"]
        .map(String::from)
        .to_vec()
}

/// The copy is a differently *named* package, not just a different directory.
///
/// Cargo keys its build artefacts on the package name and version, so two
/// crates called `pathclass 0.0.0` sharing one target directory overwrite each
/// other's test binary — and the second `cargo test` can run the first one's
/// and report its result as its own. That defect is how this function came to
/// be written this way.
fn judging_copy(work: &Path, produced: &str, cases: &str) -> PathBuf {
    let judged = work.with_file_name(format!(
        "{}-held-out",
        work.file_name().unwrap_or_default().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&judged);
    let _ = std::fs::create_dir_all(judged.join("src"));

    let manifest = std::fs::read_to_string(work.join("Cargo.toml")).unwrap_or_default();
    let _ = std::fs::write(judged.join("Cargo.toml"), rename_package(&manifest));
    let _ = std::fs::write(judged.join("src/lib.rs"), format!("{produced}\n{cases}\n"));
    judged
}

fn rename_package(manifest: &str) -> String {
    manifest
        .lines()
        .map(|line| match line.split_once('=') {
            Some((key, value)) if key.trim() == "name" => {
                format!("{key}= \"{}_held_out\"", value.trim().trim_matches('"'))
            }
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::rename_package;

    #[test]
    fn the_judging_copy_is_a_different_package() {
        let manifest = "[package]\nname = \"pathclass\"\nversion = \"0.0.0\"\n";
        let renamed = rename_package(manifest);
        assert!(
            renamed.contains("name = \"pathclass_held_out\""),
            "{renamed}"
        );
        assert!(renamed.contains("version = \"0.0.0\""), "{renamed}");
    }
}
