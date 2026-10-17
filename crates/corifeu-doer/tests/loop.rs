//! The loop, against a scripted model and an oracle that really changes colour.
//!
//! The oracle is this test binary running one of its own ignored tests, which
//! reads a file from the working directory. That makes a red-to-green
//! transition real and portable: no shell, no `cargo` inside `cargo`, and the
//! `test result:` line ADR-0057's case counter reads comes out of it for free.

use corifeu_agent::Window;
use corifeu_check::Verdict;
use corifeu_doer::{Allowance, Doer, Goal, Limits, Oracle, Outcome, Slice, Steps};
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

// A three-case oracle, so a state can be partly green and two states can be
// compared by how much of the suite passed.
fn artefact() -> String {
    std::fs::read_to_string("artefact.txt").unwrap_or_default()
}
#[ignore]
#[test]
fn marker_a() {
    assert!(artefact().contains('A'));
}
#[ignore]
#[test]
fn marker_b() {
    assert!(artefact().contains('B'));
}
#[ignore]
#[test]
fn marker_c() {
    assert!(artefact().contains('C'));
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
        artefacts: vec!["artefact.txt".into()],
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
    /// What the doer actually said to the model. Kept so a test can assert on
    /// the contract rather than on the fact that a call happened.
    said: RefCell<Vec<Message>>,
}

impl Replay {
    fn of(turns: Vec<Turn>) -> Self {
        Self {
            turns: RefCell::new(turns.into_iter().map(Ok).collect()),
            said: RefCell::new(Vec::new()),
        }
    }
    fn failing() -> Self {
        Self {
            said: RefCell::new(Vec::new()),
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
        messages: &[Message],
        _: &[serde_json::Value],
        _: &Budget,
    ) -> Result<Turn, Error> {
        self.said.borrow_mut().extend_from_slice(messages);
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
        cost_usd: None,
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
        cost_usd: None,
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
        goal: Goal::Green,
        model: "scripted".into(),
        budget: Budget::doer(),
        allowlist: allowlist(),
        guard: guard(),
        window: Window::default(),
        limits: Limits {
            steps: Steps::fixed(max_steps),
            output_budget: 4_000,
            repeats: Limits::default().repeats,
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

    assert_eq!(run.outcome, Outcome::Reached);
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

    assert_eq!(run.outcome, Outcome::Reached);
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

    assert_eq!(run.outcome, Outcome::Reached);
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

    assert_eq!(run.outcome, Outcome::Reached);
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
        Outcome::Reached,
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

    assert_eq!(run.outcome, Outcome::Reached);
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

    assert_eq!(run.outcome, Outcome::Reached);
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

fn markers_oracle() -> Oracle {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    Oracle {
        argv: vec![exe, "--ignored".into(), "marker_".into()],
        timeout: Duration::from_secs(60),
        expect_cases: Some(3),
    }
}

fn markers_slice() -> Slice {
    Slice {
        task: "Make the artefact contain A, B and C.".into(),
        artefacts: vec!["artefact.txt".into()],
        oracle: markers_oracle(),
    }
}

/// ADR-0051. The measurement's repair round went from 18 of 21 passing to not
/// building at all and could not get back. Scoring every failing run as zero
/// cannot see that, which is why the score is what passed and not what ran.
#[test]
fn losing_ground_is_rolled_back_even_though_both_states_are_red() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "nothing\n", &guard()).unwrap();

    let provider = Replay::of(vec![
        write("AB\n"), // two of three pass
        write("x\n"),  // none pass — worse, not merely still red
    ]);
    let run = doer(&provider, 2).work(&markers_slice(), &mut ws);

    let rolled = run
        .trace
        .iter()
        .find(|s| s.result.contains("ROLLED BACK"))
        .expect("going from 2 passing to 0 is a regression");
    assert!(
        rolled.result.contains("You had 0 passing; it has 2"),
        "{}",
        rolled.result
    );
    assert_eq!(
        ws.read("artefact.txt").unwrap().unwrap(),
        "AB\n",
        "the better state is what is on disk again"
    );
}

#[test]
fn a_run_carries_on_from_the_state_it_was_rolled_back_to() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "nothing\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("AB\n"), write("x\n"), write("ABC\n")]);
    let run = doer(&provider, 4).work(&markers_slice(), &mut ws);

    assert_eq!(
        run.outcome,
        Outcome::Reached,
        "a rollback is a setback, not an end"
    );
    assert!(run.trace.iter().any(|s| s.result.contains("ROLLED BACK")));
}

#[test]
fn still_red_but_no_worse_is_not_rolled_back() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "nothing\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("A\n"), write("AB\n"), write("ABC\n")]);
    let run = doer(&provider, 5).work(&markers_slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Reached);
    assert!(
        !run.trace.iter().any(|s| s.result.contains("ROLLED BACK")),
        "every step gained ground; nothing should have been undone"
    );
}

