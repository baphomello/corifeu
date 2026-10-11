# 0024 — Three autonomy levels

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Trust in an agent system is earned incrementally and is not uniform. The same user may want
to approve every change in an unfamiliar repository and approve nothing in a scratch
project. A single global "are you sure?" prompt serves neither.

The prototype has exactly two checkpoints, hardcoded: approve the SPEC, approve before
commit. Reasonable defaults, and not adjustable.

## Decision
Three levels, each defining **where the run stops for the user**.

| Level | Stops for the user at |
|---|---|
| `supervised` | Plan approval, each task's result, every escalation, every git operation |
| `semi-autonomous` | Plan approval, escalations, and git operations that leave the machine |
| `autonomous` | Escalations only |

Independent of level, two things always hold:

1. **Escalations always reach the user** when the lead cannot resolve them. Autonomy
   governs routine approval, never the handling of genuine deadlock.
2. **Git operations are governed separately** by [ADR-0025](0025-git-flow-chosen-at-install.md).
   The autonomy level sets defaults; the git flow can be stricter. "Autonomous" does not
   imply "pushes without asking" unless the git flow says so.

Autonomy is configurable per run (`corifeu run --autonomy supervised`), because trust
varies by task, not only by project.

## Alternatives considered
- **A single on/off switch.** Too coarse; the interesting cases are in between.
- **Per-operation approval rules.** Maximum precision, configuration burden too high for the
  common case. The three levels are presets over this space; the git flow already provides
  per-operation control where it matters most.
- **Trust that increases automatically with successful runs.** Appealing and unpredictable;
  the user cannot know what will happen before it happens.

## Consequences
- Every point where the runtime might stop is explicit in the run lifecycle and is testable.
- `autonomous` plus a permissive git flow plus a weak checker set is a genuinely dangerous
  combination. The wizard should not offer it as a default, and `doctor` should name it.
- Approval points are natural resumption points, and the event log already supports
  resuming ([ADR-0021](0021-sqlite-with-event-log.md)). A supervised run that stops
  overnight continues the next morning without re-running anything.
