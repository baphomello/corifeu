# 0007 — Agent CLIs as the first provider class

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Most of the intended audience already has Claude Code, Codex CLI or Gemini CLI installed
and authenticated, frequently on a subscription rather than metered API billing. Those CLIs
also bring working tool execution, repository awareness and session handling — large
amounts of functionality Corifeu would otherwise reimplement.

## Decision
MVP 1 supports **agent CLIs only**: **Claude Code**, **Codex CLI**, **Gemini CLI**, plus
local runtimes ([ADR-0008](0008-local-runtimes.md)). Corifeu invokes them in
non-interactive mode and parses structured output.

The capabilities below were verified by running `--help` locally, except where marked.

| Capability | Claude Code 2.1.295 | Codex CLI 0.162.1 | Gemini CLI |
|---|---|---|---|
| Non-interactive | `-p`/`--print` | `codex exec` | `-p`/`--prompt` |
| Structured output | `--output-format json\|stream-json` | `--json` (JSONL) | `--output-format json\|stream-json` |
| Schema-constrained output | `--json-schema <schema>` | `--output-schema <FILE>` | not found — **UNVERIFIED** |
| Restrict tools | `--allowedTools` / `--disallowedTools` / `--tools` / `--restricted` | `-s read-only\|workspace-write\|danger-full-access` | `--approval-mode`, `--yolo` — **UNVERIFIED** |
| Ignore user/global context | `--bare`, `--setting-sources`, `--strict-mcp-config` | `--ignore-user-config`, `--ignore-rules` | **UNVERIFIED** |
| Session identity | `--session-id <uuid>`, `--resume`, `--fork-session` | `exec resume`, `--last`, `fork` | **UNVERIFIED** |
| Working directory | `--add-dir` | `-C`/`--cd`, `--add-dir` | `--include-directories` |
| Model selection | `--model` | `-m`/`--model` | `-m`/`--model` |

Gemini CLI could not be verified by execution — it is not installed on the analysis
machine. Its flags come from the official headless documentation and a reported defect
(issue #9009) where `--output-format` was rejected despite being documented. **Every Gemini
row is UNVERIFIED until exercised on a real installation.**

## Alternatives considered
- **Direct APIs first.** Cleaner contracts and real token accounting, but it discards
  subscription billing — which for many users is the entire economic point — and requires
  reimplementing tool execution. Deferred: [ADR-0009](0009-direct-apis-deferred.md).
- **One CLI (Claude Code) at launch.** Faster, but it would bake Claude Code's shape into
  the adapter interface, exactly as the prototype did. Supporting three from the start is
  what keeps the abstraction honest.

## Consequences
- **Adapter churn is the dominant technical risk.** These are pre-1.0 tools; `codex` is at
  0.162.1. Flags and output shapes will change. Mitigated by
  [ADR-0039](0039-provider-conformance-suite.md) and tracked in [risks.md](../risks.md).
- Corifeu must pin and report the CLI version it detected, and refuse to run against an
  untested major version with a clear message rather than a parse error.
- Each CLI brings its own ambient context — `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, MCP
  servers, plugins, skills. Left alone, this silently breaks context contracts
  ([ADR-0014](0014-context-contracts-per-role.md)). Every adapter must neutralise it; the
  flags above exist for precisely this and generalise the prototype's `CODEX_HOME` hack.
- Token accounting differs per CLI and may be unavailable. See
  [ADR-0031](0031-token-savings-metering.md).
