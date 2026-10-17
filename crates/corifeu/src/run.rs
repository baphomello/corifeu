//! `corifeu run` — work one slice of a real project until its oracle passes.
//!
//! Everything a round has measured for six rounds ran through a driver in the
//! measurement harness. This is the same runtime, reading a project's own
//! `corifeu.yaml` instead of a `task.json`, which is the difference between a
//! thing that has been measured and a thing that can be used.
//!
//! The conversion below is the part that did not exist. `limits.steps` has been
//! in the configuration since it was written and nothing turned it into the
//! runtime's `Limits`, so a project file could say `floor: 20` and the doer
//! would run at 12 without anybody being told. Each field is converted here,
//! once, and the drift test in the measurement harness fails if the two sets of
//! defaults ever disagree.

use corifeu_agent::Window;
use corifeu_cliagent::{Agent, CliAgent, antigravity, claude_code, codex};
use corifeu_config::model::{Config, ProviderKind};
use corifeu_doer::{Doer, Goal, Limits, Oracle, Outcome, Slice, Steps};
use corifeu_exec::Allowlist;
use corifeu_llamacpp::LlamaCpp;
use corifeu_ollama::Ollama;
use corifeu_provider::{Budget, Provider};
use corifeu_workspace::{Class, Classes, Guard, Ownership, Protected, Workspace};
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

/// What the project file says, in the shapes the runtime takes.
pub struct FromConfig {
    pub limits: Limits,
    pub budget: Budget,
    pub classes: Classes,
    pub allowlist: Allowlist,
    pub protected: Option<Protected>,
}

impl FromConfig {
    pub fn read(config: &Config) -> Self {
        let limits = Limits {
            steps: Steps {
                floor: config.limits.steps.floor,
                ceiling: config.limits.steps.ceiling,
                patience: config.limits.steps.patience,
                raise_by: config.limits.steps.raise_by,
            },
            output_budget: 12_000,
            // ADR-0050. `loop_detection: false` is expressed to the runtime as
            // a limit nothing reaches, because the detector has no off switch
            // and a sentinel is clearer than a second code path.
            repeats: match config.limits.loop_detection {
                true => config.limits.repeats,
                false => u32::MAX,
            },
        };

        Self {
            limits,
            budget: Budget {
                seed: config.limits.seed,
                temperature: config.limits.temperature,
                ..Budget::doer()
            },
            classes: Classes::new(
                config
                    .file_classes
                    .iter()
                    .map(|c| Class::new(c.name.clone(), c.include.clone(), c.exclude.clone())),
            ),
            allowlist: Allowlist::new(config.execution.allowed_commands.clone()),
            protected: config
                .protected
                .as_ref()
                .map(|p| Protected::new(p.marker.clone(), p.in_classes.clone())),
        }
    }
}

/// Build the provider a role names. Returned boxed because which one it is
/// depends on a string in a file.
/// Every CLI adapter there is. One list, so adding one is one edit.
const AGENTS: [Agent; 3] = [claude_code(), codex(), antigravity()];

fn provider_for(config: &Config, role: &str) -> Result<Box<dyn Provider>, String> {
    let declared = config
        .roles
        .get(role)
        .ok_or_else(|| format!("no role `{role}` in this project's configuration"))?;
    let provider = config.providers.get(&declared.provider).ok_or_else(|| {
        format!(
            "role `{role}` names provider `{}`, which is not declared",
            declared.provider
        )
    })?;
    // A CLI agent is a subprocess, so it has no base_url to be missing. Asking
    // for one before the match would refuse the provider for lacking something
    // it cannot have; what it has instead is `command`.
    if provider.kind == ProviderKind::Cli {
        let command = provider.command.as_deref().ok_or_else(|| {
            format!(
                "provider `{}` is a CLI and declares no `command`. Name the executable: {}",
                declared.provider,
                AGENTS.map(|a| a.program).join(", ")
            )
        })?;
        let agent = AGENTS
            .into_iter()
            .find(|a| a.program == command)
            .ok_or_else(|| {
                format!(
                    "provider `{}` names command `{command}`, which has no adapter. Known: {}",
                    declared.provider,
                    AGENTS.map(|a| a.program).join(", ")
                )
            })?;
        // ADR-0007 keeps three CLIs in scope so the abstraction cannot become
        // one CLI's shape, and only one of the three has been run. A warning
        // rather than a refusal: an unverified adapter may well work, and
        // finding out is the point.
        if !agent.verified {
            eprintln!(
                "warning: the {} adapter has never been run against the real CLI; its flags \
                 come from ADR-0007's table",
                agent.runtime
            );
        }
        let driver = CliAgent::new(agent);
        // ADR-0007 calls adapter churn the dominant technical risk, and asks for
        // the detected version to be reported and an untested one refused with a
        // clear message rather than a parse error.
        let Some(found) = driver.version() else {
            return Err(format!(
                "provider `{}` names command `{command}`, which did not answer `--version`. \
                 Is it installed and on PATH?",
                declared.provider
            ));
        };
        println!("provider: {command} {found}");
        if let Some(wanted) = provider.min_version.as_deref()
            && !corifeu_cliagent::at_least(&found, wanted)
        {
            return Err(format!(
                "provider `{}` needs {command} {wanted} or newer and found {found}. \
                     Corifeu's adapter is written against the flags that version documents.",
                declared.provider
            ));
        }
        return Ok(Box::new(driver));
    }

    let base = provider
        .base_url
        .clone()
        .ok_or_else(|| format!("provider `{}` has no base_url", declared.provider))?;

    match provider.kind {
        ProviderKind::LlamaCpp => Ok(Box::new(LlamaCpp::new(base))),
        ProviderKind::Ollama => Ok(Box::new(Ollama::new(base))),
        // A plugin provider is phase 7 (ADR-0032).
        other => Err(format!(
            "provider `{}` is {other:?}, which `corifeu run` cannot drive yet",
            declared.provider
        )),
    }
}

