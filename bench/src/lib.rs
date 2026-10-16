//! What every round shares: where it runs, what it is allowed to assume, and
//! how what it produced is judged.
//!
//! Nothing here is product code. It exists so that a round can be re-run from
//! the repository by someone who was not in the room, and so that a change to
//! the runtime that breaks the harness breaks the build rather than the next
//! round.

use corifeu_check::{Cases, Check, Verdict};
use corifeu_doer::{Limits, Steps};
use corifeu_doer::{Oracle, Run, Slice};
use corifeu_exec::{Allowlist, Command};
use corifeu_workspace::{Class, Classes, Protected};
use serde::Deserialize;
use std::io::Write;
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
    /// Argv prefixes a doer may run. `[["cargo"]]` for a crate,
    /// `[["python3", "-m", "pytest"]]` for a Python task — a prefix rather than
    /// the bare program, because `python3` alone also runs `-c`.
    pub tools: Vec<Vec<String>>,
    /// Globs for the one file class the doers own. A task whose artefact is not
    /// under `src/` would otherwise have every write refused.
    pub owned: Vec<String>,
    /// A marker the coder may not remove. None for a task that has no syntax
    /// identifying a test.
    pub protected: Option<String>,
    /// Where a pair's QA is told to put the tests, with `{module}` standing in
    /// for the slice's module. Rust wants a `#[cfg(test)] mod`; pytest wants
    /// functions with a prefix. Neither belongs in a driver.
    pub tests_at: String,
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

    /// What the doers may run. Everything else goes through the allowlist's
    /// denial path, which is the point: a round where the model shells out is a
    /// round measuring something else.
    pub fn allowlist(&self) -> Allowlist {
        Allowlist::new(self.tools.clone())
    }

    /// The one file class. Both roles own it — Rust keeps a unit test inside
    /// the file it tests, so file-class ownership cannot separate them (defect
    /// 14), and a task that *could* separate them would still be comparing
    /// against Task C if it did.
    pub fn classes(&self) -> Classes {
        Classes::new([Class::new("source", self.owned.clone(), [] as [&str; 0])])
    }

    pub fn protected(&self) -> Option<Protected> {
        self.protected
            .as_ref()
            .map(|marker| Protected::new(marker, ["source"]))
    }

    pub fn tests_at(&self, which: usize) -> String {
        self.tests_at.replace("{module}", &self.at(which).module)
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

/// The seed this iteration runs with (defect 26).
///
/// Derived from the iteration so that one iteration can be replayed on its own:
/// `BENCH_SEED=<base>` fixes the base and the iteration number varies it, which
/// keeps the twenty runs of a cell from being twenty copies of each other while
/// leaving each one reproducible. `BENCH_SEED=off` restores the unseeded
/// sampling that rounds one to five used, for comparing against them.
pub fn seed(iteration: &str) -> Option<u64> {
    let base = std::env::var("BENCH_SEED").unwrap_or_else(|_| "1".into());
    if base == "off" {
        return None;
    }
    let base: u64 = base.parse().unwrap_or(1);
    let n: u64 = iteration.parse().unwrap_or(0);
    Some(base.wrapping_mul(1_000).wrapping_add(n))
}

/// Consecutive repeats before a turn stops (ADR-0050, D2).
///
/// Unset means the runtime's own default, which is what a round measuring the
/// default should use. The fifth round priced the default at 30% of its steps
/// for a 3.7% rescue rate, so the sixth should sweep it.
pub fn repeats() -> u32 {
    number("BENCH_REPEATS", Limits::default().repeats)
}

fn number(name: &str, fallback: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

/// One field is one line.
///
/// A provider error carries whatever the server said, and a server that is not
/// the one expected says it in HTML. A newline inside a field turns a TSV into
/// something no reader can parse, which is defect 22.
pub fn one_line(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

/// Append one row per step to `steps.tsv` beside the work.
///
/// `Run` has carried its whole trace since the doer was written and every
/// driver so far has thrown it away. A round that emits one summary row per
/// slice answers the questions it was designed for and nothing else. The same
/// round with its steps written down also answers the ones nobody had thought
/// of when it started, which for an eighteen-hour round is the difference
/// between a verdict and a dataset.
///
/// The `flags` column is read from the step's whole result, not from its
/// outcome. `Step::outcome` keeps the first non-empty line and the runtime
/// appends its rollback note after a blank line, so a flag taken from the
/// outcome would have reported no rollbacks ever and Q9 would have come back
/// as a confident zero.
pub fn write_steps(work: &Path, iteration: &str, model: &str, module: &str, role: &str, run: &Run) {
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(work.join("steps.tsv"))
    else {
        return;
    };
    for step in &run.trace {
        let _ = writeln!(
            file,
            "{iteration}\t{model}\t{module}\t{role}\t{}\t{}\t{}\t{}\t{}",
            step.number,
            step.action.tool,
            one_line(&step.action.argument),
            flags(&step.result),
            one_line(&step.outcome),
        );
    }
}

/// Which shipped mechanism this step tripped, if any.
///
/// Each marker is written by the runtime and each belongs to a decision that
/// has never been measured: a restore to [ADR-0051](../../docs/adr/0051-checkpoint-before-every-write.md),
/// a repeat warning to [ADR-0050](../../docs/adr/0050-loop-detection-warns-before-it-stops.md),
/// and a turn that ended without a tool call to ADR-0048's output budget.
fn flags(result: &str) -> String {
    let mut found = Vec::new();
    if result.contains("ROLLED BACK") {
        found.push("rollback");
    }
    if result.contains("REPEAT:") {
        found.push("repeat");
    }
    if result.contains("was cut off at") {
        found.push("budget");
    }
    if result.contains("without calling a tool") {
        found.push("no-tool");
    }
    if found.is_empty() {
        "-".to_string()
    } else {
        found.join("+")
    }
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
///
/// This is the one part of the harness that is still Rust-only: it appends to
/// `src/lib.rs` and runs `cargo test`. Rank rounds read the task's own fields
/// and work in any language; a pair round does not, and `NotAsked` is what a
/// task without `held-out.rs` gets.
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

#[cfg(test)]
mod defaults_agree {
    //! The configuration and the runtime each carry their own defaults and
    //! nothing converts one into the other yet: `limits.steps` is declared in
    //! `corifeu-config` and consumed nowhere. Until that path exists, the real
    //! hazard is the two drifting apart, so that a project file saying `floor:
    //! 12` and a runtime defaulting to something else would both look right.
    //!
    //! This crate is the only one that depends on both, which is why the
    //! assertion lives in the measurement harness rather than in the product.

    #[test]
    fn the_config_and_the_runtime_describe_the_same_limits() {
        let config = corifeu_config::model::Limits::default();
        let runtime = corifeu_doer::Limits::default();

        assert_eq!(config.steps.floor, runtime.steps.floor, "floor");
        assert_eq!(config.steps.ceiling, runtime.steps.ceiling, "ceiling");
        assert_eq!(config.steps.patience, runtime.steps.patience, "patience");
        assert_eq!(config.steps.raise_by, runtime.steps.raise_by, "raise_by");
        assert_eq!(config.repeats, runtime.repeats, "repeats");
    }
}
