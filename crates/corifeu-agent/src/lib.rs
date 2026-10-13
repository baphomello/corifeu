//! What a doer sees on each step.
//!
//! The loop does not append to a conversation. It rebuilds a bounded context
//! from state every step (ADR-0047), because appending runs out of room and the
//! obvious cure — compacting the history — throws away the thing the agent is
//! editing along with the noise.
//!
//! A step's context is four parts:
//!
//! | Part | Bound |
//! |---|---|
//! | the contract | fixed for the whole task |
//! | **the working artefact, read from disk this step** | always whole, always current |
//! | recent tool results | the last `k` |
//! | everything older | one line each: step, action, outcome |
//!
//! The second row is the one the measurement forced. A doer asked to repair a
//! 9,570-byte file under `k = 3` read it in chunks, and each chunk fell out of
//! the window before the next arrived. It spent **28 steps reading and made
//! zero edits**, producing 1,064 output tokens in total. It was not incapable;
//! it was structurally unable to hold the file it had been told to change. With
//! the artefact pinned, the same doer edited on its first step.
//!
//! Older results are replaced by their one-line outcome rather than dropped, so
//! an agent that ran a check fifteen steps ago sees that it ran and what came
//! back, instead of the full output again or nothing at all.

pub mod repeat;

pub use repeat::{Detector, Repeat};

/// What makes two steps the same step, for [`Detector`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Action {
    pub tool: String,
    /// The argument that identifies this call: the command, the path, the
    /// filter. Not the whole payload — two writes of different content to the
    /// same file are different actions and must not collapse.
    pub argument: String,
}

impl Action {
    pub fn new(tool: impl Into<String>, argument: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            argument: argument.into(),
        }
    }

    pub fn describe(&self) -> String {
        if self.argument.is_empty() {
            self.tool.clone()
        } else {
            format!("{} {}", self.tool, self.argument)
        }
    }
}

/// One thing the doer did, and what came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub number: u32,
    pub action: Action,
    /// The whole result. Only the most recent `k` steps carry it.
    pub result: String,
    /// One line, which is what survives once the step falls out of the window.
    pub outcome: String,
}

impl Step {
    /// Summarises `result` into an outcome. Where the caller knows better — a
    /// check verdict, a test count — [`Step::with_outcome`] takes it instead.
    pub fn new(number: u32, action: Action, result: impl Into<String>) -> Self {
        let result = result.into();
        let outcome = summarise(&result);
        Self {
            number,
            action,
            result,
            outcome,
        }
    }

    pub fn with_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.outcome = outcome.into();
        self
    }

    /// The form an older step keeps: `step 12: run cargo test -> 3 passed, 12 failed`.
    pub fn compacted(&self) -> String {
        format!(
            "step {}: {} -> {}",
            self.number,
            self.action.describe(),
            self.outcome
        )
    }
}

const OUTCOME_CHARS: usize = 120;

fn summarise(result: &str) -> String {
    let first = result.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    if first.chars().count() <= OUTCOME_CHARS {
        return first.trim().to_string();
    }
    let cut: String = first.chars().take(OUTCOME_CHARS).collect();
    format!("{}…", cut.trim_end())
}

/// The task, fixed for its whole duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contract {
    pub task: String,
    /// The one command whose red-to-green transition ends this slice
    /// (ADR-0056). Named in the contract so the doer works against it rather
    /// than against its own idea of done.
    pub acceptance: String,
}

/// The file the doer is editing, read from disk this step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artefact {
    pub path: String,
    pub body: String,
}

impl Artefact {
    /// ADR-0047 makes this sentence part of the decision. Without it the doer
    /// spends a step confirming the file is current before touching it.
    pub fn preamble(&self) -> String {
        format!(
            "Current contents of {}, read fresh from disk this step. This is always \
             current; you never need to read it again.",
            self.path
        )
    }
}

/// How much history the doer keeps in full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// Steps whose full result is kept. Everything older is compacted.
    pub recent: usize,
}

impl Default for Window {
    fn default() -> Self {
        Self { recent: 3 }
    }
}

impl Window {
    /// Build this step's context. `artefact` is read by the caller immediately
    /// before this call — it is never carried over from the previous step,
    /// which is the whole point.
    pub fn build(
        &self,
        contract: &Contract,
        artefact: Option<Artefact>,
        trace: &[Step],
    ) -> Context {
        let split = trace.len().saturating_sub(self.recent);
        Context {
            contract: contract.clone(),
            artefact,
            compacted: trace[..split].iter().map(Step::compacted).collect(),
            recent: trace[split..].to_vec(),
        }
    }
}

/// One step's worth of context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub contract: Contract,
    pub artefact: Option<Artefact>,
    /// One line per step that has fallen out of the window.
    pub compacted: Vec<String>,
    pub recent: Vec<Step>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_outcome_is_the_first_meaningful_line() {
        let step = Step::new(
            4,
            Action::new("run", "cargo test"),
            "\n\nerror[E0308]: mismatched types\n  --> src/lib.rs:12\n",
        );
        assert_eq!(step.outcome, "error[E0308]: mismatched types");
    }

    #[test]
    fn a_long_outcome_is_cut_to_one_readable_line() {
        let step = Step::new(1, Action::new("run", "x"), "e".repeat(400));
        assert!(step.outcome.chars().count() <= OUTCOME_CHARS + 1);
        assert!(step.outcome.ends_with('…'));
    }

    #[test]
    fn a_caller_that_knows_better_supplies_the_outcome() {
        let step = Step::new(12, Action::new("run", "cargo test"), "lots of output")
            .with_outcome("3 passed, 12 failed");
        assert_eq!(
            step.compacted(),
            "step 12: run cargo test -> 3 passed, 12 failed"
        );
    }
}
