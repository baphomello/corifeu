//! One doer, every slice of a task, one model: what a decomposition costs end
//! to end.
//!
//! The crate starts with every function a `todo!()` and every test present, so
//! each slice is red-to-green against cases it did not write. This is the shape
//! that produced the fourth run's ranking. Which task is whatever the scaffold
//! built in the workspace — `task.json` is copied in beside the crate.
//!
//! Usage: `rank <host> <model> <workdir> <iteration>`
//!
//! Writes one tab-separated row per slice to stdout, then the rows ADR-0063
//! asks for, then a total. Reads the step allowance from `BENCH_STEPS_*`.

use corifeu_agent::Window;
use corifeu_bench::{Task, one_line, write_steps};
use corifeu_doer::{Allowance, Doer, Goal, Limits, Outcome};
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

    let task = Task::read(&root);
    let provider = LlamaCpp::new(format!("http://{host}:9931"));
    let allow = task.allowlist();
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
            classes: task.classes(),
            ownership: Ownership::new("coder", ["source"]),
            protected: task.protected(),
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

    for at in 1..=task.slices.len() {
        let module = task.at(at).module.clone();
        let slice = task.slice(
            at,
            format!(
                "{} Do not change any test, and do not touch any other function.",
                task.at(at).implement
            ),
            Some(task.at(at).cases),
        );

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
        write_steps(&root, &iteration, &model, &module, "doer", &run);

        steps += run.steps;
        prompt += run.prompt_tokens;
        output += run.output_tokens;
        checks += run.checks;
        let gone = matches!(run.outcome, Outcome::ProviderFailed { .. });
        if run.reached() {
            closed += 1;
        } else {
            unfinished.push((module.clone(), slice));
        }

        println!(
            "{iteration}\t{model}\t{module}\t{}\t{}\t{}\t{}\t{}\t{wall:.0}\t{allowed}",
            match &run.outcome {
                Outcome::Reached => "GREEN".to_string(),
                Outcome::Exhausted { .. } => "EXHAUSTED".to_string(),
                Outcome::Looping => "LOOPING".to_string(),
                Outcome::ProviderFailed { why } => format!("PROVIDER_FAILED {}", one_line(why)),
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
        "TOTAL\t{iteration}\t{model}\tclosed={closed}/{}\tovertaken={overtaken}\tfree={free}\
         \tsteps={steps}\tprompt={prompt}\toutput={output}\tchecks={checks}\
         \tfloor_left={}\tminutes={:.1}",
        task.slices.len(),
        allowance.raised(),
        whole.elapsed().as_secs_f64() / 60.0
    );
}
