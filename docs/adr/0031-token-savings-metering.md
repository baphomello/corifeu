# 0031 — Metering reports savings in the user's own currency

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The product's first goal is reducing expensive-token consumption. A claim like that is worth
nothing unmeasured, and worse than nothing if measured dishonestly.

There is a complication that a naive implementation gets wrong. **Many users pay by
subscription, not by token.** For a Claude Code or Codex subscription the marginal cost of a
token is zero until a rate limit is reached, at which point the cost is being unable to work
at all. Reporting "you saved $4.20" to a subscriber is fiction.

## Decision
Corifeu records tokens **per role, per model, per task**, and reports savings in whatever
currency the user actually pays in.

| Billing | Report |
|---|---|
| **Metered API** | Money. Actual spend versus estimated spend had the lead model done all the work, using published per-model prices. |
| **Subscription** | **Quota percentage.** The share of the subscription window consumed, versus the share the same work would have consumed on the lead model alone. |
| **Local model** | Zero marginal cost. Reported as wall-clock time and tokens, so the latency trade-off is visible. |

The counterfactual — "what this would have cost on the lead alone" — is an **estimate and is
labelled as one**. It is computed by summing the tokens actually consumed by delegated roles
and repricing them at the lead model's rate, which assumes the lead would have needed a
comparable number of tokens. That assumption is stated wherever the number appears.

Reports never present an estimate as a measurement.

## Alternatives considered
- **Money always.** Fiction for subscribers, which is a large share of the audience.
- **Tokens only, no cost translation.** Honest and unpersuasive; "saved 400k tokens" means
  little without a denominator.
- **No counterfactual, only actuals.** Unimpeachable, and it fails to answer the one
  question the user has.
- **Measure the counterfactual by actually running the lead-only version.** Rigorous, and
  doubles the cost of every run to measure a saving.

## Consequences
- Token accounting depends on what each provider reports, and this is the weak link. Gemini
  CLI's JSON output carries a `stats.models[].tokens` block with prompt, candidates, cached
  and thought counts. Claude Code and Codex expose usage in their structured output.
  **Whether a subscription's remaining quota is readable programmatically is UNVERIFIED for
  all three** — if it is not, the subscription row degrades to a percentage of tokens rather
  than of quota, and must say so.
- Per-model pricing is data that goes stale. It ships as a data file, is updatable without a
  new binary, and is dated in the report.
- Local-model tokens are free but not costless — they cost time. Reporting wall-clock
  alongside keeps the trade-off honest.
- Metering aggregates survive compaction permanently
  ([ADR-0022](0022-run-retention-policy.md)).
