//! QA writes the test, the coder makes it pass, and a plan is walked by the two.
//!
//! [ADR-0053](https://github.com/baphomello/corifeu/blob/main/docs/adr/0053-qa-writes-the-tests.md)
//! ships two doer-side roles rather than one. The measurement's single doer
//! wrote good tests — the reviewer confirmed they honestly covered the stated
//! criteria — and wrote them from the same reading of the specification that
//! produced the implementation. The blind spot was in both halves, so three
//! runs plateaued at 18 of 21 without converging.
//!
//! The order matters more than the separation. A slice begins with a failing
//! test, and three things follow that do not otherwise work:
//!
//! - the coder has something to work against rather than its own idea of done;
//! - the case count the coder's guard needs ([ADR-0057]) exists, because QA's
//!   red run counted it — nobody knows it before the tests are written;
//! - a test that passes the moment it is written is a *result*, not a failure.
//!   It says the work was already done by an earlier slice's coder exceeding
//!   its boundary, which the measurement found is what rescues a looping slice
//!   ([ADR-0058]).
//!
//! [ADR-0057]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0057-oracle-integrity-by-test-inventory.md
//! [ADR-0058]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0058-slice-boundaries-are-observed.md

use crate::{Doer, Oracle, Outcome, Run, Slice};
use corifeu_check::Verdict;
use corifeu_planner::{Plan, PlannedSlice};
use corifeu_workspace::Workspace;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum SliceOutcome {
    /// QA wrote a failing test and the coder made it pass.
    Green { cases: u32 },
    /// QA's test passed the moment it was written, and the check was already
    /// passing before QA started. The work existed already, done by an earlier
    /// slice's coder exceeding its boundary (ADR-0058).
    DeliveredEarly { cases: u32 },
    /// QA's test passed the moment it was written, and the check was *not*
    /// passing before QA started. QA wrote the implementation as well as the
    /// test.
    ///
    /// This is ADR-0053's failure mode happening: one agent authored both the
    /// code and the oracle that judges it, so the blind spot is on both sides.
    /// In a language that keeps unit tests inside the file under test, no
    /// ownership rule can prevent it (ADR-0057) — only notice it.
    ///
    /// The slice is green and the work stands: refusing correct code would be
    /// perverse. ADR-0042's warning applies, and the run records which slices
    /// were judged by their own author.
    SelfTested { cases: u32 },
    /// QA could not produce a test that runs at all. Nothing was handed on,
    /// because a test that will not compile is not an oracle.
    NoRunnableTest { why: Outcome },
    /// The coder could not pass the test, and QA could not improve it.
    /// ADR-0059: the plan goes back to the planner only after this.
    NotPassed { why: Outcome, qa_rounds: u32 },
}

#[derive(Debug, Clone)]
pub struct SliceRun {
    pub outcome: SliceOutcome,
    pub qa: Vec<Run>,
    pub coder: Vec<Run>,
}

impl SliceRun {
    pub fn is_done(&self) -> bool {
        matches!(
            self.outcome,
            SliceOutcome::Green { .. }
                | SliceOutcome::DeliveredEarly { .. }
                | SliceOutcome::SelfTested { .. }
        )
    }

    /// Whether this slice's oracle was written by whoever satisfied it.
    pub fn self_tested(&self) -> bool {
        matches!(self.outcome, SliceOutcome::SelfTested { .. })
    }

    pub fn prompt_tokens(&self) -> u32 {
        self.qa
            .iter()
            .chain(&self.coder)
            .map(|r| r.prompt_tokens)
            .sum()
    }

    pub fn output_tokens(&self) -> u32 {
        self.qa
            .iter()
            .chain(&self.coder)
            .map(|r| r.output_tokens)
            .sum()
    }

    pub fn steps(&self) -> u32 {
        self.qa.iter().chain(&self.coder).map(|r| r.steps).sum()
    }
}

pub struct Pair<'a> {
    /// Goal::TestRuns, owning the `test` class.
    pub qa: Doer<'a>,
    /// Goal::Green, owning `source`.
    pub coder: Doer<'a>,
    /// How many times QA may be asked. ADR-0059 §2 sends a slice the coder
    /// could not pass back to QA before it goes back to the planner, because
    /// the test is one of three suspects and the cheapest to re-read.
    pub qa_rounds: u32,
}

