# 0020 — Sequential execution in MVP 1

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Parallel doers are the obvious way to make runs faster, and the obvious way to make them
unreproducible. Two agents editing one repository concurrently conflict; isolating them
requires git worktrees, merge strategy, conflict resolution and a story for what happens
when two merged branches each pass their own tests and fail together.

That is a substantial subsystem, and it is not what makes the product valuable.

## Decision
MVP 1 executes **sequentially**: one task active at a time, one agent acting at a time.

The blackboard is nonetheless designed so parallelism does not require redesign:

1. Tasks carry an **owner** and a state machine (`pending`, `assigned`, `in_progress`,
   `blocked`, `done`), so concurrent ownership is representable from the start.
2. Artefacts are **versioned and content-hashed**, so concurrent modification is
   detectable.
3. The event log is **totally ordered** with a monotonic sequence, so concurrent events have
   a defined order.
4. No component assumes "the current task" — there is always an explicit task id.

Parallel doers with git worktrees are deferred to a later phase, along with the merge and
conflict machinery they require.

## Alternatives considered
- **Parallel from the start.** Large scope increase, non-deterministic debugging, during the
  phase when the core design is least settled.
- **Sequential forever.** Forecloses a real benefit; Codex's `--worktree` flag shows the
  mechanism is becoming standard.
- **Parallel reads, sequential writes.** A genuine middle ground — researchers and checkers
  are read-only and could run concurrently. Tempting, and still deferred, because it needs
  the same concurrent event-ordering correctness as full parallelism for a fraction of the
  benefit.

## Consequences
- Runs are slower than they could be, bounded by the slowest agent in the chain. With local
  models this is noticeable.
- Runs are deterministic and debuggable, which during the phase that discovers whether the
  architecture works is worth more than speed.
- The four design constraints above are a standing obligation on every component, and they
  cost a little clarity now for a lot of rework avoided later. The review question is "would
  this still be correct with two doers?", and that question must be asked while the code is
  being written, not after.
