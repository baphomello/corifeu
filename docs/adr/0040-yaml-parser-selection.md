# 0040 — YAML parser selection

- **Status:** Proposed
- **Date:** 2026-10-11

## Context
[ADR-0028](0028-yaml-config-with-json-schema.md) selects YAML for configuration, and
[ADR-0004](0004-rust-single-binary.md) selects Rust. Their intersection is the weakest point
in the Rust choice: **`serde_yaml`, the long-standing default, was archived by its author in
March 2024** at version `0.9.34+deprecated`.

The ecosystem has not converged on one successor, and one apparent successor is actively
dangerous.

| Crate | Status |
|---|---|
| `serde_yaml` | Archived March 2024, `0.9.34+deprecated` |
| `serde_norway` | Maintained fork; uses `unsafe-libyaml-norway` |
| `serde_yaml_ng` | Maintained fork; still depends on the **unmaintained** `unsafe-libyaml` |
| `serde_yml` | **Avoid.** RUSTSEC-2025-0068: unsound and unmaintained, project archived |
| `serde-saphyr` | Pure safe Rust, no `unsafe-libyaml` dependency |
| `yaml-rust2` | Lower-level parser; successor to the unmaintained `yaml-rust` |

## Decision
**Proposed, not decided.** Current recommendation: **`serde-saphyr`**, with `serde_norway` as
the fallback if it proves immature.

Reasoning for `serde-saphyr`: it avoids the `unsafe-libyaml` lineage entirely, which is where
both maintained forks carry their risk. `serde_yaml_ng` depending on an unmaintained unsafe C
port is the specific thing worth avoiding in a tool that parses files from the internet.

`serde_yml` must be excluded explicitly, including in `cargo-deny` configuration. Its name
makes it the most likely crate to be picked by mistake, and it carries a RUSTSEC advisory.

Decided by a **time-boxed spike** in phase 1 that loads the full illustrative configuration
from [config.md](../config.md) and evaluates:

1. Round-trip fidelity for every construct the schema uses.
2. **Error message quality — line and column numbers.** This is the deciding criterion.
   Configuration errors are the most common user-facing failure, and an error without a
   location is a support burden forever.
3. Anchors and aliases, if the schema permits them.
4. Maintenance signals and `cargo-deny` cleanliness.

## Alternatives considered
- **TOML instead of YAML.** Sidesteps this entirely; `toml` is well maintained. Rejected by
  [ADR-0028](0028-yaml-config-with-json-schema.md) on nesting grounds, but if the YAML
  ecosystem proves genuinely unworkable this decision is worth reopening rather than
  suffering.
- **Vendor a parser.** Maintenance burden for no benefit.
- **JSON with a YAML front end.** Two formats, two failure modes.
- **Use `serde_yaml` anyway.** It works today. Archived dependencies accrete risk silently,
  and an unmaintained parser handling untrusted input is a poor foundation.

## Consequences
- The choice is isolated behind a thin internal module, so switching later touches one file.
  Given the ecosystem's state, that isolation is worth building deliberately.
- `cargo-deny` runs in CI with `serde_yml` and `yaml-rust` explicitly denied.
- If no crate gives line-and-column errors, custom error mapping is needed, and that cost
  belongs in the phase 1 estimate rather than being discovered later.
- This ADR is superseded by a decision record once the spike completes.
