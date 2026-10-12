# 0001 — Product name, binary name and project directory

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The project was prototyped as "orchestra", a name shared with a large number of existing
software products, including an Elixir library, several job schedulers and a Symfony
bundle. It needs a name it can own, and the name has to work simultaneously as a product
name, an executable on `$PATH`, a dot-directory inside users' repositories and a
configuration namespace.

## Decision
The product is **Corifeu**. The executable is `corifeu`. Per-project state lives in
`.corifeu/`. Environment variables are prefixed `CORIFEU_`.

The name is the Greek *coryphaeus*: the leader of the chorus, the one who addresses the
audience while conducting the others. The metaphor is load-bearing rather than decorative —
it names the central architectural constraint, that exactly one agent faces the user while
the rest are conducted behind it.

The canonical repository is `github.com/baphomello/corifeu`.

## Alternatives considered
- **Keep "orchestra".** Unsearchable, already taken many times over, and no clean
  namespace. The conductor metaphor is also weaker: an orchestra has no single member who
  speaks for it.
- **A neutral invented name.** Easier to trademark, but discards a metaphor that explains
  the architecture in one sentence to anyone who hears it.

## Consequences
- Every occurrence of `orchestra` / `.orchestra/` is renamed. [ADR-0034](0034-break-existing-personal-setup.md) accepts the breakage.
- The name is unfamiliar to most English speakers and will be mispronounced. Documentation
  gives the pronunciation and the etymology once, early.
- `.corifeu/` mixes versioned and ignored content. The split is specified in
  [ADR-0023](0023-project-memory-in-versioned-files.md) and must be unambiguous, because a
  user who commits the SQLite database will suffer merge conflicts on every pull.
