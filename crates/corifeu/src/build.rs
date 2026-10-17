//! `corifeu build` — a brief, planned by one provider and worked by another.
//!
//! This is the command the project was founded on, and until it existed the
//! hybrid claim had never been run: an expensive model plans, local models do
//! the slices, and objective checks decide when each one is done.
//!
//! `corifeu run` works one slice on one role, which is enough to measure a
//! doer and not enough to measure the split. Two providers in one process is
//! what makes the share of a task that leaves the expensive model a number
//! rather than an argument.
//!
//! # What this command does not do
//!
//! Replan. [ADR-0056](https://github.com/baphomello/corifeu/blob/main/docs/adr/0056-one-oracle-per-slice.md)
//! asks the planner once, because a planner round trip per slice spends on
//! planning what delegation saved. A plan that ends with a red slice says so
//! and stops: the next thing is a decision, not another dispatch.
//!
//! # The walk is ADR-0063's, not the obvious one
//!
//! A red slice does not end the walk, and that is measured rather than
//! assumed. Stopping at the first failure reported failure on a complete crate
//! in eight of nineteen runs of the fourth measurement, because a later slice
//! had written an earlier one's work on the way past. So every slice runs, each
//! unfinished one's oracle is asked once at the end with no model, and only
//! what is still red is evidence the decomposition was wrong.
//!
//! [`Pair::walk`](corifeu_doer::Pair) is the same policy for two doer roles,
//! where QA writes the tests the coder is judged by. This walk has one role and
//! the tests already exist, which is why the policy is written twice; they
//! should collapse once `build` grows a QA role.

use crate::run::{FromConfig, Row, ensure_runtimes, provider_for, spend_table};
use corifeu_agent::Window;
use corifeu_config::model::Config;
use corifeu_doer::{Allowance, Doer, Goal, Oracle, Outcome, Slice, Spend};
use corifeu_planner::{Brief, PlannedSlice, Planner};
use corifeu_provider::Budget;
use corifeu_workspace::{Guard, Ownership, Workspace};
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

pub struct Ask<'a> {
    pub goal: &'a str,
    /// What the planner needs in order to name real commands: the language,
    /// the test runner, the layout. Without it a planner invents an oracle,
    /// and `Plan::check` rejects the plan against the allowlist.
    pub context: &'a str,
    pub planner: &'a str,
    pub coder: &'a str,
    pub root: &'a Path,
}