/// What a caller asked for on the command line.
pub struct Ask<'a> {
    pub task: &'a str,
    pub artefacts: &'a [String],
    pub oracle: &'a str,
    pub role: &'a str,
    pub expect_cases: Option<u32>,
    pub root: &'a Path,
}

pub fn run(config: &Config, ask: Ask<'_>) -> ExitCode {
    let from = FromConfig::read(config);

    let Some(declared) = config.oracles.get(ask.oracle) else {
        let known: Vec<_> = config.oracles.keys().cloned().collect();
        eprintln!(
            "no oracle `{}` in this project's configuration. Declared: {}",
            ask.oracle,
            if known.is_empty() {
                "none".into()
            } else {
                known.join(", ")
            }
        );
        return ExitCode::FAILURE;
    };

    let provider = match provider_for(config, ask.role) {
        Ok(provider) => provider,
        Err(why) => {
            eprintln!("{why}");
            return ExitCode::FAILURE;
        }
    };

    let role = config.roles.get(ask.role).expect("checked by provider_for");
    // `ownership` is keyed by file class, and lists the roles that may write it
    // — not the other way round. A role's classes are therefore found by asking
    // every class whether it names this role.
    let owns: Vec<String> = config
        .ownership
        .iter()
        .filter(|(_, roles)| roles.iter().any(|r| r == ask.role))
        .map(|(class, _)| class.clone())
        .collect();
    if owns.is_empty() {
        eprintln!(
            "role `{}` owns no file class, so it may not write anything (ADR-0035)",
            ask.role
        );
        return ExitCode::FAILURE;
    }

    let mut workspace = match Workspace::open(ask.root, ask.root.join(".corifeu/checkpoints")) {
        Ok(workspace) => workspace,
        Err(error) => {
            eprintln!("cannot open {}: {error}", ask.root.display());
            return ExitCode::FAILURE;
        }
    };

    let doer = Doer {
        provider: provider.as_ref(),
        goal: Goal::Green,
        model: role.model.clone(),
        budget: from.budget,
        allowlist: from.allowlist,
        guard: Guard {
            classes: from.classes,
            ownership: Ownership::new(ask.role, owns),
            protected: from.protected,
        },
        window: Window::default(),
        limits: from.limits,
    };

    let slice = Slice {
        task: ask.task.to_string(),
        artefacts: ask.artefacts.to_vec(),
        oracle: Oracle {
            argv: declared.command.clone(),
            timeout: Duration::from_secs(u64::from(declared.timeout_s)),
            expect_cases: ask.expect_cases,
        },
    };

    println!("task:    {}", ask.task);
    println!("artefact: {}", ask.artefacts.join(", "));
    println!("oracle:  {}", slice.oracle.describe());
    println!(
        "role:    {} ({} on {})",
        ask.role, role.model, role.provider
    );
    println!(
        "limits:  floor={} ceiling={} repeats={} seed={}",
        doer.limits.steps.floor,
        doer.limits.steps.ceiling,
        doer.limits.repeats,
        doer.budget
            .seed
            .map_or("none".to_string(), |s| s.to_string()),
    );
    println!();

    let outcome = doer.work(&slice, &mut workspace);
    report(&outcome, ask.role, &role.provider, &role.model)
}

