# 0021 — Embedded SQLite with an append-only event log

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The blackboard ([ADR-0013](0013-blackboard-as-source-of-truth.md)) needs transactional
writes, queries across tasks and roles, and durability across crashes. It must also impose
**zero external dependencies**, because the product installs into strangers' repositories
([ADR-0004](0004-rust-single-binary.md)).

Four capabilities are wanted and are all the same capability viewed from different angles:
replaying a run, resuming an interrupted one, auditing what happened, and metering token
use per role.

## Decision
**SQLite, compiled into the binary** (`rusqlite` with the `bundled` feature), stored at
`.corifeu/run/corifeu.db`.

The schema centres on an **append-only event log**. Events are never updated or deleted;
current board state is a **projection** of the log. The four capabilities above then fall
out of one mechanism rather than needing four:

| Need | How the log provides it |
|---|---|
| Replay | Re-project from sequence 0 |
| Resume | Project to the last event, continue |
| Audit | The log *is* the audit trail |
| Metering | Usage events are ordinary events, aggregated by query |

Each event carries: monotonic sequence, run id, timestamp, actor (role), kind, payload
(typed, [ADR-0016](0016-typed-messages-with-schemas.md)), and references to artefacts.

**Large artefact content lives on disk**, content-addressed by hash under
`.corifeu/run/artifacts/`, with only metadata and hash in the database. This keeps the
database small and queryable, and it **deduplicates automatically**: agents re-read the same
files constantly, and identical content is stored once however many times it is referenced.

WAL mode is enabled, which gives durability and readers that do not block the writer —
needed for a `corifeu watch`-style live view, and later for parallel execution.

## Alternatives considered
- **JSON or JSONL files on disk.** No transactions, no queries, and concurrent access is
  racy. Rejected for run state; deliberately chosen for project memory
  ([ADR-0023](0023-project-memory-in-versioned-files.md)), where human readability and git
  diffs matter more than queries.
- **An embedded key-value store** (redb, sled). Smaller, faster, and no ad-hoc queries —
  which metering and audit need. SQLite's query surface is the point.
- **A mutable state table without an event log.** Simpler, and loses replay, audit and
  resume at once.
- **An external database.** Contradicts the zero-dependency goal outright.

## Consequences
- The log grows monotonically within a run, making retention a real concern
  ([ADR-0022](0022-run-retention-policy.md)).
- Projection logic must be deterministic and tested, because every capability above depends
  on it producing identical state from identical events.
- Schema migrations need handling from the first release; a user upgrading Corifeu must not
  lose history. Version the schema and migrate forward on open.
- `.corifeu/run/` is git-ignored. A user who commits the database will hit binary merge
  conflicts on every pull — the installer must write the `.gitignore` entry, and `doctor`
  must check it.
- SQLite over a network filesystem is unreliable. Worth a `doctor` warning when the
  repository appears to be on NFS or SMB.
