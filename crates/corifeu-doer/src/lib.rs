//! Works one slice until its oracle goes green, or says why it did not.
//!
//! This is where the decisions meet, and the shape of the loop is the shape of
//! what the measurement found:
//!
//! - the working file is read from disk every step and never recalled from the
//!   history, because bounding the history evicts the thing being edited
//!   (ADR-0047);
//! - a repeated action is answered with a warning that points at the oracle,
//!   and only six in a row stop the run (ADR-0050);
//! - commands are argument vectors behind an allowlist, so a pipeline cannot
//!   mask an exit status and a heredoc cannot go around a checkpoint
//!   (ADR-0037, ADR-0060);
//! - a write is checkpointed first and refused if it would delete the suite
//!   (ADR-0051, ADR-0057).
//!
//! # The runtime runs the oracle, not the model
//!
//! After every write that lands, the oracle runs. Three things follow that do
//! not otherwise work: a green slice is noticed the moment it happens rather
//! than when the model thinks to check; every saved state carries a score, so
//! "the best state so far" is always known (ADR-0051 §2); and a round that made
//! things worse is reversible, which the measurement's repair round needed and
//! did not have.

pub mod pair;
pub mod tools;

use corifeu_agent::{Action, Artefact, Context, Contract, Detector, Repeat, Step, Window};
use corifeu_check::{Cases, Check, Verdict};
use corifeu_exec::{Allowlist, Command};
use corifeu_provider::{Budget, Message, Provider, Stop};
use corifeu_workspace::{Guard, Score, Workspace};
use std::time::Duration;

/// The one command whose red-to-green transition ends the slice (ADR-0056).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oracle {
    pub argv: Vec<String>,
    pub timeout: Duration,
    /// Cases it must run. `None` leaves ADR-0057's guard off, which is the only
    /// honest setting for a runner whose output cannot be counted.
    pub expect_cases: Option<u32>,
}

