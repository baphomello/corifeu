# 0006 — Software development as the only MVP domain

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The blackboard-and-roles architecture generalises readily to non-software work — research,
writing, operations. Generalising early is tempting and usually fatal: it produces
abstractions justified by imagined requirements rather than real ones.

Software development also has a property no other candidate domain has, and the whole
quality thesis depends on it: **objective oracles**. Tests pass or fail. Builds succeed or
fail. Type checkers and linters are decidable. [ADR-0019](0019-tie-breaking-and-oracle-precedence.md)
makes oracles outrank agent opinion, which only works in a domain that has them.

## Decision
MVP 1 targets **software development tasks only**. Role presets, context contracts,
checkers and the default configuration all assume a code repository.

The architecture must not *foreclose* other domains: nothing in the blackboard, message
schemas, plugin protocol or configuration model may hardcode the assumption that an
artefact is a source file or that a check is a test command.

## Alternatives considered
- **Domain-agnostic from day one.** Yields a generic workflow engine competing with mature
  ones, with no sharp advantage anywhere, and no oracles.
- **Software plus one adjacent domain** (say technical writing). Doubles preset and checker
  surface before the first domain is validated.

## Consequences
- The MVP can be opinionated where opinion helps: default roles are architect, coder,
  reviewer and tester, not abstract nouns.
- "Must not foreclose" is an obligation with teeth. In review, any new core type named for
  code — `SourceFile`, `TestResult` — is challenged in favour of `Artifact` with a kind,
  and `CheckResult` with a kind. This is the cheapest form of future-proofing: naming
  discipline, not extra machinery.
- Domain extension arrives through plugins ([ADR-0033](0033-extension-points.md)) rather
  than core changes, which tests the extension points honestly.
