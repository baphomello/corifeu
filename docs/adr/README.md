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
| [0040](0040-yaml-parser-selection.md) | YAML parser selection — `serde-saphyr` | Accepted |
| [0041](0041-drop-the-mode-concept.md) | Drop the open/private mode concept | Accepted |
| [0042](0042-test-authorship.md) | Test authorship is configurable, self-testing warned | Accepted |
| [0043](0043-any-provider-may-lead.md) | Any provider may lead, with a quality warning | Accepted |
| [0044](0044-antigravity-replaces-gemini-cli.md) | Antigravity CLI replaces the retired Gemini CLI | Accepted |
| [0045](0045-prepared-workspace-for-context-isolation.md) | Prepared workspace for context isolation | Proposed — deferred past MVP 1 |
| [0046](0046-release-integrity-without-code-signing.md) | Release integrity without code-signing certificates | Accepted |

### Found by measurement
These record decisions forced by the first run of the thesis
[measurement](../measurement.md), which traced twelve causes for stopped runs — nine of them
defects in the runtime rather than in any model.

| ADR | Title | Status |
|---|---|---|
| [0047](0047-bounded-agent-context-within-a-task.md) | The agent loop rebuilds a bounded context each step | Accepted |
| [0048](0048-bounded-generation-for-local-runtimes.md) | Local runtimes get an explicit generation budget | Accepted |
| [0049](0049-faithful-check-results.md) | Check results reach the doer intact | Accepted |
| [0050](0050-loop-detection-warns-before-it-stops.md) | Loop detection warns before it stops | Accepted |
| [0051](0051-checkpoint-before-every-write.md) | Every artefact is checkpointed before it is overwritten | Accepted |
| [0052](0052-write-restriction-in-the-sandbox.md) | Write restriction is enforced by the sandbox | Accepted |
| [0053](0053-qa-writes-the-tests.md) | The default role pair is QA and coder, QA owning the tests | Accepted |
| [0054](0054-findings-become-work-only-as-tests.md) | A finding becomes work only after QA reproduces it | Accepted |
| [0055](0055-one-local-doer-by-default.md) | One local doer by default, raised per provider | Accepted |
| [0056](0056-one-oracle-per-slice.md) | A slice has exactly one oracle | Accepted |

### Found by the second measurement run
Task C was re-sliced under [ADR-0056](0056-one-oracle-per-slice.md) and went from a
135-minute hang to a green suite in 39 minutes. Three more causes surfaced on the way, all of
them holes in decisions taken from the first run rather than new discoveries. These refine
those decisions; none supersedes one.

| ADR | Title | Status |
|---|---|---|
| [0057](0057-oracle-integrity-by-test-inventory.md) | Oracle integrity is checked by test count, not file ownership | Accepted |
| [0058](0058-slice-boundaries-are-observed.md) | A slice boundary is observed, not enforced | Accepted |
| [0059](0059-a-test-is-unvalidated-until-green.md) | A test is unvalidated until it has gone green | Accepted |

### Found while implementing
A decision the documents disagreed about, discovered because code had to pick one.

| ADR | Title | Status |
|---|---|---|
| [0060](0060-argv-execution-and-where-pipefail-applies.md) | Commands run as argument vectors; pipefail governs the shell opt-in | Accepted |

### Privacy
The thesis was written as two claims, cost and quality. Confidentiality is a third thing
users want and the first two do not supply.

| ADR | Title | Status |
|---|---|---|
| [0061](0061-privacy-by-file-class.md) | Privacy is decided per file class, and what leaves is minimised | Accepted |

### Fourth measurement run
Twenty runs per cell, enough slices to replace two constants with evidence.

| ADR | Title | Status |
|---|---|---|
| [0062](0062-step-allowance-has-a-floor-not-a-ceiling.md) | The step allowance has a floor that grows, not a ceiling that is guessed | Accepted |
| [0063](0063-an-unfinished-slice-is-asked-again.md) | A slice that did not close is asked again at the end of the plan | Accepted |

### Scaling past one language
Supporting Python cost one declaration per Rust assumption the harness held. That worked and
does not scale to any language, let alone to image and video. Two decisions about direction,
taken before the work that would test them, so that the reasoning is not re-derived later.

| ADR | Title | Status |
|---|---|---|
| [0064](0064-a-model-writes-the-rule-it-is-not-the-rule.md) | A model writes the rule; a model is not the rule | Accepted, not implemented |
| [0065](0065-an-oracle-and-a-judge-are-different-contracts.md) | An oracle and a judge are different contracts, and are never mixed | Accepted, not implemented |

### Decisions taken on round five
The fifth round answered eight of its twelve questions and produced seven decisions. Five of
them are a field or a value with the measured behaviour as the default, so nothing changed by
default and all of them are testable; [round six](../measurement/round6.md) is what tests them.
Two needed an ADR of their own.

| ADR | Title | Status |
|---|---|---|
| [0066](0066-a-round-is-replayable-or-it-is-an-anecdote.md) | A measured round is replayable, or it is an anecdote | Accepted |
| [0067](0067-a-check-that-cannot-be-counted-is-not-a-verdict.md) | A check that cannot be counted is not a verdict on the code | Accepted |

Amended by the same round: [0050](0050-loop-detection-warns-before-it-stops.md) (the repeat
threshold is now a setting, and priced at 30% of a round's steps for a 3.7% rescue rate),
[0051](0051-checkpoint-before-every-write.md) (untested rather than proven — four firings in
1,076 steps), and [0063](0063-an-unfinished-slice-is-asked-again.md) (the re-probe is confirmed;
carrying on is worth 66% in a chained plan and 2% in an independent one, so it became a policy).