/// A model that checks its own work with an extra flag is running the same
/// check, and must get the same judging, scoring and rollback.
#[test]
fn the_oracle_is_matched_by_prefix_not_by_exact_argv() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "DONE\n", &guard()).unwrap();

    let mut with_flag = oracle().argv;
    with_flag.push("--nocapture".into());

    let provider = Replay::of(vec![calls("run", json!({"argv": with_flag}))]);
    let run = doer(&provider, 3).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Reached);
    assert_eq!(
        run.checks, 1,
        "it was judged as the oracle, not run as a bare command"
    );
}

/// A score only means something next to the check that produced it.
///
/// Found in the twelve-run benchmark: a model closed slice 3 green with nine
/// cases passing, then closed slice 4 green with five. Five is fewer than
/// nine, so the rollback read the second green as a regression, restored the
/// slice-3 state over it, and reported the slice green anyway. The run said
/// 4 of 4 and the crate still held a `todo!()`.
#[test]
fn a_green_slice_is_not_a_regression_from_a_larger_one() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "nothing\n", &guard()).unwrap();

    // A three-case slice goes green.
    let wide = Replay::of(vec![write("ABC\n")]);
    assert_eq!(
        doer(&wide, 3).work(&markers_slice(), &mut ws).outcome,
        Outcome::Reached
    );

    // A one-case slice on the same file goes green with fewer cases passing.
    let narrow_slice = Slice {
        task: "Keep A, B, C and add DONE.".into(),
        artefacts: vec!["artefact.txt".into()],
        oracle: oracle(), // artefact_is_done: one case
    };
    let narrow = Replay::of(vec![write("ABC DONE\n")]);
    let run = doer(&narrow, 3).work(&narrow_slice, &mut ws);

    assert_eq!(run.outcome, Outcome::Reached);
    assert!(
        !run.trace.iter().any(|s| s.result.contains("ROLLED BACK")),
        "one check's count cannot be compared with another's"
    );
    assert_eq!(
        ws.read("artefact.txt").unwrap().unwrap(),
        "ABC DONE\n",
        "the second slice's work must survive"
    );
}

// ------------------------------------------------------------- the allowance

/// ADR-0062. Three per-slice outcomes, three different things to do with the
/// allowance, and only one of them buys more steps.
fn steps() -> Steps {
    Steps {
        floor: 12,
        ceiling: 30,
        patience: 2,
        raise_by: 6,
    }
}

#[test]
fn an_allowance_starts_at_the_floor() {
    assert_eq!(Allowance::new(steps()).now(), 12);
}

#[test]
fn the_allowance_grows_only_once_patience_has_run_out() {
    let mut allowance = Allowance::new(steps());

    assert_eq!(allowance.saw(&Outcome::Exhausted { last: None }), None);
    assert_eq!(allowance.now(), 12, "one cut is not evidence");

    assert_eq!(allowance.saw(&Outcome::Exhausted { last: None }), Some(18));
    assert_eq!(allowance.now(), 18);
    assert!(allowance.raised());
}

/// A turn that went in circles was never short of steps. Counting it would
/// hand the larger allowance to exactly the runs that waste it.
#[test]
fn going_in_circles_never_buys_more_steps() {
    let mut allowance = Allowance::new(steps());

    for _ in 0..10 {
        assert_eq!(allowance.saw(&Outcome::Looping), None);
    }
    assert_eq!(allowance.now(), 12);
    assert!(!allowance.raised());
}

/// The floor was enough, so nothing is carried forward. Without this the
/// allowance only ever ratchets up, and the ceiling becomes the default.
#[test]
fn a_turn_that_reached_its_goal_puts_the_allowance_back_at_the_floor() {
    let mut allowance = Allowance::new(steps());
    allowance.saw(&Outcome::Exhausted { last: None });
    allowance.saw(&Outcome::Exhausted { last: None });
    assert_eq!(allowance.now(), 18);

    allowance.saw(&Outcome::Reached);

    assert_eq!(allowance.now(), 12);
    assert!(!allowance.raised());
}

