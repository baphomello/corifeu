//! The translation, both ways, against output captured from a real CLI.
//!
//! Nothing here spawns a process: a test that needs a logged-in CLI and a
//! network is a test that is skipped, and a skipped test is not a test.

use corifeu_cliagent::{Agent, CliAgent, antigravity, claude_code, codex, object, prompt, turn};
use corifeu_provider::{Error, Message, Stop};
use serde_json::{Value, json};

fn write_file_tool() -> Value {
    json!({"type": "function", "function": {"name": "write_file"}})
}

/// Captured from `claude -p ... --output-format json` on 2.1.302.
fn captured() -> &'static str {
    r#"{"stop_reason":"end_turn","session_id":"77d0b1d6","total_cost_usd":0.114536,
        "usage":{"input_tokens":2,"cache_creation_input_tokens":7701,
                 "cache_read_input_tokens":11639,"output_tokens":42},
        "is_error":false,
        "result":"{\"name\":\"write_file\",\"arguments\":{\"path\":\"src/a.ts\",\"content\":\"export const n = 1;\"}}"}"#
}

#[test]
fn a_captured_reply_becomes_a_turn_with_the_call_in_it() {
    let reply = object(captured(), "claude-code").expect("the capture parses");
    let turn = turn(&reply, "claude-code").expect("it is not an error reply");

    assert_eq!(turn.tool_calls.len(), 1);
    assert_eq!(turn.tool_calls[0].name, "write_file");
    assert_eq!(turn.tool_calls[0].arguments["path"], "src/a.ts");
    assert_eq!(turn.stop, Stop::Done);
}

/// Defect 46. `input_tokens` was 2 on a call that cost eleven cents, because a
/// CLI agent's own context arrives as cache. Metering the split from
/// `input_tokens` alone would show the expensive side as free.
#[test]
fn the_prompt_count_includes_what_arrived_as_cache() {
    let reply = object(captured(), "claude-code").unwrap();
    let turn = turn(&reply, "claude-code").unwrap();

    assert_eq!(
        turn.prompt_tokens,
        2 + 7_701 + 11_639,
        "two tokens is what the CLI calls input; it is not what was sent"
    );
    assert_eq!(turn.output_tokens, 42);
}

/// Defect 48. Without stdin redirected the CLI warns before its JSON. The
/// adapter redirects it; reading from the first brace is the second defence.
#[test]
fn a_warning_ahead_of_the_json_is_not_fatal() {
    let noisy = format!(
        "Warning: no stdin data received in 3s, proceeding without it.\n{}",
        captured()
    );
    let reply = object(&noisy, "claude-code").expect("the warning is skipped");
    assert_eq!(turn(&reply, "claude-code").unwrap().tool_calls.len(), 1);
}

#[test]
fn output_with_no_json_at_all_names_what_the_cli_said() {
    let error = object("command not found: claude\n", "claude-code")
        .expect_err("there is no object in that");
    let message = error.to_string();
    assert!(
        message.contains("command not found"),
        "the error carries the CLI's own words: {message}"
    );
}

/// Defect 47. `--bare` answers this, and it is an error with a cost of zero
/// rather than a reply of the wrong shape, so it is `Process` not `Shape`.
#[test]
fn an_unauthenticated_cli_is_a_process_error_carrying_its_reason() {
    let unauthenticated = r#"{"is_error":true,"stop_reason":"stop_sequence",
        "result":"Not logged in · Please run /login","usage":{"input_tokens":0}}"#;
    let reply = object(unauthenticated, "claude-code").unwrap();
    let error = turn(&reply, "claude-code").expect_err("is_error is true");

    assert!(matches!(error, Error::Process { .. }));
    assert!(error.to_string().contains("Not logged in"));
}

#[test]
fn an_unfamiliar_stop_reason_is_carried_rather_than_called_done() {
    let reply = object(
        r#"{"stop_reason":"refusal","result":"no","usage":{"output_tokens":1}}"#,
        "claude-code",
    )
    .unwrap();
    assert_eq!(
        turn(&reply, "claude-code").unwrap().stop,
        Stop::Other("refusal".into())
    );
}

// ------------------------------------------------------------------- the argv

#[test]
fn the_argv_carries_the_flags_the_probe_proved_necessary() {
    let argv =
        CliAgent::new(claude_code()).argv("claude-opus-5", "do the thing", &[write_file_tool()]);

    assert_eq!(argv[0], "claude");
    for required in [
        "--restricted",
        "--strict-mcp-config",
        "--output-format",
        "json",
    ] {
        assert!(
            argv.iter().any(|a| a == required),
            "{required} missing: {argv:?}"
        );
    }
    assert!(
        argv.iter().any(|a| a == "--json-schema"),
        "the shape is constrained, not requested: {argv:?}"
    );
    assert!(argv.iter().any(|a| a == "claude-opus-5"));
    let at = argv
        .iter()
        .position(|a| a == "-p")
        .expect("the prompt flag");
    assert_eq!(argv[at + 1], "do the thing");
}

