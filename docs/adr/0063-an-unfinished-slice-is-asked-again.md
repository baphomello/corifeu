# 0063 — A slice that did not close is asked again at the end of the plan

- **Status:** Accepted
- **Date:** 2026-10-14
- **Refines:** [ADR-0058](0058-slice-boundaries-are-observed.md)
- **Supersedes part of:** [ADR-0056](0056-one-oracle-per-slice.md)
- **Evidence:** [measurement](../measurement.md#fourth-run--ten-per-cell),
  [round five](../measurement/round5-cell3.md#q2--overflow-is-a-property-of-the-decomposition-and-the-answer-is-not-close)
- **Amended:** 2026-10-15. The re-probe is confirmed and keeps its scope. The second half —
  carrying on past a failure — is now known to be worth very different amounts depending on the
  plan's shape, and is a policy rather than a rule.

## Context
[ADR-0058](0058-slice-boundaries-are-observed.md) established that a coder overflows its slice
and that the runtime should notice: when a slice closes, the pending slices' oracles are run,
and whatever is already green is recorded as delivered early. That covers the case where the
slice *closed*.

The fourth run measured the case where it did not. Of nineteen runs that ended with the whole
suite green — 26 of 26 cases, the complete crate — **eight** held a slice whose own oracle
never passed. The work existed. An adjacent slice's coder had written it on the way past,
which is the same overflow ADR-0058 describes, arriving in the other direction.

Those eight runs were also the expensive ones, and the two groups do not touch:

| | Runs | Minutes | Prompt tokens |
|---|---|---|---|
| Every slice closed | 11 | 1.0 – 6.7, avg **4.5** | 2,869 – 40,619, avg **18,016** |
| A slice never closed | 8 | 13.0 – 23.2, avg **17.5** | 69,512 – 115,277, avg **93,697** |

Both groups ended with the identical crate. This is defect 18.

Two things were wrong, and only one of them is about cost.

**The verdict was wrong.** A run that produced the complete crate reported a failed slice.
Under [ADR-0056](0056-one-oracle-per-slice.md) that failure is evidence the decomposition was
wrong and the planner is consulted again — so a correct plan is thrown away and re-planned on
the strength of a slice whose work the suite already had.

**The walk gave up too early.** The runtime's `walk` stopped at the first slice that did not
finish, on the reasoning that slices may depend on each other so a failure early invalidates
what follows. The reasoning is plausible and the measurement went the other way: carrying on
is what produced the complete crate in eight of nineteen runs. Stopping would have thrown away
work that the next slice was about to do.

## Decision
**A slice that does not close no longer ends the walk, and every slice that did not close is
asked once more when the rest of the plan has run.**

1. **The walk runs every slice in the plan**, in order, whether or not the one before it
   closed.
2. **At the end, each unfinished slice's oracle is run once, with no model.** Green means the
   work was late, not missing. Red means it is missing.
3. **Only what is still red goes back to the planner.** That, and nothing else, is evidence
   the decomposition was wrong.
4. **A run whose slices all either closed or are green at the end has delivered.** The run
   records both numbers, because they are different facts: how many slices closed under their
   own role, and how many were overtaken.
5. **One failure still ends the walk: an unreachable provider.** Nothing after it can succeed,
   and every later slice would spend a dispatch to learn the same thing.

## Alternatives considered
- **Stop at the first failure, as before.** What was measured. It reports failure on a
  complete crate, and in this round it would have done so eight times in nineteen.
- **Ask the downstream oracles while the slice is still running**, and abandon it the moment
  the work appears elsewhere. It cannot fire: at the moment a slice is burning its allowance,
  the later slices have not run yet and their work does not exist. The probe only has an
  answer once the plan has.
- **Treat a green re-probe as a pass for that slice.** Loses the distinction. A slice that was
  overtaken cost a full allowance and delivered nothing, which is a planning signal
  ([ADR-0058](0058-slice-boundaries-are-observed.md) §4); recording it as a pass hides exactly
  the number that would fix the decomposition.
- **Re-dispatch the unfinished slice after the plan.** Pays again for work the re-probe says
  is already there.

## What round five measured (amendment, 2026-10-15)
This ADR was taken on a re-reading of the fourth run, which had not been collected to answer it.
Round five collected it on purpose, across three cells, and split the two halves of the decision
apart.

**The re-probe is confirmed and is cheap in every case.** One oracle run per unfinished slice, no
model. In all three cells, every delivered run had each slice either closed, already green, or
green on the re-probe — and a red re-probe appeared in 14 runs across the round, **none of which
delivered**. A red re-probe is a reliable signal that a run is lost, which is more than this ADR
claimed for it.

**Carrying on is worth what the plan's shape says it is worth.**

| | Slices that did not close | Green on the re-probe |
|---|---:|---:|
| Task C — Rust, slices call each other | 18 | **12 (66%)** |
| Task D — the same plan in Python | 24 | **5 (20%)** |
| Task E — Rust, slices call nothing | 38 | **1 (2%)** |

Slices arriving already green tell the same story: 13, 7 and zero.

So [ADR-0058](0058-slice-boundaries-are-observed.md)'s overflow is a property of the plan rather
than of coders, and the decomposition matters more than the language. Carrying on past a failure
pays out two times in three where the slices sit in each other's line of sight and one time in
fifty where they do not.

**What did not follow.** The obvious adaptive rule — carry on only while this walk has seen a
slice arrive already green — does not survive the data. The rescues come from a *later* slice
writing an *earlier* slice's code, so observed overflow does not predict future rescue, and the
runs that make the strongest case against the rule are the ones it would have killed: runs
closing one slice of four with three rescued delivered the complete crate, and all three
failures came before any overlap had shown itself.

The signal that would decide it is whether a later slice depends on an earlier one, which is a
property of the plan and not of anything a walk can observe.
[ADR-0056](0056-one-oracle-per-slice.md)'s planner could declare it and has never cut up a task.

**So the behaviour is a value, not a rule.** `Carry::Always` is this ADR as measured and stays
the default. `Carry::Never` is what it replaced, kept so the reversal can be re-measured rather
than re-argued. `Carry::UntilOverlap` is the hypothesis, carrying the case against it in its own
doc comment. [Round six's Q17](../measurement/round6.md) runs `Never` against `Always` on Task E,
where carrying on is worth 2% and stopping should therefore cost nothing.

## Consequences
- A plan with a genuinely dependent chain now spends the later slices' allowances on work that
  cannot succeed. [ADR-0062](0062-step-allowance-has-a-floor-not-a-ceiling.md) bounds that
  cost: each wasted slice costs a floor, not a ceiling. The two decisions are taken together
  and should be measured together.
- **Overtaking becomes a counted quantity per run.** A planner producing slices that cost a
  full allowance and deliver nothing is now visible as a number rather than as an impression.
- The planner is consulted less often, and on better evidence when it is.
- The re-probe costs one oracle run per unfinished slice and no model at all. It is the
  cheapest question in the runtime.
- `all_done` and `delivered` are now different questions and the run answers both. A caller
  that wants "is the work there" must ask the second one.