impl Oracle {
    pub fn describe(&self) -> String {
        self.argv.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slice {
    pub task: String,
    /// The file this slice is about, pinned into every step.
    pub artefact: String,
    pub oracle: Oracle,
}

/// What ends a role's turn on a slice.
///
/// ADR-0053 puts QA before the coder, and they have different finish lines.
///
/// The coder is done when the check passes. QA is done when the check *runs* —
/// which is not the same as failing. A test that will not compile is broken,
/// not red (ADR-0059 §4), and handing the coder one is handing it nothing to
/// work against. A test that passes the moment it is written is also finished
/// QA work: it says the thing was already built, by an earlier slice's coder
/// exceeding its boundary (ADR-0058). Both are results QA has to see, and
/// neither of them is "keep writing".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    /// The coder's.
    Green,
    /// QA's: at least one case executed, whatever it said.
    TestRuns,
}

impl Goal {
    fn reached(self, verdict: &Verdict, cases: &Cases) -> bool {
        match self {
            Goal::Green => verdict.is_green(),
            Goal::TestRuns => {
                matches!(verdict, Verdict::Passed | Verdict::Failed)
                    && cases.ran().is_some_and(|ran| ran > 0)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_steps: u32,
    /// Characters of command output a step may carry (ADR-0049 §2).
    pub output_budget: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_steps: 60,
            output_budget: 12_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The role's goal was met: green for a coder, a running test for QA.
    Reached,
    /// The step budget ran out with the oracle still not green.
    Exhausted { last: Option<Verdict> },
    /// Six consecutive repeats (ADR-0050).
    Looping,
    /// The provider could not be reached, or answered something unusable.
    ProviderFailed { why: String },
}

#[derive(Debug, Clone)]
pub struct Run {
    pub outcome: Outcome,
    pub steps: u32,
    pub trace: Vec<Step>,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
    /// How many times the oracle ran, which is what the slice really cost in
    /// machine time next to the model's own.
    pub checks: u32,
    /// What the last oracle run counted. This is how QA hands a case count to
    /// the coder's guard (ADR-0057): nobody knows how many tests a slice has
    /// until they are written.
    pub cases: Cases,
    /// What the last oracle run said. For QA this is the difference between a
    /// red test, which is work for the coder, and one that passed the moment it
    /// was written, which says the work was already done.
    pub verdict: Option<Verdict>,
}

impl Run {
    pub fn reached(&self) -> bool {
        self.outcome == Outcome::Reached
    }
}

pub struct Doer<'a> {
    pub provider: &'a dyn Provider,
    pub goal: Goal,
    pub model: String,
    pub budget: Budget,
    pub allowlist: Allowlist,
    pub guard: Guard,
    pub window: Window,
    pub limits: Limits,
}

impl Doer<'_> {
    pub fn work(&self, slice: &Slice, workspace: &mut Workspace) -> Run {
        let contract = Contract {
            task: slice.task.clone(),
            acceptance: slice.oracle.describe(),
        };
        let mut state = State::default();

        for number in 1..=self.limits.max_steps {
            let context = self
                .window
                .build(&contract, self.pinned(slice, workspace), &state.trace);
            let turn = match self.provider.chat(
                &self.model,
                &self.render(&context),
                &tools::schemas(),
                &self.budget,
            ) {
                Ok(turn) => turn,
                Err(error) => {
                    return state.finish(
                        number - 1,
                        Outcome::ProviderFailed {
                            why: error.to_string(),
                        },
                    );
                }
            };
            state.prompt_tokens += turn.prompt_tokens;
            state.output_tokens += turn.output_tokens;

            let Some(call) = turn.tool_calls.first() else {
                // ADR-0048 §4: a spent budget is why a turn can end mid-thought,
                // and saying so is more use to the model than silence.
                let nudge = match turn.stop {
                    Stop::BudgetSpent { num_predict } => format!(
                        "Your answer was cut off at {num_predict} tokens before you called a \
                         tool. Answer with a tool call and nothing else."
                    ),
                    _ => "You answered without calling a tool, so nothing happened. Call a \
                          tool."
                        .to_string(),
                };
                state.trace.push(
                    Step::new(number, Action::new("say", ""), nudge)
                        .with_outcome("no tool call; nothing happened"),
                );
                continue;
            };

            let action = Action::new(
                call.name.clone(),
                tools::identifying_argument(&call.name, &call.arguments),
            );
            let result = match state.detector.consider(&action) {
                Repeat::Spent { .. } => return state.finish(number, Outcome::Looping),
                Repeat::Warned { warning, .. } => warning,
                Repeat::New => {
                    let done = self.perform(call, slice, workspace, &mut state);
                    if done {
                        state
                            .trace
                            .push(Step::new(number, action, state.last_result.clone()));
                        return state.finish(number, Outcome::Reached);
                    }
                    state.last_result.clone()
                }
            };

            let mut step = Step::new(number, action, result);
            if turn.tool_calls.len() > 1 {
                // Running the rest blindly would act on state the first call
                // may have changed. Say so instead.
                let noted = format!(
                    "{} (only the first of {} calls ran; the state may have changed)",
                    step.outcome,
                    turn.tool_calls.len()
                );
                step = step.with_outcome(noted);
            }
            state.trace.push(step);
        }

        let last = state.last_verdict;
        state.finish(self.limits.max_steps, Outcome::Exhausted { last })
    }

    /// Run a slice's oracle once, without a model, and say whether it passes
    /// with at least one case. Used to tell "this was already done" from "the
    /// role that judged it also did it".
    pub fn probe(&self, slice: &Slice, workspace: &Workspace) -> bool {
        let Some(command) = Command::from_argv(&slice.oracle.argv) else {
            return false;
        };
        let Ok(observed) = corifeu_exec::run(
            &command,
            workspace.root(),
            slice.oracle.timeout,
            &self.allowlist,
        ) else {
            return false;
        };
        let report = Check {
            expect_cases: None,
            output_budget: self.limits.output_budget,
        }
        .judge(observed);
        report.verdict.is_green() && report.cases.ran().is_some_and(|ran| ran > 0)
    }

    fn pinned(&self, slice: &Slice, workspace: &Workspace) -> Option<Artefact> {
        workspace
            .read(&slice.artefact)
            .ok()
            .flatten()
            .map(|body| Artefact {
                path: slice.artefact.clone(),
                body,
            })
    }

    /// Returns whether the oracle is green.
    fn perform(
        &self,
        call: &corifeu_provider::ToolCall,
        slice: &Slice,
        workspace: &mut Workspace,
        state: &mut State,
    ) -> bool {
        match call.name.as_str() {
            tools::RUN => {
                let Some(argv) = tools::argv(&call.arguments) else {
                    state.last_result =
                        "`argv` must be a non-empty array of strings. There is no shell."
                            .to_string();
                    return false;
                };
                // A prefix, not equality: `cargo test --lib x` and
                // `cargo test --lib x -- --nocapture` are the same check, and
                // a model that adds a flag should still get the judging,
                // scoring and rollback rather than a bare command run.
                if argv.starts_with(&slice.oracle.argv) {
                    return self.judge(slice, workspace, state);
                }
                state.last_result = self.run(&argv, workspace);
                false
            }
            tools::WRITE_FILE => {
                let (Some(path), Some(content)) = (
                    tools::string(&call.arguments, "path"),
                    tools::string(&call.arguments, "content"),
                ) else {
                    state.last_result = "`path` and `content` are both required.".to_string();
                    return false;
                };
                match workspace.write(&path, &content, &self.guard) {
                    Ok(()) => {
                        state.detector.artefact_changed();
                        state.last_result = format!("wrote {} bytes to {path}\n", content.len());
                        // The runtime checks; the model does not have to think of it.
                        let green = self.judge(slice, workspace, state);
                        state.last_result = format!(
                            "wrote {} bytes to {path}\n\n{}",
                            content.len(),
                            state.last_result
                        );
                        green
                    }
                    Err(refused) => {
                        state.last_result = format!("REFUSED: {refused}");
                        false
                    }
                }
            }
            tools::READ_FILE => {
                let Some(path) = tools::string(&call.arguments, "path") else {
                    state.last_result = "`path` is required.".to_string();
                    return false;
                };
                state.last_result = match workspace.read(&path) {
                    Ok(Some(body)) => corifeu_check::truncate(&body, self.limits.output_budget),
                    Ok(None) => format!("{path} does not exist"),
                    Err(error) => format!("{error}"),
                };
                false
            }
            other => {
                state.last_result = format!("there is no tool called `{other}`");
                false
            }
        }
    }

    fn run(&self, argv: &[String], workspace: &Workspace) -> String {
        let Some(command) = Command::from_argv(argv) else {
            return "`argv` must name a program.".to_string();
        };
        match corifeu_exec::run(&command, workspace.root(), slice_timeout(), &self.allowlist) {
            Ok(observed) => {
                let check = Check {
                    expect_cases: None,
                    output_budget: self.limits.output_budget,
                };
                let report = check.judge(observed);
                format!("{}\n{}", report.verdict, report.output)
            }
            Err(error) => format!("{error}"),
        }
    }

    /// Run the slice's oracle, score the state, and roll back a regression.
    fn judge(&self, slice: &Slice, workspace: &mut Workspace, state: &mut State) -> bool {
        state.checks += 1;
        let Some(command) = Command::from_argv(&slice.oracle.argv) else {
            state.last_result = "the slice's oracle names no program".to_string();
            return false;
        };
        let observed = match corifeu_exec::run(
            &command,
            workspace.root(),
            slice.oracle.timeout,
            &self.allowlist,
        ) {
            Ok(observed) => observed,
            Err(error) => {
                state.last_result = format!("the oracle could not run: {error}");
                return false;
            }
        };

        let report = Check {
            expect_cases: slice.oracle.expect_cases,
            output_budget: self.limits.output_budget,
        }
        .judge(observed);

        let score = Score {
            green: report.verdict.is_green(),
            cases_passing: passing(&report.cases, &report.verdict),
        };
        // Scored under this slice's own check. A count from another slice's
        // oracle is a different question and comparing the two reads every
        // smaller check as a regression.
        let scope = slice.oracle.describe();
        let mut note = String::new();
        // ADR-0051 protects a coder's progress, which is measured by how much
        // passes. QA is working the other way, and scoring its states by the
        // same number would read every new failing test as a regression.
        if self.goal == Goal::TestRuns {
            state.last_verdict = Some(report.verdict);
            state.last_cases = report.cases;
            state.last_result = format!(
                "$ {}\n{}\n{}",
                slice.oracle.describe(),
                report.verdict,
                report.output
            );
            return self.goal.reached(&report.verdict, &report.cases);
        }
        if let Some(better) = workspace.regression(&slice.artefact, &scope, score) {
            let id = better.id;
            if let Ok(restored) = workspace.restore(id) {
                note = format!(
                    "\n\nROLLED BACK: this change scored worse than a state already saved, so \
                     {} was restored to it. You had {} passing; it has {}. The file above is \
                     the restored one.",
                    restored.path,
                    score.cases_passing,
                    restored.regained(&scope).map_or(0, |s| s.cases_passing),
                );
            }
        } else {
            workspace.score(&slice.artefact, &scope, score);
        }

        state.last_verdict = Some(report.verdict);
        state.last_cases = report.cases;
        state.last_result = format!(
            "$ {}\n{}\n{}{}",
            slice.oracle.describe(),
            state
                .last_verdict
                .as_ref()
                .map_or(String::new(), ToString::to_string),
            report.output,
            note
        );
        self.goal.reached(&report.verdict, &report.cases)
    }

    fn render(&self, context: &Context) -> Vec<Message> {
        let mut messages = vec![Message::system(format!(
            "You are changing one file until one command passes.\n\n\
             Task: {}\n\
             Done when this passes: {}\n\n\
             Rules:\n\
             - `run` takes an argument vector. There is no shell: no pipes, no `&&`, no \
             redirection, no globs.\n\
             - `write_file` replaces the whole file. Send all of it, never a patch.\n\
             - The file you are changing is included below in full on every step, read fresh \
             from disk. You never need to read it.\n\
             - The command above is run for you after every write. You do not have to run it.\n\
             - Answer with a tool call, not with prose.",
            context.contract.task, context.contract.acceptance
        ))];

        if let Some(artefact) = &context.artefact {
            messages.push(Message::user(format!(
                "{}\n\n```\n{}\n```",
                artefact.preamble(),
                artefact.body
            )));
        }
        if !context.compacted.is_empty() {
            messages.push(Message::user(format!(
                "What you have already done:\n{}",
                context.compacted.join("\n")
            )));
        }
        for step in &context.recent {
            messages.push(Message::assistant(format!(
                "I ran: {}",
                step.action.describe()
            )));
            messages.push(Message::tool(step.result.clone()));
        }
        messages
    }
}

/// A plain `run` is not the oracle, so it gets a bounded deadline of its own
/// rather than the slice's, which may be minutes.
fn slice_timeout() -> Duration {
    Duration::from_secs(120)
}

/// What a state is worth, for ADR-0051's comparison.
///
/// The runner reports how many cases passed even when the run failed, and that
/// number is the whole point: a repair that goes from 18 passing to 3 is a
/// regression, and scoring every failing run as zero cannot see it. That was
/// the measurement's worst round and this is what would have caught it.
fn passing(cases: &Cases, verdict: &Verdict) -> u32 {
    match verdict {
        // Nothing ran, so nothing was learned about any case.
        Verdict::DidNotTerminate { .. } => 0,
        _ => cases.passed().unwrap_or(0),
    }
}

#[derive(Default)]
struct State {
    trace: Vec<Step>,
    detector: Detector,
    last_result: String,
    last_verdict: Option<Verdict>,
    last_cases: Cases,
    prompt_tokens: u32,
    output_tokens: u32,
    checks: u32,
}

impl State {
    fn finish(self, steps: u32, outcome: Outcome) -> Run {
        Run {
            outcome,
            steps,
            trace: self.trace,
            prompt_tokens: self.prompt_tokens,
            output_tokens: self.output_tokens,
            checks: self.checks,
            cases: self.last_cases,
            verdict: self.last_verdict,
        }
    }
}

/// Re-exported so a caller can build a slice without naming five crates.
pub use corifeu_provider::ToolCall;
