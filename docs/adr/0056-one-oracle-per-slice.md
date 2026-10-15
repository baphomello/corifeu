# 0056 — A slice has exactly one oracle, and the planner emits them all at once

- **Status:** Accepted
- **Date:** 2026-10-13 (amended 2026-10-14)
- **Supports:** [ADR-0011](0011-user-defined-granularity.md), [ADR-0019](0019-tie-breaking-and-oracle-precedence.md)
- **Evidence:** [measurement](../measurement.md)

## Context
"Small scoped tasks" is half of the product thesis and it was the least tested half, because
the measurement handed the doer tasks that were not small. Each was a 40-line specification
expecting roughly 300 lines of implementation and tests in one shot.

The results separate cleanly by task size:

| Task given | Outcome |
|---|---|
| "implement this specification" | 18/21, three times, and once a wholesale rewrite that broke the build |
| "the crate does not compile; fix the first error, re-run, fix the next" | **21/21 green** |
| "implement this specification" (Task C) | a glob matcher that hangs, never diagnosed in 101 steps |

The winning round was not a better model or a longer budget. It was a task with one
unambiguous check and one next action.

"Small" needs a definition that is checkable, or it becomes taste. The definition that falls
out of the measurement is the oracle: the task that worked had exactly one, and it was named
in the task text.

## Decision
**A slice is small enough when it has exactly one oracle. The planner names that oracle, and
emits the whole list of slices in one pass.**

1. **One slice, one check.** A slice carries a single command whose transition from failing
   to passing means the slice is done. If the planner cannot name that command, the slice is
   not ready and is decomposed further.
2. **The oracle is named before the work starts**, which makes it the thing QA writes
   ([ADR-0053](0053-qa-writes-the-tests.md)) and the thing the coder works against.
3. **The planner emits every slice in one pass.** It is asked once, produces the ordered list,
   and is not consulted again unless a slice fails or the list is exhausted.
4. **Scaffolding is not a slice.** Work with a deterministic tool — a project generator, a
   formatter, a dependency install — is executed directly. "Create a React boilerplate" is
   `create-vite`, not a prompt.

### Why the planner is asked once

A planner is an expensive model. If each slice costs a planner round trip, decomposition
spends on planning what delegation saved on implementation, and the product's central claim
quietly inverts. Asking once and executing the list is what keeps the saving.

The failure path is the exception: a slice that fails its oracle after its repair budget goes
back to the planner, because that is evidence the decomposition was wrong.

## Alternatives considered
- **Let the doer decompose its own task.** The agent least able to see the whole shape, at
  the moment it is most loaded.
- **Plan one slice at a time, with full knowledge of the last result.** Better plans, and a
  planner call per slice. Rejected on cost, which is the project's reason for existing.
- **Define "small" by lines of code or file count.** Measurable and wrong: a 20-line change
  with no check is harder for an agent than a 200-line one with a precise test.
- **Let the user slice the work.** Available to any user who wants it, and it makes the
  product a task runner rather than an orchestrator.
- **Allow several checks per slice.** Each additional check adds a way to be partially done,
  which is the state the measurement showed agents handle worst — three runs sat at 18 of 21
  without converging.

## Consequences
- The planner's output format carries a named check per slice, and a plan whose slices lack
  one is rejected before execution rather than discovered mid-run.
- Slices are ordered and may depend on each other, so a failure early in the list invalidates
  what follows. The planner is re-consulted with the failure rather than the run being
  abandoned. **Amended 2026-10-14 by [ADR-0063](0063-an-unfinished-slice-is-asked-again.md):**
  the fourth run measured this and it came out the other way. Carrying on past a failed slice
  is what produced the complete crate in eight of nineteen runs, and the slice that had not
  closed was green when asked again at the end. A failure now ends the walk for one reason
  only — an unreachable provider — and the planner is consulted only about slices still red
  after the whole plan has run.
- The one-oracle rule interacts with [ADR-0049](0049-faithful-check-results.md): a slice whose
  check does not terminate is not a failing slice, and should not consume the repair budget as
  though it were.
- This is the thesis's quality claim made testable. Each slice is an experiment with a
  recorded verdict, so "small scoped tasks raise quality" becomes a number rather than a
  belief.
