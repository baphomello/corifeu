# 0018 — Rounds, budgets, loop detection and timeouts

- **Status:** Accepted
- **Date:** 2026-10-11 (amended 2026-10-13, 2026-10-14)

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
| **Steps per turn** | floor 12, ceiling 30 | A role's turn on one slice ([ADR-0062](0062-step-allowance-has-a-floor-not-a-ceiling.md)) |

Any control firing **escalates** — it never silently continues and never silently abandons.
Escalation goes to the lead, and from the lead to the user per the autonomy level
([ADR-0024](0024-autonomy-levels.md)).

Loop detection compares content hashes: an artefact whose hash equals a previous version's,
or a finding whose normalised text matches an unresolved earlier finding. This generalises
the prototype's identical-tool-call check from one harness to the whole system.

> **Amended 2026-10-13, by measurement.** The prototype compared each action against the
> **immediately preceding** one. That never fires on a cycle. In Task A of the thesis
> measurement, steps 20 and 26 were byte-identical with different steps between them, and
> the detector stayed silent while the agent burned 57 steps.
>
> Detection therefore compares against a **window of recent actions**, not the last one.
> Two further signals proved necessary in the same run:
>
> - **No progress.** The agent made **one** `write_file` at step 3 and then 54 `run` plus
>   5 `read_file` calls without editing anything. A doer that has not changed an artefact
>   in *n* steps is looping, whatever its commands look like.
> - **Near-duplicate commands.** Steps differing only in a `grep` pattern or a `sed` range
>   are the same action. Normalise before comparing.
>
> The no-progress signal is the strongest of the three, because it is about the task rather
> than the text: a doer that is not producing artefacts is not doing its job, however
> varied its tool calls look.
>
> How the runtime *responds* to a detected repeat is
> [ADR-0050](0050-loop-detection-warns-before-it-stops.md): it warns, and only
> consecutive warnings stop the run.
>
> **Detection must also be state-aware, or it fires on correct behaviour.** The first
> implementation of the above stopped a run at step 5 because `cargo test` matched step 3 —
> ignoring that step 4 had rewritten the file between them. Re-running a check after an
> edit is the loop working, not looping. **An artefact change resets the comparison
> window**: the same command against different code is a different action.

> **Amended 2026-10-14, by measurement.** The step limit was one constant for every turn.
> The fourth run worked 69 slices and the two populations do not overlap: 57 reached green,
> 38 of them in one step and the slowest in twelve, while 11 of the 12 that stuck ran past
> twelve and bought nothing. A constant is therefore either too high for the work that
> finishes or a guess about work nobody has measured, so the limit became a floor that grows
> toward a ceiling — and only turns that actually ran out of steps make it grow.
> [ADR-0062](0062-step-allowance-has-a-floor-not-a-ceiling.md) holds the numbers and the
> reasoning.

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
