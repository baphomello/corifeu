//! Driving a coding CLI as a provider (ADR-0007).
//!
//! An HTTP runtime answers a question. A CLI agent wants to *do* the task: told
//! to call `write_file`, Claude Code reached for its own Write tool, failed on
//! permissions and billed for reporting that. Corifeu's loop owns tool
//! execution, so every dispatch says the agent is proposing one action for
//! another program to carry out.
//!
//! Three CLIs are in scope and the shape is the same for all of them — a
//! non-interactive flag, JSON on stdout, a schema flag to constrain it — so
//! this is one adapter with a [`Agent`] descriptor per CLI rather than a crate
//! each. ADR-0007 keeps three in scope precisely so the abstraction cannot
//! quietly become Claude Code's shape.
//!
//! Everything in [`claude_code`] was verified by execution; the other two
//! descriptors carry ADR-0007's flags and are **unverified**, which the
//! descriptor says out loud.

use corifeu_provider::{Budget, Error, Message, Provider, Stop, Turn, call_in_text};
use serde_json::{Value, json};
use std::process::{Command, Stdio};

/// One CLI, as a set of command-line facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agent {
    /// The name reported in errors and in metering.
    pub runtime: &'static str,
    /// The executable, looked up on `PATH`.
    pub program: &'static str,
    /// Subcommands before the prompt, such as `exec` for Codex.
    pub lead: &'static [&'static str],
    /// The flag that takes the prompt, or none when it is positional.
    pub prompt_flag: Option<&'static str>,
    /// Flags that make the output machine-readable and the agent inert: no MCP
    /// servers, no built-in tools, one JSON object out.
    pub quiet: &'static [&'static str],
    /// The flag that takes a JSON schema for the reply.
    pub schema_flag: Option<&'static str>,
    /// The flag that takes a model name.
    pub model_flag: Option<&'static str>,
    /// Whether this descriptor has been exercised against the real CLI.
    pub verified: bool,
}

/// The version string a CLI prints, reduced to its leading numbers.
///
/// `claude --version` answers `2.1.302 (Claude Code)`, and ADR-0007 asks
/// Corifeu to report the version it detected and refuse an untested one with a
/// clear message rather than a parse error. Anything after the numbers is the
/// CLI's own decoration.
pub fn version_in(output: &str) -> Option<String> {
    let at = output.find(|c: char| c.is_ascii_digit())?;
    let rest = &output[at..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    let found = rest[..end].trim_end_matches('.');
    (!found.is_empty()).then(|| found.to_string())
}

/// Whether `found` is at least `wanted`, compared number by number.
///
/// Not a semver implementation: these are pre-1.0 tools whose versions are
/// three numbers, and a dependency to compare three numbers would be the
/// larger risk. A part that is not a number makes the comparison fail closed,
/// because refusing to run is recoverable and running against an unknown
/// version is what ADR-0007 asks to prevent.
pub fn at_least(found: &str, wanted: &str) -> bool {
    let parts =
        |v: &str| -> Option<Vec<u64>> { v.split('.').map(|p| p.parse::<u64>().ok()).collect() };
    let (Some(found), Some(wanted)) = (parts(found), parts(wanted)) else {
        return false;
    };
    let at = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    for i in 0..found.len().max(wanted.len()) {
        if at(&found, i) != at(&wanted, i) {
            return at(&found, i) > at(&wanted, i);
        }
    }
    true
}

/// Claude Code. Every flag here was run; see the probe.
pub const fn claude_code() -> Agent {
    Agent {
        runtime: "claude-code",
        program: "claude",
        lead: &[],
        prompt_flag: Some("-p"),
        // `--bare` is deliberately absent: ADR-0007 names it for ignoring
        // ambient context and it also removes the setting source the
        // credential lives in, so it answers "Not logged in" (defect 47).
        quiet: &[
            "--restricted",
            "--strict-mcp-config",
            "--output-format",
            "json",
        ],
        schema_flag: Some("--json-schema"),
        model_flag: Some("--model"),
        verified: true,
    }
}

/// Codex CLI, from ADR-0007's table. **Unverified.**
pub const fn codex() -> Agent {
    Agent {
        runtime: "codex",
        program: "codex",
        lead: &["exec"],
        prompt_flag: None,
        quiet: &["--json", "-s", "read-only", "--ignore-user-config"],
        schema_flag: Some("--output-schema"),
        model_flag: Some("-m"),
        verified: false,
    }
}

/// Antigravity CLI, from ADR-0044. **Unverified.**
pub const fn antigravity() -> Agent {
    Agent {
        runtime: "antigravity",
        program: "agy",
        lead: &[],
        prompt_flag: Some("-p"),
        quiet: &["--output-format", "json"],
        schema_flag: None,
        model_flag: Some("-m"),
        verified: false,
    }
}

/// The reply shape every dispatch is constrained to: one proposed call.
pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "arguments": {"type": "object"}
        },
        "required": ["name", "arguments"],
        "additionalProperties": false
    })
}

pub struct CliAgent {
    agent: Agent,
}

