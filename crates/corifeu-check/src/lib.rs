//! Judges what a check command produced.
//!
//! A check is the command whose red-to-green transition ends a slice
//! (ADR-0056). Everything downstream reads its verdict: the slice is done, the
//! coder repairs, the artefact rolls back, the plan goes back to the planner.
//! Getting the verdict wrong therefore costs more than any single wrong answer
//! a model can give, and the first measurement run spent most of its wasted
//! time on exactly that — a failing suite reported as passing, compiler errors
//! truncated away from the head, a non-terminating test charged as a failure.
//!
//! So a check has four verdicts, not two:
//!
//! | Verdict | Means | ADR |
//! |---|---|---|
//! | `Passed` | the slice is done | |
//! | `Failed` | the coder repairs, spending the slice's budget | |
//! | `Vacuous` | the oracle ran fewer cases than the slice declares | [0057] |
//! | `DidNotTerminate` | evidence about the code, not a failing test | [0049] |
//!
//! `Vacuous` is the one that is easy to leave out and expensive to miss. A test
//! runner answers a filter that matches nothing with success: `cargo test
//! --lib gone` exits 0, and so does `go test -run NoSuchTest`. In the second
//! measurement run a coder deleted every test in its slice and the next check
//! reported a pass, so nothing rolled back and the run spent its remaining 44
//! steps looking for the module it had removed.
//!
//! # What is not here
//!
//! This crate does not run the command. [ADR-0049] says every oracle runs under
//! `set -o pipefail`, and [ADR-0037] says commands are executed as argument
//! vectors with no shell at all. Both are Accepted and they cannot both be
//! implemented, so the executor waits on that being resolved. Judgement does
//! not depend on it, and is the part the measurement showed was wrong.
//!
//! [0049]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0049-faithful-check-results.md
//! [0057]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0057-oracle-integrity-by-test-inventory.md
//! [ADR-0037]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0037-sandboxed-command-execution.md
//! [ADR-0049]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0049-faithful-check-results.md

pub mod cases;

pub use cases::Cases;

use std::fmt;
use std::time::Duration;

/// What an executor observed while running a check.
#[derive(Debug, Clone)]
pub struct Observed {
    /// `None` when the process was killed rather than exiting.
    pub exit_status: Option<i32>,
    /// Standard output and standard error, interleaved as the agent would read
    /// them, and untruncated. Truncation is this crate's job.
    pub output: String,
    pub elapsed: Duration,
    /// The executor stopped the process at its deadline.
    pub timed_out: bool,
}

/// What a slice declares about its check, beyond the command itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// Cases the oracle must run, recorded when QA hands the slice over
    /// (ADR-0057). `None` leaves the guard off, which is the only honest
    /// setting for a runner whose output cannot be counted.
    pub expect_cases: Option<u32>,
    /// Characters of output the agent receives. The default is what the
    /// measurement settled on: enough for thirty Rust compiler errors, short
    /// enough to leave room for the rest of the step's context (ADR-0047).
    pub output_budget: usize,
}

impl Default for Check {
    fn default() -> Self {
        Self {
            expect_cases: None,
            output_budget: 12_000,
        }
    }
}

/// What a check run amounts to.
#[derive(Debug, Clone)]
pub struct Report {
    pub verdict: Verdict,
    pub cases: Cases,
    /// The slice declared a case count and the runner reported none, so the
    /// ADR-0057 guard did not run. The check still has a verdict; it is just
    /// not protected. `doctor` reports this per project rather than letting it
    /// pass unmentioned.
    pub unguarded: bool,
    /// Truncated to the check's budget, head first.
    pub output: String,
    pub exit_status: Option<i32>,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    Failed,
    /// Fewer cases ran than the slice declares. The oracle is damaged, so no
    /// statement about the code under it means anything yet.
    Vacuous {
        ran: u32,
        expected: u32,
    },
    /// The check did not finish inside its deadline.
    DidNotTerminate {
        after: Duration,
    },
}

impl Verdict {
    /// The slice is done.
    pub fn is_green(self) -> bool {
        matches!(self, Verdict::Passed)
    }

    /// ADR-0057: a vacuous check means the suite itself was damaged, so the
    /// artefact is restored from its checkpoint (ADR-0051) rather than handed
    /// back to the coder to repair against a suite that no longer judges it.
    pub fn triggers_rollback(self) -> bool {
        matches!(self, Verdict::Vacuous { .. })
    }

    /// ADR-0049: non-termination is evidence about the code rather than a
    /// failing test, and must not be charged to the slice's repair budget as
    /// though the coder had been given something to fix.
    pub fn spends_repair_budget(self) -> bool {
        matches!(self, Verdict::Failed)
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Passed => f.write_str("passed"),
            Verdict::Failed => f.write_str("failed"),
            Verdict::Vacuous { ran, expected } => write!(
                f,
                "vacuous: the check ran {ran} of the {expected} cases this slice declares"
            ),
            Verdict::DidNotTerminate { after } => {
                write!(f, "did-not-terminate: still running after {after:.0?}")
            }
        }
    }
}

impl Check {
    /// Judge one run of this check.
    pub fn judge(&self, observed: Observed) -> Report {
        let cases = cases::count(&observed.output);
        Report {
            verdict: self.verdict(&observed, cases),
            cases,
            unguarded: self.expect_cases.is_some() && cases == Cases::Unknown,
            output: truncate(&observed.output, self.output_budget),
            exit_status: observed.exit_status,
            elapsed: observed.elapsed,
        }
    }

    /// The order is the decision. A timeout outranks everything, because a run
    /// that was cut short evidences nothing about its assertions. A vacuous
    /// result outranks both pass and fail, because a damaged suite cannot
    /// pronounce on the code either way.
    fn verdict(&self, observed: &Observed, cases: Cases) -> Verdict {
        if observed.timed_out {
            return Verdict::DidNotTerminate {
                after: observed.elapsed,
            };
        }
        if let (Some(expected), Some(ran)) = (self.expect_cases, cases.counted())
            && ran < expected
        {
            return Verdict::Vacuous { ran, expected };
        }
        if observed.exit_status == Some(0) {
            Verdict::Passed
        } else {
            Verdict::Failed
        }
    }
}

/// Cut `text` to `budget` characters by removing the middle (ADR-0049).
///
/// The head carries the diagnosis, the tail carries the summary, and the middle
/// is repetition — thirty compiler errors truncated from the tail leave the
/// agent reading the last four, which is how one measurement round spent
/// fifteen thousand characters saying nothing. The marker is added on top of
/// the budget rather than counted against it, so the kept text is always the
/// full allowance.
pub fn truncate(text: &str, budget: usize) -> String {
    let total = text.chars().count();
    if total <= budget {
        return text.to_owned();
    }
    let head = budget * 7 / 10;
    let tail = budget - head;
    format!(
        "{}\n[... {} characters of the middle omitted ...]\n{}",
        &text[..byte_index(text, head)],
        separated(total - budget),
        &text[byte_index(text, total - tail)..],
    )
}

fn byte_index(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(at, _)| at)
}

/// Thousands separators, because the marker is read by an agent under a token
/// budget and `15148` invites a second look that `15,148` does not.
fn separated(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, digit) in digits.char_indices() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
