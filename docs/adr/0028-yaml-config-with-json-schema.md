# 0028 — YAML configuration with a published JSON Schema

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Configuration must be hand-editable, reviewable in pull requests, expressive enough for
nested roles and contracts, and usable without the wizard — in CI, in a container, or across
a team standardising on one setup.

Shell `export` statements, the prototype's mechanism, fail every one of those except the
first ([diagnosis](../diagnosis.md)).

## Decision
Configuration is **YAML**, at `.corifeu/config.yaml`, with a **published JSON Schema** at a
stable versioned URL.

The schema is published so that editors provide completion and validation while typing, and
so the structure is documented in a machine-checkable form rather than only in prose.
Corifeu validates against the same schema at load time, and schema validation is part of CI.

**Non-interactive installation** accepts a config file answering some or all wizard
questions:

```sh
corifeu install --config team-defaults.yaml --non-interactive
```

Partially-answered files are legitimate: supplied values are used, missing ones are either
prompted for or defaulted, depending on `--non-interactive`.

**Secrets are never written to configuration.** Corifeu relies on the provider CLIs' own
credential storage and on environment variables. A config file is designed to be committed,
and anything that could leak must therefore be impossible to put there — the schema forbids
secret-shaped fields rather than merely discouraging them.

## Alternatives considered
- **TOML.** Better for flat configuration, awkward for deeply nested structures like role
  lists with per-role contracts. Corifeu's configuration is deeply nested.
- **JSON.** No comments, and a configuration file people are expected to read and edit
  needs comments.
- **A dedicated DSL.** Expressive and requires users to learn it, with no editor support.
- **Schema inferred from Rust types.** Convenient, and couples the published contract to
  internal representation. The schema is published to plugin authors and is part of the
  public interface, so it is authored deliberately. Generating it from types as a *check* is
  a reasonable later refinement.

## Consequences
- YAML's sharp edges apply: significant whitespace, the Norway problem, tab-versus-space
  confusion. Validation messages must point to line and column, or users will suffer.
- The schema must be versioned alongside the binary and remain available at a stable URL for
  older versions.
- The Rust YAML ecosystem needs a decision — `serde_yaml` was archived in March 2024. See
  [ADR-0040](0040-yaml-parser-selection.md).
- A committed `.corifeu/config.yaml` means role and model choices are reviewable, which is a
  real benefit for teams and the reason secrets must be structurally excluded.
