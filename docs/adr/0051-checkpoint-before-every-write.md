# 0051 — Every artefact is checkpointed before it is overwritten

- **Status:** Accepted
- **Date:** 2026-10-13
- **Supports:** [ADR-0013](0013-blackboard-as-source-of-truth.md), [ADR-0019](0019-tie-breaking-and-oracle-precedence.md)
- **Evidence:** [measurement](../measurement.md)

- **Amended:** 2026-10-15. Untested rather than proven: it fired four times in 1,076 steps
  across round five. A task that can regress at all now exists and round six measures it.

## Context
A doer reached 18 of 21 tests passing. A repair round then rewrote the whole file three times
and left the crate not compiling. The 18/21 state was gone: no snapshot, no history, nothing
to return to. Work that had cost 60 steps and roughly an hour was destroyed by a round
intended to improve it, and the loss was permanent.

Two things made it possible. The first is that there was no checkpoint. The second is that
nothing noticed the regression: the runtime had run the oracle before the repair and after
it, held both results, and drew no conclusion from the fact that one was better.

This is not a rare case. An agent asked to fix three failing tests will sometimes rewrite
everything, and a rewrite that loses 18 working tests to fix 3 broken ones is a bad trade the
runtime is in a position to see.

## Decision
**The runtime snapshots an artefact before every write, and an oracle regression is
automatically reverted.**

1. **Checkpoint on write.** Before any write to a tracked artefact, the current content is
   copied to the run's checkpoint store, keyed by artefact and time.
2. **The best verified state is recorded.** Whenever a check runs, its result is attached to
   the current checkpoint. The best-scoring state of the run is known at all times.
3. **A regression rolls back.** When a check that previously passed now fails, or a count of
   passing tests drops, the runtime restores the last better checkpoint, tells the agent what
   happened and what it lost, and the step counts as failed.
4. **Rollback is reported, never silent.** The agent is told the revert happened and why,
   because an agent that finds its edit undone with no explanation will simply make it again.

A regression is judged by the oracle, not by an opinion. Where no check has run, there is no
regression to detect and the write stands.

## Alternatives considered
- **Use git.** Attractive, and wrong here: the user's repository history belongs to the user.
  A doer committing on every write pollutes it, and a run that ends badly should leave no
  trace in the project's history. The checkpoint store is internal to the run.
- **Keep only the last version.** Covers the measured case and fails on a slow slide, where
  each step is slightly worse than the last and none is a sharp regression.
- **Let the agent manage its own backups.** Depends on the agent being competent at exactly
  the moment it is demonstrably not.
- **Warn instead of reverting.** Leaves the broken state in place and requires the agent to
  act on the warning. The measured agent, given a precise warning, rewrote the file again.
- **Never revert; the suite is the judge at the end.** The end never arrives if each round
  starts from a worse state than the last.

## Consequences
- The checkpoint store is per run, bounded by [ADR-0022](0022-run-retention-policy.md), and
  deleted with the run.
- "Best verified state" needs a comparable score. Pass/fail is enough for a binary check;
  a test count is better; both are weaker than a per-test record, which is what makes a
  partial regression visible. The oracle contract should carry counts where the tool reports
  them.
- A run that ends badly can still deliver its best state rather than its last. This is worth
  surfacing: ending with 18/21 is a usable result, and ending with a broken build is not.
- Checkpointing only covers writes the runtime performs. An agent that writes through a shell
  command bypasses it entirely, which is why
  [ADR-0052](0052-write-restriction-in-the-sandbox.md) exists.

## What round five found, and what it could not (amendment, 2026-10-15)
Across three cells, 56 iterations and 1,076 steps, the restore fired **four times**. One in Task
C, zero in Task D, three in Task E.

[Round five's plan](../measurement/round5.md) registered the reading beforehand: near-zero
firings mean the protection is **untested rather than proven**, and that should be said out loud
rather than assumed. It is said here.

The distribution is the useful part. Task C and Task D's slices are four to nine lines with four
to nine cases each; Task E's are whole algorithms. Three of the four firings were in Task E,
which is the first evidence the mechanism needs something to regress before it can do anything
at all. A slice whose cases are independent of each other cannot score worse by being changed.

Round five's tasks therefore did not test this decision — they gave it nothing to do.
[`f-expr`](../../bench/tasks/f-expr) was built for the question: two slices, the second holding
sixteen cases that pull against each other, where getting precedence right is the usual way to
break unary minus and making division total is the usual way to break left associativity.
[Round six's Q18](../measurement/round6.md) runs it and names all three outcomes, including the
one that says to remove this mechanism: still near zero, in which case
checkpoint-before-every-write protects against something that does not happen at any slice size
Corifeu has been given.

Nothing about the decision changes in the meantime. Untested insurance is kept while it is being
tested, not removed on the strength of not having fired.
