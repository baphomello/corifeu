//! Parsed against replies captured from a running Ollama, not from the docs.

use corifeu_ollama::{Stop, parse};

/// `ornith1.5:9b`, asked to run a test suite with one tool available.
const WITH_NATIVE_CALL: &str = r#"{
  "model": "ornith1.5:9b",
  "done": true,
  "done_reason": "stop",
  "message": {
    "role": "assistant",
    "content": "I'll start by exploring the project structure.\n\n",
    "thinking": "The user wants me to run the test suite for the lib target.",
    "tool_calls": [
      { "id": "SiAuux0JhYUxyyuSPLNYFWv4KQDiPwzX",
        "function": { "index": 0, "name": "run", "arguments": { "command": "pwd && ls -la" } } }
    ]
  },
  "prompt_eval_count": 159,
  "eval_count": 412
}"#;

/// `qwen2.5-coder:7b`, same request. Ollama parsed no tool call; the model put
/// it in `content` as text.
const WITH_CALL_AS_TEXT: &str = r#"{
  "model": "qwen2.5-coder:7b",
  "done": true,
  "done_reason": "stop",
  "message": {
    "role": "assistant",
    "content": "{\"name\": \"run\", \"arguments\": {\"command\": \"cargo test --lib\"}}"
  },
  "prompt_eval_count": 159,
  "eval_count": 19
}"#;

const TRUNCATED: &str = r#"{
  "done": true,
  "done_reason": "length",
  "message": { "role": "assistant", "content": "Rust is a" },
  "prompt_eval_count": 20,
  "eval_count": 3
}"#;

#[test]
fn a_native_tool_call_is_read_with_its_arguments_already_parsed() {
    let turn = parse(WITH_NATIVE_CALL, 8192).unwrap();

    assert_eq!(turn.tool_calls.len(), 1);
    assert_eq!(turn.tool_calls[0].name, "run");
    assert_eq!(turn.tool_calls[0].arguments["command"], "pwd && ls -la");
    assert_eq!(turn.stop, Stop::Done);
}

#[test]
fn thinking_is_kept_apart_from_content_and_counted_in_output() {
    let turn = parse(WITH_NATIVE_CALL, 8192).unwrap();

    assert!(turn.thinking.starts_with("The user wants me"));
    assert!(!turn.content.contains("The user wants me"));
    assert_eq!(
        turn.output_tokens, 412,
        "ADR-0048: the budget covers reasoning plus output, so metering must too"
    );
}

#[test]
fn a_call_emitted_as_text_is_still_a_call() {
    // Half the models that fit 8 GB of VRAM come back like this. Refusing to
    // read it would rule them out for no reason.
    let turn = parse(WITH_CALL_AS_TEXT, 8192).unwrap();

    assert_eq!(turn.tool_calls.len(), 1);
    assert_eq!(turn.tool_calls[0].name, "run");
    assert_eq!(turn.tool_calls[0].arguments["command"], "cargo test --lib");
}

#[test]
fn a_fenced_call_is_read_too() {
    let fenced = WITH_CALL_AS_TEXT.replace(
        r#""{\"name\": \"run\", \"arguments\": {\"command\": \"cargo test --lib\"}}""#,
        r#""```json\n{\"name\": \"run\", \"arguments\": {\"command\": \"cargo test --lib\"}}\n```""#,
    );
    assert_eq!(parse(&fenced, 8192).unwrap().tool_calls.len(), 1);
}

#[test]
fn prose_about_a_call_is_not_a_call() {
    let talking = WITH_CALL_AS_TEXT.replace(
        r#""{\"name\": \"run\", \"arguments\": {\"command\": \"cargo test --lib\"}}""#,
        r#""I could send {\"name\": \"run\"} but I would rather ask first.""#,
    );
    assert!(parse(&talking, 8192).unwrap().tool_calls.is_empty());
}

#[test]
fn a_spent_budget_is_a_variant_the_caller_has_to_match() {
    let turn = parse(TRUNCATED, 3).unwrap();

    assert_eq!(turn.stop, Stop::BudgetSpent { num_predict: 3 });
    assert_eq!(
        turn.content, "Rust is a",
        "the partial answer is kept; it is evidence, not rubbish"
    );
}

#[test]
fn an_unfamiliar_stop_reason_cannot_pass_as_success() {
    let odd = TRUNCATED.replace(r#""length""#, r#""load""#);
    assert_eq!(parse(&odd, 3).unwrap().stop, Stop::Other("load".into()));
}

#[test]
fn a_reply_without_a_message_is_an_error_not_an_empty_turn() {
    assert!(parse(r#"{"done": true}"#, 10).is_err());
}
