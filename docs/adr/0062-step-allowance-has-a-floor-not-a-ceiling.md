# 0062 — The step allowance has a floor that grows, not a ceiling that is guessed

- **Status:** Accepted
- **Date:** 2026-10-14
- **Refines:** [ADR-0018](0018-loop-and-budget-control.md)
- **Supports:** [ADR-0031](0031-token-savings-metering.md), [ADR-0050](0050-loop-detection-warns-before-it-stops.md)
- **Evidence:** [measurement](../measurement.md#fourth-run--ten-per-cell)

## Context
A role working a slice is given a number of steps and stopped when it runs out. Until now that
number was one constant, set to 30 by the runs that produced the measurement. The fourth run
was the first with enough slices to say what the constant should be, and the answer was that
it should not be a constant.

Sixty-nine slices were worked across twenty runs. Where they stopped:

| | Slices | Steps |
|---|---|---|
| Reached green | 57 | 38 of them in **1**; the slowest in **12** |
| Went in circles ([ADR-0050](0050-loop-detection-warns-before-it-stops.md)) | 11 | 8, 14, 15, 15, 16, 17, 17, 23, 23, 24, 27 |
| Spent all 30 | 1 | 30 |

The two distributions do not overlap above twelve. A limit of twelve would have stopped **11
of the 12** slices that were going nowhere and **none of the 57** that finished — 89 of the
round's 274 steps, 32% of the budget, for no loss that this round can see.

What the round cannot say is whether twelve holds anywhere else. Every slice was Rust, in one
crate, on one task, and much of the work was done by compiler errors rather than by the model.
A language without that feedback, or a slice with more to implement, is a different
distribution, and a constant of twelve would be a guess about it exactly as thirty was.

## Decision
**A role starts every turn at a floor. The floor grows, up to a ceiling, only where turns are
observed running out of steps.**

```
Steps { floor: 12, ceiling: 30, patience: 2, raise_by: 6 }
```

1. **Every turn starts at `floor`.** Twelve by default, which is the slowest green slice the
   fourth run measured.
2. **Only running out of steps counts toward a rise.** A turn stopped by loop detection was
   never short of steps; counting it would hand the larger allowance to exactly the runs that
   waste it. A turn stopped by an unreachable provider says nothing about steps at all.
3. **`patience` turns must run out before the allowance rises**, and it rises by `raise_by`,
   never past `ceiling`.
4. **Anything that reaches its goal puts the allowance back at the floor.** Without this the
   allowance only ratchets, and the ceiling quietly becomes the default again.
5. **One allowance per run, not per role or per slice.** The question it answers — is this
   workload harder than the floor assumes? — is about the work, not about who is doing it.
6. **All four numbers are configuration** (`limits.steps`), because the evidence behind the
   default covers one language and one task.

## Alternatives considered
- **Keep a single number and set it to 12.** The whole saving, none of the mechanism. It also
  hard-codes a figure measured on one crate in one language into every future run, which is
  the mistake 30 already was.
- **Keep 30.** What was measured. 32% of the round's steps bought nothing.
- **Raise the allowance when the slice is still making progress**, using the case count
  [ADR-0051](0051-checkpoint-before-every-write.md) already scores each state by. Sharper than
  counting cuts, and probably right. It is left out because QA is not scored that way — a new
  failing test is not a regression — so the signal exists for one of the two roles. It stays
  on the list for the fifth run to measure.
- **Let the model ask for more steps.** [ADR-0036](0036-runtime-enforced-tool-restriction.md):
  a restriction the subject can lift is not a restriction. The measurement's models also could
  not diagnose their own loops (defect 11), so the one agent least able to judge would be
  judging.
- **Retry a cut slice from scratch with a bigger allowance.** Pays for the whole slice twice.
  The rise already reaches the next attempt, because a slice the coder could not pass goes
  back to QA for another round ([ADR-0059](0059-a-test-is-unvalidated-until-green.md)).

## Consequences
- The common case gets cheaper and the pathological case gets cheaper. Nothing gets more
  expensive except a workload that genuinely needs more than twelve steps, which pays
  `patience` cut turns before it is believed.
- **A raised allowance is a signal worth reporting.** It says this workload does not fit the
  configured default, which is either a planner cutting slices too large
  ([ADR-0056](0056-one-oracle-per-slice.md)) or a floor set for the wrong language.
- The floor of 12 is evidence from one language and one task, and the fifth run is designed to
  test it elsewhere. Until then it is a default, not a finding.
- Loop detection ([ADR-0050](0050-loop-detection-warns-before-it-stops.md)) now fires less
  often, because most of the slices it used to catch run out of steps first. The two limits
  overlap by design: one is about spending, the other about going in circles, and either is
  enough on its own.
