# 0026 — Failure handling is configurable

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Failures are routine, not exceptional: a local model fails a tool call, a CLI exits
non-zero, a provider rate-limits, a checker finds an unfixable problem, a budget runs out.
Whether to retry or stop is a genuine preference. An unattended overnight run should retry
and escalate the model; a supervised run should stop and ask.

The prototype always stops, returning a distinct exit code per failure mode — a reasonable
default, and the only option.

## Decision
Failure handling is configured, with two top-level strategies:

1. **Stop and inform.** Halt the task, record the failure, escalate to the lead and the
   user.
2. **Retry by policy**, where the policy specifies:

| Setting | Meaning |
|---|---|
| `attempts` | Retries before giving up |
| `backoff` | Fixed or exponential, with a ceiling |
| `escalate_to` | Another model to try after local retries are exhausted |
| `budget_cap` | Spend ceiling for the whole retry sequence |
| `on_exhausted` | What happens at the end: stop, or skip and continue |

Policies are set per failure class, because the classes are genuinely different:

| Class | Sensible default |
|---|---|
| Transient (network, rate limit, timeout) | retry with exponential backoff |
| Malformed output (schema violation) | retry with the violation fed back, 2 attempts |
| Tool-call failure (model will not call tools) | escalate to a stronger model |
| Check failure (tests fail) | **not a failure** — it is a finding, routed to the doer |
| Budget exhaustion | stop and escalate, never retry |

The last two rows matter most. A failing test is the system working correctly, and must not
be retried as an error. Budget exhaustion must never be retried, or the budget means
nothing.

## Alternatives considered
- **Always stop.** Today's behaviour. Safe, and makes unattended operation impossible.
- **Always retry.** Burns budget on unrecoverable failures, and a retried schema violation
  from an incapable model is an infinite loop.
- **One policy for all classes.** Cannot express that transient failures should be retried
  and budget exhaustion must not.

## Consequences
- Every failure needs a class at the point it is raised, which is a constraint on error
  typing throughout the codebase.
- `escalate_to` requires a second configured model with its own budget, and the escalation
  itself must be recorded and metered — an escalation that silently costs cloud tokens would
  undermine the product's central claim.
- Retries consume task budgets ([ADR-0018](0018-loop-and-budget-control.md)). The
  interaction must be explicit: the budget is the ceiling, and the retry policy operates
  beneath it.
