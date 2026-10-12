# 0042 — Test authorship is configurable, with a recommendation against self-testing

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Tests are the acceptance oracle, and oracles outrank every agent opinion
([ADR-0019](0019-tie-breaking-and-oracle-precedence.md)). That makes *who writes them* a
question about the integrity of the whole quality model, not a staffing detail.

The prototype gives `tests/` to the orchestrator, specifically so the coder does not author
the criteria it will be judged by. But [ADR-0010](0010-role-categories.md) states the `lead`
role writes no artefacts, and these two positions contradicted each other in the first draft
of this documentation set.

The contradiction came from conflating two things: the **role** and the **model behind it**.

## Decision
**Any role may be assigned authorship of the test file class. The user decides.** There is no
built-in rule naming a particular role as the test author.

Three constraints shape that freedom.

**1. The `lead` role never writes tests, or anything else.** Its no-write rule
([ADR-0010](0010-role-categories.md)) is absolute, because it is what
[ADR-0036](0036-runtime-enforced-tool-restriction.md) enforces.

**2. The model behind the lead may write tests — as a different role.** This is the
resolution. A user may assign `claude/opus` to both the `lead` role and a `qa` role. They are
two agents: separate contracts, separate sessions, separate views, no shared context
([ADR-0011](0011-user-defined-granularity.md)). The restriction binds the role, never the
model.

**3. A role that writes both `source` and `test` is permitted and warned about.**
Configuration validation emits:

> Warning: role `coder` owns both `source` and `test`. An agent that writes the code and the
> tests that judge it is marking its own homework, and the oracle is no longer independent.
> Consider a separate role for tests — it may use the same model.

This is a **warning, not an error**. A solo user on a small project may reasonably accept the
trade, and refusing to run would be paternalistic. But it must never happen silently, because
the resulting configuration looks like it has an oracle while the oracle has been quietly
compromised.

## Alternatives considered
- **The lead writes tests**, as the prototype does. It works, and it requires granting the
  lead a write capability, which punctures the cleanest rule in the design for one file class.
  The role-versus-model distinction achieves the same outcome without the exception.
- **A dedicated `qa` role, mandatory.** Stronger guarantee; forces a second required role on
  every user including those who do not want one, and contradicts
  [ADR-0011](0011-user-defined-granularity.md)'s principle that the user chooses granularity.
- **The coder always writes tests.** Simplest, and silently removes oracle independence.
- **Forbid one role owning both classes.** Would make the guarantee real, at the cost of
  rejecting a configuration that is defensible for small projects. Warning preserves the
  information without removing the choice.

## Consequences
- The `ownership` map names a role for the `test` class like any other class. Nothing in the
  core special-cases tests.
- The shadowing and self-testing warnings belong in config validation and in `corifeu doctor`,
  so the trade-off resurfaces after installation rather than only during it.
- The install wizard should present test authorship as its own question, with the
  recommendation stated rather than buried in a default.
- Presets differ deliberately: `many-roles` ships a separate `qa` role; `few-roles` lets the
  coder own both and says so during installation.
- This is the clearest instance of a general principle worth stating once in the
  documentation: **roles are the unit of restriction, models are a resource roles consume.**
  Several questions that look hard become easy once that distinction is held firmly.
