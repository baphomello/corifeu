# 0015 — Direct messages only between declared pairs

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Some exchanges genuinely need to be direct. A coder and a reviewer iterating on a finding
should not route every turn through the lead: it burns expensive tokens on relay and
degrades the content, since the lead must compress what it forwards.

Unrestricted agent-to-agent messaging is the opposite failure. It scales as n², makes runs
non-reproducible, and reintroduces the telephone game between every pair.

## Decision
Direct messages are permitted **only between explicitly declared pairs**. Presets define the
default pairs; the user's configuration may override them.

Default pairs in the software-development presets:

```
coder ↔ reviewer
coder ↔ qa
architect ↔ lead
```

**Every direct message is also recorded as an event on the blackboard.** Direct means "not
relayed through the lead", never "private". There is no off-board channel.

Pairs are symmetric and declared once. A message to an undeclared peer is a configuration
error, reported as such.

## Alternatives considered
- **Everything through the lead.** Maximum control and maximum cost; the lead becomes a
  bottleneck and a lossy relay.
- **Free messaging among all agents.** n² growth, unbounded cost, unreproducible runs.
- **Dynamic channels negotiated at runtime.** Flexible and unpredictable; impossible to
  budget or audit in advance.

## Consequences
- The channel graph is static for a run and can be rendered, budgeted and reasoned about
  before execution.
- Pair exchanges are the natural unit for round limits and budgets
  ([ADR-0018](0018-loop-and-budget-control.md)).
- The lead is deliberately not in the loop for pair exchanges, so it must learn of outcomes
  through escalation and phase summaries. If a pair deadlocks without escalating, the lead
  is blind — hence mandatory loop detection and timeouts, not merely optional ones.
- Visibility of pair conversations to the user is configurable; **summaries by default**,
  with full transcripts available because every message is on the board.