#[test]
fn the_allowance_stops_at_the_ceiling() {
    let mut allowance = Allowance::new(steps());
    for _ in 0..40 {
        allowance.saw(&Outcome::Exhausted { last: None });
    }
    assert_eq!(allowance.now(), 30);
}

#[test]
fn a_fixed_allowance_never_grows() {
    let mut allowance = Allowance::new(Steps::fixed(7));
    for _ in 0..10 {
        assert_eq!(allowance.saw(&Outcome::Exhausted { last: None }), None);
    }
    assert_eq!(allowance.now(), 7);
}

// ------------------------------------------- defect 24: an unusable oracle

/// An oracle that cannot produce a case count at all. The test binary with a
/// flag it does not know: it exits non-zero and prints usage, so the counter
/// declines the output rather than reading a zero out of it.
fn uncountable_oracle() -> Oracle {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    Oracle {
        argv: vec![exe, "--no-such-flag".into()],
        timeout: Duration::from_secs(60),
        expect_cases: Some(1),
    }
}

/// Defect 24. The doer must not spend a single step repairing code against a
/// check that never judged it. Eight slices of one measurement cell did exactly
/// that: `cargo` was refusing to run and the verdict read as red.
#[test]
fn a_check_that_cannot_be_counted_ends_the_slice_before_any_step() {
    let (root, store) = scratch();
    let provider = Replay::of(vec![calls(
        "write_file",
        json!({"path": "artefact.txt", "contents": "DONE"}),
    )]);
    let mut ws = Workspace::open(&root, &store).unwrap();

    let run = doer(&provider, 10).work(
        &Slice {
            oracle: uncountable_oracle(),
            ..slice()
        },
        &mut ws,
    );

    assert!(
        matches!(run.outcome, Outcome::OracleUnusable { .. }),
        "{:?}",
        run.outcome
    );
    assert_eq!(run.steps, 0, "no step may be spent against a broken check");
    assert_eq!(run.prompt_tokens, 0, "and no dispatch either");
}

/// The negative case, which matters more than the positive one: a slice whose
/// work is merely unfinished must still run. A stubbed artefact fails its cases
/// and that is a count, not an absence of one — a guard that confused the two
/// would refuse every red slice, which is every slice at the start.
#[test]
fn an_unfinished_slice_is_not_mistaken_for_a_broken_check() {
    let (root, store) = scratch();
    std::fs::write(root.join("artefact.txt"), "nothing yet").unwrap();
    let provider = Replay::of(vec![calls(
        "write_file",
        json!({"path": "artefact.txt", "contents": "DONE"}),
    )]);
    let mut ws = Workspace::open(&root, &store).unwrap();

    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert!(
        !matches!(run.outcome, Outcome::OracleUnusable { .. }),
        "a red slice is work to do, not a broken oracle: {:?}",
        run.outcome
    );
    assert!(run.steps > 0, "the slice must have been worked");
}

// ------------------------------- defect 27: a wasted step must say what it was

/// Five hundred and seventy-eight wasted steps in round five all read "no tool
/// call; nothing happened", so nothing in the data could say what the model had
/// been doing instead, and D6's prompt change had to be designed blind.
#[test]
fn a_step_with_no_tool_call_records_what_the_model_said() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    let provider = Replay::of(vec![says(
        "Let me think about how to approach this problem carefully.",
    )]);

    let run = doer(&provider, 1).work(&slice(), &mut ws);

    let step = run.trace.first().expect("the wasted step is in the trace");
    assert!(
        step.outcome.contains("Let me think about how to approach"),
        "{}",
        step.outcome
    );
    assert!(
        step.result.contains("Call a tool"),
        "the model still reads the nudge, not its own words: {}",
        step.result
    );
}

/// A turn that says nothing at all is a different failure from a turn that
/// argues with the task, and the trace has to tell them apart.
#[test]
fn an_empty_answer_is_recorded_as_empty() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    let provider = Replay::of(vec![says("")]);

    let run = doer(&provider, 1).work(&slice(), &mut ws);

    assert!(
        run.trace[0].outcome.contains("(nothing at all)"),
        "{}",
        run.trace[0].outcome
    );
}

// ------------------ defect 34: an identical write is not a change

