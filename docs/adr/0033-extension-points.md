# 0033 — Five extension points, one interface

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
"Extensible" without named extension points is an aspiration. The Open/Closed Principle
requires knowing which axes vary, because an extension point in the wrong place is cost
without benefit, and a missing one forces a core change for every new case.

## Decision
Five extension points in MVP, each a plugin-implementable interface
([ADR-0032](0032-out-of-process-plugins.md)).

| Point | Implements | Example third-party plugin |
|---|---|---|
| **Provider** | start a turn, stream events, report usage, end a turn | LM Studio, vLLM, a direct Anthropic API adapter |
| **Role** | prompt scaffolding, context contract, required artefacts | A domain-specific `dba` or `sre` role |
| **Preset** | a named bundle of roles, channels, contracts and budgets | A team's house standard |
| **Checker / oracle** | run a check, return structured findings | A proprietary SAST tool, a licence scanner |
| **Reporter** | consume run events, emit a report | Slack notifier, HTML dashboard, CI annotations |

Two more are **recognised but deferred**, because their shape is not yet clear enough to
freeze into a public contract:

- **Git host** — GitHub, GitLab, Gitea, Bitbucket. MVP 1 supports GitHub through `gh`. The
  interface needs at least two real implementations before it can be designed honestly, and
  freezing it after one would be guessing.
- **Storage backend** — remote or shared blackboards. No MVP use case.

**Checker is the most valuable of the five.** It is how
[ADR-0019](0019-tie-breaking-and-oracle-precedence.md)'s rule — oracles outrank opinions —
becomes extensible: any tool that returns a structured pass/fail becomes an oracle, and
oracles are what make the quality claim real.

## Alternatives considered
- **Only providers.** The obvious axis, and it leaves roles, presets and checkers requiring
  core changes — the thing the OCP goal forbids.
- **Everything is a plugin, including the core loop.** Maximum flexibility, no coherent
  product, and the extension points become untestable.
- **Design the git-host interface now.** Rejected on the above reasoning; recorded here so
  the omission is deliberate rather than overlooked.

## Consequences
- Five interfaces to specify, version and document, each needing a conformance test
  ([ADR-0039](0039-provider-conformance-suite.md)).
- Built-in implementations use the same interfaces
  ([ADR-0032](0032-out-of-process-plugins.md)), so the core proves them continuously.
- A plugin registry is explicitly **not** in scope. Plugins are referenced by path or URL in
  configuration. A registry is a product in itself.
- Reporter is the cheapest point to implement and the easiest to demonstrate, which makes it
  the natural subject for the example plugin.
