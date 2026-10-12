# 0012 — Importance means obligatoriness

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
"How important is this role" invites a numeric weight, and a weight invites the question of
what the number does. In practice the only consequential distinction is binary: **can a run
exist without this role?** A software project with no coding-capable doer cannot produce
software. A project without a security checker can still ship.

## Decision
Each role is **required** or **optional**. There is no weighting.

A configuration lacking any required role is **invalid** and is rejected at load time with a
message naming the missing role — not at the point of first use, halfway through a run.

Minimum required set for the software-development domain:

| Category | Minimum | Rationale |
|---|---|---|
| `lead` | exactly 1 | Someone must face the user |
| `doers` | ≥ 1 capable of writing code | Otherwise no deliverable |
| `planners` | 0 | The lead can decompose simple work |
| `researchers` | 0 | A cost optimisation, not a requirement |
| `checkers` | 0 in MVP 1 | Objective oracles already gate the work |

Each preset declares its own required set, which may be stricter. The `many-roles` preset
requires at least one checker.

Required also governs tie-breaking: a required role's objection outranks an optional role's
([ADR-0019](0019-tie-breaking-and-oracle-precedence.md)).

## Alternatives considered
- **A 1–5 importance weight.** Expressive-looking, but no mechanism consumes it
  meaningfully. Weighted voting among agents is precisely the subjective mechanism
  [ADR-0019](0019-tie-breaking-and-oracle-precedence.md) rejects in favour of oracles.
- **Everything optional, warn on a thin configuration.** Allows a configuration that cannot
  possibly succeed, and defers the failure to runtime where it is more expensive to
  diagnose.

## Consequences
- Validation runs at config load, in `doctor`, and in CI for the repository's own presets.
- "Capable of writing code" is a property of the *role*, declared in configuration, not
  inferred from the model. Corifeu cannot know what a model can do; it can know what the
  user said the role is for — and the tool-call test
  ([ADR-0027](0027-install-wizard-with-tool-call-test.md)) checks the mechanical part.
- Marking checkers optional in MVP 1 is a deliberate bet that objective oracles carry more
  weight than agent review. If dogfooding contradicts it, the default changes.
