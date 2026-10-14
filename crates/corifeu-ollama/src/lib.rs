//! One chat turn against Ollama, with tools and a budget.
//!
//! # The budget is not optional (ADR-0048)
//!
//! `num_predict` is always set. There is no unlimited setting, because a local
//! model without one generates until the context is full, and the budget has to
//! cover reasoning as well as output — a budget sized for the expected code
//! starves a thinking model before it writes anything.
//!
//! A turn that ran out of budget says so. [`Stop::BudgetSpent`] is a variant the
//! caller has to match, not a short answer it has to guess about.
//!
//! # Two shapes of tool call
//!
//! Ollama parses a tool call into `message.tool_calls` only when the model's
//! template supports it. Measured on this project's two models:
//!
//! | Model | What came back |
//! |---|---|
//! | `ornith1.5:9b` | `tool_calls`, plus a separate `thinking` field |
//! | `qwen2.5-coder:7b` | no `tool_calls`; the JSON call as plain `content` |
//!
//! Both are read. A model that emits its call as text is not broken, and
//! refusing to understand it would silently rule out half the models a user
//! might run on hardware that fits one.

pub mod http;

use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] http::Error),

    #[error("the reply was not the shape Ollama documents: {0}")]
    Shape(String),

    #[error("could not read the reply as JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// A message in the conversation Corifeu rebuilds each step (ADR-0047).
#[derive(Debug, Clone, Serialize)]
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

/// ADR-0048. Every field here exists because leaving it unset was measured
/// going wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    /// Reasoning plus output. Mandatory.
    pub num_predict: u32,
    /// Left unset, Ollama picks its own and the result is a context size the
    /// run never recorded. The measurement ran its whole first pass without
    /// this, and found out afterwards that a third of the model had been
    /// pushed onto the CPU by a context it never chose.
    pub num_ctx: Option<u32>,
    /// ADR-0048 §3: off for roles whose work is transcription, on for roles
    /// whose output is a judgement.
    pub think: bool,
}

impl Budget {
    /// A doer writing a file: room to transcribe, no reasoning competing for it.
    pub fn doer() -> Self {
        Self {
            num_predict: 8192,
            num_ctx: Some(8192),
            think: false,
        }
    }

    /// A role whose output is a judgement rather than a file.
    pub fn judge() -> Self {
        Self {
            num_predict: 2048,
            num_ctx: Some(8192),
            think: true,
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
    /// Ollama reported something else. Carried through rather than mapped onto
    /// `Done`, so an unfamiliar reason cannot look like success.
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub name: String,
    /// A JSON object. Ollama sends the arguments parsed, unlike the OpenAI
    /// shape where they arrive as a string that still needs decoding.
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub content: String,
    /// Where the model has a reasoning mode, what it thought. Counted in
    /// `output_tokens`, which is why the budget has to cover it.
    pub thinking: String,
    pub tool_calls: Vec<ToolCall>,
    pub stop: Stop,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
}

pub struct Ollama {
    base: String,
    timeout: Duration,
}

impl Ollama {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            timeout: Duration::from_secs(600),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn chat(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Value],
        budget: &Budget,
    ) -> Result<Turn, Error> {
        let mut options = json!({ "num_predict": budget.num_predict });
        if let Some(num_ctx) = budget.num_ctx {
            options["num_ctx"] = json!(num_ctx);
        }
        let mut request = json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "think": budget.think,
            "options": options,
        });
        if !tools.is_empty() {
            request["tools"] = json!(tools);
        }

        let body = http::post_json(
            &format!("{}/api/chat", self.base),
            &request.to_string(),
            self.timeout,
        )?;
        parse(&body, budget.num_predict)
    }
}

pub fn parse(body: &str, num_predict: u32) -> Result<Turn, Error> {
    let reply: Value = serde_json::from_str(body)?;
    let message = reply
        .get("message")
        .ok_or_else(|| Error::Shape("no `message`".into()))?;

    let content = text(message, "content");
    let mut tool_calls = native_calls(message);
    if tool_calls.is_empty()
        && let Some(call) = call_in_text(&content)
    {
        tool_calls.push(call);
    }

    Ok(Turn {
        content,
        thinking: text(message, "thinking"),
        tool_calls,
        stop: match reply.get("done_reason").and_then(Value::as_str) {
            Some("length") => Stop::BudgetSpent { num_predict },
            Some("stop") | None => Stop::Done,
            Some(other) => Stop::Other(other.to_string()),
        },
        prompt_tokens: count(&reply, "prompt_eval_count"),
        output_tokens: count(&reply, "eval_count"),
    })
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn count(value: &Value, key: &str) -> u32 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0) as u32
}

fn native_calls(message: &Value) -> Vec<ToolCall> {
    message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let function = call.get("function")?;
                    Some(ToolCall {
                        name: function.get("name")?.as_str()?.to_string(),
                        arguments: function
                            .get("arguments")
                            .cloned()
                            .unwrap_or_else(|| json!({})),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A model whose template Ollama cannot parse emits the call as content.
///
/// Only a whole message that is one JSON object with `name` and `arguments`
/// counts, optionally inside a fenced block. Hunting for JSON embedded in prose
/// would turn a model talking *about* a call into a call.
fn call_in_text(content: &str) -> Option<ToolCall> {
    let trimmed = content.trim();
    let trimmed = match trimmed.strip_prefix("```") {
        Some(rest) => rest
            .trim_start_matches("json")
            .trim()
            .strip_suffix("```")?
            .trim(),
        None => trimmed,
    };
    let value: Value = serde_json::from_str(trimmed).ok()?;
    let name = value.get("name")?.as_str()?.to_string();
    let arguments = value.get("arguments")?.clone();
    if !arguments.is_object() {
        return None;
    }
    Some(ToolCall { name, arguments })
}
