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
//!   (ADR-0051, ADR-0057);
//! - a turn starts at a floor of steps rather than a guessed ceiling, and the
//!   floor only grows where turns are seen running out (ADR-0062).
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

// A spend is a sum of turns, so it lives beside `Turn` and a planner — which
// is one dispatch and no loop — can report one without depending on the doer.
pub use corifeu_provider::Spend;
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
    /// The files this slice is about, pinned into every step and checkpointed
    /// together. One entry is the ordinary case; a feature that changes a
    /// component and its test in one go is why it is a list (S4).
    pub artefacts: Vec<String>,
    pub oracle: Oracle,
}

impl Slice {
    /// The artefact a message means when it names just one, which is every
    /// message written before a slice could name more than one.
    pub fn artefact(&self) -> &str {
        self.artefacts.first().map_or("", String::as_str)
    }
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

/// How many steps a turn gets, and what makes that grow (ADR-0062).
///
/// The fourth run measured 57 slices that reached green and 12 that did not.
/// Thirty-eight of the green ones closed in one step and the slowest in twelve;
/// eleven of the stuck ones ran past twelve and bought nothing. A ceiling set
/// where the stuck ones die is therefore paid for by every slice that finishes,
/// and a ceiling set where the green ones finish is a guess about work nobody
/// has measured yet. So it is not set: a turn starts at the floor, and the
/// floor is only left behind where it is shown to be too low.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Steps {
    /// What every turn starts with.
    pub floor: u32,
    /// What the allowance may grow to. Never exceeded.
    pub ceiling: u32,
    /// Turns cut at the current allowance before it grows. Zero never grows.
    pub patience: u32,
    /// What one growth adds.
    pub raise_by: u32,
}

impl Default for Steps {
    fn default() -> Self {
        Self {
            floor: 12,
            ceiling: 30,
            patience: 2,
            raise_by: 6,
        }
    }
}

impl Steps {
    /// One number and no growth, for a caller that wants a flat budget.
    pub fn fixed(steps: u32) -> Self {
        Self {
            floor: steps,
            ceiling: steps,
            patience: 0,
            raise_by: 0,
        }
    }
}

/// The allowance as it stands, and what it has seen. One per run.
///
/// Growth is earned by one outcome and no other. A turn that spent every step
/// it had may have been close to finishing. A turn that went in circles was
/// never short of steps, and counting it would hand the larger allowance to
/// exactly the runs that waste it.
#[derive(Debug, Clone, Copy)]
pub struct Allowance {
    steps: Steps,
    now: u32,
    cuts: u32,
}

impl Allowance {
    pub fn new(steps: Steps) -> Self {
        Self {
            now: steps.floor.max(1),
            cuts: 0,
            steps,
        }
    }

    /// What the next turn gets.
    pub fn now(&self) -> u32 {
        self.now
    }

    /// Whether the floor has been left behind, which is worth reporting: it
    /// says this workload needs more than the configured default.
    pub fn raised(&self) -> bool {
        self.now > self.steps.floor
    }

