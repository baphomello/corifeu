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
//! # Why an unfinished slice is not a missing deliverable
//!
//! The fourth run ended nineteen times with the whole suite green, and eight of
//! those runs held a slice whose own check never passed. The work was there;
//! the slice next to it had written it on the way past. So the walk carries on
//! past a failure and asks the check once more at the end ([ADR-0063]).
//!
//! [ADR-0057]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0057-oracle-integrity-by-test-inventory.md
//! [ADR-0063]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0063-an-unfinished-slice-is-asked-again.md
//! [ADR-0058]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0058-slice-boundaries-are-observed.md

use crate::{Allowance, Doer, Oracle, Outcome, Run, Slice};
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

/// What a walk does when a slice does not close (ADR-0063, D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Carry {
    /// Work every slice in the plan regardless. ADR-0063 as measured, and the
    /// default because it is the only one with data behind it.
    #[default]
    Always,
    /// Stop at the first slice that does not close. The behaviour ADR-0063
    /// replaced, kept so the reversal can be re-measured rather than argued.
    Never,
    /// Carry on until `patience` slices in a row have failed with no sign that
    /// this plan's slices overlap, then stop.
    ///
    /// **A hypothesis, not a finding.** The fifth round measured the re-probe
    /// rescuing 66% of unclosed slices in a chained Rust plan, 20% in the same
    /// plan in Python and 2% in a plan whose slices call nothing, which says
    /// carrying on is worth very different amounts depending on the plan's
    /// shape. It does not say this rule detects the difference, and one case in
    /// that round argues it does not: runs that closed one slice of four and
    /// had three rescued delivered the complete crate, and those three failures
    /// came before any overlap had shown itself. Under this policy with a
    /// patience of two they would have been abandoned.
    ///
    /// The signal that would actually decide it is whether a later slice
    /// depends on an earlier one, which is a property of the plan rather than
    /// of anything observable while walking it. ADR-0056's planner could
    /// declare it and has never cut up a task.
    UntilOverlap { patience: usize },
}

impl Pair<'_> {
    /// One slice, with an allowance of its own.
    pub fn work(&self, slice: &Slice, workspace: &mut Workspace) -> SliceRun {
        let mut allowance = Allowance::new(self.qa.limits.steps);
        self.work_within(slice, &mut allowance, workspace)
    }

    /// One slice, against an allowance the caller carries across the plan.
    ///
    /// Both roles draw on the same allowance, because the question it answers
    /// — is this workload harder than the floor assumes? — is about the run,
    /// not about the role (ADR-0062).
    pub fn work_within(
        &self,
        slice: &Slice,
        allowance: &mut Allowance,
        workspace: &mut Workspace,
    ) -> SliceRun {
        let mut qa = Vec::new();
        let mut coder = Vec::new();
        // What the check said before anyone touched it. A green check after QA
        // means two different things depending on this, and the run has no way
        // to tell them apart afterwards.
        let green_before = self.qa.probe(slice, workspace);

        for round in 1..=self.qa_rounds.max(1) {
            let qa_run = self
                .qa
                .work_within(&qa_slice(slice, round), allowance.now(), workspace);
            allowance.saw(&qa_run.outcome);
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

            let coder_run =
                self.coder
                    .work_within(&coder_slice(slice, cases), allowance.now(), workspace);
            allowance.saw(&coder_run.outcome);
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
        artefacts: slice.artefacts.clone(),
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
        artefacts: slice.artefacts.clone(),
        oracle: Oracle {
            expect_cases: Some(cases),
            ..slice.oracle.clone()
        },
    }
}

// ------------------------------------------------------------------ the walk

/// A plan, walked, and what the checks said at the end of it.
///
/// A slice that does not close is not the same as work that is missing. The
/// fourth run measured both: of nineteen runs that ended with the whole suite
/// green, **eight** held a slice whose own check never passed. The work was
/// there — an adjacent slice's coder had written it on the way past
/// ([ADR-0058]) — and the run called itself failed anyway, and spent four times
/// the budget of a clean run doing it (defect 18).
///
/// So the walk asks twice. Every slice that did not close is checked once more
/// at the end, when the rest of the plan has run. What is green then was late,
/// not missing, and only what is still red goes back to the planner.
///
/// [ADR-0058]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0058-slice-boundaries-are-observed.md
#[derive(Debug, Clone)]
pub struct Walked {
    pub slices: Vec<(PlannedSlice, SliceRun)>,
    /// Every slice that did not close, by its index in `slices`, and what its
    /// oracle said once the rest of the plan had run.
    pub asked_again: Vec<(usize, bool)>,
    /// How many slices the plan held. `slices` is shorter only when the walk
    /// was abandoned, which happens for one reason: the provider is gone.
    pub planned: usize,
}

impl Walked {
    /// Every slice closed under its own role, with nothing to ask twice.
    pub fn all_done(&self) -> bool {
        self.slices.len() == self.planned && self.asked_again.is_empty()
    }

