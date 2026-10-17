//! The two failures these rules come from, written as tests.

use corifeu_agent::{Action, Artefact, Contract, Detector, Repeat, Step, Window};

fn contract() -> Contract {
    Contract {
        task: "Make match_seg pass its tests.".into(),
        acceptance: "cargo test --lib slice2_seg".into(),
    }
}

fn artefact(bytes: usize) -> Artefact {
    Artefact {
        path: "src/lib.rs".into(),
        body: "x".repeat(bytes),
    }
}

fn trace(steps: u32) -> Vec<Step> {
    (1..=steps)
        .map(|n| {
            Step::new(
                n,
                Action::new("run", format!("cargo test --lib slice{n}")),
                format!("exit=101\nerror number {n}\n"),
            )
        })
        .collect()
}

/// The measured failure: a 9,570-byte file, `k = 3`, read in chunks, each chunk
/// evicted before the next arrived. 28 steps reading, 0 edits, 1,064 output
/// tokens. Nothing about the window size fixes it — the artefact has to stop
/// travelling through the window at all.
#[test]
fn the_artefact_survives_a_window_that_has_evicted_everything_else() {
    let long_run = trace(28);
    let context = Window { recent: 3 }.build(&contract(), vec![artefact(9_570)], &long_run);

    assert_eq!(context.recent.len(), 3);
    assert_eq!(context.compacted.len(), 25);

    let [pinned] = &context.artefacts[..] else {
        panic!("the file being edited is always present");
    };
    assert_eq!(pinned.body.len(), 9_570, "and it is always whole");
}

#[test]
fn the_pinned_file_says_it_is_current_so_the_doer_does_not_check() {
    let preamble = artefact(10).preamble();
    assert!(preamble.contains("read fresh from disk this step"));
    assert!(preamble.contains("never need to read it again"));
}

#[test]
fn older_steps_are_replaced_by_one_line_rather_than_dropped() {
    let context = Window { recent: 2 }.build(&contract(), Vec::new(), &trace(5));

    assert_eq!(context.compacted.len(), 3);
    assert_eq!(
        context.compacted[0],
        "step 1: run cargo test --lib slice1 -> exit=101"
    );
    assert_eq!(
        context.recent.first().map(|s| s.number),
        Some(4),
        "the last two steps keep their full result"
    );
    assert!(context.recent[0].result.contains("error number 4"));
}

#[test]
fn a_short_run_compacts_nothing() {
    let context = Window::default().build(&contract(), Vec::new(), &trace(2));
    assert!(context.compacted.is_empty());
    assert_eq!(context.recent.len(), 2);
}

#[test]
fn the_contract_does_not_move_with_the_window() {
    let early = Window::default().build(&contract(), Vec::new(), &trace(1));
    let late = Window::default().build(&contract(), Vec::new(), &trace(200));
    assert_eq!(early.contract, late.contract);
    assert_eq!(late.contract.acceptance, "cargo test --lib slice2_seg");
}

/// The other measured failure: the loop detector killed a run at step 12 of 400
/// while it held 18 of 21 tests passing. Repeats are a warning first.
#[test]
fn a_run_that_searches_is_not_a_run_that_loops() {
    let mut detector = Detector::default();
    let mut stopped = false;

    // Four dead ends in a row, then a real change — Task B's winning round,
    // twice over.
    for _ in 0..4 {
        if let Repeat::Spent { .. } = detector.consider(&Action::new("run", "cargo test")) {
            stopped = true;
        }
    }
    detector.artefact_changed();
    for _ in 0..4 {
        if let Repeat::Spent { .. } = detector.consider(&Action::new("run", "cargo test")) {
            stopped = true;
        }
    }

    assert!(
        !stopped,
        "this run reached 21 of 21 after exactly this shape"
    );
}

#[test]
fn the_warning_arrives_where_the_doer_is_already_looking() {
    let mut detector = Detector::default();
    let read = Action::new("read_file", "src/lib.rs");
    detector.consider(&read);

    let Repeat::Warned {
        warning,
        strike,
        of,
    } = detector.consider(&read)
    else {
        panic!("a second identical read warns");
    };

    assert_eq!((strike, of), (1, 6));
    // It is returned as the result of the call the doer just made, so it is a
    // tool result and not a new system message it has to be told to read.
    assert!(warning.starts_with("REPEAT:"));
    assert!(warning.contains("included in full, fresh from disk"));
}

#[test]
fn a_write_makes_every_earlier_command_worth_running_again() {
    let mut detector = Detector::default();
    let check = Action::new("run", "cargo test --lib slice2_seg");

    detector.consider(&check);
    assert!(matches!(detector.consider(&check), Repeat::Warned { .. }));

    detector.artefact_changed();

    assert_eq!(detector.consider(&check), Repeat::New);
}