/// ADR-0007 names `--bare` for ignoring ambient context and it also removes the
/// credential. It stays out of the argv, and this is the test that says why so
/// nobody adds it back from the ADR.
#[test]
fn the_argv_never_asks_for_bare_mode() {
    let argv = CliAgent::new(claude_code()).argv("m", "p", &[write_file_tool()]);
    assert!(!argv.iter().any(|a| a == "--bare"));
}

/// A dispatch with no tools is asking for prose, and a schema demanding a call
/// would refuse it.
#[test]
fn no_tools_means_no_schema() {
    let argv = CliAgent::new(claude_code()).argv("m", "p", &[]);
    assert!(!argv.iter().any(|a| a == "--json-schema"), "{argv:?}");
}

#[test]
fn a_cli_whose_prompt_is_positional_puts_it_last() {
    let argv = CliAgent::new(codex()).argv("gpt", "do the thing", &[write_file_tool()]);
    assert_eq!(argv[0], "codex");
    assert_eq!(argv[1], "exec", "the subcommand comes before the flags");
    assert_eq!(argv.last().unwrap(), "do the thing");
}

/// ADR-0007 keeps three CLIs in scope so the abstraction cannot become Claude
/// Code's shape. Two of the three have never been run, and the descriptor says
/// so rather than the knowledge living in a comment.
#[test]
fn only_the_exercised_descriptor_claims_to_be_verified() {
    let verified: Vec<&str> = [claude_code(), codex(), antigravity()]
        .iter()
        .filter(|a: &&Agent| a.verified)
        .map(|a| a.runtime)
        .collect();
    assert_eq!(verified, ["claude-code"]);
}

// ----------------------------------------------------------------- the prompt

/// Defect 48, as the prompt rather than as the process. Told to call a tool,
/// the CLI used its own and billed for failing. It has to be told it is
/// proposing.
#[test]
fn the_prompt_tells_the_agent_not_to_act() {
    let text = prompt(&[Message::user("make the test pass")], &[write_file_tool()]);
    assert!(text.contains("Do not act"), "{text}");
    assert!(
        text.contains("write_file"),
        "the actions are listed: {text}"
    );
    assert!(text.contains("make the test pass"));
}

#[test]
fn a_prompt_with_no_tools_does_not_promise_an_action() {
    let text = prompt(&[Message::user("summarise this")], &[]);
    assert!(!text.contains("Do not act"), "{text}");
    assert!(text.contains("summarise this"));
}

#[test]
fn every_message_reaches_the_prompt_with_its_role() {
    let text = prompt(
        &[
            Message::system("you are a doer"),
            Message::user("the file is empty"),
            Message::assistant("I ran: write_file"),
        ],
        &[],
    );
    for expected in [
        "[system]",
        "you are a doer",
        "[user]",
        "the file is empty",
        "[assistant]",
    ] {
        assert!(text.contains(expected), "{expected} missing from: {text}");
    }
}

// ----------------------------------------------------------------- the version

/// ADR-0007: Corifeu reports the CLI version it detected and refuses an
/// untested one with a clear message rather than a parse error.
#[test]
fn the_version_is_read_out_of_what_the_cli_prints() {
    use corifeu_cliagent::version_in;

    assert_eq!(
        version_in("2.1.302 (Claude Code)").as_deref(),
        Some("2.1.302")
    );
    assert_eq!(
        version_in("codex-cli 0.162.1\n").as_deref(),
        Some("0.162.1")
    );
    assert_eq!(version_in("agy version 1.4\n").as_deref(), Some("1.4"));
    assert_eq!(version_in("command not found"), None);
    assert_eq!(version_in(""), None);
}

#[test]
fn a_version_is_compared_number_by_number_not_as_text() {
    use corifeu_cliagent::at_least;

    assert!(at_least("2.1.302", "2.1.302"), "equal is enough");
    assert!(
        at_least("2.1.302", "2.1.9"),
        "302 is above 9, as text it is below"
    );
    assert!(at_least("2.2.0", "2.1.302"));
    assert!(at_least("2.1", "2.1.0"), "a missing part is zero");
    assert!(!at_least("2.1.301", "2.1.302"));
    assert!(!at_least("1.9.9", "2.0.0"));
}

/// An unparseable version fails closed. Refusing to run is recoverable;
/// running against a version nobody has tested is what the ADR prevents.
#[test]
fn a_version_that_is_not_numbers_is_refused_rather_than_assumed() {
    use corifeu_cliagent::at_least;

    assert!(!at_least("nightly", "2.1.302"));
    assert!(!at_least("2.1.302", "latest"));
}
