# 0025 — Git behaviour decided step by step at install

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Git workflow is where a tool most easily violates a developer's expectations, and the
violations are expensive: an unexpected push, a branch deleted, a PR opened on a repository
with a strict review culture. Conventions vary widely and are strongly held.

A single `--auto-commit` flag cannot express the distinctions that matter. The project
owner's own flow illustrates why: automatic local commits, **nothing pushed** until the work
is complete, then a presentation of every commit with its rationale, then push and PR after
approval, then merge and branch deletion after review.

That flow has a push gate in the middle, which an on/off switch cannot represent.

## Decision
The installer asks about **each git operation separately**, and the answers become explicit
configuration.

| Step | Options |
|---|---|
| Branch strategy | current branch / new branch per run / new branch per task |
| Branch naming | pattern, e.g. `corifeu/{run-slug}` |
| Commit cadence | per task / per phase / once at the end / never |
| Commit message | who writes it, and which convention |
| Push | never / after approval / automatically |
| Pull request | never / after approval / automatically |
| PR description | generated / template / none |
| PR labels | fixed set / inferred / none |
| Merge | never / after approval / automatically |
| Branch deletion | never / after merge / after approval |

Every step defaults to its **most conservative** option. A user who clicks through the
wizard without reading ends up with a configuration that cannot surprise them.

The owner's flow is expressible and appears as a worked example in
[config.md](../config.md).

## Alternatives considered
- **A few named presets** (`conservative`, `balanced`, `yolo`). Fewer questions, and the
  combinations users actually want fall between presets — the owner's own flow is one such.
  Presets remain useful as *starting points* that pre-fill the per-step answers.
- **Corifeu never touches git.** Safe, and discards real value: commit hygiene and
  rationale-carrying messages are among the things agents do well.
- **Infer the convention from repository history.** Guessing, applied to irreversible
  operations.

## Consequences
- The wizard is longer. Mitigated by presets pre-filling answers and by non-interactive
  installation ([ADR-0028](0028-yaml-config-with-json-schema.md)).
- Git operations are a distinct capability in the runtime, subject to the same restriction
  mechanism as any other tool ([ADR-0036](0036-runtime-enforced-tool-restriction.md)). An
  agent cannot push by running `git push` in a shell if pushing is disabled — this is
  exactly why [ADR-0037](0037-sandboxed-command-execution.md) matters.
- Pushing and PR creation need credentials, which come from the user's existing git and
  `gh` configuration. Corifeu stores no tokens.
- Interaction with autonomy ([ADR-0024](0024-autonomy-levels.md)) must be unambiguous: the
  **stricter** of the two always wins.
