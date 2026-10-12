# 0013 — A shared blackboard is the source of truth

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
In the prototype, the run's state lives in the orchestrator's conversation context. That has
three consequences: the run cannot be resumed after an interruption, cannot be audited
afterwards, and grows the expensive model's context monotonically — defeating the goal that
justifies the product.

## Decision
A **blackboard** holds all run state: tasks, artefacts, messages, findings, verdicts,
budgets and events. It is the single source of truth. **No agent holds the whole board.**
Each role receives a **view** assembled by its context contract
([ADR-0014](0014-context-contracts-per-role.md)).

Nothing is real until it is on the board. An agent's assertion in prose is not state; the
typed message or artefact it produces is ([ADR-0016](0016-typed-messages-with-schemas.md)).

The board is backed by SQLite with an append-only event log
([ADR-0021](0021-sqlite-with-event-log.md)). Board state is a projection of the event log,
which makes replay, resumption and audit fall out of the storage design rather than needing
separate machinery.

## Alternatives considered
- **Conversation history as state.** What exists today. Unresumable, unauditable,
  unbounded.
- **Files on disk as the board.** Human-readable and git-diffable, with no transactions, no
  queries and racy concurrent access — which forecloses
  [ADR-0020](0020-sequential-execution-in-mvp1.md)'s later parallelism. Project *memory*
  does use files, deliberately ([ADR-0023](0023-project-memory-in-versioned-files.md));
  run state does not.
- **An in-memory board persisted at checkpoints.** Faster, loses the audit trail, and
  reintroduces the crash-recovery problem the event log solves.

## Consequences
- The lead's context stays bounded regardless of run length. **This is where the token
  saving actually comes from** — not from using cheaper models, but from no single context
  accumulating the whole run.
- Every agent interaction becomes a board read and a board write, which is more plumbing
  than passing a string, and is what makes the system inspectable.
- The board must be designed so that parallel doers do not require redesigning it
  ([ADR-0020](0020-sequential-execution-in-mvp1.md)): task ownership, artefact versioning
  and event ordering are modelled now and exercised later.
- A corrupted or lost board loses the run, not the work — artefacts are files in the
  repository and survive independently.
