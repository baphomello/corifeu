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
    /// The oracle went green.
    Green,
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
}

impl Run {
    pub fn is_green(&self) -> bool {
        self.outcome == Outcome::Green
    }
}

pub struct Doer<'a> {
    pub provider: &'a dyn Provider,
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
                        return state.finish(number, Outcome::Green);
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
                if argv == slice.oracle.argv {
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
        let mut note = String::new();
        if let Some(better) = workspace.regression(&slice.artefact, score) {
            let id = better.id;
            if let Ok(restored) = workspace.restore(id) {
                note = format!(
                    "\n\nROLLED BACK: this change scored worse than a state already saved, so \
                     {} was restored to it. You had {} passing; it has {}. The file above is \
                     the restored one.",
                    restored.path,
                    score.cases_passing,
                    restored.regained.map_or(0, |s| s.cases_passing),
                );
            }
        } else {
            workspace.score(&slice.artefact, score);
        }

        state.last_verdict = Some(report.verdict);
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
        state.last_verdict.as_ref().is_some_and(|v| v.is_green())
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

fn passing(cases: &Cases, verdict: &Verdict) -> u32 {
    match (cases, verdict) {
        (Cases::Counted(n), Verdict::Passed) => *n,
        (Cases::Counted(n), Verdict::Vacuous { ran, .. }) => (*n).min(*ran),
        // A failing run's count includes the failures, so it is not a score.
        _ => 0,
    }
}

#[derive(Default)]
struct State {
    trace: Vec<Step>,
    detector: Detector,
    last_result: String,
    last_verdict: Option<Verdict>,
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
        }
    }
}

/// Re-exported so a caller can build a slice without naming five crates.
pub use corifeu_provider::ToolCall;
