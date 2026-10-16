//! QA writes the test, the coder makes it pass.
//!
//! The oracle is this test binary. `tests.txt` holds one demand per line — that
//! is QA's artefact — and `artefact.txt` must satisfy them, which is the
//! coder's. Three cases always run, so red and green are both real and the
//! count ADR-0057 needs is real too.

use corifeu_agent::Window;
use corifeu_doer::pair::{Pair, SliceOutcome};
use corifeu_doer::{Doer, Goal, Limits, Oracle, Slice, Steps};
use corifeu_exec::Allowlist;
use corifeu_planner::{Plan, PlannedSlice};
use corifeu_provider::{Budget, Error, Message, Provider, Stop, ToolCall, Turn};
use corifeu_workspace::{Class, Classes, Guard, Ownership, Protected, Workspace};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn demands() -> Vec<String> {
    std::fs::read_to_string("tests.txt")
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn satisfies(n: usize) {
    let demands = demands();
    let Some(demand) = demands.get(n - 1) else {
        return; // QA has not asked for this one; nothing to judge.
    };
    let answer = std::fs::read_to_string("artefact.txt").unwrap_or_default();
    assert!(answer.contains(demand.trim()), "missing {demand:?}");
}

#[ignore]
#[test]
fn demand_1() {
    satisfies(1);
}
#[ignore]
#[test]
fn demand_2() {
    satisfies(2);
}
#[ignore]
#[test]
fn demand_3() {
    satisfies(3);
}

// ---------------------------------------------------------------- scaffolding

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch() -> (PathBuf, PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!("corifeu-pair-{}-{n}", std::process::id()));
    let (root, store) = (base.join("repo"), base.join("checkpoints"));
    std::fs::create_dir_all(&root).unwrap();
    (root, store)
}

fn exe() -> String {
    std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn oracle() -> Oracle {
    Oracle {
        argv: vec![exe(), "--ignored".into(), "demand_".into()],
        timeout: Duration::from_secs(60),
        expect_cases: None,
    }
}

fn slice() -> Slice {
    Slice {
        task: "The artefact must satisfy every demand.".into(),
        artefact: "artefact.txt".into(),
        oracle: oracle(),
    }
}

fn classes() -> Classes {
    Classes::new([
        Class::new("test", ["tests.txt"], [] as [&str; 0]),
        Class::new("source", ["artefact.txt"], [] as [&str; 0]),
    ])
}

fn guard(role: &str, owns: &str) -> Guard {
    Guard {
        classes: classes(),
        ownership: Ownership::new(role, [owns]),
        protected: Some(Protected::new("#[test]", ["source"])),
    }
}

struct Replay(RefCell<Vec<Turn>>);

impl Provider for Replay {
    fn runtime(&self) -> &'static str {
        "replay"
    }
    fn chat(&self, _: &str, _: &[Message], _: &[Value], _: &Budget) -> Result<Turn, Error> {
        let mut turns = self.0.borrow_mut();
        if turns.is_empty() {
            return Ok(says("I have nothing more to add."));
        }
        Ok(turns.remove(0))
    }
}

/// A provider that is not there. Nothing after it can succeed.
struct Gone;

impl Provider for Gone {
    fn runtime(&self) -> &'static str {
        "gone"
    }
    fn chat(&self, _: &str, _: &[Message], _: &[Value], _: &Budget) -> Result<Turn, Error> {
        Err(Error::Shape {
            runtime: "gone",
            detail: "connection refused".into(),
        })
    }
}

fn turn(tool_calls: Vec<ToolCall>, content: &str) -> Turn {
    Turn {
        content: content.into(),
        thinking: String::new(),
        tool_calls,
        stop: Stop::Done,
        prompt_tokens: 100,
        output_tokens: 20,
        speed: None,
    }
}

fn says(what: &str) -> Turn {
    turn(vec![], what)
}

fn write(path: &str, content: &str) -> Turn {
    turn(
        vec![ToolCall {
            name: "write_file".into(),
            arguments: json!({"path": path, "content": content}),
        }],
        "",
    )
}

fn reads(path: &str) -> Turn {
    turn(
        vec![ToolCall {
            name: "read_file".into(),
            arguments: json!({"path": path}),
        }],
        "",
    )
}

fn doer<'a>(provider: &'a dyn Provider, goal: Goal, role: &str, owns: &str) -> Doer<'a> {
    Doer {
        provider,
        goal,
        model: "scripted".into(),
        budget: Budget::doer(),
        allowlist: Allowlist::new([vec![exe()]]),
        guard: guard(role, owns),
        window: Window::default(),
        limits: Limits {
            steps: Steps::fixed(6),
            output_budget: 4_000,
            repeats: Limits::default().repeats,
        },
    }
}

fn pair<'a>(qa: &'a dyn Provider, coder: &'a dyn Provider, qa_rounds: u32) -> Pair<'a> {
    Pair {
        qa: doer(qa, Goal::TestRuns, "qa", "test"),
        coder: doer(coder, Goal::Green, "coder", "source"),
        qa_rounds,
    }
}

