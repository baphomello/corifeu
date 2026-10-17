//! What every provider adapter speaks.
//!
//! A turn is a turn whichever runtime produced it, so the vocabulary lives here
//! and an adapter's whole job is translating one wire format into it. The two
//! local runtimes disagree about almost every field name:
//!
//! | | Ollama | llama.cpp |
//! |---|---|---|
//! | reasoning | `message.thinking` | `message.reasoning_content` |
//! | tool arguments | a JSON object | a JSON **string** |
//! | why it stopped | `done_reason` | `finish_reason` |
//! | token counts | `prompt_eval_count` | `usage.prompt_tokens` |
//! | a prompt over the context | served anyway | `400 Bad Request` |
//!
//! None of that is a difference the orchestrator should ever see.

pub mod http;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] http::Error),

    #[error("the reply was not the shape {runtime} documents: {detail}")]
    Shape {
        runtime: &'static str,
        detail: String,
    },

    #[error("could not read the reply as JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// A provider that is a subprocess rather than an endpoint. Separate from
    /// [`Error::Shape`] because a CLI that is missing, unauthenticated or
    /// killed produced no reply to be the wrong shape.
    #[error("{runtime} could not be run: {detail}")]
    Process {
        runtime: &'static str,
        detail: String,
    },
}

/// One message in the context the loop rebuilds each step (ADR-0047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
    Tool { content: String },
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self::User {
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant {
            content: content.into(),
        }
    }
    pub fn tool(content: impl Into<String>) -> Self {
        Self::Tool {
            content: content.into(),
        }
    }
}

/// ADR-0048. Every field exists because leaving it unset was measured going
/// wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct Budget {
    /// Reasoning plus output. There is no unlimited setting: a local model
    /// without one generates until the context is full.
    pub num_predict: u32,
    /// Left unset, the runtime picks, and the run never records what it got.
    /// The measurement's whole first pass ran this way and found out afterwards
    /// that a third of the model had been pushed onto the CPU by a context
    /// nobody chose.
    pub num_ctx: Option<u32>,
    /// ADR-0048 §3: off for roles whose work is transcription, on for roles
    /// whose output is a judgement.
    pub think: bool,
    /// The sampler's seed, or None to let the runtime pick one.
    ///
    /// Defect 26. Five measurement rounds ran without this and none of them can
    /// be replayed: the same model against the same slice reached green between
    /// four and ten times in ten, so a round could not tell a model's weakness
    /// from a bad draw, and a paired comparison controlled the starting state
    /// while leaving the sampling free.
    ///
    /// Reproducible, not bit-exact. A local runtime splitting a batch
    /// differently across devices can still diverge; this removes the sampler
    /// as a source of it, which is the one a measurement controls.
    pub seed: Option<u64>,
    /// How much the sampler is allowed to wander, or None to let the runtime
    /// decide.
    ///
    /// The same class of problem as the seed and found the same way, by someone
    /// asking why it was not set. Six rounds and every probe ran at llama.cpp's
    /// default of 0.80, which is a reasonable number for prose and a high one
    /// for code: prose survives a word chosen second-best and code does not
    /// compile. The probe watched one model score five of ten and then ten of
    /// ten under identical flags, and that was read as the model's variance
    /// when it may have been the sampler's.
    ///
    /// None is the library default because an unset temperature is how the
    /// product behaves. A measurement sets it, and records it.
    pub temperature: Option<f32>,
}

impl Budget {
    /// A doer writing a file: room to transcribe, no reasoning competing for it.
    pub fn doer() -> Self {
        Self {
            num_predict: 8192,
            num_ctx: Some(8192),
            think: false,
            seed: None,
            temperature: None,
        }
    }

    /// A role whose output is a judgement rather than a file.
    pub fn judge() -> Self {
        Self {
            num_predict: 2048,
            num_ctx: Some(8192),
            think: true,
            seed: None,
            temperature: None,
        }
    }
}

/// Why generation stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    Done,
    /// ADR-0048 §4: the budget ran out mid-answer. Retryable with a larger one,
    /// and never to be read as the model having finished.
    BudgetSpent {
        num_predict: u32,
    },
    /// Anything else the runtime said, carried through rather than mapped onto
    /// `Done`, so an unfamiliar reason cannot look like success.
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub name: String,
    /// Always an object by the time it is here. Ollama sends it parsed and
    /// llama.cpp sends a string; the adapter settles that.
    pub arguments: Value,
}

/// What the runtime measured about its own work, where it reports it.
///
/// ADR-0031 reports savings to the user, and a figure the runtime measured
/// beats one the orchestrator inferred from a wall clock that also contains
/// model loading and HTTP.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Speed {
    pub prefill_per_second: f64,
    pub decode_per_second: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    pub content: String,
    /// Counted in `output_tokens`, which is why the budget has to cover it.
    pub thinking: String,
    pub tool_calls: Vec<ToolCall>,
    pub stop: Stop,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
    pub speed: Option<Speed>,
}

/// One turn, from whichever runtime.
pub trait Provider {
    /// The name this adapter reports in errors and in metering.
    fn runtime(&self) -> &'static str;

    fn chat(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Value],
        budget: &Budget,
    ) -> Result<Turn, Error>;
}

/// A model that emits its call as text rather than through the runtime's
/// tool-call field.
///
/// Measured: `qwen2.5-coder:7b` does this under both runtimes. Refusing to read
/// it would rule out models that otherwise fit hardware that fits one.
///
/// Only a whole message that is one JSON object with `name` and `arguments`
/// counts, optionally fenced. Hunting for JSON inside prose would turn a model
/// talking *about* a call into a call.
pub fn call_in_text(content: &str) -> Option<ToolCall> {
    let trimmed = content.trim();
    let trimmed = match trimmed.strip_prefix("```") {
        // The tag is whatever the model felt like: measured `json`, and `xml`
        // from a model emitting JSON under it. Drop the rest of that line
        // rather than matching a list that has no end.
        Some(rest) => {
            let body = rest.split_once('\n').map_or(rest, |(_tag, body)| body);
            body.trim().strip_suffix("```")?.trim()
        }
        None => trimmed,
    };
    let value: Value = serde_json::from_str(trimmed).ok()?;
    let name = value.get("name")?.as_str()?.to_string();
    let arguments = value.get("arguments")?.clone();
    arguments
        .is_object()
        .then_some(ToolCall { name, arguments })
}