fn report(run: &corifeu_doer::Run, role: &str, provider: &str, model: &str) -> ExitCode {
    for step in &run.trace {
        println!(
            "  {:>3}  {:<12} {}",
            step.number, step.action.tool, step.outcome
        );
    }
    println!();
    println!(
        "{} after {} step(s), {} check(s), {} prompt tokens",
        match &run.outcome {
            Outcome::Reached => "green".to_string(),
            Outcome::Exhausted { .. } => "out of steps".to_string(),
            Outcome::Looping => "stopped: going in circles".to_string(),
            Outcome::ProviderFailed { why } => format!("provider failed: {why}"),
            Outcome::OracleUnusable { detail } => format!("the check cannot judge this: {detail}"),
        },
        run.steps,
        run.checks,
        run.prompt_tokens,
    );

    // Acceptance 18. One role per `run` today, so this is a one-line table —
    // but it is the table the hybrid split is reported in, and money sits
    // beside tokens because for a provider that bills the two do not convert
    // (defect 46).
    println!();
    println!("spend by role:");
    println!(
        "  {:<10} {:<10} {:<14} {:>4} dispatch(es)  {:>7} prompt  {:>6} output  {}",
        role,
        provider,
        model,
        run.spend.dispatches,
        run.spend.prompt_tokens,
        run.spend.output_tokens,
        run.spend
            .cost_usd
            .map_or_else(|| "no charge".to_string(), |c| format!("${c:.4}")),
    );

    match run.outcome {
        Outcome::Reached => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::FromConfig;
    use corifeu_config::model::Config;

    /// The project file as a user writes it. Anything not named here is a
    /// default, which is the point: the conversion must carry both.
    fn config(extra: &str) -> Config {
        let text = format!(
            "version: 1\n\
             providers: {{ local: {{ kind: llama-cpp, base_url: \"http://x\" }} }}\n\
             roles:\n  \
               lead: {{ category: lead, provider: local, model: m, required: true }}\n  \
               coder: {{ category: doers, provider: local, model: m, writes: [source] }}\n  \
               qa: {{ category: doers, provider: local, model: m, writes: [test] }}\n\
             file_classes:\n  \
               - {{ name: test, include: [\"**/*.test.ts\"] }}\n  \
               - {{ name: source, include: [\"src/**\"] }}\n\
             ownership: {{ source: [coder], test: [qa] }}\n\
             {extra}"
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

    /// The gap this module exists to close. `limits.steps` had been in the
    /// configuration since it was written and nothing turned it into the
    /// runtime's `Limits`, so a project could ask for twenty steps and get
    /// twelve without being told.
    #[test]
    fn the_step_allowance_in_the_file_reaches_the_runtime() {
        let from = FromConfig::read(&config(
            "limits: { steps: { floor: 20, ceiling: 40, patience: 3, raise_by: 5 } }\n",
        ));
        assert_eq!(from.limits.steps.floor, 20);
        assert_eq!(from.limits.steps.ceiling, 40);
        assert_eq!(from.limits.steps.patience, 3);
        assert_eq!(from.limits.steps.raise_by, 5);
    }

    #[test]
    fn the_repeat_threshold_and_the_seed_reach_the_runtime() {
        let from = FromConfig::read(&config("limits: { repeats: 1, seed: 99 }\n"));
        assert_eq!(from.limits.repeats, 1);
        assert_eq!(from.budget.seed, Some(99));
    }

    /// Absent is the right default for a product run: an unseeded run is how
    /// the thing behaves when nobody is measuring it (ADR-0066).
    #[test]
    fn the_temperature_in_the_file_reaches_the_runtime() {
        let from = FromConfig::read(&config("limits: { temperature: 0.2 }\n"));
        assert_eq!(from.budget.temperature, Some(0.2));
        assert_eq!(FromConfig::read(&config("")).budget.temperature, None);
    }

    #[test]
    fn no_seed_in_the_file_means_no_seed_on_the_wire() {
        assert_eq!(FromConfig::read(&config("")).budget.seed, None);
    }

    /// The detector has no off switch, so `loop_detection: false` is expressed
    /// as a threshold nothing reaches rather than as a second code path.
    #[test]
    fn switching_loop_detection_off_leaves_a_threshold_nothing_reaches() {
        let from = FromConfig::read(&config("limits: { loop_detection: false, repeats: 2 }\n"));
        assert_eq!(from.limits.repeats, u32::MAX);
    }

    #[test]
    fn file_classes_and_their_exclusions_are_carried_over() {
        let from = FromConfig::read(&config(""));
        assert_eq!(from.classes.classify("src/a.ts"), Some("source"));
        // First match wins, which is what lets a sibling test file classify as
        // a test rather than as source (ADR-0035).
        assert_eq!(from.classes.classify("src/a.test.ts"), Some("test"));
        assert_eq!(from.classes.classify("README.md"), None);
    }

    #[test]
    fn the_protected_marker_is_carried_over_when_declared() {
        let from = FromConfig::read(&config(
            "protected: { marker: \"test(\", in_classes: [source] }\n",
        ));
        let protected = from.protected.expect("declared");
        assert_eq!(protected.marker, "test(");
        assert_eq!(protected.in_classes, vec!["source".to_string()]);
        assert!(FromConfig::read(&config("")).protected.is_none());
    }

    #[test]
    fn the_allowlist_comes_from_the_file_and_defaults_to_nothing() {
        let from = FromConfig::read(&config(
            "execution: { allowed_commands: [[\"npx\", \"vitest\", \"run\"]] }\n",
        ));
        assert_eq!(from.allowlist.entries(), vec!["npx vitest run".to_string()]);
        // ADR-0037: nothing is pre-approved, so a file that says nothing
        // permits nothing rather than everything.
        assert!(FromConfig::read(&config("")).allowlist.entries().is_empty());
    }
}