/// ADR-0050 §4 clears the repeat history when an artefact changes, because the
/// same command against changed code is a different action. Writing the same
/// bytes back is not changed code, and counting it as one disabled loop
/// detection exactly where it was needed — a model rewriting an identical file
/// reset the detector on every step and was never warned.
#[test]
fn rewriting_the_same_bytes_does_not_clear_the_repeat_history() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    // The first write is new and the rest are repeats, so the default limit of
    // six needs seven of them. The oracle never goes green, so nothing but the
    // detector can end this.
    let provider = Replay::of((0..8).map(|_| write("not yet\n")).collect());
    let run = doer(&provider, 20).work(&slice(), &mut ws);

    assert_eq!(
        run.outcome,
        Outcome::Looping,
        "an identical rewrite must count as a repeat: {:?}",
        run.outcome
    );
    assert!(run.steps < 20, "it stopped before the allowance ran out");
}

/// And the model is told, because one that cannot tell its write landed will
/// write it again.
#[test]
fn an_identical_write_says_nothing_changed() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("not yet\n")]);
    let run = doer(&provider, 1).work(&slice(), &mut ws);

    assert!(
        run.trace[0].result.contains("Nothing changed"),
        "{}",
        run.trace[0].result
    );
}

/// The negative case: a write that does change the file must still clear the
/// history, or a model correcting itself would be accused of looping.
#[test]
fn a_write_that_changes_the_file_still_clears_the_history() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("A\n"), write("A\nB\n"), write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Reached, "{:?}", run.outcome);
}

// ------------------------------ the doer is asked for rigour, not just action

/// Measured on the probe before it was put in the prompt: two sentences asking
/// for rigour took one model from six of ten to ten of ten, three times out of
/// three, where suppressing thinking, forcing greedy sampling and quadrupling
/// the token budget had each bought nothing.
///
/// The wording names no specific case, because naming them would be teaching to
/// the test — and a doer's task is not known in advance anyway.
#[test]
fn the_doer_is_told_to_be_exhaustive() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("DONE\n")]);
    let _ = doer(&provider, 1).work(&slice(), &mut ws);

    // Message is an enum, so the assertion goes through its serialised form —
    // which is also what the model receives.
    let said = provider.said.borrow();
    let system = serde_json::to_string(said.first().expect("a system message was sent"))
        .expect("a message serialises");
    assert!(
        system.contains("Be exhaustive"),
        "the contract has to ask for it: {system}"
    );
}

// ------------------------------------------------------------ S4, several files

/// A two-file oracle: it passes only when the file and its test agree, so
/// neither can be finished alone. This is the shape S4 exists for — a
/// component and its test changing in one slice.
#[ignore]
#[test]
fn pair_agrees() {
    let body = std::fs::read_to_string("component.txt").unwrap_or_default();
    let test = std::fs::read_to_string("component_test.txt").unwrap_or_default();
    assert!(!body.trim().is_empty(), "nothing written yet");
    assert_eq!(body.trim(), test.trim(), "the test does not match the file");
}

fn pair_oracle() -> Oracle {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    Oracle {
        argv: vec![
            exe,
            "--exact".into(),
            "pair_agrees".into(),
            "--ignored".into(),
        ],
        timeout: Duration::from_secs(60),
        expect_cases: Some(1),
    }
}

fn pair_slice() -> Slice {
    Slice {
        task: "Make the component and its test agree on the label.".into(),
        artefacts: vec!["component.txt".into(), "component_test.txt".into()],
        oracle: pair_oracle(),
    }
}

fn writes(path: &str, content: &str) -> Turn {
    calls("write_file", json!({"path": path, "content": content}))
}

/// S4 acceptance 13. Both files are in front of the model on every step, read
/// fresh from disk — so the second step sees what the first one wrote, without
/// being told.
#[test]
fn every_artefact_the_slice_names_is_pinned_into_every_step() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("component.txt", "Save\n", &guard()).unwrap();
    ws.write("component_test.txt", "Cancel\n", &guard())
        .unwrap();

    let provider = Replay::of(vec![
        writes("component_test.txt", "Submit\n"),
        writes("component.txt", "Submit\n"),
    ]);
    let run = doer(&provider, 10).work(&pair_slice(), &mut ws);
    assert_eq!(run.outcome, Outcome::Reached);
    assert_eq!(run.steps, 2);

    let said = serde_json::to_string(&*provider.said.borrow()).expect("messages serialise");
    assert_eq!(
        said.matches("Current contents of component.txt").count(),
        2,
        "pinned on both steps: {said}"
    );
    assert_eq!(
        said.matches("Current contents of component_test.txt")
            .count(),
        2,
        "and so is the other one: {said}"
    );
    assert!(
        said.contains("Cancel") && said.contains("Submit"),
        "read fresh: step one saw the stale test, step two saw the written one"
    );
    assert!(
        said.contains("files that change together"),
        "and the contract stops calling it one file: {said}"
    );
}

