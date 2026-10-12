# Architecture Decision Records

One file per decision. A decision earns a record when it closes off an alternative that a
reasonable person would otherwise take.

## Status

- **Accepted** — decided by the project owner in the foundation brief. Implementation may
  not contradict it; changing it requires a superseding ADR.
- **Proposed** — raised during the foundation analysis. Not yet decided. Listed in
  [open-questions.md](../open-questions.md).
- **Superseded by NNNN** — replaced. The file stays; history is not rewritten.

## Template

```markdown
# NNNN — Title

- **Status:** Accepted | Proposed | Superseded by NNNN
- **Date:** YYYY-MM-DD

## Context
What forces this decision, and what breaks without one.

## Decision
What we do. Present tense, unambiguous.

## Alternatives considered
What else was on the table and why it lost. An ADR with no alternatives
documents a preference, not a decision.

## Consequences
What this makes easy, what it makes hard, and what it obliges us to build.
```

## Index

### Identity and licence
| ADR | Title | Status |
|---|---|---|
| [0001](0001-product-name-and-directory.md) | Product name, binary name and project directory | Accepted |
| [0002](0002-english-only.md) | English for every artefact | Accepted |
| [0003](0003-agpl-with-notice-and-dco.md) | AGPL-3.0 with NOTICE and DCO, no CLA | Accepted |

### Runtime and scope
| ADR | Title | Status |
|---|---|---|
| [0004](0004-rust-single-binary.md) | Rust, distributed as a single prebuilt binary | Accepted |
| [0005](0005-target-platforms.md) | Linux, macOS, native Windows and WSL at launch | Accepted |
| [0006](0006-mvp-scope-software-development.md) | Software development as the only MVP domain | Accepted |

### Providers
| ADR | Title | Status |
|---|---|---|
| [0007](0007-cli-providers-first.md) | Agent CLIs as the first provider class | Accepted |
| [0008](0008-local-runtimes.md) | Ollama and llama.cpp as local runtimes | Accepted |
| [0009](0009-direct-apis-deferred.md) | Direct provider APIs deferred to MVP 2 | Accepted |

### Roles
| ADR | Title | Status |
|---|---|---|
| [0010](0010-role-categories.md) | Five role categories | Accepted |
| [0011](0011-user-defined-granularity.md) | The user decides role granularity | Accepted |
| [0012](0012-required-and-optional-roles.md) | Importance means obligatoriness | Accepted |

### Coordination
| ADR | Title | Status |
|---|---|---|
| [0013](0013-blackboard-as-source-of-truth.md) | A shared blackboard is the source of truth | Accepted |
| [0014](0014-context-contracts-per-role.md) | Every role reads through a context contract | Accepted |
| [0015](0015-contracted-direct-channels.md) | Direct messages only between declared pairs | Accepted |
| [0016](0016-typed-messages-with-schemas.md) | Typed messages with published schemas | Accepted |
| [0017](0017-artifact-references-not-summaries.md) | Agents exchange references, not summaries | Accepted |
| [0018](0018-loop-and-budget-control.md) | Rounds, budgets, loop detection and timeouts | Accepted |
| [0019](0019-tie-breaking-and-oracle-precedence.md) | Oracles outrank opinions; defined tie-breaking | Accepted |
| [0020](0020-sequential-execution-in-mvp1.md) | Sequential execution in MVP 1 | Accepted |

### Storage and memory
| ADR | Title | Status |
|---|---|---|
| [0021](0021-sqlite-with-event-log.md) | Embedded SQLite with an append-only event log | Accepted |
| [0022](0022-run-retention-policy.md) | Retention by compaction | Accepted |
| [0023](0023-project-memory-in-versioned-files.md) | Project memory as versioned human-readable files | Accepted |

### Autonomy, git and failure
| ADR | Title | Status |
|---|---|---|
| [0024](0024-autonomy-levels.md) | Three autonomy levels | Accepted |
| [0025](0025-git-flow-chosen-at-install.md) | Git behaviour decided step by step at install | Accepted |
| [0026](0026-configurable-failure-fallback.md) | Failure handling is configurable | Accepted |

### Installer and configuration
| ADR | Title | Status |
|---|---|---|
| [0027](0027-install-wizard-with-tool-call-test.md) | Interactive wizard gated by a real tool-call test | Accepted |
| [0028](0028-yaml-config-with-json-schema.md) | YAML configuration with a published JSON Schema | Accepted |
| [0029](0029-config-precedence-order.md) | Four-level configuration precedence | Accepted |
| [0030](0030-marker-based-file-merging.md) | Foreign files edited only inside markers | Accepted |
| [0031](0031-token-savings-metering.md) | Metering reports savings in the user's own currency | Accepted |

### Extensibility
| ADR | Title | Status |
|---|---|---|
| [0032](0032-out-of-process-plugins.md) | Plugins run out of process over a versioned protocol | Accepted |
| [0033](0033-extension-points.md) | Five extension points, one interface | Accepted |

### Migration
| ADR | Title | Status |
|---|---|---|
| [0034](0034-break-existing-personal-setup.md) | The existing personal setup is not preserved | Accepted |

### Raised by the foundation analysis, accepted in review
| ADR | Title | Status |
|---|---|---|
| [0035](0035-file-ownership-by-glob-classes.md) | File ownership by glob classes, not path prefixes | Accepted |
| [0036](0036-runtime-enforced-tool-restriction.md) | Tool restriction enforced by the runtime | Accepted |
| [0037](0037-sandboxed-command-execution.md) | Command execution is sandboxed by default | Accepted |
| [0038](0038-config-as-uninstall-restore.md) | The generated config file is the restore point | Accepted |
| [0039](0039-provider-conformance-suite.md) | Every provider passes a conformance suite | Accepted |
| [0040](0040-yaml-parser-selection.md) | YAML parser selection | Accepted (process); crate pending spike |
| [0041](0041-drop-the-mode-concept.md) | Drop the open/private mode concept | Accepted |
| [0042](0042-test-authorship.md) | Test authorship is configurable, self-testing warned | Accepted |
| [0043](0043-any-provider-may-lead.md) | Any provider may lead, with a quality warning | Accepted |
| [0044](0044-antigravity-replaces-gemini-cli.md) | Antigravity CLI replaces the retired Gemini CLI | Accepted |
| [0045](0045-prepared-workspace-for-context-isolation.md) | Prepared workspace for context isolation | Proposed — deferred past MVP 1 |