impl CliAgent {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
    }

    pub fn agent(&self) -> Agent {
        self.agent
    }

    /// Ask the CLI what version it is. `None` when it cannot be run at all,
    /// which the caller reports differently from a version that is too old.
    pub fn version(&self) -> Option<String> {
        let output = Command::new(self.agent.program)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .ok()?;
        version_in(&String::from_utf8_lossy(&output.stdout))
    }

    /// The argv for one dispatch, as a testable value rather than as a
    /// `Command` nobody can assert on.
    pub fn argv(&self, model: &str, prompt: &str, tools: &[Value]) -> Vec<String> {
        let mut argv: Vec<String> = vec![self.agent.program.to_string()];
        argv.extend(self.agent.lead.iter().map(|s| (*s).to_string()));
        argv.extend(self.agent.quiet.iter().map(|s| (*s).to_string()));
        if let (Some(flag), false) = (self.agent.model_flag, model.is_empty()) {
            argv.push(flag.to_string());
            argv.push(model.to_string());
        }
        // Only when there are tools to propose. A dispatch with none is asking
        // for prose, and a schema would refuse it.
        if let (Some(flag), false) = (self.agent.schema_flag, tools.is_empty()) {
            argv.push(flag.to_string());
            argv.push(schema().to_string());
        }
        match self.agent.prompt_flag {
            Some(flag) => {
                argv.push(flag.to_string());
                argv.push(prompt.to_string());
            }
            None => argv.push(prompt.to_string()),
        }
        argv
    }
}

/// The messages, flattened into one prompt, with the sentence that stops the
/// agent acting on them.
pub fn prompt(messages: &[Message], tools: &[Value]) -> String {
    let mut out = String::new();
    if !tools.is_empty() {
        out.push_str(
            "You are proposing ONE action for another program to carry out. Do not act, do \
             not use your own tools, and do not explain. Answer with the action only.\n\n\
             The actions available to you:\n",
        );
        for tool in tools {
            out.push_str(&tool.to_string());
            out.push('\n');
        }
        out.push('\n');
    }
    for message in messages {
        let value = serde_json::to_value(message).unwrap_or(Value::Null);
        let role = value.get("role").and_then(Value::as_str).unwrap_or("user");
        let content = value.get("content").and_then(Value::as_str).unwrap_or("");
        out.push_str(&format!("[{role}]\n{content}\n\n"));
    }
    out
}

/// Read a CLI's JSON object out of its stdout.
///
/// From the first `{`, because without stdin redirected Claude Code prints a
/// three-second stdin warning first and a whole-output parse then fails
/// (defect 48). The adapter redirects stdin as well; this is the second line of
/// defence, not the first.
pub fn object(stdout: &str, runtime: &'static str) -> Result<Value, Error> {
    let start = stdout.find('{').ok_or_else(|| Error::Shape {
        runtime,
        detail: format!("no JSON object in the output: {}", said(stdout)),
    })?;
    Ok(serde_json::from_str(&stdout[start..])?)
}

/// What a CLI reported, as a turn.
///
/// `usage.input_tokens` is 2 whatever the call cost, because a CLI agent's own
/// context arrives as cache reads. `prompt_tokens` therefore counts the cache
/// fields in with it, so metering cannot read the expensive side as free
/// (defect 46).
pub fn turn(reply: &Value, runtime: &'static str) -> Result<Turn, Error> {
    if reply.get("is_error").and_then(Value::as_bool) == Some(true) {
        return Err(Error::Process {
            runtime,
            detail: reply
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or("the CLI reported an error and said nothing")
                .to_string(),
        });
    }
    let content = reply
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let usage = reply.get("usage");
    let count = |key: &str| {
        usage
            .and_then(|u| u.get(key))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32
    };
    let tool_calls = call_in_text(&content).map(|c| vec![c]).unwrap_or_default();
    Ok(Turn {
        tool_calls,
        content,
        thinking: String::new(),
        stop: stop(reply),
        prompt_tokens: count("input_tokens")
            + count("cache_read_input_tokens")
            + count("cache_creation_input_tokens"),
        output_tokens: count("output_tokens"),
        speed: None,
    })
}

fn stop(reply: &Value) -> Stop {
    match reply.get("stop_reason").and_then(Value::as_str) {
        Some("end_turn") | None => Stop::Done,
        Some("max_tokens") => Stop::BudgetSpent { num_predict: 0 },
        Some(other) => Stop::Other(other.to_string()),
    }
}

/// What a CLI said, trimmed to something an error can carry.
fn said(text: &str) -> String {
    let line = text.trim().lines().next().unwrap_or_default();
    line.chars().take(200).collect()
}

impl Provider for CliAgent {
    fn runtime(&self) -> &'static str {
        self.agent.runtime
    }

    fn chat(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Value],
        _budget: &Budget,
    ) -> Result<Turn, Error> {
        let argv = self.argv(model, &prompt(messages, tools), tools);
        let runtime = self.agent.runtime;
        let output = Command::new(&argv[0])
            .args(&argv[1..])
            // Defect 48: without this the CLI waits three seconds for stdin and
            // prints a warning ahead of its JSON.
            .stdin(Stdio::null())
            .output()
            .map_err(|source| Error::Process {
                runtime,
                detail: format!("{}: {source}", argv[0]),
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err(Error::Process {
                runtime,
                detail: format!(
                    "no output; exit {:?}, stderr: {}",
                    output.status.code(),
                    said(&String::from_utf8_lossy(&output.stderr))
                ),
            });
        }
        turn(&object(&stdout, runtime)?, runtime)
    }
}
