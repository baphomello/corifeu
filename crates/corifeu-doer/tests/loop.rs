//! The loop, against a scripted model and an oracle that really changes colour.
//!
//! The oracle is this test binary running one of its own ignored tests, which
//! reads a file from the working directory. That makes a red-to-green
//! transition real and portable: no shell, no `cargo` inside `cargo`, and the
//! `test result:` line ADR-0057's case counter reads comes out of it for free.

use corifeu_agent::Window;
use corifeu_check::Verdict;
use corifeu_doer::{Doer, Limits, Oracle, Outcome, Slice};
use corifeu_exec::Allowlist;
use corifeu_provider::{Budget, Error, Message, Provider, Stop, ToolCall, Turn, http};
use corifeu_workspace::{Class, Classes, Guard, Ownership, Protected, Workspace};
use serde_json::json;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// The oracle. Ignored so an ordinary run skips it; the doer names it exactly.
#[ignore]
#[test]
fn artefact_is_done() {
    let body = std::fs::read_to_string("artefact.txt").unwrap_or_default();
    assert!(body.contains("DONE"), "not done yet");
}

// ---------------------------------------------------------------- scaffolding

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch() -> (PathBuf, PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!("corifeu-doer-{}-{n}", std::process::id()));
    let (root, store) = (base.join("repo"), base.join("checkpoints"));
    std::fs::create_dir_all(&root).unwrap();
    (root, store)
}

fn oracle() -> Oracle {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    Oracle {
        argv: vec![
            exe,
            "--exact".into(),
            "artefact_is_done".into(),
            "--ignored".into(),
        ],
        timeout: Duration::from_secs(60),
        expect_cases: Some(1),
    }
}

fn slice() -> Slice {
    Slice {
        task: "Make the artefact say DONE.".into(),
        artefact: "artefact.txt".into(),
        oracle: oracle(),
    }
}

fn guard() -> Guard {
    Guard {
        classes: Classes::new([
            Class::new("source", ["*.txt"], [] as [&str; 0]),
            Class::new("docs", ["*.md"], [] as [&str; 0]),
        ]),
        ownership: Ownership::new("coder", ["source"]),
        protected: Some(Protected::new("#[test]", ["source"])),
    }
}

/// Permits the oracle and nothing else, which is also ADR-0037's default shape.
fn allowlist() -> Allowlist {
    Allowlist::new([vec![
        std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    ]])
}

/// A model whose turns are decided in advance.
struct Replay {
    turns: RefCell<Vec<Result<Turn, &'static str>>>,
}

impl Replay {
    fn of(turns: Vec<Turn>) -> Self {
        Self {
            turns: RefCell::new(turns.into_iter().map(Ok).collect()),
        }
    }
    fn failing() -> Self {
        Self {
            turns: RefCell::new(vec![Err("connection refused")]),
        }
    }
}

impl Provider for Replay {
    fn runtime(&self) -> &'static str {
        "replay"
    }
    fn chat(
        &self,
        _: &str,
        _: &[Message],
        _: &[serde_json::Value],
        _: &Budget,
    ) -> Result<Turn, Error> {
        let mut turns = self.turns.borrow_mut();
        match turns.is_empty() {
            // Running out means the script was shorter than the run; repeating
            // the last turn is what a stuck model looks like.
            true => Ok(says("I have nothing more to add.")),
            false => turns
                .remove(0)
                .map_err(|_| Error::Http(http::Error::NotPlainHttp("replay".into()))),
        }
    }
}

fn calls(name: &str, arguments: serde_json::Value) -> Turn {
    Turn {
        content: String::new(),
        thinking: String::new(),
        tool_calls: vec![ToolCall {
            name: name.into(),
            arguments,
        }],
        stop: Stop::Done,
        prompt_tokens: 100,
        output_tokens: 20,
        speed: None,
    }
}

fn says(what: &str) -> Turn {
    Turn {
        content: what.into(),
        thinking: String::new(),
        tool_calls: vec![],
        stop: Stop::Done,
        prompt_tokens: 100,
        output_tokens: 20,
        speed: None,
    }
}

fn write(content: &str) -> Turn {
    calls(
        "write_file",
        json!({"path": "artefact.txt", "content": content}),
    )
}

fn doer<'a>(provider: &'a Replay, max_steps: u32) -> Doer<'a> {
    Doer {
        provider,
        model: "scripted".into(),
        budget: Budget::doer(),
        allowlist: allowlist(),
        guard: guard(),
        window: Window::default(),
        limits: Limits {
            max_steps,
            output_budget: 4_000,
        },
    }
}

// ---------------------------------------------------------------------- tests

