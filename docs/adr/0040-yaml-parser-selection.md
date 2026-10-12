# 0040 — YAML parser selection

- **Status:** Accepted
- **Decided:** 2026-10-12, by measurement
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
**`serde-saphyr`**, decided by running the spike rather than reading about the crates.

`serde_yml` and `yaml-rust` are denied permanently in `deny.toml`. The parser is isolated
behind one internal module, so this choice can change by touching one file — worth building
deliberately given the state of this ecosystem.

### What the spike measured

Three candidates, each parsing five deliberately broken configs shaped like Corifeu's own.

**`serde_yaml_ng` eliminated itself.** It fails Corifeu's own dependency gate:

```
error[unmaintained]: unsafe-libyaml is unmaintained
advisories FAILED
```

No judgement needed — `cargo deny` rejects it.

The remaining two:

| | `serde_norway` | `serde-saphyr` |
|---|---|---|
| Latest | 0.9.42 | **1.3.0** |
| Last release | 2024-12-21 (~22 months) | **2026-09-16** |
| Backing parser | `unsafe-libyaml-norway`, a C port, also stale since 2024-12 | **pure Rust** |
| Crates in tree | **15** | 26 |
| Cold build | **6.0 s** | 7.3 s |
| Drop-in for `serde_yaml` | **yes** | no |
| Advisories | pass | pass |

**Error quality decided it**, as this ADR committed in advance. Tabs instead of spaces, the
most common YAML mistake there is:

```
serde_norway : found character that cannot start any token at line 3 column 1,
               while scanning for the next token

serde-saphyr : error: line 3 column 2: tabs disallowed within this context
               (block indentation)
 --> <input>:3:2
  |
2 | roles:
3 |     lead:
  |     ^ tabs disallowed within this context (block indentation)
```

One names the problem. The other describes the parser's internal state. Across all five
cases `serde-saphyr` produced rustc-style output with source context and a caret; both forks
produced a single cryptic line.

A "maintained fork" that has not shipped in 22 months is maintained in name. `serde-saphyr`
is the only candidate under active development, and it wins on the criterion chosen before
the evidence was gathered.

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
- **Costs accepted knowingly.** `serde-saphyr` adds ~1.3 s to a cold build and 11 crates
  over `serde_norway`, pulling `encoding_rs`, `base64` and `annotate-snippets`. All are
  healthy, but it is more surface. Its own description advertises "fast builds", which this
  measurement mildly contradicts.
- **Migration is not free.** `serde_norway` is API-compatible with `serde_yaml`, so moving
  between them is a rename. `serde-saphyr` is not, so a future switch costs real work. The
  isolation module exists for exactly this.
- **It is a small project**, essentially one maintainer. If it stalls we migrate — and we
  would be migrating *from* good errors *to* worse ones. Revisit if it goes a year without
  a release.
- No custom error mapping is needed. `serde-saphyr` already reports line and column with
  source context, which was the thing worth paying for.
