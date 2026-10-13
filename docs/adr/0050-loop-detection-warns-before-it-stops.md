# 0050 — Loop detection warns before it stops

- **Status:** Accepted
- **Date:** 2026-10-13
- **Amends:** [ADR-0018](0018-loop-and-budget-control.md)
- **Evidence:** [measurement](../measurement.md)

## Context
[ADR-0018](0018-loop-and-budget-control.md) stops an agent that repeats itself. The
measurement showed the rule was right and the response was wrong.

**Killing on the first repeat discards nearly-finished work.** A doer holding 18 of 21 tests
green re-ran `cargo test` after an inspection step. That matched an action from four steps
earlier, and the run was terminated at **step 12 of a 400-step budget**. Nothing was wrong
with the work; the run was ended because of one redundant command.

**Agents recover from dead ends if allowed to.** The run that eventually reached 21/21 green
hit **four consecutive repeated actions twice** — at steps 28–31 and again at 50–53 — and
finished successfully at step 57. Under a one-strike rule it would have died at step 26.

**The warning text decides what the agent does next.** The first version said:

> Do not run it again. Change the code to fix a failing test, or inspect a part of the file
> you have not read yet.

A stuck agent took the second clause and read. It spent 5 steps and 291 output tokens in a
pure read loop. The warning was telling it to do the thing that was not working.

## Decision
**A repeated action produces a warning delivered to the agent as a tool result. Only
consecutive warnings stop the run.**

1. **Strikes, not a trigger.** `loop.strikes` consecutive repeats end the run; the measured
   working value is 6. A non-repeated action resets the count to zero.
2. **The warning is delivered in-band**, as the result of the call the agent just made, so it
   arrives where the agent is already looking.
3. **The warning points at the oracle, never at more reading.** The agent is told the state
   has not changed, that re-reading is futile because the artefact is already pinned
   ([ADR-0047](0047-bounded-agent-context-within-a-task.md)), and that only running a check
   or writing a change can produce new information.
4. **Changing an artefact clears the history.** The same command against changed code is a
   different action. This is [ADR-0018](0018-loop-and-budget-control.md)'s existing rule and
   it stands.

## Alternatives considered
- **Stop on the first repeat.** What was measured. It ended a run holding 18/21 with 97% of
  the budget unspent.
- **Never stop on repetition, rely on the step ceiling.** A genuinely looping agent then burns
  the entire budget. Task C's doer repeated 24 times across its run; without strikes it would
  have spent 400 steps doing it.
- **Stop only on the "no artefact changed in n steps" signal.** Slower to fire and catches a
  different failure. Both signals are kept; they detect different things.
- **Let a model judge whether the agent is looping.** Adds a model call per step to replace a
  comparison, and introduces an opinion where there is currently a fact.

## Consequences
- `loop.strikes` and `loop.window` are configuration. A patient local run and a metered
  remote one want different values.
- Warning text is product surface. It is read by a model in difficulty, which is the worst
  moment for an ambiguous instruction, and it should be revised whenever a measurement shows
  an agent taking the wrong branch of it.
- The run record distinguishes *stopped while looping* from *stopped making no progress* from
  *budget exhausted*. These were one outcome in the measurement and they are three different
  diagnoses.