    /// Record how a turn ended. Returns the new allowance when it grew.
    pub fn saw(&mut self, outcome: &Outcome) -> Option<u32> {
        match outcome {
            // The floor was enough. Nothing is carried into the next turn.
            Outcome::Reached => {
                self.now = self.steps.floor.max(1);
                self.cuts = 0;
                None
            }
            Outcome::Exhausted { .. } => {
                self.cuts += 1;
                if self.steps.patience == 0
                    || self.cuts < self.steps.patience
                    || self.now >= self.steps.ceiling
                {
                    return None;
                }
                self.cuts = 0;
                self.now = self
                    .now
                    .saturating_add(self.steps.raise_by)
                    .min(self.steps.ceiling);
                Some(self.now)
            }
            // Neither says the allowance was too small. A loop is the model
            // going in circles inside it, and an unusable oracle or an absent
            // provider means the slice never got to use it.
            Outcome::Looping | Outcome::ProviderFailed { .. } | Outcome::OracleUnusable { .. } => {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub steps: Steps,
    /// Characters of command output a step may carry (ADR-0049 §2).
    pub output_budget: usize,
    /// Consecutive repeats of one action before the turn stops (ADR-0050).
    ///
    /// Six is what three measurement rounds ran with, and the fifth round
    /// priced it: across three cells and two languages, 54 slices repeated an
    /// action and 2 reached green, while the steps spent after a first repeat
    /// were 30% of everything the round spent. Whether the right number is two
    /// or six is a measurement, not an argument, so it is a number here rather
    /// than a constant in the detector.
    pub repeats: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            steps: Steps::default(),
            output_budget: 12_000,
            // ADR-0050's measured default, kept so that changing the limit is
            // a decision someone takes rather than one they inherit.
            repeats: 6,
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
    /// The slice's own check could not be counted before any step was spent.
    ///
    /// Defect 24. A check that cannot execute looks exactly like a check that
    /// failed, so a doer repairs correct code against a verdict that was never
    /// about the code. In one measurement cell eight slices spent their whole
    /// allowance this way: `cargo` was answering "current package believes it's
    /// in a workspace when it's not" and the doer read it as red.
    ///
    /// `Report::unguarded` has always said this and nothing acted on it.
    OracleUnusable { detail: String },
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
    /// What this run spent, and on whose account (ADR-0031).
    pub spend: Spend,
    /// A fingerprint of what the model was shown, one per step.
    ///
    /// ADR-0066 wants a round replayable, and a round that keeps its trace but
    /// not the text behind it cannot say *where* two rounds parted. Two hours
    /// of round eight went on inferring that from the code because this column
    /// did not exist. The hash is for comparing two runs of one binary, which
    /// is the question it answers; it is not stable across Rust versions.
    pub prompts: Vec<u64>,
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
    /// One turn at the configured floor.
    pub fn work(&self, slice: &Slice, workspace: &mut Workspace) -> Run {
        self.work_within(slice, self.limits.steps.floor, workspace)
    }

    /// One turn with the allowance a caller is holding (ADR-0062).
    pub fn work_within(&self, slice: &Slice, steps: u32, workspace: &mut Workspace) -> Run {
        let steps = steps.max(1);
        // Defect 24, before the first dispatch: a slice that declares a case
        // count and whose check cannot produce one is not red, it is unrunnable.
        // A stubbed artefact still runs its cases and fails them, so this does
        // not fire on work that is merely unfinished.
        if let Some(detail) = self.oracle_unusable(slice, workspace) {
            return State::for_limits(&self.limits).finish(0, Outcome::OracleUnusable { detail });
        }
        // The workspace has to know what this slice is before the first write,
        // or a checkpoint saves one file out of a set and a rollback restores a
        // combination nothing was scored on (S4, ADR-0051 scoped per slice).
        workspace.stage(&slice.artefacts);
        let contract = Contract {
            task: slice.task.clone(),
            acceptance: slice.oracle.describe(),
        };
        let mut state = State::for_limits(&self.limits);

        for number in 1..=steps {
            let context = self
                .window
                .build(&contract, self.pinned(slice, workspace), &state.trace);
            let rendered = self.render(&context);
            state.prompts.push(fingerprint(&rendered));
            if let Some(dir) = dump_dir() {
                dump(&dir, slice, number, &rendered);
            }
            let turn =
                match self
                    .provider
                    .chat(&self.model, &rendered, &tools::schemas(), &self.budget)
                {
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
            state.spend.saw(&turn);

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
                // The outcome carries the model's own words and the result
                // carries the nudge, so the trace records what was said while
                // the next step's context does not pay to read it back. Defect
                // 27: without this a wasted step is indistinguishable from any
                // other wasted step, and a prompt cannot be designed against
                // "nothing happened" five hundred times.
                state.trace.push(
                    Step::new(number, Action::new("say", ""), nudge)
                        .with_outcome(format!("no tool call; said: {}", said(&turn.content))),
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
        state.finish(steps, Outcome::Exhausted { last })
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

    /// Why this slice's check cannot be trusted to judge anything, if so.
    ///
    /// Only asked of a slice that declares a case count. Where nobody has said
    /// how many cases to expect — QA writing a test that does not exist yet —
    /// there is nothing to compare against and the question is meaningless.
    fn oracle_unusable(&self, slice: &Slice, workspace: &Workspace) -> Option<String> {
        let expect_cases = slice.oracle.expect_cases?;
        let Some(command) = Command::from_argv(&slice.oracle.argv) else {
            return Some("the slice's oracle names no program".to_string());
        };
        let observed = match corifeu_exec::run(
            &command,
            workspace.root(),
            slice.oracle.timeout,
            &self.allowlist,
        ) {
            Ok(observed) => observed,
            Err(error) => return Some(format!("the oracle could not run: {error}")),
        };
        let report = Check {
            expect_cases: Some(expect_cases),
            output_budget: self.limits.output_budget,
        }
        .judge(observed);
        report.unguarded.then(|| {
            format!(
                "`{}` produced no case count, so it cannot say whether the {expect_cases} cases \
                 this slice declares passed. The check itself is broken, not the code under it.",
                slice.oracle.describe()
            )
        })
    }

    /// Every artefact that exists, in the order the slice named them. One that
    /// does not exist yet is left out rather than shown empty: a doer told a
    /// file is missing writes it, and a doer shown an empty body edits it.
    fn pinned(&self, slice: &Slice, workspace: &Workspace) -> Vec<Artefact> {
        slice
            .artefacts
            .iter()
            .filter_map(|path| {
                workspace.read(path).ok().flatten().map(|body| Artefact {
                    path: path.clone(),
                    body,
                })
            })
            .collect()
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
                // Defect 34. ADR-0050 §4 clears the repeat history when an
                // artefact changes, because the same command against changed
                // code is a different action. Writing the same bytes back is
                // not changed code, and counting it as one disabled loop
                // detection exactly where it was needed: a model rewriting an
                // identical file forever reset the detector on every step and
                // was never warned.
                let unchanged = workspace
                    .read(&path)
                    .ok()
                    .flatten()
                    .is_some_and(|before| before == content);
                match workspace.write(&path, &content, &self.guard) {
                    Ok(()) => {
                        if !unchanged {
                            state.detector.artefact_changed();
                        }
                        state.last_result = match unchanged {
                            // Said out loud, because a model that cannot tell
                            // its write landed will write it again.
                            true => format!(
                                "wrote {} bytes to {path}, which is byte for byte what was \
                                 already there. Nothing changed.\n",
                                content.len()
                            ),
                            false => format!("wrote {} bytes to {path}\n", content.len()),
                        };
                        // The runtime checks; the model does not have to think of it.
                        let wrote = state.last_result.clone();
                        let green = self.judge(slice, workspace, state);
                        state.last_result = format!("{wrote}\n{}", state.last_result);
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
        if let Some(better) = workspace.regression(&scope, score) {
            let id = better.id;
            if let Ok(restored) = workspace.restore(id) {
                note = format!(
                    "\n\nROLLED BACK: this change scored worse than a state already saved, so \
                     {} was restored to it. You had {} passing; it has {}. The files above are \
                     the restored ones.",
                    restored.describe(),
                    score.cases_passing,
                    restored.regained(&scope).map_or(0, |s| s.cases_passing),
                );
            }
        } else {
            workspace.score(&scope, score);
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
        // A slice can name more than one file (S4), and a prompt that says
        // "one file" to a doer holding two is telling it something false about
        // its own task. One `write_file` still replaces one file, which is the
        // part that has to stay unambiguous.
        let (subject, pinned) = match context.artefacts.len() {
            0 | 1 => (
                "one file",
                "The file you are changing is included below in full on every step, read \
                 fresh from disk. You never need to read it.",
            ),
            _ => (
                "a small set of files that change together",
                "Every file you are changing is included below in full on every step, read \
                 fresh from disk. You never need to read any of them. Each `write_file` \
                 replaces one of them, so changing two takes two steps.",
            ),
        };
        let mut messages = vec![Message::system(format!(
            "You are changing {subject} until one command passes.\n\n\
             Task: {}\n\
             Done when this passes: {}\n\n\
             Rules:\n\
             - Every answer is a tool call. Prose is discarded unread and the step is spent, \
             so explaining what you will do costs you the step you would have done it in.\n\
             - `run` takes an argument vector. There is no shell: no pipes, no `&&`, no \
             redirection, no globs.\n\
             - `write_file` replaces the whole file. Send all of it, never a patch.\n\
             - {pinned}\n\
             - The command above is run for you after every write. You do not have to run it.\n\
             - Be exhaustive about what the task asks for. Inputs that are malformed in ways a \
             naive implementation would accept must still be handled, and the cases a naive \
             implementation would get wrong are the ones the check is looking for.",
            context.contract.task, context.contract.acceptance
        ))];

        for artefact in &context.artefacts {
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
        // Defect 52. This used to put one assistant message per past step
        // reading `I ran: <action>`, which is a worked example of answering in
        // prose sitting in the slot the model is about to fill. It copied it:
        // in one hybrid run ten of twelve steps answered `no tool call; said:
        // I ran: write_file src/excerpt.ts` and the slice never closed. Under
        // ADR-0047 there is no conversation to reconstruct — the context is
        // rebuilt from state every step — so a past step is carried as its
        // result alone, with the action naming it, and nothing in the
        // transcript shows an assistant answering without a call.
        for step in &context.recent {
            messages.push(Message::tool(format!(
                "step {}, `{}`:\n{}",
                step.number,
                step.action.describe(),
                step.result
            )));
        }
        messages
    }
}

/// The model's own words, flattened to one line and cut to a sample.
///
/// Enough to tell one kind of wasted step from another — a refusal, a plan, a
/// diff in prose, a repetition of the task — and short enough that a trace of
/// a thousand steps stays a file someone opens.
fn said(content: &str) -> String {
    let flat = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = flat.chars().take(160).collect();
    if cut.is_empty() {
        "(nothing at all)".to_string()
    } else if flat.chars().count() > 160 {
        format!("{cut}…")
    } else {
        cut
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
    prompts: Vec<u64>,
    spend: Spend,
}

impl State {
    fn for_limits(limits: &Limits) -> Self {
        Self {
            spend: Spend::default(),
            prompts: Vec::new(),
            detector: Detector::with_strikes(limits.repeats),
            ..Self::default()
        }
    }

    fn finish(self, steps: u32, outcome: Outcome) -> Run {
        Run {
            outcome,
            steps,
            trace: self.trace,
            prompt_tokens: self.prompt_tokens,
            output_tokens: self.output_tokens,
            checks: self.checks,
            cases: self.last_cases,
            prompts: self.prompts,
            spend: self.spend,
            verdict: self.last_verdict,
        }
    }
}

/// Re-exported so a caller can build a slice without naming five crates.
pub use corifeu_provider::ToolCall;

/// A fingerprint of the messages a step sent.
///
/// `DefaultHasher` rather than a cryptographic digest and rather than a new
/// dependency: the question is whether two runs of one binary sent the same
/// bytes, and for that an equality check is the whole requirement.
fn fingerprint(messages: &[Message]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for message in messages {
        serde_json::to_string(message)
            .unwrap_or_default()
            .hash(&mut hasher);
    }
    hasher.finish()
}

/// Where to write the prompts themselves, when something asked for them.
///
/// A fingerprint says which step two rounds parted on. It does not say what
/// changed, and that is the question after it, so the text is available behind
/// an environment variable rather than kept for every round.
fn dump_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("CORIFEU_DUMP_PROMPTS").map(std::path::PathBuf::from)
}

fn dump(dir: &std::path::Path, slice: &Slice, step: u32, messages: &[Message]) {
    let scope: String = slice
        .oracle
        .describe()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let scope: String = scope
        .chars()
        .rev()
        .take(40)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let body = messages
        .iter()
        .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(dir.join(format!("{scope}-{step:03}.json")), body);
}
