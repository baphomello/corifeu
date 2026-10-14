//! Parsed against replies captured from a running `llama-server`, not from docs.

use corifeu_llamacpp::parse;
use corifeu_provider::Stop;

/// `ornith1.5-9b-Q5_K_M` through the router, asked to run a test suite with one
/// tool available. Note `arguments`: a JSON **string**, not an object.
const WITH_TOOL_CALL: &str = r#"{
  "choices": [{
    "finish_reason": "tool_calls",
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "I'll first explore the project structure.\n\n",
      "reasoning_content": "The user wants to run the library test suite.",
      "tool_calls": [{
        "type": "function",
        "function": { "name": "run", "arguments": "{\"argv\":[\"ls\", \"-la\"]}" },
        "id": "Q41rAKt07SPMCFsCAXMM7qaZh1ZDe1pb"
      }]
    }
  }],
  "usage": { "completion_tokens": 81, "prompt_tokens": 304, "total_tokens": 385 },
  "timings": { "prompt_per_second": 230.9, "predicted_per_second": 40.6 }
}"#;

const TRUNCATED: &str = r#"{
  "choices": [{ "finish_reason": "length", "index": 0,
                "message": { "role": "assistant", "content": "Hello! How can" } }],
  "usage": { "completion_tokens": 4, "prompt_tokens": 30, "total_tokens": 34 }
}"#;

/// `qwen2.5-coder-7b-Q4_K_M` with the same tools. It does not tool-call; it
/// invents its own markup.
const INVENTED_MARKUP: &str = r#"{
  "choices": [{ "finish_reason": "stop", "index": 0,
                "message": { "role": "assistant",
                             "content": "<run argv=\"cargo\" args=\"[\\\"test\\\"]\">" } }],
  "usage": { "completion_tokens": 22, "prompt_tokens": 180, "total_tokens": 202 }
}"#;

#[test]
fn tool_arguments_arrive_as_a_string_and_leave_as_an_object() {
    let turn = parse(WITH_TOOL_CALL, 8192).unwrap();

    assert_eq!(turn.tool_calls.len(), 1);
    assert_eq!(turn.tool_calls[0].name, "run");
    assert_eq!(
        turn.tool_calls[0].arguments["argv"][0], "ls",
        "left encoded, every caller would get a quoted blob where it wanted a map"
    );
}

#[test]
fn a_turn_that_ends_in_a_tool_call_is_finished_not_truncated() {
    // `finish_reason: "tool_calls"` means the model wants a tool run. Reading
    // it as anything but Done would make every successful call look like a
    // failure.
    assert_eq!(parse(WITH_TOOL_CALL, 8192).unwrap().stop, Stop::Done);
}

#[test]
fn reasoning_is_kept_apart_from_content() {
    let turn = parse(WITH_TOOL_CALL, 8192).unwrap();
    assert!(turn.thinking.starts_with("The user wants"));
    assert!(!turn.content.contains("The user wants"));
}

#[test]
fn the_server_reports_its_own_throughput() {
    // ADR-0031 reports savings to the user. A rate the server measured beats
    // one inferred from a wall clock that also holds model loading and HTTP.
    let speed = parse(WITH_TOOL_CALL, 8192).unwrap().speed.expect("timings");
    assert_eq!(speed.decode_per_second, 40.6);
    assert_eq!(speed.prefill_per_second, 230.9);
}

#[test]
fn token_counts_come_from_usage() {
    let turn = parse(WITH_TOOL_CALL, 8192).unwrap();
    assert_eq!((turn.prompt_tokens, turn.output_tokens), (304, 81));
}

#[test]
fn a_spent_budget_is_a_variant_the_caller_has_to_match() {
    let turn = parse(TRUNCATED, 4).unwrap();
    assert_eq!(turn.stop, Stop::BudgetSpent { num_predict: 4 });
    assert_eq!(turn.content, "Hello! How can");
    assert!(turn.speed.is_none(), "this reply carried no timings");
}

#[test]
fn invented_markup_is_not_guessed_at() {
    // Reading this would mean a parser per model's imagination. It is read as
    // what it is: content, with no call. The model is simply not one that
    // tool-calls, and the run should say so rather than half-work.
    let turn = parse(INVENTED_MARKUP, 8192).unwrap();
    assert!(turn.tool_calls.is_empty());
    assert!(turn.content.starts_with("<run"));
}

#[test]
fn an_unfamiliar_finish_reason_cannot_pass_as_success() {
    let odd = TRUNCATED.replace(r#""length""#, r#""content_filter""#);
    assert_eq!(
        parse(&odd, 4).unwrap().stop,
        Stop::Other("content_filter".into())
    );
}

#[test]
fn a_reply_without_choices_is_an_error_not_an_empty_turn() {
    assert!(parse(r#"{"usage":{}}"#, 10).is_err());
    assert!(parse(r#"{"choices":[]}"#, 10).is_err());
}
