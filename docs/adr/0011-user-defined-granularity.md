# 0011 — The user decides role granularity

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
How finely work should be divided has no universal answer. A solo developer on a small
repository may want a single doer that writes code, configures CI and updates docs. A team
with strict review requirements may want a coder, a DevOps engineer, a technical writer and
a DBA, each on a different model chosen for that work.

Both are legitimate. More roles means better specialisation and more coordination overhead,
more latency and more tokens.

## Decision
The user chooses **how many roles exist in each category**, names them, and assigns each to
a model. **One model may hold several roles.** Presets (few roles / many roles / custom) are
starting points, not a closed set.

Granularity is chosen during the install wizard and editable afterwards in
`.corifeu/config.yaml`.

## Alternatives considered
- **A fixed role set.** Simple, and wrong for most repositories. It also makes the product's
  own roadmap dependent on anticipating users' team shapes.
- **Automatic granularity inferred from the repository.** Attractive, unpredictable, and
  hard to debug when it guesses wrong. Could become an advisory in `doctor` later; must not
  be a hidden decision.

## Consequences
- The configuration schema carries roles as a user-defined list, not an enum. Validation is
  structural ([ADR-0012](0012-required-and-optional-roles.md)), not name-based.
- Role names are user-chosen identifiers, so they appear in prompts, the blackboard and
  reports. They need a naming rule — lowercase kebab-case, unique within a run — enforced
  by the schema.
- A model holding several roles must not carry context between them. Each role invocation
  gets its own contracted view ([ADR-0014](0014-context-contracts-per-role.md)) and its own
  session, or roles leak into each other and independent review becomes theatre.
- More roles means more coordination cost. The wizard should state the trade-off plainly
  rather than presenting granularity as free.
