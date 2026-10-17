//! `corifeu doctor` — what this machine will actually do, measured.
//!
//! Every check here runs something. ADR-0055 asked for a `doctor` that measures
//! rather than assumes, and the reason is in the measurement: a whole first
//! pass ran with a third of the model decoding on the CPU, under a context
//! nobody had chosen, and the run recorded none of it. A report that repeats
//! the configuration back is worth nothing; the configuration is what is in
//! doubt.

use corifeu_config::{Config, Loaded, model::ProviderKind};
use corifeu_provider::{Budget, Message, Provider};
use serde_json::json;
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Ok,
    Warn,
    Fail,
}

impl Level {
    fn tag(self) -> &'static str {
        match self {
            Level::Ok => "ok  ",
            Level::Warn => "warn",
            Level::Fail => "FAIL",
        }
    }
}

struct Report {
    findings: Vec<(Level, String, String)>,
}

impl Report {
    fn new() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    fn add(&mut self, level: Level, check: impl Into<String>, detail: impl Into<String>) {
        self.findings.push((level, check.into(), detail.into()));
    }

    fn worst(&self) -> Level {
        self.findings
            .iter()
            .map(|(l, _, _)| *l)
            .max()
            .unwrap_or(Level::Ok)
    }

    fn render(&self) -> String {
        let width = self
            .findings
            .iter()
            .map(|(_, check, _)| check.len())
            .max()
            .unwrap_or(0);
        let mut out = String::new();
        for (level, check, detail) in &self.findings {
            let _ = writeln!(out, "{}  {check:width$}  {detail}", level.tag());
        }
        out
    }
}

pub fn run(file: Option<&Path>, quick: bool) -> ExitCode {
    let mut report = Report::new();

    report.add(
        Level::Ok,
        "platform",
        format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
    );

    let loaded = load(file, &mut report);
    if let Some(loaded) = &loaded {
        configuration(loaded, &mut report);
        ownership(&loaded.config, &mut report);
        oracles(&loaded.config, &mut report);
        providers(&loaded.config, quick, &mut report);
    }

    print!("{}", report.render());
    match report.worst() {
        Level::Fail => ExitCode::FAILURE,
        _ => ExitCode::SUCCESS,
    }
}

fn load(file: Option<&Path>, report: &mut Report) -> Option<Loaded> {
    let layers = crate::config_cmd::collect_layers(file).ok()?;
    if layers
        .iter()
        .all(|l| matches!(l.source, corifeu_config::Source::Env(_)))
    {
        report.add(
            Level::Warn,
            "configuration",
            "none found; everything below is the built-in default",
        );
    }
    match corifeu_config::load(layers) {
        Ok(loaded) => Some(loaded),
        Err(e) => {
            report.add(Level::Fail, "configuration", e.to_string());
            None
        }
    }
}

fn configuration(loaded: &Loaded, report: &mut Report) {
    for warning in &loaded.warnings {
        report.add(Level::Warn, "configuration", warning.to_string());
    }
    report.add(
        Level::Ok,
        "privacy",
        if loaded.config.privacy.allow_remote_providers {
            "remote providers allowed; no class is local-only yet (ADR-0061)"
        } else {
            "remote providers refused everywhere"
        },
    );
}