pub fn build(config: &Config, ask: Ask<'_>) -> ExitCode {
    let from = FromConfig::read(config);
    let steps = from.limits.steps;

    let planner_role = match config.roles.get(ask.planner) {
        Some(role) => role,
        None => {
            eprintln!("no role `{}` in this project's configuration", ask.planner);
            return ExitCode::FAILURE;
        }
    };
    let coder_role = match config.roles.get(ask.coder) {
        Some(role) => role,
        None => {
            eprintln!("no role `{}` in this project's configuration", ask.coder);
            return ExitCode::FAILURE;
        }
    };
    let owns: Vec<String> = config
        .ownership
        .iter()
        .filter(|(_, roles)| roles.iter().any(|r| r == ask.coder))
        .map(|(class, _)| class.clone())
        .collect();
    if owns.is_empty() {
        eprintln!(
            "role `{}` owns no file class, so it may not write anything (ADR-0035)",
            ask.coder
        );
        return ExitCode::FAILURE;
    }

    // Any managed runtime comes up first, and is held for the whole build:
    // dropping these stops the ones Corifeu started, and only those.
    let _runtimes = match ensure_runtimes(config, &[ask.planner, ask.coder]) {
        Ok(held) => held,
        Err(why) => {
            eprintln!("{why}");
            return ExitCode::FAILURE;
        }
    };

    // Both providers are built before either is used, so a project that names
    // a CLI that is not installed fails without having spent a dispatch.
    let planning = match provider_for(config, ask.planner) {
        Ok(provider) => provider,
        Err(why) => {
            eprintln!("{why}");
            return ExitCode::FAILURE;
        }
    };
    let coding = match provider_for(config, ask.coder) {
        Ok(provider) => provider,
        Err(why) => {
            eprintln!("{why}");
            return ExitCode::FAILURE;
        }
    };

    println!("goal:    {}", ask.goal);
    println!(
        "planner: {} ({} on {})",
        ask.planner, planner_role.model, planner_role.provider
    );
    println!(
        "coder:   {} ({} on {})",
        ask.coder, coder_role.model, coder_role.provider
    );
    println!();

    let brief = Brief {
        goal: ask.goal.to_string(),
        context: ask.context.to_string(),
    };
    let planner = Planner {
        provider: planning.as_ref(),
        model: planner_role.model.clone(),
        budget: Budget {
            seed: config.limits.seed,
            temperature: config.limits.temperature,
            ..Budget::judge()
        },
    };

    let planned = match planner.plan(&brief, &from.allowlist) {
        Ok(planned) => planned,
        Err(why) => {
            eprintln!("the plan was not usable: {why}");
            return ExitCode::FAILURE;
        }
    };

    println!("plan: {} slice(s)", planned.plan.slices.len());
    for (at, slice) in planned.plan.slices.iter().enumerate() {
        println!("  {}. {}", at + 1, slice.task);
        println!(
            "     {} -> {}",
            slice.artefacts.join(", "),
            slice.describe_oracle()
        );
    }
    println!();

    let mut workspace = match Workspace::open(ask.root, ask.root.join(".corifeu/checkpoints")) {
        Ok(workspace) => workspace,
        Err(error) => {
            eprintln!("cannot open {}: {error}", ask.root.display());
            return ExitCode::FAILURE;
        }
    };

    let doer = Doer {
        provider: coding.as_ref(),
        goal: Goal::Green,
        model: coder_role.model.clone(),
        budget: from.budget,
        allowlist: from.allowlist,
        guard: Guard {
            classes: from.classes,
            ownership: Ownership::new(ask.coder, owns),
            protected: from.protected,
        },
        window: Window::default(),
        limits: from.limits,
    };

    // One allowance for the whole plan, not one per slice. `corifeu build`
    // called `Doer::work`, which always takes the floor, so `ceiling`,
    // `patience` and `raise_by` were declared in the project file and reached
    // nothing — the same shape of gap S2 found in `limits.steps`.
    let mut allowance = Allowance::new(steps);
    let mut coded = Spend::default();
    let mut worked: Vec<(usize, Outcome)> = Vec::new();
    let mut abandoned = None;

    for (at, planned_slice) in planned.plan.slices.iter().enumerate() {
        let slice = slice_from(config, planned_slice);
        println!("slice {}: {}", at + 1, slice.task);
        let outcome = loop {
            let run = doer.work_within(&slice, allowance.now(), &mut workspace);
            for step in &run.trace {
                println!(
                    "  {:>3}  {:<12} {}",
                    step.number, step.action.tool, step.outcome
                );
            }
            coded = coded.with(run.spend);
            println!("  {}", describe(&run.outcome, run.steps, run.checks));
            let grown = allowance.saw(&run.outcome);
            if run.outcome == Outcome::Reached {
                break run.outcome;
            }
            // ADR-0062: a turn that spent every step it had may have been
            // close to finishing. Running out of steps is evidence the
            // allowance was short, which is not the evidence that a
            // decomposition was wrong, and the two were treated alike.
            if let Some(now) = grown {
                println!("  the allowance grew to {now} steps; the same slice runs again");
                continue;
            }
            break run.outcome;
        };
        println!();
        // ADR-0063 §5. Everything else carries on: the fourth run measured
        // that stopping at the first red slice reports failure on a complete
        // crate, in eight of nineteen runs.
        let pointless = matches!(
            outcome,
            Outcome::ProviderFailed { .. } | Outcome::OracleUnusable { .. }
        );
        worked.push((at, outcome));
        if pointless {
            abandoned = Some(at + 1);
            break;
        }
    }

    // ADR-0063 §2: one oracle run per unfinished slice and no model at all,
    // which is the cheapest question in the runtime. Green means the work was
    // late rather than missing — a later slice wrote it on the way past.
    let mut overtaken = Vec::new();
    let mut stalled = Vec::new();
    for (at, outcome) in &worked {
        if *outcome == Outcome::Reached {
            continue;
        }
        let slice = slice_from(config, &planned.plan.slices[*at]);
        match doer.probe(&slice, &workspace) {
            true => overtaken.push(at + 1),
            false => stalled.push(at + 1),
        }
    }
    if !overtaken.is_empty() {
        println!(
            "slice(s) {} never closed under the coder and are green now: a later slice wrote \
             the work on the way past (ADR-0058).",
            numbers(&overtaken)
        );
        println!();
    }

    spend_table(&[
        Row {
            role: ask.planner,
            provider: &planner_role.provider,
            model: &planner_role.model,
            spend: planned.spend,
        },
        Row {
            role: ask.coder,
            provider: &coder_role.provider,
            model: &coder_role.model,
            spend: coded,
        },
    ]);
    split(planned.spend, coded);

    println!();
    let closed = worked
        .iter()
        .filter(|(_, outcome)| *outcome == Outcome::Reached)
        .count();
    // Two different facts, so both are reported: how many slices closed under
    // their own role, and whether the work is there (ADR-0063 §4).
    println!(
        "{closed} of {} slice(s) closed under the coder, {} overtaken, {} still red",
        planned.plan.slices.len(),
        overtaken.len(),
        stalled.len()
    );

    if let Some(at) = abandoned {
        let outcome = &worked.last().expect("the abandoning slice is in it").1;
        println!(
            "the walk stopped at slice {at} and {} slice(s) were never attempted.",
            planned.plan.slices.len() - worked.len()
        );
        println!("  {}", why(outcome));
        return ExitCode::FAILURE;
    }
    if stalled.is_empty() {
        return ExitCode::SUCCESS;
    }
    // ADR-0056 reads a red slice as evidence the decomposition was wrong, and
    // that is one of several reasons a slice can be red. Which one it was
    // decides what to do next, so the outcome says it rather than the ADR.
    for at in &stalled {
        let outcome = &worked
            .iter()
            .find(|(slice, _)| slice + 1 == *at)
            .expect("stalled slices come from worked")
            .1;
        println!("slice {at} is still red. {}", why(outcome));
    }
    ExitCode::FAILURE
}

