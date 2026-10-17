//! `llama-server`'s `/v1/chat/completions`, translated into a [`Turn`].
//!
//! The shape is OpenAI's, which differs from Ollama's at every field that
//! matters, and in one place that matters more than the names: **tool-call
//! arguments arrive as a JSON string**, not as an object. A caller that read
//! them without decoding would get a string where it expected a map, and only
//! find out at the first tool call.
//!
//! Measured on this project's three models, served from one router:
//!
//! | Model | decode | tool calls |
//! |---|---:|---|
//! | `qwopus3.5-9b-coder-Q4_K_M` | 53.8 t/s | native |
//! | `qwen2.5-coder-7b-Q4_K_M` | 64.5 t/s | none — emits its own markup |
//! | `ornith1.5-9b-Q5_K_M` | 40.6 t/s | native |
//!
//! A prompt larger than the server's context comes back as `400`, not as a
//! silently truncated answer. That is the behaviour ADR-0049 asks for and it
//! reaches the caller as [`Error::Http`] rather than as a short reply.

use corifeu_provider::{
    Budget, Error, Message, Provider, Speed, Stop, ToolCall, Turn, call_in_text, http,
};
use serde_json::{Value, json};
use std::time::Duration;

pub struct LlamaCpp {
    base: String,
    timeout: Duration,
}

impl LlamaCpp {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            // A router loads a model on demand, so the first turn against a
            // cold model pays for reading six gigabytes off disk.
            timeout: Duration::from_secs(900),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The models the router serves, by the name `chat` takes.
    pub fn models(&self) -> Result<Vec<String>, Error> {
        let body = http::get(&format!("{}/v1/models", self.base), self.timeout)?;
        let reply: Value = serde_json::from_str(&body)?;
        Ok(reply
            .get("data")
            .and_then(Value::as_array)
            .map(|models| {
                models
                    .iter()
                    .filter_map(|m| Some(m.get("id")?.as_str()?.to_string()))
                    .collect()
            })
            .unwrap_or_default())
    }
}

impl Provider for LlamaCpp {
    fn runtime(&self) -> &'static str {
        "llama.cpp"
    }

    fn chat(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Value],
        budget: &Budget,
    ) -> Result<Turn, Error> {
        // Defect 36: this built its own copy of the request for a while, beside
        // the `body` the tests assert against. Two constructions of the same
        // thing drift, and this pair already had: the temperature reached the
        // copy that ships and not the one under test.
        let request = body(model, messages, tools, budget);

        let reply = http::post_json(
            &format!("{}/v1/chat/completions", self.base),
            &request.to_string(),
            self.timeout,
        )?;
        parse(&reply, budget.num_predict)
    }
}

/// The request as it goes on the wire.
///
/// Separated from [`LlamaCpp::chat`] so that what is sent can be asserted
/// without a server. Defect 26 was a missing field here, and a missing field
/// here is invisible in every test that stubs the response.
pub fn body(model: &str, messages: &[Message], tools: &[Value], budget: &Budget) -> Value {
    let mut request = json!({
        "model": model,
        "messages": messages,
        "stream": false,
        "max_tokens": budget.num_predict,
    });
    if !tools.is_empty() {
        request["tools"] = json!(tools);
    }
    // Defect 39. This used to send `reasoning_format: "none"`, and the flag's
    // own help says what that does: "leaves thoughts unparsed in
    // message.content". The intent was "do not think". The effect was "think,
    // and write the thinking into your answer" — spending the output budget and
    // putting prose where a tool call belongs.
    //
    // Two things are asked for instead. The template is told not to think,
    // which is what actually stops it and costs two tokens where the thinking
    // cost 175. And the format is `deepseek`, so that a model whose template
    // ignores the first still puts its thoughts in `reasoning_content` where
    // they are separate from the answer rather than inside it.
    if !budget.think {
        request["chat_template_kwargs"] = json!({ "enable_thinking": false });
        request["reasoning_format"] = json!("deepseek");
    }
    // Defect 26: a round that cannot be replayed cannot be checked.
    if let Some(seed) = budget.seed {
        request["seed"] = json!(seed);
    }
    // Its sibling: a number that moves the result and that nobody wrote down.
    if let Some(temperature) = budget.temperature {
        request["temperature"] = json!(temperature);
    }
    request
}