    /// The walk did not reach the end of the plan. Two things cause it, and
    /// neither is a verdict on the work: the provider went away, or the
    /// project's own check cannot run.
    pub fn abandoned(&self) -> bool {
        self.slices.len() != self.planned
    }

    /// The work exists. Either every slice closed, or the ones that did not
    /// are green now that the rest of the plan has run.
    pub fn delivered(&self) -> bool {
        self.slices.len() == self.planned && self.asked_again.iter().all(|(_, green)| *green)
    }

    /// Slices that did not close and are still red. These, and only these, are
    /// evidence that the decomposition was wrong (ADR-0056).
    pub fn stalled(&self) -> impl Iterator<Item = &(PlannedSlice, SliceRun)> {
        self.asked_again
            .iter()
            .filter(|(_, green)| !*green)
            .map(|(at, _)| &self.slices[*at])
    }

    /// Slices whose check never passed under their own role and passes now.
    /// This is ADR-0058's overflow, counted rather than argued about.
    pub fn overtaken(&self) -> usize {
        self.asked_again.iter().filter(|(_, green)| *green).count()
    }

    pub fn prompt_tokens(&self) -> u32 {
        self.slices.iter().map(|(_, r)| r.prompt_tokens()).sum()
    }

    pub fn output_tokens(&self) -> u32 {
        self.slices.iter().map(|(_, r)| r.output_tokens()).sum()
    }
}

impl Pair<'_> {
    /// Work a plan in order, then ask the checks of whatever did not close.
    ///
    /// A failed slice no longer ends the walk. It used to, on the reasoning
    /// that slices may depend on each other so a failure early invalidates what
    /// follows. The fourth run put a number on that reasoning and it came out
    /// the other way: carrying on is what produced the complete crate in eight
    /// of nineteen runs. Giving up early would have thrown away work that was
    /// about to be done by the next slice.
    ///
    /// Two things still end the walk, because nothing after them can succeed:
    /// the provider being unreachable, and the project's check being
    /// uncountable.
    pub fn walk(&self, plan: &Plan, timeout: Duration, workspace: &mut Workspace) -> Walked {
        self.walk_with(plan, Carry::default(), timeout, workspace)
    }

    /// [`Pair::walk`] with the carry-on policy named rather than defaulted.
    pub fn walk_with(
        &self,
        plan: &Plan,
        carry: Carry,
        timeout: Duration,
        workspace: &mut Workspace,
    ) -> Walked {
        let mut allowance = Allowance::new(self.qa.limits.steps);
        let mut slices = Vec::new();
        let (mut failures, mut overlap_seen) = (0usize, false);

        for planned in &plan.slices {
            let run = self.work_within(&from_plan(planned, timeout), &mut allowance, workspace);
            // A slice that arrived already green is this plan's slices sitting
            // in each other's line of sight. It is the only overlap a walk can
            // observe while it is still walking.
            overlap_seen |= matches!(run.outcome, SliceOutcome::DeliveredEarly { .. });
            failures = if run.is_done() { 0 } else { failures + 1 };

            let stop = pointless_to_continue(&run)
                || match carry {
                    Carry::Always => false,
                    Carry::Never => failures > 0,
                    Carry::UntilOverlap { patience } => !overlap_seen && failures >= patience,
                };
            slices.push((planned.clone(), run));
            if stop {
                break;
            }
        }

        // One oracle run per unfinished slice, and no model at all. This is the
        // cheapest question in the runtime and the fourth run showed it is the
        // one worth asking.
        let asked_again = slices
            .iter()
            .enumerate()
            .filter(|(_, (_, run))| !run.is_done())
            .map(|(at, (planned, _))| {
                let slice = from_plan(planned, timeout);
                (at, self.coder.probe(&slice, workspace))
            })
            .collect();

        Walked {
            slices,
            asked_again,
            planned: plan.slices.len(),
        }
    }
}

/// The failures that make the rest of the plan pointless.
///
/// An absent provider is the obvious one: nothing after it can succeed. An
/// unusable oracle is the same shape and was not noticed until defect 24 — the
/// slices of one plan share a project and therefore a test command, so a check
/// that cannot be counted for one slice will not be countable for the next, and
/// carrying on spends every remaining allowance to learn it again.
fn pointless_to_continue(run: &SliceRun) -> bool {
    let why = match &run.outcome {
        SliceOutcome::NoRunnableTest { why } | SliceOutcome::NotPassed { why, .. } => why,
        _ => return false,
    };
    matches!(
        why,
        Outcome::ProviderFailed { .. } | Outcome::OracleUnusable { .. }
    )
}

fn from_plan(planned: &PlannedSlice, timeout: Duration) -> Slice {
    Slice {
        task: planned.task.clone(),
        artefacts: planned.artefacts.clone(),
        oracle: Oracle {
            argv: planned.oracle.clone(),
            timeout,
            // QA sets it.
            expect_cases: None,
        },
    }
}
