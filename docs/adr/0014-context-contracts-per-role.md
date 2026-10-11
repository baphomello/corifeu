# 0014 — Every role reads through a context contract

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
This is the prototype's best idea and the one carried forward most directly. There, the
coder sees `{task + SPEC excerpt + target files + neighbour signatures}` and the reviewer
sees `{diff + acceptance criteria}` — nothing else. The result is lower token cost and
better focus: an agent given less irrelevant material produces fewer irrelevant changes.

The prototype enforces this by instructing the orchestrator to curate carefully. That is a
promise, not a mechanism, and it degrades exactly when the run gets difficult and the
temptation to "just give it everything" is strongest.

## Decision
Every role has a **context contract**: a declarative specification of what its view
contains, assembled by the runtime from the blackboard. A role receives its view and cannot
reach past it.

A contract declares:

| Field | Meaning |
|---|---|
| `task` | The task assignment; always present |
| `artifacts` | Which artefact kinds and which referenced instances |
| `files` | Which repository files, by glob or by explicit reference |
| `messages` | Which channels and how many recent turns |
| `memory` | Which project-memory documents |
| `max_tokens` | Hard ceiling for the assembled view |

Contracts ship with presets and are overridable per role in configuration.

Crucially, the contract is enforced by **assembly**, not by instruction. The runtime builds
the view; the agent never had access to anything else. The provider adapter additionally
suppresses the CLI's ambient context — `CLAUDE.md`, `AGENTS.md`, MCP servers, plugins —
using the flags recorded in [ADR-0007](0007-cli-providers-first.md), generalising the
prototype's `CODEX_HOME` hack.

## Alternatives considered
- **Lead curates by instruction.** Today's approach. Unenforceable, invisible when
  violated, and the violation is silent.
- **Every agent reads the whole board.** Simple, and abandons the product's premise.
- **Retrieval over the board.** Adaptive, non-deterministic, and unauditable — two runs with
  identical inputs could legitimately diverge. A possible later refinement for
  `researchers`, where it fits; not a default.

## Consequences
- View assembly is a core component with its own tests. A contract violation is a bug, not a
  judgement call.
- `max_tokens` requires per-provider token counting, which is approximate across families.
  Over-estimate rather than truncate silently; truncation that the agent cannot see is
  exactly the `read_file` bug found in the prototype
  ([diagnosis](../diagnosis.md)).
- Contracts are the main tuning surface for the cost/quality trade-off, and the main way a
  user can misconfigure quality downward. `doctor` should flag contracts that exclude
  acceptance criteria from a doer, or the diff from a checker.
- Context leakage is a named risk ([risks.md](../risks.md)): a model holding several roles,
  a resumed CLI session, or an unsuppressed `CLAUDE.md` can each breach a contract without
  any Corifeu code being wrong.
