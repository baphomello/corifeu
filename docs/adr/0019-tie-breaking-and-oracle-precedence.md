# 0019 — Oracles outrank opinions; defined tie-breaking

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Agents disagree. A reviewer calls a change wrong, the coder defends it, and both arguments
are fluent. Fluency is not evidence: a model can argue convincingly for an incorrect
position, and two models can disagree confidently in opposite directions.

A system that resolves such disputes by discussion resolves them by persuasiveness. That
selects for the most articulate model, not the most correct one.

## Decision
Resolution follows a fixed order. The first rule that applies decides.

1. **Objective oracles decide, always.** Tests, build, lint, type check, and any
   user-configured check. An oracle outranks every agent opinion, including the lead's and
   including unanimity against it. If the test fails, the code is wrong, whatever any agent
   says.
2. **A required role outranks an optional role**
   ([ADR-0012](0012-required-and-optional-roles.md)).
3. **The lead decides.**
4. **The user decides**, when the lead cannot or when the autonomy level requires it.

Additionally, **cross-family review is recommended**: assign a checker from a different
model family than the doer. The prototype already does this — a Claude architect with a
Codex reviewer — and the reasoning is that models from one family share failure modes, so
same-family review systematically misses a correlated class of errors. This is a
recommendation surfaced by the wizard and by `doctor`, not a hard requirement.

## Alternatives considered
- **Majority vote among agents.** Correlated errors make a majority of same-family models
  confidently wrong together. Buying more votes buys more of the same bias.
- **Weighted voting by role importance.** Requires importance to be a number, which
  [ADR-0012](0012-required-and-optional-roles.md) rejects, and still resolves facts by
  ballot.
- **Always escalate disagreement to the user.** Safe and exhausting; it relocates the work
  the product exists to do.
- **The lead always decides.** Simple, and it puts the most expensive model in the loop for
  every disagreement — the cost the design is trying to avoid.

## Consequences
- Oracles must be configurable per project, since every project's test command differs.
  Oracle configuration is part of the install wizard, and a project with no oracles loses
  rule 1 entirely — `doctor` should say so loudly.
- Rule 1 means a passing test can override a correct human-style objection, for instance a
  reviewer right about a security flaw that no test covers. Mitigation is better oracles —
  a security checker whose finding *is* an oracle result — rather than weakening rule 1.
- Cross-family review needs at least two providers configured, which not every user has.
  Degrades to same-family with a warning.
- Tie-breaking order is recorded on the board for every resolved dispute, so the audit
  trail shows which rule decided and why.
