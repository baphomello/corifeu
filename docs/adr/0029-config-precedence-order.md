# 0029 — Four-level configuration precedence

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Configuration arrives from several places with different lifetimes. A user has global
preferences; a project has settings that belong to it and should be committed; CI needs to
override without editing files; a one-off run needs a single value changed.

Without a declared order, overriding becomes guesswork and "why is it using that model?"
becomes a recurring support question.

## Decision
Highest precedence first:

| Precedence | Source | Typical use |
|---|---|---|
| 1 | **CLI flags** | One-off override: `--autonomy supervised` |
| 2 | **Environment variables** (`CORIFEU_*`) | CI and containers |
| 3 | **Project config** (`.corifeu/config.yaml`) | Committed, shared with the team |
| 4 | **Global user config** (`~/.config/corifeu/config.yaml`) | Personal defaults |

Merging is **deep, per key**: a project config may override one role's model without
restating every role. Lists are replaced wholesale rather than merged, because merging lists
by position or identity produces results nobody predicts.

**`corifeu config show --explain` prints the effective configuration with the source of
every value.** This is not a nicety — without it, four-level precedence is a debugging
problem of the project's own making.

## Alternatives considered
- **Project config wins over everything.** Reproducible, and makes CI override impossible
  without editing committed files.
- **Flat override, no deep merge.** Any project config would have to restate the entire
  global config.
- **Deep merge for lists too.** Merging a list of roles requires identity matching and
  conflict rules; the result is rarely what the author intended.

## Consequences
- The precedence chain is exercised by tests at every level, including the interactions.
- Environment variable naming must map unambiguously onto nested keys. `CORIFEU_` plus
  uppercased, underscore-joined path segments, documented with examples, and the mapping must
  be reversible — ambiguity here is a long tail of confusing bugs.
- `--explain` is a phase 1 deliverable, not a later addition.
