//! One doer, four slices, one model: what a decomposition costs end to end.
//!
//! The crate starts with every function a `todo!()` and all 26 tests present,
//! so each slice is red-to-green against cases it did not write. This is the
//! shape that produced the fourth run's ranking.
//!
//! Usage: `rank <host> <model> <workdir> <iteration>`
//!
//! Writes one tab-separated row per slice to stdout, then the rows ADR-0063
//! asks for, then a total. Reads the step allowance from `BENCH_STEPS_*`.

use corifeu_agent::Window;
use corifeu_doer::{Allowance, Doer, Goal, Limits, Oracle, Outcome, Slice};
use corifeu_exec::Allowlist;
use corifeu_llamacpp::LlamaCpp;
use corifeu_provider::Budget;
use corifeu_workspace::{Class, Classes, Guard, Ownership, Protected, Workspace};
use std::time::{Duration, Instant};

/// Module, expected case count, and what the slice is told.
const SLICES: [(&str, u32, &str); 4] = [
    (
        "slice1_segs",
        4,
        "`split_segs` is unimplemented. Implement it so its tests pass.",
    ),
    (
        "slice2_seg",
        8,
        "`match_seg` is unimplemented. Implement it so its tests pass. It matches ONE path \
         component against one pattern segment, where `*` is any run of characters and `?` is \
         exactly one.",
    ),
    (
        "slice3_glob",
        9,
        "`glob_match` is unimplemented. Implement it so its tests pass. `**` crosses `/`, `*` \
         and `?` do not.",
    ),
    (
        "slice4_classify",
        5,
        "`classify` is unimplemented. Implement it so its tests pass. The first class whose \
         include globs match wins.",
    ),
];

fn slice(module: &str, cases: u32, task: &str) -> Slice {
    Slice {
        task: format!("{task} Do not change any test, and do not touch any other function."),
        artefact: "src/lib.rs".into(),
        oracle: Oracle {
            argv: ["cargo", "test", "--lib", module]
                .map(String::from)
                .to_vec(),
            timeout: Duration::from_secs(240),
            expect_cases: Some(cases),
        },
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let host = args.next().expect("host");
    let model = args.next().expect("model");
    let root = std::path::PathBuf::from(args.next().expect("workdir"));
    let iteration = args.next().unwrap_or_else(|| "1".into());

    let provider = LlamaCpp::new(format!("http://{host}:9931"));
    let allow = Allowlist::new([vec!["cargo".into()]]);
    let mut workspace = Workspace::open(&root, root.join(".checkpoints")).expect("workspace");

    let doer = Doer {
        provider: &provider,
        goal: Goal::Green,
        model: model.clone(),
        budget: Budget {
            num_predict: 4096,
            num_ctx: Some(8192),
            think: false,
        },
        allowlist: allow.clone(),
        guard: Guard {
            classes: Classes::new([Class::new("source", ["src/**"], [] as [&str; 0])]),
            ownership: Ownership::new("coder", ["source"]),
            protected: Some(Protected::new("#[test]", ["source"])),
        },
        window: Window::default(),
        limits: Limits {
            steps: corifeu_bench::steps(),
            output_budget: 8_000,
        },
    };

    let whole = Instant::now();
    let mut allowance = Allowance::new(doer.limits.steps);
    let (mut steps, mut prompt, mut output, mut checks) = (0, 0, 0, 0);
    let (mut closed, mut free) = (0, 0);
    let mut unfinished = Vec::new();

    for (module, cases, task) in SLICES {
        let slice = slice(module, cases, task);

        // ADR-0058: someone may have built it already on the way past.
        if doer.probe(&slice, &workspace) {
            free += 1;
            closed += 1;
            println!("{iteration}\t{model}\t{module}\tDELIVERED_EARLY\t0\t0\t0\t0\t0\t0");
            continue;
        }

        let allowed = allowance.now();
        let started = Instant::now();
        let run = doer.work_within(&slice, allowed, &mut workspace);
        let wall = started.elapsed().as_secs_f64();
        allowance.saw(&run.outcome);

        steps += run.steps;
        prompt += run.prompt_tokens;
        output += run.output_tokens;
        checks += run.checks;
        let gone = matches!(run.outcome, Outcome::ProviderFailed { .. });
        if run.reached() {
            closed += 1;
        } else {
            unfinished.push((module, slice));
        }

        println!(
            "{iteration}\t{model}\t{module}\t{}\t{}\t{}\t{}\t{}\t{wall:.0}\t{allowed}",
            match &run.outcome {
                Outcome::Reached => "GREEN".to_string(),
                Outcome::Exhausted { .. } => "EXHAUSTED".to_string(),
                Outcome::Looping => "LOOPING".to_string(),
                Outcome::ProviderFailed { why } => format!("PROVIDER_FAILED {why}"),
            },
            run.steps,
            run.prompt_tokens,
            run.output_tokens,
            run.checks
        );

        // ADR-0063: nothing after an unreachable provider can succeed.
        if gone {
            break;
        }
    }

    // ADR-0063. One oracle run per unfinished slice, no model. A green answer
    // here says the work was late, not missing.
    let mut overtaken = 0;
    for (module, slice) in &unfinished {
        let green = doer.probe(slice, &workspace);
        if green {
            overtaken += 1;
        }
        println!(
            "AGAIN\t{iteration}\t{model}\t{module}\t{}",
            if green { "GREEN" } else { "RED" }
        );
    }

    println!(
        "TOTAL\t{iteration}\t{model}\tclosed={closed}/4\tovertaken={overtaken}\tfree={free}\
         \tsteps={steps}\tprompt={prompt}\toutput={output}\tchecks={checks}\
         \tfloor_left={}\tminutes={:.1}",
        allowance.raised(),
        whole.elapsed().as_secs_f64() / 60.0
    );
}