/// The two shares, side by side, because they disagree by design.
///
/// The planner can be every cent spent and a twentieth of the tokens. Reported
/// alone, the money figure says the expensive model did everything and the
/// token figure says it did almost nothing; both are true of the same run and
/// only the pair is honest ([ADR-0031](https://github.com/baphomello/corifeu/blob/main/docs/adr/0031-token-savings-metering.md)).
fn split(planner: Spend, coder: Spend) {
    let total = planner.with(coder);
    let percent = |share: Option<f64>| match share {
        Some(share) => format!("{:.1}%", share * 100.0),
        None => "unmeasured".to_string(),
    };
    println!();
    println!(
        "the planner's share: {} of the money, {} of the tokens",
        match total.cost_usd {
            Some(_) => percent(planner.share_of(total)),
            // Nothing billed, so there is no money to take a share of. Saying
            // 0% would read as the expensive side having been free.
            None => "no money was spent".to_string(),
        },
        percent(planner.token_share_of(total)),
    );
}

fn numbers(slices: &[usize]) -> String {
    slices
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn slice_from(config: &Config, planned: &PlannedSlice) -> Slice {
    Slice {
        task: planned.task.clone(),
        artefacts: planned.artefacts.clone(),
        oracle: Oracle {
            argv: planned.oracle.clone(),
            timeout: timeout_for(config, &planned.oracle),
            // Nobody knows how many cases a slice has until they are written,
            // and the planner is told not to guess (ADR-0057).
            expect_cases: None,
        },
    }
}

/// What a stopped build says to do next, which is not the same sentence for
/// every way a slice can fail. ADR-0056 reads a failed slice as evidence the
/// decomposition was wrong; that holds for a slice nothing could finish, and
/// not for one whose allowance ran out at the ceiling.
fn why(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::Reached => "",
        Outcome::Exhausted { .. } => {
            "The allowance is at its ceiling and the oracle is still red. Either the slice is \
             larger than one turn, which makes it a planning problem (ADR-0056), or the \
             ceiling is low for this task."
        }
        Outcome::Looping => {
            "The doer repeated itself until ADR-0050 stopped it. More steps would not have \
             helped; the slice or its oracle is not giving it new information."
        }
        Outcome::ProviderFailed { .. } => {
            "The provider could not be reached. Nothing here \
             says anything about the plan."
        }
        Outcome::OracleUnusable { .. } => {
            "The slice's own check could not produce a verdict, so no step was spent on it \
             (ADR-0067). Fix the command before reading anything else."
        }
    }
}