/// S4 acceptance 14, read against [ADR-0058](../../../docs/adr/0058-slice-boundaries-are-observed.md):
/// naming two files does not widen what a role may write. The slice is not the
/// permission — the file class is — so a write outside the owned class is
/// refused and the refusal names the class.
#[test]
fn naming_two_artefacts_does_not_grant_a_third() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("component.txt", "Save\n", &guard()).unwrap();
    ws.write("component_test.txt", "Save\n", &guard()).unwrap();

    // `notes.md` is class `docs`, which the coder does not own.
    let refused = ws.write("notes.md", "a note\n", &guard());
    let message = refused.expect_err("docs is not the coder's").to_string();
    assert!(
        message.contains("class `docs`") && message.contains("source"),
        "the refusal names the class and what the role does own: {message}"
    );
    assert!(
        ws.read("notes.md").unwrap().is_none(),
        "a refused write leaves nothing behind"
    );
}

/// S4 acceptance 16, as the mechanism rather than as a site feature: two files
/// change in one slice and the oracle that needs both goes green.
#[test]
fn a_file_and_its_test_change_in_one_slice() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let provider = Replay::of(vec![
        writes("component.txt", "Submit\n"),
        writes("component_test.txt", "Submit\n"),
    ]);
    let run = doer(&provider, 10).work(&pair_slice(), &mut ws);

    assert_eq!(run.outcome, Outcome::Reached);
    assert_eq!(run.checks, 2, "the oracle ran after each write");
    assert_eq!(ws.read("component.txt").unwrap().unwrap(), "Submit\n");
    assert_eq!(ws.read("component_test.txt").unwrap().unwrap(), "Submit\n");
}

// Four markers across the two files, so a state can be partly right and two
// states can be compared by how much of the suite passed.
fn spread_body() -> String {
    std::fs::read_to_string("component.txt").unwrap_or_default()
}
fn spread_test() -> String {
    std::fs::read_to_string("component_test.txt").unwrap_or_default()
}
#[ignore]
#[test]
fn spread_a() {
    assert!(spread_body().contains('A'));
}
#[ignore]
#[test]
fn spread_b() {
    assert!(spread_body().contains('B'));
}
#[ignore]
#[test]
fn spread_c() {
    assert!(spread_test().contains('C'));
}
#[ignore]
#[test]
fn spread_d() {
    assert!(spread_test().contains('D'));
}

fn spread_slice() -> Slice {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    Slice {
        task: "Put A and B in the component, C and D in its test.".into(),
        artefacts: vec!["component.txt".into(), "component_test.txt".into()],
        oracle: Oracle {
            argv: vec![exe, "spread_".into(), "--ignored".into()],
            timeout: Duration::from_secs(60),
            expect_cases: Some(4),
        },
    }
}

/// S4 acceptance 15 at the doer rather than the workspace: the note a doer is
/// given after a rollback has to name every file that went back, or it will
/// re-apply the half it still believes in.
#[test]
fn a_rollback_tells_the_doer_about_both_files() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("component.txt", "A\n", &guard()).unwrap();
    ws.write("component_test.txt", "C\n", &guard()).unwrap();

    // Two of four pass, then a write that loses one of them. The last step
    // makes no tool call, so the run ends with the rollback note in the trace.
    let provider = Replay::of(vec![
        writes("component_test.txt", "C\n"),
        writes("component.txt", "nothing\n"),
        says("I am done."),
    ]);
    let run = doer(&provider, 3).work(&spread_slice(), &mut ws);

    let rolled = run
        .trace
        .iter()
        .find_map(|step| {
            let text = step.result.clone();
            text.contains("ROLLED BACK").then_some(text)
        })
        .expect("the regression fired");
    assert!(
        rolled.contains("component.txt") && rolled.contains("component_test.txt"),
        "the note names both: {rolled}"
    );
    assert_eq!(
        ws.read("component.txt").unwrap().unwrap(),
        "A\n",
        "and the file really went back"
    );
}

