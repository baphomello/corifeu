//! QA writes the test, the coder makes it pass — and then twelve cases neither
//! of them saw decide whether it was any good.
//!
//! The fourth run measured who wrote what: in 11 of 20 pairs QA produced the
//! implementation as well as the test (`SELF_TESTED`), which is ADR-0053's
//! failure mode happening. What it did not measure is whether that code is
//! worse, and that is the question the ADR actually turns on. So this round
//! ends by running `held-out.rs` against the crate the pair produced.
//!
//! Usage: `pair <host> <model> <workdir> <iteration> [slice]`
//!
//! `slice` is 1 by default. Which slice is the variable round five's Q4 is
//! about: slice 1 is four lines, so writing the test costs about what writing
//! the implementation costs, which is the condition under which QA would write
//! both without noticing. Slice 3 is recursive and nine cases.
//!
//! Reads the step allowance from `BENCH_STEPS_*`.

use corifeu_agent::Window;
use corifeu_bench::{Task, held_out, write_steps};
use corifeu_doer::pair::{Pair, SliceOutcome};
use corifeu_doer::{Doer, Goal, Limits};
use corifeu_llamacpp::LlamaCpp;
use corifeu_provider::Budget;
use corifeu_workspace::{Guard, Ownership, Workspace};
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let host = args.next().expect("host");
    let model = args.next().expect("model");
    let root = std::path::PathBuf::from(args.next().expect("workdir"));
    let iteration = args.next().unwrap_or_else(|| "1".into());
    let which: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(1);

    let task = Task::read(&root);
    let provider = LlamaCpp::new(format!("http://{host}:9931"));
    let allow = task.allowlist();
    let mut workspace = Workspace::open(&root, root.join(".checkpoints")).expect("workspace");

    let classes = task.classes();
    let budget = Budget {
        num_predict: 4096,
        num_ctx: Some(8192),
        think: false,
        seed: corifeu_bench::seed(&iteration),
    };
    let limits = Limits {
        steps: corifeu_bench::steps(),
        output_budget: 8_000,
        repeats: corifeu_bench::repeats(),
    };

    // Rust keeps a unit test inside the file it tests, so file-class ownership
    // cannot separate the two roles (defect 14, ADR-0057). Both own `source`;
    // only the coder is held to the case count QA's red run reported.
    let pair = Pair {
        qa: Doer {
            provider: &provider,
            goal: Goal::TestRuns,
            model: model.clone(),
            budget: budget.clone(),
            allowlist: allow.clone(),
            guard: Guard {
                classes: classes.clone(),
                ownership: Ownership::new("qa", ["source"]),
                protected: None,
            },
            window: Window::default(),
            limits,
        },
        coder: Doer {
            provider: &provider,
            goal: Goal::Green,
            model: model.clone(),
            budget,
            allowlist: allow.clone(),
            guard: Guard {
                classes,
                ownership: Ownership::new("coder", ["source"]),
                protected: task.protected(),
            },
            window: Window::default(),
            limits,
        },
        qa_rounds: 2,
    };

    let slice = task.slice(
        which,
        format!(
            "{} Put the tests in {}.",
            task.at(which).specify,
            task.tests_at(which)
        ),
        // QA's red run sets it (ADR-0057): nobody knows how many cases a slice
        // has until the tests exist.
        None,
    );

    let started = Instant::now();
    let run = pair.work(&slice, &mut workspace);
    let seconds = started.elapsed().as_secs_f64();

    // Both roles, labelled. ADR-0053 turns on who wrote what, and a trace that
    // did not say which role took the step would be no use for it.
    let module = &task.at(which).module;
    for (role, turns) in [("qa", &run.qa), ("coder", &run.coder)] {
        for turn in turns {
            write_steps(&root, &iteration, &model, module, role, turn);
        }
    }

    let outcome = match &run.outcome {
        SliceOutcome::Green { cases } => format!("GREEN:{cases}"),
        SliceOutcome::SelfTested { cases } => format!("SELF_TESTED:{cases}"),
        SliceOutcome::DeliveredEarly { cases } => format!("EARLY:{cases}"),
        SliceOutcome::NoRunnableTest { .. } => "NO_RUNNABLE_TEST".into(),
        SliceOutcome::NotPassed { .. } => "NOT_PASSED".into(),
    };

    // The quality question, asked of every run and not only the green ones: a
    // crate that failed its own check still has code in it to judge.
    let quality = held_out(&root, &allow);

    println!(
        "{iteration}\t{model}\t{which}\t{outcome}\t{}\t{}\t{}\t{}\t{}\t{seconds:.0}\t{quality}",
        run.qa.len(),
        run.qa.iter().map(|r| r.steps).sum::<u32>(),
        run.coder.len(),
        run.coder.iter().map(|r| r.steps).sum::<u32>(),
        run.prompt_tokens(),
    );
}
