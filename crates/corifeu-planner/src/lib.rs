//! Cuts a brief into slices, each with one oracle, in a single pass.
//!
//! "Small" has to be checkable or it becomes taste. ADR-0056's definition came
//! out of the measurement: the task that worked had exactly one command whose
//! transition from failing to passing meant it was done, and that command was
//! named in the task text. Every task that was given a specification instead
//! plateaued or hung.
//!
//! # Why the whole list at once
//!
//! A planner is an expensive model. If each slice costs a planner round trip,
//! decomposition spends on planning what delegation saved on implementation and
//! the product's central claim quietly inverts. The planner is asked once; the
//! list is executed; it is consulted again only when a slice fails, because
//! that is evidence the decomposition was wrong.
//!
//! # Why a plan is checked before any of it runs
//!
//! A slice with no oracle cannot finish. Two slices sharing an oracle cannot be
//! told apart — neither can go red to green on its own. An oracle the sandbox
//! would refuse ([ADR-0037](https://github.com/baphomello/corifeu/blob/main/docs/adr/0037-sandboxed-command-execution.md))
//! can never run at all. Each of those is a planning failure, and finding it at
//! plan time costs one rejection rather than a doer round apiece.
//!
//! # What the planner does not decide
//!
//! How many cases a slice's oracle must run. That number comes from QA's red
//! run (ADR-0057), because until the test is written nobody knows how many
//! there are. A planner that guessed it would hand every slice a guard set to
//! the wrong number.

use corifeu_exec::{Allowlist, Command};
use corifeu_provider::{Budget, Message, Provider, ToolCall};
use serde_json::{Value, json};

pub const EMIT_PLAN: &str = "emit_plan";

/// What the planner is asked to cut up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    /// What the user wants built.
    pub goal: String,
    /// What the planner needs to know about the project to name real commands:
    /// the language, the test runner, the layout.
    pub context: String,
}

/// One slice as planned. It carries no case count: see the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedSlice {
    pub task: String,
    /// The file the slice is about, pinned into every step (ADR-0047).
    pub artefact: String,
    /// The one command whose red-to-green transition ends the slice.
    pub oracle: Vec<String>,
}

impl PlannedSlice {
    pub fn describe_oracle(&self) -> String {
        self.oracle.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub slices: Vec<PlannedSlice>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Rejected {
    #[error("the plan has no slices")]
    Empty,

    #[error("slice {at} ({task:?}) names no oracle, so nothing could end it")]
    NoOracle { at: usize, task: String },

    #[error("slice {at} ({task:?}) names no artefact to work on")]
    NoArtefact { at: usize, task: String },

    #[error(
        "slices {first} and {second} share the oracle `{oracle}`, so neither can go from \
         failing to passing on its own"
    )]
    SharedOracle {
        first: usize,
        second: usize,
        oracle: String,
    },

    #[error(
        "slice {at}'s oracle `{oracle}` is not permitted by the sandbox, so it could never run"
    )]
    OracleNotPermitted { at: usize, oracle: String },
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Provider(#[from] corifeu_provider::Error),

    #[error("the planner answered without calling `{EMIT_PLAN}`: {said:?}")]
    NoPlan { said: String },

    #[error("the planner's call did not fit the schema: {0}")]
    Shape(String),

    #[error(transparent)]
    Rejected(#[from] Rejected),
}

impl Plan {
    /// Every reason a plan cannot be executed, checked before any of it is.
    pub fn check(&self, allowlist: &Allowlist) -> Result<(), Rejected> {
        if self.slices.is_empty() {
            return Err(Rejected::Empty);
        }
        for (at, slice) in self.slices.iter().enumerate() {
            if slice.oracle.is_empty() {
                return Err(Rejected::NoOracle {
                    at,
                    task: slice.task.clone(),
                });
            }
            if slice.artefact.trim().is_empty() {
                return Err(Rejected::NoArtefact {
                    at,
                    task: slice.task.clone(),
                });
            }
            let Some(command) = Command::from_argv(&slice.oracle) else {
                return Err(Rejected::NoOracle {
                    at,
                    task: slice.task.clone(),
                });
            };
            if !allowlist.permits(&command) {
                return Err(Rejected::OracleNotPermitted {
                    at,
                    oracle: slice.describe_oracle(),
                });
            }
            if let Some(first) = self.slices[..at]
                .iter()
                .position(|earlier| earlier.oracle == slice.oracle)
            {
                return Err(Rejected::SharedOracle {
                    first,
                    second: at,
                    oracle: slice.describe_oracle(),
                });
            }
        }
        Ok(())
    }
}

pub struct Planner<'a> {
    pub provider: &'a dyn Provider,
    pub model: String,
    pub budget: Budget,
}

impl Planner<'_> {
    /// Ask once, and reject what comes back if it cannot be executed.
    pub fn plan(&self, brief: &Brief, allowlist: &Allowlist) -> Result<Plan, Error> {
        let turn = self
            .provider
            .chat(&self.model, &messages(brief), &[schema()], &self.budget)?;

        let call = turn
            .tool_calls
            .iter()
            .find(|c| c.name == EMIT_PLAN)
            .ok_or_else(|| Error::NoPlan {
                said: turn.content.chars().take(200).collect(),
            })?;

        let plan = read(call)?;
        plan.check(allowlist)?;
        Ok(plan)
    }
}

fn read(call: &ToolCall) -> Result<Plan, Error> {
    let slices = call
        .arguments
        .get("slices")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Shape("no `slices` array".into()))?;

    let mut out = Vec::with_capacity(slices.len());
    for (at, slice) in slices.iter().enumerate() {
        let text = |key: &str| {
            slice
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let oracle: Vec<String> = slice
            .get("oracle")
            .and_then(Value::as_array)
            .map(|argv| {
                argv.iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect()
            })
            .ok_or_else(|| Error::Shape(format!("slice {at} has no `oracle` array")))?;
        out.push(PlannedSlice {
            task: text("task"),
            artefact: text("artefact"),
            oracle,
        });
    }
    Ok(Plan { slices: out })
}

fn schema() -> Value {
    json!({"type": "function", "function": {
        "name": EMIT_PLAN,
        "description": "Emit the whole ordered list of slices. You are asked once.",
        "parameters": {"type": "object", "properties": {
            "slices": {"type": "array", "items": {"type": "object", "properties": {
                "task": {"type": "string",
                         "description": "What this slice changes, in one or two sentences."},
                "artefact": {"type": "string",
                             "description": "The one file this slice edits."},
                "oracle": {"type": "array", "items": {"type": "string"},
                           "description": "The command that passes only once this slice is \
                                           done, as an argument vector. No shell."}},
                "required": ["task", "artefact", "oracle"]}}},
            "required": ["slices"]}}})
}

fn messages(brief: &Brief) -> Vec<Message> {
    vec![
        Message::system(
            "You cut work into slices. A slice is small enough when exactly one command goes \
             from failing to passing because of it, and you name that command.\n\n\
             Rules:\n\
             - Emit every slice in one call. You are asked once.\n\
             - Each slice names one file it edits and one command that ends it.\n\
             - Two slices may not share a command: neither could then be finished on its own.\n\
             - A command is an argument vector. There is no shell: no pipes, no `&&`, no \
             redirection.\n\
             - Do not plan scaffolding that a tool does deterministically. Creating a project \
             from a generator is a command to run, not a slice to think about.\n\
             - Do not say how many tests a slice needs. Someone else writes them and counts.",
        ),
        Message::user(format!(
            "Goal:\n{}\n\nThe project:\n{}",
            brief.goal, brief.context
        )),
    ]
}
