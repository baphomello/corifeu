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
        artefact: "artefact.txt".into(),
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
        artefact: "artefact.txt".into(),
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