pub fn parse(body: &str, num_predict: u32) -> Result<Turn, Error> {
    let reply: Value = serde_json::from_str(body)?;
    let shape = |detail: &str| Error::Shape {
        runtime: "llama.cpp",
        detail: detail.to_string(),
    };
    let choice = reply
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
        .ok_or_else(|| shape("no `choices`"))?;
    let message = choice.get("message").ok_or_else(|| shape("no `message`"))?;

    let content = text(message, "content");
    let mut tool_calls = native_calls(message);
    if tool_calls.is_empty()
        && let Some(call) = call_in_text(&content)
    {
        tool_calls.push(call);
    }

    let usage = reply.get("usage").cloned().unwrap_or_else(|| json!({}));
    Ok(Turn {
        content,
        thinking: text(message, "reasoning_content"),
        tool_calls,
        stop: match choice.get("finish_reason").and_then(Value::as_str) {
            Some("length") => Stop::BudgetSpent { num_predict },
            // `tool_calls` means the model stopped because it wants a tool run,
            // which is a finished turn, not a truncated one.
            Some("stop") | Some("tool_calls") | None => Stop::Done,
            Some(other) => Stop::Other(other.to_string()),
        },
        prompt_tokens: count(&usage, "prompt_tokens"),
        output_tokens: count(&usage, "completion_tokens"),
        speed: speed(&reply),
    })
}

/// llama.cpp reports rates directly, measured by the server itself.
fn speed(reply: &Value) -> Option<Speed> {
    let timings = reply.get("timings")?;
    Some(Speed {
        prefill_per_second: timings.get("prompt_per_second")?.as_f64()?,
        decode_per_second: timings.get("predicted_per_second")?.as_f64()?,
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

/// OpenAI sends the arguments as a string holding JSON. Decoding it here is the
/// difference between a caller getting a map and getting a quoted blob.
fn native_calls(message: &Value) -> Vec<ToolCall> {
    message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let function = call.get("function")?;
                    let raw = function.get("arguments");
                    let arguments = match raw.and_then(Value::as_str) {
                        Some(encoded) => serde_json::from_str(encoded).ok()?,
                        None => raw.cloned().unwrap_or_else(|| json!({})),
                    };
                    Some(ToolCall {
                        name: function.get("name")?.as_str()?.to_string(),
                        arguments,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::body;
    use corifeu_provider::Budget;

    fn budget() -> Budget {
        Budget {
            num_predict: 4096,
            num_ctx: Some(8192),
            think: false,
            seed: None,
            temperature: None,
        }
    }

    #[test]
    fn no_seed_sends_no_seed() {
        // The runtime picks one, which is what every round before defect 26 did.
        assert!(body("m", &[], &[], &budget()).get("seed").is_none());
    }

    #[test]
    fn a_temperature_reaches_the_wire() {
        let budget = Budget {
            temperature: Some(0.2),
            ..budget()
        };
        // Compared with a tolerance: an f32 widened to the f64 serde_json
        // stores is 0.20000000298, and asserting equality would be asserting
        // something about IEEE 754 rather than about the request.
        let sent = body("m", &[], &[], &budget)["temperature"]
            .as_f64()
            .expect("a number");
        assert!((sent - 0.2).abs() < 1e-6, "{sent}");
    }

    /// Unset means llama.cpp's own 0.80, which is what every round before this
    /// ran at. Sending nothing is how a product run keeps that behaviour.
    #[test]
    fn no_temperature_sends_no_temperature() {
        assert!(body("m", &[], &[], &budget()).get("temperature").is_none());
    }

    #[test]
    fn a_seed_reaches_the_wire() {
        let budget = Budget {
            seed: Some(7),
            ..budget()
        };
        assert_eq!(body("m", &[], &[], &budget)["seed"], 7);
    }

    /// `num_ctx` is deliberately not sent: llama.cpp fixes the context when the
    /// model is loaded, so a per-request value would be silently ignored and
    /// reading it back would be reading our own wish.
    #[test]
    fn the_context_is_the_servers_to_decide() {
        assert!(body("m", &[], &[], &budget()).get("num_ctx").is_none());
    }

    #[test]
    fn a_transcription_role_asks_the_template_not_to_think() {
        let request = body("m", &[], &[], &budget());
        assert_eq!(request["chat_template_kwargs"]["enable_thinking"], false);
        // And if the template ignores that, the thoughts still go somewhere
        // other than the answer.
        assert_eq!(request["reasoning_format"], "deepseek");
    }

    #[test]
    fn a_judging_role_is_left_alone() {
        let judging = Budget {
            think: true,
            ..budget()
        };
        let request = body("m", &[], &[], &judging);
        assert!(request.get("chat_template_kwargs").is_none());
        assert!(request.get("reasoning_format").is_none());
    }
}
