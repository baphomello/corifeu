# 0044 — Antigravity CLI replaces Gemini CLI

- **Status:** Accepted
- **Date:** 2026-10-12
- **Amends:** [ADR-0007](0007-cli-providers-first.md) (the Gemini portion only)

## Context
[ADR-0007](0007-cli-providers-first.md) named Gemini CLI as one of three supported agent
CLIs, with every capability marked UNVERIFIED because it was not installed on the analysis
machine.

Verification found the product had been retired. Google announced at I/O on 2026-05-19 that
it was consolidating developer tooling under the Antigravity brand, and **Gemini CLI was
retired for individual users on 2026-06-18**. The replacement is **Antigravity CLI**,
invoked as `agy`. Gemini CLI reportedly remains available to enterprise customers and to
holders of paid Gemini or Gemini Enterprise Agent Platform API keys — that narrower
availability is secondary-sourced and **UNVERIFIED**.

The two are not the same software. Gemini CLI is TypeScript on Node, Apache-2.0. Antigravity
CLI is a **Go single binary, proprietary and closed source**.

## Decision
**Corifeu supports Antigravity CLI (`agy`) in place of Gemini CLI** as its third agent-CLI
provider. Gemini CLI is not supported in MVP 1.

The following was verified by execution against `agy` 1.3.3 on Linux, not read from
documentation.

| Capability | Mechanism | Verified |
|---|---|---|
| Non-interactive | `-p` / `--print` / `--prompt` | Yes |
| Structured output | `--output-format text\|json\|stream-json` | Yes |
| Schema-constrained output | `--json-schema <string or file>` | Flag present; **output not yet exercised** |
| Streaming input | `--input-format stream-json` (NDJSON, one turn per line) | Flag present; **not exercised** |
| Session identity | `--continue`, `--conversation <id>`; id returned in JSON | Yes |
| Working directory | `--add-dir` (repeatable) | Flag present |
| Model / effort | `--model`, `--effort low…max` | Yes |
| Sandbox | `--sandbox` — "terminal restrictions enabled" | Flag present; **semantics not exercised** |
| Execution mode | `--mode accept-edits\|plan` | Yes |
| Usage reporting | `usage` block in JSON output | Yes |
| **Tool restriction** | **Headless default-deny plus `permissions.allow`** | **Yes — adversarially** |

### Tool restriction works, by a different mechanism

Antigravity has **no `--allowedTools` / `--disallowedTools` flag**. Its model is stronger in
one respect and weaker in another: in print mode, **any tool requiring permission is
auto-denied**, because headless mode cannot prompt. Selective permission is granted through
`permissions.allow` in `~/.gemini/antigravity-cli/settings.json`, with rules of the form
`command(<target>)`.

[ADR-0039](0039-provider-conformance-suite.md) test 5 was run adversarially, both directions:

- Instructed to create a file, under `--mode plan` with default permissions: the agent
  **attempted** the write, was auto-denied, and **no file was created**. The JSON output
  reported it explicitly:
  `"denied_actions":[{"action":"command","display_name":"RunCommand"}]`
- The same instruction with `--dangerously-skip-permissions`: **the file was created**.

The second half is what makes the first meaningful. Without it, "no file" would only show
the model never tried.

**Default-deny plus an explicit allowlist is a better fit for Corifeu than a deny-list
flag**, because it fails closed and matches [ADR-0037](0037-sandboxed-command-execution.md)'s
design directly.

### Two findings that affect other decisions

**Usage reporting is unusually good.** The JSON result carries
`input_tokens`, `output_tokens`, `thinking_tokens`, `cache_read_tokens` and `total_tokens`,
plus `duration_seconds` and `num_turns`. This is the richest usage block of any supported
CLI and directly serves [ADR-0031](0031-token-savings-metering.md). `denied_actions` is
equally useful for auditing restriction.

**Baseline context cost is high.** A six-word prompt consumed **11,859 input tokens**
without tools and **24,362 with tools available** — roughly 12k tokens of tool definitions
on every turn. Budgets ([ADR-0018](0018-loop-and-budget-control.md)) must account for
per-provider baseline overhead, not only the assembled view. A contract capped at 24k tokens
is meaningless against a provider that spends 24k before reading it.

## Alternatives considered
- **Keep Gemini CLI.** Retired for individual users, which is most of the audience.
  Supporting it would mean targeting a product most users cannot obtain.
- **Support both.** Doubles adapter surface for a variant available only to enterprise
  customers. Revisit if such users appear; the extension points make it a plugin, not a core
  change ([ADR-0033](0033-extension-points.md)).
- **Drop Google entirely.** Would reduce Corifeu to two families and weaken cross-family
  review ([ADR-0019](0019-tie-breaking-and-oracle-precedence.md)), which is a core quality
  mechanism.

## Consequences
- The adapter targets a **closed-source binary**. Behaviour cannot be read from source, only
  observed, which raises the value of the conformance suite from useful to necessary. It
  also means breaking changes arrive without a readable diff.
- Permission configuration lives in a **settings file, not flags**. The adapter must write
  and manage a settings file, which is closer to the prototype's `CODEX_HOME` approach than
  to passing arguments.
- **Context isolation is unresolved.** Antigravity exposes no `--bare` or
  `--ignore-user-config` equivalent. Configuration lives under `~/.gemini/`, and a
  trust model (`trustedFolders.json`, `trustedWorkspaces`) governs whether project agents,
  hooks, MCP servers and skills load — an untrusted folder skips them. Whether that can be
  driven deliberately, or a config home redirected as `CODEX_HOME` allows, is **UNVERIFIED**
  and is an open question.
- `--dangerously-skip-permissions` must **never** be Corifeu's default. It is the correct
  flag only for a role that genuinely holds full capability, and even then the allowlist is
  preferable.
- The product name shift means documentation should say "Antigravity CLI (`agy`)" rather
  than "Gemini", while noting the model family is still Gemini.