/// ADR-0053 promises the coder cannot write tests. Whether that is true is
/// decided by the language, so it is reported rather than claimed.
fn ownership(config: &Config, report: &mut Report) {
    let classes: Vec<&str> = config
        .file_classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    report.add(Level::Ok, "file classes", classes.join(", "));

    let Some(test_owners) = config.ownership.get("test") else {
        report.add(
            Level::Warn,
            "test authorship",
            "no role owns the `test` class, so nobody may write tests",
        );
        return;
    };
    let source_owners = config.ownership.get("source").cloned().unwrap_or_default();
    let both: Vec<&String> = test_owners
        .iter()
        .filter(|r| source_owners.contains(r))
        .collect();

    if both.is_empty() {
        report.add(
            Level::Ok,
            "test authorship",
            format!(
                "`test` is {} and `source` is {}; separate, as ADR-0053 ships",
                test_owners.join(", "),
                source_owners.join(", ")
            ),
        );
    } else {
        report.add(
            Level::Warn,
            "test authorship",
            format!(
                "{} writes both `source` and `test`, so its oracle is its own (ADR-0042)",
                both.iter()
                    .map(|r| r.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    report.add(
        Level::Warn,
        "in-file tests",
        "Rust keeps unit tests inside the file under test, so class ownership cannot \
         separate them. The test-count guard (ADR-0057) is what protects the suite here.",
    );
}

/// ADR-0037: a doer runs only what the allowlist permits, so an oracle that is
/// not on it is an oracle that will never run.
fn oracles(config: &Config, report: &mut Report) {
    if config.oracles.is_empty() {
        report.add(
            Level::Warn,
            "oracles",
            "none declared; no slice can have a check",
        );
        return;
    }
    let allow = corifeu_exec::Allowlist::new(config.execution.allowed_commands.clone());
    for (name, oracle) in &config.oracles {
        let Some(command) = corifeu_exec::Command::from_argv(&oracle.command) else {
            report.add(Level::Fail, format!("oracle {name}"), "names no program");
            continue;
        };
        if allow.permits(&command) {
            report.add(Level::Ok, format!("oracle {name}"), command.display());
        } else {
            report.add(
                Level::Fail,
                format!("oracle {name}"),
                format!(
                    "`{}` is not permitted by execution.allowed_commands, so it can never run",
                    command.display()
                ),
            );
        }
    }
}

fn providers(config: &Config, quick: bool, report: &mut Report) {
    agent_clis(config, report);

    let locals: Vec<(&String, &corifeu_config::model::Provider)> = config
        .providers
        .iter()
        .filter(|(_, p)| matches!(p.kind, ProviderKind::Ollama | ProviderKind::LlamaCpp))
        .collect();

    if locals.is_empty() {
        report.add(Level::Warn, "local runtimes", "none configured");
        return;
    }

    for (name, provider) in locals {
        let Some(base) = &provider.base_url else {
            report.add(Level::Fail, format!("provider {name}"), "no base_url");
            continue;
        };
        match provider.kind {
            ProviderKind::LlamaCpp => {
                let llama =
                    corifeu_llamacpp::LlamaCpp::new(base).with_timeout(Duration::from_secs(20));
                match llama.models() {
                    Ok(models) if models.is_empty() => report.add(
                        Level::Warn,
                        format!("provider {name}"),
                        format!("{base} answers and serves no model"),
                    ),
                    Ok(models) => {
                        report.add(
                            Level::Ok,
                            format!("provider {name}"),
                            format!("{base} serves {}", models.join(", ")),
                        );
                        if !quick {
                            for model in models {
                                tool_call(&llama, &model, report);
                            }
                        }
                    }
                    Err(e) => report.add(
                        Level::Fail,
                        format!("provider {name}"),
                        format!("{base}: {e}"),
                    ),
                }
            }
            _ => {
                // Ollama has no model-listing in the shared trait; a turn is
                // the only honest probe, and it is the one that matters.
                let ollama =
                    corifeu_ollama::Ollama::new(base).with_timeout(Duration::from_secs(120));
                match &provider.min_version {
                    _ if quick => report.add(
                        Level::Ok,
                        format!("provider {name}"),
                        format!("{base} (not probed; --quick)"),
                    ),
                    _ => tool_call(&ollama, "", report),
                }
            }
        }
    }
}

/// ADR-0027 gates the installer on a real tool call, not on a version string.
/// The same question decides whether a model can be a doer at all: the
/// measurement found one that answers well and cannot call a tool.
fn tool_call(provider: &dyn Provider, model: &str, report: &mut Report) {
    let tools = vec![json!({"type": "function", "function": {
        "name": "run",
        "description": "Run a command as an argument vector.",
        "parameters": {"type": "object",
            "properties": {"argv": {"type": "array", "items": {"type": "string"}}},
            "required": ["argv"]}}})];
    let messages = vec![
        Message::system("You run commands as argument vectors. There is no shell."),
        Message::user("Run the Rust test suite for the library target."),
    ];
    let budget = Budget {
        num_predict: 512,
        num_ctx: Some(4096),
        think: false,
        // A reachability probe, not a measurement. Whatever the runtime picks.
        seed: None,
        temperature: None,
    };

    let started = Instant::now();
    let check = format!("model {model}");
    match provider.chat(model, &messages, &tools, &budget) {
        Err(e) => report.add(Level::Fail, check, e.to_string()),
        Ok(turn) if turn.tool_calls.is_empty() => report.add(
            Level::Warn,
            check,
            format!(
                "answers but calls no tool, so it cannot be a doer. It said: {:?}",
                turn.content.chars().take(60).collect::<String>()
            ),
        ),
        Ok(turn) => {
            let speed = turn
                .speed
                .map(|s| format!("{:.0} tok/s", s.decode_per_second))
                .unwrap_or_else(|| {
                    format!(
                        "{:.0} tok/s measured here",
                        f64::from(turn.output_tokens) / started.elapsed().as_secs_f64()
                    )
                });
            report.add(
                Level::Ok,
                check,
                format!("calls tools, {speed}, {} output tokens", turn.output_tokens),
            );
        }
    }
}

/// Defect 54. A project whose only provider was a CLI passed with nothing
/// checked — not the executable, not the version, not that it answers — while
/// `corifeu run` detected and reported the version all along. The check
/// existed and this is what calls it before a step is spent.
fn agent_clis(config: &Config, report: &mut Report) {
    for (name, provider) in config
        .providers
        .iter()
        .filter(|(_, p)| p.kind == ProviderKind::Cli)
    {
        let label = format!("provider {name}");
        let Some(command) = provider.command.as_deref() else {
            report.add(Level::Fail, label, "a CLI provider and no `command`");
            continue;
        };
        let Some(agent) = corifeu_cliagent::ADAPTERS
            .into_iter()
            .find(|a| a.program == command)
        else {
            report.add(
                Level::Fail,
                label,
                format!(
                    "`{command}` has no adapter. Known: {}",
                    corifeu_cliagent::programs()
                ),
            );
            continue;
        };
        let Some(found) = corifeu_cliagent::CliAgent::new(agent).version() else {
            report.add(
                Level::Fail,
                label,
                format!("`{command}` did not answer `--version`. Installed and on PATH?"),
            );
            continue;
        };
        match provider.min_version.as_deref() {
            Some(wanted) if !corifeu_cliagent::at_least(&found, wanted) => report.add(
                Level::Fail,
                label,
                format!("{command} {found} is older than the {wanted} this project asks for"),
            ),
            _ if !agent.verified => report.add(
                Level::Warn,
                label,
                format!(
                    "{command} {found}; this adapter has never been run against the real CLI, \
                     its flags come from ADR-0007's table"
                ),
            ),
            _ => report.add(Level::Ok, label, format!("{command} {found}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corifeu_config::{Source, layer_from_str};

    /// A line continuation in a Rust string eats the next line's indentation,
    /// which YAML cannot survive. These are raw strings for that reason.
    const BASE: &str = r#"version: 1
providers:
  local:
    kind: llama-cpp
    base_url: http://127.0.0.1:1
roles:
  lead:
    category: lead
    provider: local
    model: m
"#;

    const TWO_DOERS: &str = r#"  coder:
    category: doers
    provider: local
    model: m
    writes: [source]
  qa:
    category: doers
    provider: local
    model: m
    writes: [test]
"#;

    fn config(yaml: &str) -> Config {
        let layer = layer_from_str(&format!("{BASE}{yaml}"), Source::Env("test".into())).unwrap();
        corifeu_config::load(vec![layer]).expect("valid").config
    }

    const SPLIT: &str = r#"ownership:
  source: [coder]
  test: [qa]
"#;

    const SHARED: &str = r#"ownership:
  source: [coder]
  test: [coder]
"#;

    const ORACLES: &str = r#"oracles:
  test: { command: ["cargo", "test"] }
  lint: { command: ["cargo", "clippy"] }
execution:
  allowed_commands: [["cargo", "test"]]
"#;

    fn findings(report: &Report) -> Vec<(Level, String)> {
        report
            .findings
            .iter()
            .map(|(l, check, detail)| (*l, format!("{check}: {detail}")))
            .collect()
    }

    #[test]
    fn separate_owners_for_source_and_test_is_what_adr_0053_ships() {
        let mut report = Report::new();
        ownership(&config(&format!("{TWO_DOERS}{SPLIT}")), &mut report);
        let said = findings(&report);
        assert!(
            said.iter()
                .any(|(l, s)| *l == Level::Ok && s.contains("separate"))
        );
    }

    #[test]
    fn one_role_owning_both_is_warned_about_not_refused() {
        // ADR-0042 keeps this legal and warns. Refusing would be paternalistic.
        let mut report = Report::new();
        ownership(&config(&format!("{TWO_DOERS}{SHARED}")), &mut report);
        assert!(
            findings(&report)
                .iter()
                .any(|(l, s)| *l == Level::Warn && s.contains("its oracle is its own"))
        );
    }

    #[test]
    fn the_in_file_test_limit_is_always_reported() {
        // ADR-0053 promises the coder cannot write tests. In Rust that promise
        // has no mechanism, and saying so is the honest part of the report.
        let mut report = Report::new();
        ownership(&config(&format!("{TWO_DOERS}{SPLIT}")), &mut report);
        assert!(
            findings(&report)
                .iter()
                .any(|(_, s)| s.contains("ADR-0057"))
        );
    }

    #[test]
    fn an_oracle_outside_the_allowlist_can_never_run_and_that_is_a_failure() {
        let mut report = Report::new();
        oracles(
            &config(&format!("{TWO_DOERS}{SPLIT}{ORACLES}")),
            &mut report,
        );
        let said = findings(&report);
        assert!(
            said.iter()
                .any(|(l, s)| *l == Level::Fail && s.contains("oracle lint"))
        );
        assert!(
            said.iter()
                .any(|(l, s)| *l == Level::Ok && s.contains("oracle test"))
        );
    }

    #[test]
    fn declaring_no_oracle_is_a_warning_because_no_slice_can_have_a_check() {
        let mut report = Report::new();
        oracles(&config(&format!("{TWO_DOERS}{SPLIT}")), &mut report);
        assert_eq!(report.worst(), Level::Warn);
    }

    #[test]
    fn the_exit_code_follows_the_worst_finding() {
        let mut report = Report::new();
        report.add(Level::Ok, "a", "fine");
        assert_eq!(report.worst(), Level::Ok);
        report.add(Level::Warn, "b", "hmm");
        assert_eq!(report.worst(), Level::Warn);
        report.add(Level::Fail, "c", "no");
        assert_eq!(report.worst(), Level::Fail);
    }

    #[test]
    fn checks_line_up_so_the_report_can_be_skimmed() {
        let mut report = Report::new();
        report.add(Level::Ok, "short", "x");
        report.add(Level::Fail, "a much longer check", "y");
        let rendered = report.render();
        let columns: Vec<usize> = rendered
            .lines()
            .map(|l| l.rfind(char::is_whitespace).unwrap_or(0))
            .collect();
        assert_eq!(columns[0], columns[1]);
    }
}