// ---------------------------------------------------------------------- tests

#[test]
fn qa_writes_a_failing_test_and_the_coder_makes_it_pass() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let qa = Replay(RefCell::new(vec![write("tests.txt", "A\nB\n")]));
    let coder = Replay(RefCell::new(vec![write("artefact.txt", "A and B\n")]));
    let run = pair(&qa, &coder, 2).work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::Green { cases: 3 });
    assert_eq!(run.qa.len(), 1);
    assert_eq!(run.coder.len(), 1);
}

/// ADR-0057: nobody knows how many cases a slice has until the tests exist, so
/// the planner cannot set the guard and QA's red run does.
#[test]
fn the_coders_guard_gets_its_count_from_qas_red_run() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let qa = Replay(RefCell::new(vec![write("tests.txt", "A\n")]));
    let coder = Replay(RefCell::new(vec![write("artefact.txt", "A\n")]));
    let run = pair(&qa, &coder, 2).work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::Green { cases: 3 });
    assert_eq!(
        run.qa[0].cases.ran(),
        Some(3),
        "QA's run is what counted them"
    );
}

/// ADR-0058, seen from the other side. A test that passes the moment it is
/// written says the work was already done. The measurement found this is what
/// rescues a looping slice, so it is a result and not a QA failure.
#[test]
fn a_test_that_passes_when_written_means_the_work_already_existed() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("artefact.txt", "A and B\n", &guard("setup", "source"))
        .unwrap();

    let qa = Replay(RefCell::new(vec![write("tests.txt", "A\nB\n")]));
    let coder = Replay(RefCell::new(vec![]));
    let run = pair(&qa, &coder, 2).work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::DeliveredEarly { cases: 3 });
    assert!(run.coder.is_empty(), "the coder was never asked");
}

#[test]
fn qa_that_never_writes_hands_the_coder_nothing() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    // Six identical reads: the loop detector stops it, and the oracle never ran.
    let qa = Replay(RefCell::new((0..9).map(|_| reads("tests.txt")).collect()));
    let coder = Replay(RefCell::new(vec![]));
    let run = pair(&qa, &coder, 2).work(&slice(), &mut ws);

    assert!(matches!(run.outcome, SliceOutcome::NoRunnableTest { .. }));
    assert!(
        run.coder.is_empty(),
        "a test that never ran is not an oracle"
    );
}

/// ADR-0059 §2: the test is one of three suspects and the cheapest to re-read,
/// so it goes back to QA before the plan goes back to the planner.
#[test]
fn a_coder_that_cannot_pass_sends_the_test_back_to_qa() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let qa = Replay(RefCell::new(vec![
        write("tests.txt", "IMPOSSIBLE\n"),
        write("tests.txt", "B\n"), // QA re-reads the task and corrects itself
    ]));
    let mut coder_turns: Vec<Turn> = (0..6).map(|_| write("artefact.txt", "A\n")).collect();
    coder_turns.push(write("artefact.txt", "A and B\n"));
    let coder = Replay(RefCell::new(coder_turns));
    let run = pair(&qa, &coder, 2).work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::Green { cases: 3 });
    assert_eq!(run.qa.len(), 2, "QA was asked twice");
    assert_eq!(run.coder.len(), 2);
}

#[test]
fn a_slice_neither_role_can_finish_says_how_many_times_qa_tried() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let qa = Replay(RefCell::new(vec![
        write("tests.txt", "IMPOSSIBLE\n"),
        write("tests.txt", "ALSO IMPOSSIBLE\n"),
    ]));
    let coder = Replay(RefCell::new(
        (0..12).map(|_| write("artefact.txt", "no\n")).collect(),
    ));
    let run = pair(&qa, &coder, 2).work(&slice(), &mut ws);

    assert!(matches!(
        run.outcome,
        SliceOutcome::NotPassed { qa_rounds: 2, .. }
    ));
}

#[test]
fn neither_role_can_write_the_others_class() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    // QA tries to implement; the coder tries to edit the tests. Both refused.
    let qa = Replay(RefCell::new(vec![
        write("artefact.txt", "A\n"),
        write("tests.txt", "A\n"),
    ]));
    let coder = Replay(RefCell::new(vec![
        write("tests.txt", ""),
        write("artefact.txt", "A\n"),
    ]));
    let run = pair(&qa, &coder, 1).work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::Green { cases: 3 });
    assert!(run.qa[0].trace[0].result.starts_with("REFUSED:"));
    assert!(run.coder[0].trace[0].result.starts_with("REFUSED:"));
}

// ----------------------------------------------------------------- the walk

fn planned(task: &str) -> PlannedSlice {
    PlannedSlice {
        task: task.into(),
        artefact: "artefact.txt".into(),
        oracle: oracle().argv,
    }
}