impl Pair<'_> {
    pub fn work(&self, slice: &Slice, workspace: &mut Workspace) -> SliceRun {
        let mut qa = Vec::new();
        let mut coder = Vec::new();
        // What the check said before anyone touched it. A green check after QA
        // means two different things depending on this, and the run has no way
        // to tell them apart afterwards.
        let green_before = self.qa.probe(slice, workspace);

        for round in 1..=self.qa_rounds.max(1) {
            let qa_run = self.qa.work(&qa_slice(slice, round), workspace);
            let reached = qa_run.reached();
            let cases = qa_run.cases.ran().unwrap_or(0);
            let verdict = qa_run.verdict;
            let why = qa_run.outcome.clone();
            qa.push(qa_run);

            if !reached {
                return SliceRun {
                    outcome: SliceOutcome::NoRunnableTest { why },
                    qa,
                    coder,
                };
            }
            if verdict == Some(Verdict::Passed) {
                let outcome = if green_before {
                    SliceOutcome::DeliveredEarly { cases }
                } else {
                    SliceOutcome::SelfTested { cases }
                };
                return SliceRun { outcome, qa, coder };
            }

            let coder_run = self.coder.work(&coder_slice(slice, cases), workspace);
            let passed = coder_run.reached();
            let why = coder_run.outcome.clone();
            coder.push(coder_run);

            if passed {
                return SliceRun {
                    outcome: SliceOutcome::Green { cases },
                    qa,
                    coder,
                };
            }
            if round == self.qa_rounds.max(1) {
                return SliceRun {
                    outcome: SliceOutcome::NotPassed {
                        why,
                        qa_rounds: round,
                    },
                    qa,
                    coder,
                };
            }
        }
        unreachable!("the loop returns on its last round")
    }
}

/// QA's slice carries no case expectation: there is nothing to guard until the
/// tests it is about to write exist.
fn qa_slice(slice: &Slice, round: u32) -> Slice {
    let again = if round > 1 {
        " The coder could not make your last test pass. Read it against the task again: if it \
         asks for something the task does not, correct it; if it is right, make it clearer."
    } else {
        ""
    };
    Slice {
        task: format!(
            "Write the tests for this, and nothing else. They must fail now, for the right \
             reason — a test that does not compile is broken, not failing.\n\n{}{again}",
            slice.task
        ),
        artefact: slice.artefact.clone(),
        oracle: Oracle {
            expect_cases: None,
            ..slice.oracle.clone()
        },
    }
}

/// The coder's slice carries the count QA's red run reported, which is what
/// ADR-0057's guard compares against for the rest of the slice.
fn coder_slice(slice: &Slice, cases: u32) -> Slice {
    Slice {
        task: format!(
            "The tests for this are written and failing. Make them pass without changing \
             them.\n\n{}",
            slice.task
        ),
        artefact: slice.artefact.clone(),
        oracle: Oracle {
            expect_cases: Some(cases),
            ..slice.oracle.clone()
        },
    }
}

// ------------------------------------------------------------------ the walk

#[derive(Debug, Clone)]
pub struct Walked {
    pub slices: Vec<(PlannedSlice, SliceRun)>,
}

impl Walked {
    pub fn all_done(&self) -> bool {
        self.slices.iter().all(|(_, run)| run.is_done())
    }

    /// The first slice that did not finish, which is what goes back to the
    /// planner (ADR-0056).
    pub fn stalled(&self) -> Option<&(PlannedSlice, SliceRun)> {
        self.slices.iter().find(|(_, run)| !run.is_done())
    }

    pub fn prompt_tokens(&self) -> u32 {
        self.slices.iter().map(|(_, r)| r.prompt_tokens()).sum()
    }

    pub fn output_tokens(&self) -> u32 {
        self.slices.iter().map(|(_, r)| r.output_tokens()).sum()
    }
}

impl Pair<'_> {
    /// Work a plan in order, stopping at the first slice that does not finish.
    ///
    /// A slice ordered after a failed one is not attempted: ADR-0056's slices
    /// may depend on each other, so a failure early invalidates what follows,
    /// and the planner is re-consulted rather than the run being carried on
    /// against a decomposition already known to be wrong.
    pub fn walk(&self, plan: &Plan, timeout: Duration, workspace: &mut Workspace) -> Walked {
        let mut slices = Vec::new();
        for planned in &plan.slices {
            let run = self.work(&from_plan(planned, timeout), workspace);
            let done = run.is_done();
            slices.push((planned.clone(), run));
            if !done {
                break;
            }
        }
        Walked { slices }
    }
}

fn from_plan(planned: &PlannedSlice, timeout: Duration) -> Slice {
    Slice {
        task: planned.task.clone(),
        artefact: planned.artefact.clone(),
        oracle: Oracle {
            argv: planned.oracle.clone(),
            timeout,
            // QA sets it.
            expect_cases: None,
        },
    }
}