fn describe(outcome: &Outcome, steps: u32, checks: u32) -> String {
    let verdict = match outcome {
        Outcome::Reached => "green".to_string(),
        Outcome::Exhausted { .. } => "out of steps".to_string(),
        Outcome::Looping => "stopped: going in circles".to_string(),
        Outcome::ProviderFailed { why } => format!("provider failed: {why}"),
        Outcome::OracleUnusable { detail } => format!("the check cannot judge this: {detail}"),
    };
    format!("{verdict} after {steps} step(s), {checks} check(s)")
}

/// The timeout for a command the planner named.
///
/// How long a test runner may take is a fact about the project, not a planning
/// decision, so it comes from the declaration whose command this is. The match
/// is by prefix and the longest one wins, which is the same rule the allowlist
/// permitted the command by: a planner may legitimately narrow a permitted
/// command to one test file, and that invocation gets the timeout declared for
/// the command it narrows rather than a default.
fn timeout_for(config: &Config, argv: &[String]) -> Duration {
    let seconds = config
        .oracles
        .values()
        .filter(|declared| {
            argv.len() >= declared.command.len()
                && argv[..declared.command.len()] == declared.command[..]
        })
        .max_by_key(|declared| declared.command.len())
        .map_or(300, |declared| u64::from(declared.timeout_s));
    Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests {
    use super::timeout_for;
    use corifeu_config::model::Config;

    fn config(oracles: &str) -> Config {
        let text = format!(
            "version: 1\n\
             providers: {{ local: {{ kind: llama-cpp, base_url: \"http://x\" }} }}\n\
             roles:\n  \
               lead: {{ category: lead, provider: local, model: m, required: true }}\n  \
               coder: {{ category: doers, provider: local, model: m, writes: [source] }}\n\
             file_classes: [{{ name: source, include: [\"src/**\"] }}]\n\
             ownership: {{ source: [coder] }}\n\
             {oracles}"
        );
        let layer = corifeu_config::layer_from_str(
            &text,
            corifeu_config::Source::Project(std::path::PathBuf::from("corifeu.yaml")),
        )
        .expect("the fixture parses");
        corifeu_config::load(vec![layer])
            .expect("the fixture is valid")
            .config
    }

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| (*p).to_string()).collect()
    }

    /// The planner narrows `npx vitest run` to one file, which is the shape a
    /// per-slice oracle takes, and the timeout has to follow the command it
    /// narrowed rather than drop to a default.
    #[test]
    fn a_narrowed_oracle_keeps_the_declared_timeout() {
        let config = config(
            "oracles:\n  suite: { command: [\"npx\", \"vitest\", \"run\"], timeout_s: 180 }\n",
        );
        assert_eq!(
            timeout_for(
                &config,
                &argv(&["npx", "vitest", "run", "src/slug.test.ts"])
            )
            .as_secs(),
            180
        );
    }

    #[test]
    fn the_longest_declaration_wins_over_a_shorter_prefix() {
        let config = config(
            "oracles:\n  \
               all: { command: [\"npx\", \"vitest\"], timeout_s: 600 }\n  \
               one: { command: [\"npx\", \"vitest\", \"run\"], timeout_s: 90 }\n",
        );
        assert_eq!(
            timeout_for(&config, &argv(&["npx", "vitest", "run"])).as_secs(),
            90
        );
        assert_eq!(
            timeout_for(&config, &argv(&["npx", "vitest", "watch"])).as_secs(),
            600
        );
    }

    #[test]
    fn a_command_no_declaration_covers_falls_back() {
        let config = config("oracles:\n  one: { command: [\"cargo\", \"test\"] }\n");
        assert_eq!(
            timeout_for(&config, &argv(&["npx", "vitest", "run"])).as_secs(),
            300
        );
    }
}
