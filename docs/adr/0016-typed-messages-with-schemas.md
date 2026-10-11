# 0016 — Typed messages with published schemas

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
When agents communicate in free prose, the system cannot tell whether a task finished,
whether a finding was addressed, or whether a verdict was given. It can only tell that text
was produced. The prototype shows the consequence: the reviewer is asked to reply exactly
`APPROVED`, and a string comparison on model output becomes the control flow.

## Decision
All agent-to-agent and agent-to-board communication uses **typed messages** with published
JSON Schemas. Message kinds in MVP 1:

| Kind | From | Carries |
|---|---|---|
| `task` | lead, planner | goal, acceptance criteria, target refs, budget |
| `question` | any | question, context refs |
| `answer` | any | answer, supporting refs |
| `finding` | checker | severity, location, description, evidence ref |
| `patch` | doer | artefact ref, summary of change |
| `verdict` | checker | `pass` / `fail` / `blocked`, reasons, finding refs |
| `escalation` | any | reason, what was tried, decision needed |
| `summary` | any | phase summary for the lead or user |

**A task closes only when the artefacts its role requires exist on the board.** Not when an
agent says it is done. This is the structural fix for the prototype's weakest point, where
a model could call `finish` having verified nothing.

Providers that support schema-constrained output are given the schema directly —
`--json-schema` for Claude Code, `--output-schema` for Codex
([ADR-0007](0007-cli-providers-first.md)). Where unsupported, the adapter parses and
validates, and a validation failure is a retryable error with the schema violation fed back.

## Alternatives considered
- **Free-text with conventions** (`APPROVED`). What exists. Fragile against paraphrase,
  translation, politeness and formatting.
- **Typed messages without schemas.** Types in Rust only; no validation at the boundary
  where the data actually arrives untrusted, and nothing publishable for plugin authors.
- **A general envelope with an opaque payload.** Defers the problem to every consumer.

## Consequences
- Schemas are published artefacts, versioned with the plugin protocol
  ([ADR-0032](0032-out-of-process-plugins.md)), because plugins produce and consume them.
- Weak local models will fail schema validation more often than strong ones. The retry-with-
  violation-feedback loop is a core mechanism, not an edge case, and it must count against
  the task's budget or it becomes an infinite loop.
- Free-form reasoning still exists inside a turn; only the *output* is typed. Agents are not
  forced to think in JSON.
- "Required artefacts per role" is configuration, which means a user can weaken the closing
  condition. `doctor` warns when a doer role declares no required artefact.