/// Defect 18: a slice that does not close is not the same as work that is
/// missing. The walk carries on, and asks the unfinished slice's check once
/// more when the rest of the plan has run.
#[test]
fn a_slice_that_did_not_close_is_asked_again_at_the_end() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let qa = Replay(RefCell::new(vec![
        write("tests.txt", "A\n"),
        write("tests.txt", "A\nB\n"),
    ]));
    // Six writes spend the first slice's whole allowance without satisfying
    // anything. The seventh belongs to the second slice, and satisfies both.
    let mut coder_turns: Vec<Turn> = (0..6).map(|_| write("artefact.txt", "no\n")).collect();
    coder_turns.push(write("artefact.txt", "A and B\n"));
    let coder = Replay(RefCell::new(coder_turns));

    let plan = Plan {
        slices: vec![planned("First."), planned("Second.")],
    };
    let walked = pair(&qa, &coder, 1).walk(&plan, Duration::from_secs(60), &mut ws);

    assert_eq!(
        walked.slices.len(),
        2,
        "the failed slice did not end the walk"
    );
    assert!(!walked.slices[0].1.is_done());
    assert!(walked.slices[1].1.is_done());

    assert!(
        !walked.all_done(),
        "the first slice never closed under its role"
    );
    assert!(walked.delivered(), "but its check passes now");
    assert_eq!(walked.overtaken(), 1);
    assert_eq!(walked.stalled().count(), 0);
}

/// Nothing after an unreachable provider can succeed, so nothing after it runs.
#[test]
fn an_unreachable_provider_ends_the_walk() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let plan = Plan {
        slices: vec![planned("First."), planned("Second.")],
    };
    let walked = pair(&Gone, &Gone, 1).walk(&plan, Duration::from_secs(60), &mut ws);

    assert_eq!(walked.slices.len(), 1, "the second slice was attempted");
    assert!(walked.abandoned());
    assert!(!walked.delivered());
}

#[test]
fn a_walk_that_finishes_reports_every_slice() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let qa = Replay(RefCell::new(vec![
        write("tests.txt", "A\n"),
        write("tests.txt", "A\nB\n"),
    ]));
    let coder = Replay(RefCell::new(vec![
        write("artefact.txt", "A\n"),
        write("artefact.txt", "A and B\n"),
    ]));
    let plan = Plan {
        slices: vec![planned("Satisfy A."), planned("Satisfy B too.")],
    };

    let walked = pair(&qa, &coder, 1).walk(&plan, Duration::from_secs(60), &mut ws);

    assert!(walked.all_done());
    assert_eq!(walked.slices.len(), 2);
    assert_eq!(walked.stalled().count(), 0);
    assert!(walked.prompt_tokens() > 0);
}

/// The other way QA's second round can end: the coder had already written what
/// the corrected demand asks for, while failing the first one.
#[test]
fn a_corrected_demand_the_coder_already_met_is_delivered_early() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let qa = Replay(RefCell::new(vec![
        write("tests.txt", "IMPOSSIBLE\n"),
        write("tests.txt", "A\n"),
    ]));
    let coder = Replay(RefCell::new(
        (0..6).map(|_| write("artefact.txt", "A\n")).collect(),
    ));
    let run = pair(&qa, &coder, 2).work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::DeliveredEarly { cases: 3 });
    assert_eq!(run.coder.len(), 1, "the coder was not asked a second time");
}

/// ADR-0053's failure mode, caught. In Rust a unit test lives inside the file
/// it tests, so QA must own `source` and nothing stops it writing the
/// implementation too. Observed against a live model on the very first run:
/// QA wrote six tests and the function, and the coder was never asked.
#[test]
fn qa_writing_the_implementation_too_is_not_the_same_as_finding_it_done() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    // The demands exist and are unmet, so the check is red before QA is asked.
    ws.write("tests.txt", "A\n", &guard("setup", "test"))
        .unwrap();
    let qa = Replay(RefCell::new(vec![
        write("artefact.txt", "A\n"), // QA doing the coder's job
    ]));
    let coder = Replay(RefCell::new(vec![]));
    let both = Pair {
        // One role that owns everything, which is what Rust forces.
        qa: doer(&qa, Goal::TestRuns, "qa", "source"),
        coder: doer(&coder, Goal::Green, "coder", "source"),
        qa_rounds: 1,
    };
    let run = both.work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::SelfTested { cases: 3 });
    assert!(run.self_tested());
    assert!(
        run.is_done(),
        "the code is correct; refusing it would be perverse"
    );
}

#[test]
fn work_that_existed_before_qa_started_is_still_delivered_early() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    // Already satisfied, and already under test, before QA is asked.
    ws.write("artefact.txt", "A\n", &guard("setup", "source"))
        .unwrap();
    ws.write("tests.txt", "A\n", &guard("setup", "test"))
        .unwrap();

    let qa = Replay(RefCell::new(vec![write("tests.txt", "A\n")]));
    let coder = Replay(RefCell::new(vec![]));
    let run = pair(&qa, &coder, 1).work(&slice(), &mut ws);

    assert_eq!(run.outcome, SliceOutcome::DeliveredEarly { cases: 3 });
    assert!(!run.self_tested());
}
