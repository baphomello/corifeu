# 0023 — Project memory as versioned human-readable files

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Two kinds of state exist, and conflating them has caused one of the prototype's defects
already — `.orchestra/SPEC.md`, per-run state, was committed to git.

**Run state** is machine-oriented, high-volume, ephemeral and uninteresting a month later.
**Project memory** is the opposite: decisions, conventions and constraints learned about
*this project*, which future runs should consult, which humans should be able to read and
correct, and which belongs in code review.

## Decision
The two are stored differently, and the split is structural rather than conventional.

| | Run state | Project memory |
|---|---|---|
| Location | `.corifeu/run/` | `.corifeu/memory/` |
| Format | SQLite + content-addressed blobs | Markdown with YAML front matter |
| Git | **ignored** | **committed** |
| Audience | the runtime | humans and future runs |
| Lifetime | retention policy | the project's lifetime |

Memory documents are consulted by future runs through context contracts
([ADR-0014](0014-context-contracts-per-role.md)): a role declares which memory documents
enter its view. Memory is not injected globally — that would reintroduce the unbounded
context the design exists to avoid.

Agents may *propose* memory changes; writing them follows the autonomy level
([ADR-0024](0024-autonomy-levels.md)). Under supervised autonomy a memory change is a
proposal the user approves, exactly like a code change.

## Alternatives considered
- **Memory in the database.** Consistent with run state, and invisible to humans, undiffable
  in review, and lost with the database.
- **Memory in the existing `CLAUDE.md` / `AGENTS.md`.** Already read by the CLIs, and
  provider-specific, unstructured, and in direct tension with the suppression of ambient
  context that context contracts require ([ADR-0007](0007-cli-providers-first.md)).
  Corifeu may *generate* a pointer in those files ([ADR-0030](0030-marker-based-file-merging.md))
  but does not store memory there.
- **Automatic memory extraction from every run.** Accumulates noise fast, and noisy memory
  is worse than none because it is consulted with the same authority as curated memory.

## Consequences
- Memory is reviewable in pull requests, which is where a wrong convention gets caught.
- Memory documents must be small and specific, since they are included in contracted views
  and therefore cost tokens on every run that reads them. A size budget belongs in `doctor`.
- Stale memory is actively harmful — a convention that no longer holds will be followed
  confidently. Memory entries carry a date and the run that proposed them, and `doctor`
  flags entries referencing files that no longer exist.
- `.corifeu/` therefore contains both committed and ignored content. The `.gitignore` entry
  must be precise: ignore `.corifeu/run/`, not `.corifeu/`.
