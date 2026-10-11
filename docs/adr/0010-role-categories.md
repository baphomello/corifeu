# 0010 — Five role categories

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The prototype has three fixed roles: architect/orchestrator, coder, reviewer. Real projects
need more — a researcher that explores the codebase so the lead does not have to, a tester
distinct from a reviewer, a security checker, a DevOps engineer. Enumerating every possible
role in the core would be endless and would require a core change for each new one.

What the core actually needs to know about a role is not its job title but **how it
participates**: who may talk to the user, who may write files, who may only read and
report, whose verdict can block.

## Decision
Roles are grouped into five **categories**, which the core understands. Individual roles
are configuration.

| Category | Cardinality | Talks to user | Writes artefacts | Purpose |
|---|---|---|---|---|
| `lead` | exactly 1 | **yes, only this one** | no | Coordinates, escalates, reports |
| `planners` | 0..n | no | specs, plans | Decompose work |
| `researchers` | 0..n | no | findings | Gather context cheaply |
| `doers` | 1..n | no | **yes** | Produce the deliverable |
| `checkers` | 0..n | no | findings, verdicts | Verify independently |

`lead` holding exactly one member is the architectural core of the product — it is what
makes the experience "one agent to talk to" rather than a committee.

**A role is not a model.** The `lead` *role* writes nothing. The *model* assigned to it is
free to hold other roles that do write — a strong model may be both the lead and the author
of tests, as two separate agents with separate contracts and separate sessions. The
no-write rule binds the role, never the model behind it
([ADR-0011](0011-user-defined-granularity.md), [ADR-0042](0042-test-authorship.md)).

## Alternatives considered
- **A flat list of roles with per-role capability flags.** More flexible, and worse: every
  preset would re-derive the same five clusters by hand, and invalid combinations (two
  user-facing agents) become expressible.
- **Only lead and worker.** Too coarse. Budget policy, channel rules and tie-breaking all
  need to distinguish a checker from a doer.
- **Categories as a plugin extension point.** Rejected: categories define the coordination
  semantics the core implements. Roles extend; categories do not.

## Consequences
- The core reasons about categories; presets and configuration name concrete roles.
- `lead` has no write capability. Combined with
  [ADR-0036](0036-runtime-enforced-tool-restriction.md), this is what replaces the
  prototype's `guard-src.py` hook.
- A researcher category pays for itself directly: codebase exploration is token-expensive
  and quality-cheap, exactly the work to move off the lead.
- Category membership is not a competence claim. A `doers` role assigned to a weak model is
  still a doer; it will simply fail its checkers.
