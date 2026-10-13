# 0058 — A slice boundary is observed, not enforced

- **Status:** Accepted
- **Date:** 2026-10-13
- **Refines:** [ADR-0056](0056-one-oracle-per-slice.md)
- **Supports:** [ADR-0031](0031-token-savings-metering.md)
- **Evidence:** [measurement](../measurement.md#slices-3-and-4-never-ran)

## Context
[ADR-0056](0056-one-oracle-per-slice.md) hands the coder one slice with one oracle. The second
measurement run was the first test of that rule, and it produced the intended result together
with an unintended one.

Task C was cut into four slices, one function each, the other three left as `todo!()` and the
slice text saying not to touch them. Slice 2's coder implemented its own function and then
implemented `glob_match`, `glob_match_suffix` and `classify` as well. Slices 3 and 4 were
green the first time they were checked, so no round was ever run for either of them.

Nothing could have stopped it. The four functions live in one file, so
[ADR-0052](0052-write-restriction-in-the-sandbox.md)'s file-class sandbox has no finer
granularity to offer, and the `todo!()` markers were an honour system. The agent also did not
think it was disobeying: it finished its slice, and kept going.

The headline comparison survives this — 79 steps against 121, 362,231 prompt tokens against
921,578, green against a hang. What does not survive is the per-slice arithmetic. Those 79
steps bought slices 1 and 2; the other two arrived as a side effect and were recorded as free.
Left unexamined, that is a decomposition reporting a discount it did not earn, into the
savings figure [ADR-0031](0031-token-savings-metering.md) shows the user.

## Decision
**Corifeu does not enforce slice boundaries. It checks the other slices after each one closes
and records what was delivered early.**

1. **When a slice closes, the pending slices' oracles are run.** Each one already green is
   recorded as delivered by the slice that closed, and is not dispatched.
2. **Cost is attributed to the slice that did the work.** A slice that was green before it
   started costs nothing and is reported as costing nothing, with a pointer to the slice that
   paid for it.
3. **Early delivery is not a failure.** The slice's own oracle went green, which is the whole
   contract in [ADR-0056](0056-one-oracle-per-slice.md). The coder is not corrected, reverted
   or told off.
4. **Overflow is a planning signal.** A planner that repeatedly splits work the model treats
   as one unit is producing slices that cost a dispatch and deliver nothing. The count is kept
   per run so decomposition can be tuned against evidence rather than taste.

## Alternatives considered
- **Enforce at sub-file granularity.** A writable set expressed as an AST region rather than a
  path. It needs a structural editor per language, and agents write whole files, so almost
  every legitimate write would be refused.
- **Revert the out-of-scope work and dispatch those slices properly.** Pays twice for work
  that is already correct, and the revert can break the slice that just passed.
- **Ask the coder to confirm it stayed in scope.** Self-report. The measurement's coder
  believed it had complied, and said so.
- **Accept it silently.** What the second run did. It is why the 79 steps read as four slices
  rather than two.
- **Write the slice text more firmly.** [ADR-0036](0036-runtime-enforced-tool-restriction.md)
  rules this out as a mechanism: restriction by instruction is not restriction.

## Consequences
- A pending slice may close before it is dispatched, so the planner's ordered list is a
  proposal rather than a schedule.
- Running the pending oracles after each slice costs one check per remaining slice. On a local
  provider that is cheap next to a dispatch, and it is the same work the slice would have cost
  to start.
- It catches the opposite case for free: work that overflows and *breaks* a later slice shows
  up immediately as a red oracle, rather than as a surprise when that slice is reached.
- [ADR-0056](0056-one-oracle-per-slice.md)'s rule is unchanged. What changes is what the run
  records about how the rule actually played out.
- The measurement's own numbers are now qualified rather than corrected, and the
  [measurement](../measurement.md#what-this-measurement-does-not-show) says so.
