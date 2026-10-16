//! Ollama's `/api/chat`, translated into a [`Turn`].
//!
//! Ollama parses a tool call into `message.tool_calls` only when the model's
//! template supports it. Measured on this project's models:
//!
//! | Model | What came back |
//! |---|---|
//! | `ornith1.5:9b` | `tool_calls`, with arguments already parsed, plus `thinking` |
//! | `qwen2.5-coder:7b` | no `tool_calls`; the JSON call as plain `content` |
//!
//! Both are read, through [`corifeu_provider::call_in_text`] for the second.

use corifeu_provider::{
    Budget, Error, Message, Provider, Speed, Stop, ToolCall, Turn, call_in_text, http,
};
use serde_json::{Value, json};
use std::time::Duration;

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
}

impl Provider for Ollama {
    fn runtime(&self) -> &'static str {
        "ollama"
    }

    fn chat(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Value],
        budget: &Budget,
    ) -> Result<Turn, Error> {
        let mut options = json!({ "num_predict": budget.num_predict });
        // Defect 26: a round that cannot be replayed cannot be checked.
        if let Some(seed) = budget.seed {
            options["seed"] = json!(seed);
        }
        if let Some(temperature) = budget.temperature {
            options["temperature"] = json!(temperature);
        }
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
    let message = reply.get("message").ok_or_else(|| Error::Shape {
        runtime: "ollama",
        detail: "no `message`".into(),
    })?;

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
        speed: speed(&reply),
    })
}

/// Ollama reports durations in nanoseconds rather than rates.
fn speed(reply: &Value) -> Option<Speed> {
    let per_second = |n: &str, d: &str| {
        let count = reply.get(n)?.as_f64()?;
        let nanos = reply.get(d)?.as_f64()?;
        (nanos > 0.0).then(|| count / (nanos / 1e9))
    };
    Some(Speed {
        prefill_per_second: per_second("prompt_eval_count", "prompt_eval_duration")?,
        decode_per_second: per_second("eval_count", "eval_duration")?,
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
