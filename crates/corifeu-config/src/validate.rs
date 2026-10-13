//! Structural rules and warnings.
//!
//! JSON Schema handles shape. These are the rules it cannot express, and they
//! are checked at load time with the offending role named — never discovered
//! halfway through a run (ADR-0012).

use crate::model::{Category, Config};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The configuration cannot run.
    Error,
    /// It will run, and probably not as intended.
    Warning,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{tag}: {}", self.message)
    }
}

fn error(message: impl Into<String>) -> Problem {
    Problem {
        severity: Severity::Error,
        message: message.into(),
    }
}

fn warning(message: impl Into<String>) -> Problem {
    Problem {
        severity: Severity::Warning,
        message: message.into(),
    }
}

/// Every problem found, errors and warnings together, in a stable order.
pub fn check(config: &Config) -> Vec<Problem> {
    let mut problems = Vec::new();
    check_lead(config, &mut problems);
    check_doers(config, &mut problems);
    check_references(config, &mut problems);
    check_privacy(config, &mut problems);
    check_self_testing(config, &mut problems);
    check_shadowed_classes(config, &mut problems);
    check_contracts(config, &mut problems);
    problems
}

pub fn errors(problems: &[Problem]) -> impl Iterator<Item = &Problem> {
    problems.iter().filter(|p| p.severity == Severity::Error)
}

fn roles_in(config: &Config, category: Category) -> Vec<&String> {
    config
        .roles
        .iter()
        .filter(|(_, r)| r.category == category)
        .map(|(name, _)| name)
        .collect()
}

/// Exactly one agent faces the user. This is the architectural core, not a
/// preference (ADR-0010).
fn check_lead(config: &Config, out: &mut Vec<Problem>) {
    let leads = roles_in(config, Category::Lead);
    match leads.len() {
        1 => {}
        0 => out.push(error(
            "no role has category `lead`; exactly one is required",
        )),
        n => out.push(error(format!(
            "{n} roles have category `lead` ({}); exactly one is required, because the lead is \
             the only agent that talks to the user",
            leads
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }

    for name in leads {
        let role = &config.roles[name];
        if !role.writes.is_empty() {
            out.push(error(format!(
                "lead role `{name}` declares `writes: {:?}`; the lead role writes nothing. Assign \
                 the same model to a second role instead — a role is not a model (ADR-0042)",
                role.writes
            )));
        }
    }
}

/// A software project without a coding-capable doer cannot produce software,
/// so the configuration is invalid without one (ADR-0012).
fn check_doers(config: &Config, out: &mut Vec<Problem>) {
    let doers = roles_in(config, Category::Doers);
    if doers.is_empty() {
        out.push(error(
            "no role has category `doers`; at least one is required",
        ));
        return;
    }
    let writing = doers
        .iter()
        .filter(|name| !config.roles[**name].writes.is_empty())
        .count();
    if writing == 0 {
        out.push(error(
            "no `doers` role declares `writes`; at least one must own a file class, or nothing \
             can produce a deliverable",
        ));
    }
}

/// Every role named elsewhere must exist. A typo here is otherwise found at the
/// moment the run needs the role.
fn check_references(config: &Config, out: &mut Vec<Problem>) {
    let known: BTreeSet<&str> = config.roles.keys().map(String::as_str).collect();
    let classes: BTreeSet<&str> = config
        .file_classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();

    for (class, owners) in &config.ownership {
        if !classes.contains(class.as_str()) {
            out.push(error(format!(
                "`ownership` names file class `{class}`, which is not declared in `file_classes`"
            )));
        }
        for owner in owners {
            if !known.contains(owner.as_str()) {
                out.push(error(format!(
                    "`ownership.{class}` names role `{owner}`, which does not exist"
                )));
            }
        }
    }

    for [a, b] in &config.channels {
        for role in [a, b] {
            if !known.contains(role.as_str()) {
                out.push(error(format!(
                    "`channels` names role `{role}`, which does not exist"
                )));
            }
        }
        if a == b {
            out.push(error(format!("`channels` pairs role `{a}` with itself")));
        }
    }

    for (name, role) in &config.roles {
        if !config.providers.is_empty() && !config.providers.contains_key(&role.provider) {
            out.push(error(format!(
                "role `{name}` uses provider `{}`, which is not declared in `providers`",
                role.provider
            )));
        }
        for class in &role.writes {
            if !classes.contains(class.as_str()) {
                out.push(error(format!(
                    "role `{name}` writes file class `{class}`, which is not declared in \
                     `file_classes`"
                )));
            }
        }
        if role.reads_only && !role.writes.is_empty() {
            out.push(error(format!(
                "role `{name}` is `reads_only` and also declares `writes`"
            )));
        }
    }
}

/// `allow_remote_providers: false` is enforced, not descriptive (ADR-0041).
fn check_privacy(config: &Config, out: &mut Vec<Problem>) {
    if config.privacy.allow_remote_providers {
        return;
    }
    for (name, role) in &config.roles {
        let Some(provider) = config.providers.get(&role.provider) else {
            continue;
        };
        if !provider.kind.is_local() {
            out.push(error(format!(
                "`privacy.allow_remote_providers` is false, but role `{name}` uses provider \
                 `{}`, which is not local",
                role.provider
            )));
        }
    }
}

/// Permitted, and never silent: the configuration would otherwise look like it
/// has an independent oracle when it does not (ADR-0042).
fn check_self_testing(config: &Config, out: &mut Vec<Problem>) {
    for (name, role) in &config.roles {
        let writes_source = role.writes.iter().any(|c| c == "source");
        let writes_test = role.writes.iter().any(|c| c == "test");
        if writes_source && writes_test {
            out.push(warning(format!(
                "role `{name}` owns both `source` and `test`. An agent that writes the code and \
                 the tests that judge it is marking its own homework, and the oracle is no longer \
                 independent. Consider a separate role — it may use the same model"
            )));
        }
    }
}

/// Ordering is load-bearing, so a rule that can never match is a mistake worth
/// naming (ADR-0035).
fn check_shadowed_classes(config: &Config, out: &mut Vec<Problem>) {
    let mut seen: Vec<(&str, BTreeSet<&str>)> = Vec::new();
    for class in &config.file_classes {
        let patterns: BTreeSet<&str> = class.include.iter().map(String::as_str).collect();
        for (earlier_name, earlier) in &seen {
            if !patterns.is_empty() && patterns.is_subset(earlier) {
                out.push(warning(format!(
                    "file class `{}` is fully shadowed by `{earlier_name}`, which is declared \
                     earlier and matches every one of its patterns. First match wins, so `{}` can \
                     never apply",
                    class.name, class.name
                )));
            }
        }
        seen.push((class.name.as_str(), patterns));
    }
}

/// Configurations that run and produce poor work.
fn check_contracts(config: &Config, out: &mut Vec<Problem>) {
    for (name, role) in &config.roles {
        match role.category {
            Category::Doers if !role.contract.acceptance => out.push(warning(format!(
                "doer `{name}` excludes `acceptance` from its contract; it will be asked to do \
                 work without being told how the result is judged"
            ))),
            Category::Checkers if role.contract.artifacts.is_empty() => out.push(warning(format!(
                "checker `{name}` declares no `artifacts` in its contract; it will be asked to \
                 verify work it cannot see"
            ))),
            _ => {}
        }
    }
}