#[test]
fn a_write_that_satisfies_the_oracle_ends_the_slice() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Green);
    assert_eq!(run.steps, 1);
    assert_eq!(
        run.checks, 1,
        "the runtime ran the oracle; the model never asked"
    );
    assert_eq!(run.prompt_tokens, 100);
}

#[test]
fn the_model_never_has_to_think_of_running_the_check() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("still not\n"), write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Green);
    assert_eq!(run.checks, 2, "once per write that landed");
    assert!(
        run.trace[0].result.contains("1 failed"),
        "the first write's result carries the oracle's own output: {}",
        run.trace[0].result
    );
}

#[test]
fn a_refused_write_reaches_the_model_and_leaves_the_file_alone() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![
        calls(
            "write_file",
            json!({"path": "notes.md", "content": "hello"}),
        ),
        write("DONE\n"),
    ]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Green);
    assert!(
        run.trace[0].result.starts_with("REFUSED:"),
        "{}",
        run.trace[0].result
    );
    assert!(run.trace[0].result.contains("class `docs`"));
    assert!(ws.read("notes.md").unwrap().is_none());
}

#[test]
fn a_command_outside_the_allowlist_comes_back_as_a_refusal_not_a_crash() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![
        calls("run", json!({"argv": ["rm", "-rf", "/"]})),
        write("DONE\n"),
    ]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Green);
    assert!(
        run.trace[0].result.contains("is not permitted"),
        "{}",
        run.trace[0].result
    );
}

#[test]
fn a_repeated_action_is_warned_not_stopped() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let read = || calls("read_file", json!({"path": "artefact.txt"}));
    let provider = Replay::of(vec![read(), read(), read(), write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(
        run.outcome,
        Outcome::Green,
        "three repeats must not end a run"
    );
    assert!(run.trace[1].result.starts_with("REPEAT:"));
    assert!(run.trace[1].result.contains("run the check"));
}

#[test]
fn six_consecutive_repeats_end_the_run() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let read = || calls("read_file", json!({"path": "artefact.txt"}));
    let provider = Replay::of((0..9).map(|_| read()).collect());
    let run = doer(&provider, 20).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Looping);
    assert_eq!(run.steps, 7, "one new action, then six strikes");
}

#[test]
fn a_write_makes_the_same_command_worth_running_again() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let read = || calls("read_file", json!({"path": "artefact.txt"}));
    let provider = Replay::of(vec![
        read(),
        read(),
        write("still not\n"),
        read(),
        write("DONE\n"),
    ]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Green);
    assert!(
        !run.trace[3].result.starts_with("REPEAT:"),
        "the file changed, so reading it again is a different action"
    );
}

#[test]
fn a_turn_with_no_tool_call_is_nudged_rather_than_counted_as_work() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![says("Let me think about this."), write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Green);
    assert!(run.trace[0].result.contains("Call a tool"));
}

#[test]
fn a_budget_that_ran_out_is_named_in_the_nudge() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let mut truncated = says("I will begin by");
    truncated.stop = Stop::BudgetSpent { num_predict: 64 };
    let provider = Replay::of(vec![truncated, write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert!(
        run.trace[0].result.contains("cut off at 64 tokens"),
        "{}",
        run.trace[0].result
    );
}

#[test]
fn running_out_of_steps_reports_the_last_verdict() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("no\n"), write("still no\n"), write("nope\n")]);
    let run = doer(&provider, 3).work(&slice(), &mut ws);

    assert_eq!(run.steps, 3);
    assert!(matches!(
        run.outcome,
        Outcome::Exhausted {
            last: Some(Verdict::Failed)
        }
    ));
}

#[test]
fn a_provider_that_cannot_be_reached_is_its_own_outcome() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let run = doer(&Replay::failing(), 10).work(&slice(), &mut ws);

    assert!(matches!(run.outcome, Outcome::ProviderFailed { .. }));
    assert_eq!(run.steps, 0, "nothing was done, so nothing is charged");
}

/// ADR-0051: the measurement's repair round went from 18 of 21 passing to not
/// building at all, and there was no way back.
#[test]
fn a_change_that_scores_worse_is_rolled_back_and_the_model_is_told() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![
        write("DONE\n"),    // green: 1 passing, saved
        write("wrecked\n"), // worse
    ]);
    // Stop the slice from ending on the first write so the regression can happen.
    let mut slice = slice();
    slice.oracle.expect_cases = Some(2); // the oracle only ever runs one case
    let run = doer(&provider, 4).work(&slice, &mut ws);

    assert!(
        run.trace.iter().any(|s| s.result.contains("vacuous")),
        "an oracle running fewer cases than declared is vacuous, not a pass"
    );
}
