# 0041 — Drop the open/private mode concept

- **Status:** Proposed
- **Date:** 2026-10-11

## Context
The prototype declares `ORCHESTRA_MODE` with two values: `open` (cloud Claude orchestrates,
Codex reviews, code leaves the machine) and `private` (all-local, no cloud). `ORCHESTRA.md`
documents `private` as "to be designed".

Two observations from reading the code. The variable is **read by nothing** — it is exported
and never consulted. And the distinction it gestures at is already fully expressible in the
role configuration: "all-local" simply means every role is assigned to a local model.

## Decision
**Do not carry `mode` into Corifeu.** There is no `corifeu --mode private`.

What the user actually wants from `private` — a guarantee that no code leaves the machine —
is better served by an explicit, checkable property than by a mode label:

```yaml
privacy:
  allow_remote_providers: false    # refuse any non-local provider, anywhere
```

This is strictly better than a mode in three ways. It is **enforced** rather than
descriptive: configuration validation refuses a cloud provider in any role, with a message
naming the offending role. It is **composable**: it combines with any preset, granularity or
git flow instead of being a parallel axis. And it is **verifiable**: `doctor` can assert it,
and a run can record that it held.

## Alternatives considered
- **Implement `private` as designed.** It would reproduce, as a mode, something the role
  configuration already expresses — two ways to say one thing, which eventually disagree.
- **Keep `mode` as a preset shortcut.** A preset that assigns local models to every role is
  a fine idea, and it should be a *preset*, which is already a concept
  ([ADR-0011](0011-user-defined-granularity.md)), not a separate mode axis.
- **Keep the name for familiarity.** Only one user has the habit, and
  [ADR-0034](0034-break-existing-personal-setup.md) already accepts breaking it.

## Consequences
- One less concept, and one less dead variable of the kind this repository has already
  produced twice ([diagnosis](../diagnosis.md)).
- `allow_remote_providers: false` needs enforcement at configuration load **and** at provider
  dispatch, since a plugin could otherwise introduce a remote provider at runtime.
- A local-only preset should ship, named for what it does — `all-local` — rather than for a
  mode.
- "No code leaves the machine" is a strong claim. It must account for the provider CLIs'
  own telemetry, which Corifeu does not control and should not silently claim to cover.
