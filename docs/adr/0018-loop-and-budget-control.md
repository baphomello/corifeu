# 0018 — Rounds, budgets, loop detection and timeouts

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Two agents correcting each other can run indefinitely. A reviewer finds an issue, a coder
changes something, the reviewer finds it again. Nothing in the exchange is obviously wrong
at any single step, and the cost accumulates without bound — with a cloud model, in money.

The prototype already learned part of this. `code.py` caps steps at 30, counts nudges when
the model will not call tools, and stops after identical repeated actions. `ORCHESTRA.md`
caps the review loop at three rounds. These controls were added in response to real
failures, not anticipated ones.

## Decision
Four independent controls, all configurable, all defaulting to values that fail closed.

| Control | Default | Trigger |
|---|---|---|
| **Rounds per pair** | 3 | Exchanges on one task between a declared pair |
| **Token budget per task** | preset-dependent | Cumulative tokens across all roles on a task |
| **Loop detection** | always on | Same diff, or same finding, repeated |
| **Timeout** | per role, per task | Wall-clock, sized for local-model latency |

Any control firing **escalates** — it never silently continues and never silently abandons.
Escalation goes to the lead, and from the lead to the user per the autonomy level
([ADR-0024](0024-autonomy-levels.md)).

Loop detection compares content hashes: an artefact whose hash equals a previous version's,
or a finding whose normalised text matches an unresolved earlier finding. This generalises
the prototype's identical-tool-call check from one harness to the whole system.

## Alternatives considered
- **Token budget alone.** Lets a cheap loop run a long time before tripping, and reports the
  symptom rather than the cause.
- **Round limits alone.** Blind to a single runaway turn.
- **Model-judged convergence** ("are we making progress?"). Asks the failing component to
  assess its own failure.

## Consequences
- Budgets need per-task token accounting, shared with metering
  ([ADR-0031](0031-token-savings-metering.md)).
- Defaults set too low will interrupt legitimate work. Three rounds comes from the
  prototype's practical experience rather than measurement, and should be revisited with
  real data from dogfooding.
- Timeouts must be set against measured local latency — 9.8 s for one call from a 4B model
  ([ADR-0008](0008-local-runtimes.md)) — not against cloud-model intuitions.
- Escalation is itself a cost. A system that escalates constantly is as unusable as one that
  loops; the ratio is a quality metric worth reporting.