/// Why the allowance never grew across round eight's floor sweep, as a test
/// rather than as a reading of the logs.
///
/// Patience counts *consecutive* cuts, and a slice that reaches its goal puts
/// the count back. A task whose slices mostly succeed therefore never reaches
/// patience, however often it is cut: the floor stops being a starting point
/// and becomes a hard limit. ADR-0062's growth is built for a model that fails
/// repeatedly, and a model that fails occasionally never collects it.
#[test]
fn a_success_between_two_cuts_means_patience_never_runs_out() {
    let mut allowance = Allowance::new(steps());

    for _ in 0..5 {
        assert_eq!(allowance.saw(&Outcome::Exhausted { last: None }), None);
        assert_eq!(allowance.saw(&Outcome::Reached), None);
    }

    assert_eq!(
        allowance.now(),
        12,
        "five cuts, and the allowance is still the floor"
    );
    assert!(!allowance.raised());
}

// ------------------------------------------------------------- what it spent

/// S5 acceptance 18. The split has to be a number, and for a provider that
/// bills it has to be money: a CLI agent reports two input tokens on a call
/// that cost eleven cents, so tokens cannot be converted into the price
/// (defect 46).
#[test]
fn a_run_records_what_each_dispatch_spent() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("still not\n"), write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);

    assert_eq!(run.spend.dispatches, 2);
    assert_eq!(run.spend.prompt_tokens, 200, "100 a turn, as the stub says");
    assert_eq!(run.spend.output_tokens, 40);
    assert_eq!(
        run.spend.cost_usd, None,
        "a local runtime costs electricity, which is not a number it reports"
    );
}

/// `None` and `Some(0.0)` are different answers: one is a provider that does
/// not bill, the other is a bill of nothing.
#[test]
fn a_local_run_has_no_cost_rather_than_a_cost_of_zero() {
    use corifeu_doer::Spend;

    let local = Spend {
        dispatches: 3,
        prompt_tokens: 300,
        output_tokens: 60,
        cost_usd: None,
    };
    let billed = Spend {
        dispatches: 1,
        prompt_tokens: 19_342,
        output_tokens: 42,
        cost_usd: Some(0.1145),
    };

    assert_eq!(local.with(local).cost_usd, None);
    let together = local.with(billed);
    assert_eq!(together.cost_usd, Some(0.1145));
    assert_eq!(together.dispatches, 4);
}

/// The share is by money where money was spent, because that is the claim the
/// project makes. Where nothing billed, tokens are the only unit there is.
#[test]
fn the_expensive_share_is_measured_in_what_it_charged() {
    use corifeu_doer::Spend;

    let planner = Spend {
        dispatches: 2,
        prompt_tokens: 40_000,
        output_tokens: 500,
        cost_usd: Some(0.20),
    };
    let coder = Spend {
        dispatches: 20,
        prompt_tokens: 60_000,
        output_tokens: 8_000,
        cost_usd: None,
    };
    let total = planner.with(coder);

    assert_eq!(
        planner.share_of(total),
        Some(1.0),
        "the only money spent was the planner's, whatever the token counts say"
    );

    let all_local = coder.with(coder);
    let half = coder.share_of(all_local).expect("tokens are the fallback");
    assert!((half - 0.5).abs() < 1e-9, "{half}");
}

/// Defect 52, measured on a hybrid run: ten of twelve steps answered
/// `no tool call; said: I ran: write_file src/excerpt.ts`. The transcript was
/// putting one assistant message per past step reading `I ran: <action>`,
/// which is a worked example of answering in prose sitting in the slot the
/// model fills next. Nothing in the suite looked at the shape of the
/// transcript, so the loop was free to teach that.
#[test]
fn a_past_step_is_never_shown_as_the_model_answering_in_prose() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "not yet\n", &guard()).unwrap();

    let provider = Replay::of(vec![write("still not\n"), write("DONE\n")]);
    let run = doer(&provider, 10).work(&slice(), &mut ws);
    assert_eq!(run.outcome, Outcome::Reached);

    let said = provider.said.borrow();
    assert!(
        !said.iter().any(|m| matches!(m, Message::Assistant { .. })),
        "the context is rebuilt from state every step (ADR-0047), so there is no \
         assistant turn to reconstruct: {said:#?}"
    );
    // The result still has to arrive, or the fix would have cured the lesson
    // by removing the information.
    assert!(
        said.iter().any(|m| match m {
            Message::Tool { content } =>
                content.contains("write_file") && content.contains("1 failed"),
            _ => false,
        }),
        "the past step's action and its result both reach the next step: {said:#?}"
    );
}
