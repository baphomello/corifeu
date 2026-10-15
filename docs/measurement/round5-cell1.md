# Round five, cell 1 — Task C, `rank`, the defaults

Written while the round is still running, because the answers below are complete and the cell
they came from is closed. Cells 2 to 7 are separate questions and are not in here.

- **Cell:** `bench/run.sh rank 10 qwopus3.5-9b-coder-Q4_K_M ornith1.5-9b-Q5_K_M`, Task C, the
  runtime's own step allowance.
- **Commit:** `28af2b4`. **Ran:** 2026-10-15 02:31 to 06:27, 235 minutes, 20 iterations.
- **Oracle, proved before the round:** `test result: FAILED. 0 passed; 26 failed`.
- Raw rows: `results.tsv` and, for the first time, `steps.tsv` — one row per step.
- Every number below is printed by `bench/analyse.py`, which exists because the first draft of
  this page carried two numbers typed by hand and both were wrong.

This cell exists to be the baseline the other `rank` cells are compared against, and it is the
first measurement of [ADR-0062](../adr/0062-step-allowance-has-a-floor-not-a-ceiling.md) and
[ADR-0063](../adr/0063-an-unfinished-slice-is-asked-again.md) in the runtime.

## The crate at the end, twenty times

| | Count |
|---|---:|
| Complete crate, 26/26 | **17** |
| 25 of 26 | 1 |
| 17 of 26 | 1 |
| Did not build | 1 |

The fourth run delivered 19 of 20 under a flat 30-step ceiling. 17 of 20 under a floor of 12 is
not a distinguishable difference at this sample size, and the three decisions taken since are
not refuted by it.

Of the 80 slice runs: **49 closed** under their own oracle, **13 were already green** when the
slice was first probed, **12 were green on the end-of-plan re-probe**, and **6 were genuinely
missing**. Those six sit in three runs, and all three are runs that did not deliver.

**Every one of the 17 delivered runs had each slice either closed, already green, or green on
the re-probe.** No delivered run finished holding work that was simply absent.

| Slice | Green | Delivered early | Exhausted | Looping |
|---|---:|---:|---:|---:|
| 1 `split_segs` | 16 | 0 | 3 | 1 |
| 2 `match_seg` | 10 | 3 | 4 | 3 |
| 3 `glob_match` | 12 | 3 | 5 | 0 |
| 4 `classify` | 11 | 7 | 2 | 0 |

## Q6 — asking again is right two times in three

[ADR-0063](../adr/0063-an-unfinished-slice-is-asked-again.md) claims that a slice which did not
close is often green once the rest of the plan has run, and that carrying on is cheaper than
re-planning. The first half is now measured on data collected for the purpose rather than
re-read from a round that was not asking.

| The 18 slices that did not close | |
|---|---:|
| Green on the re-probe — late, not missing | **12** |
| Red — the work is actually absent | 6 |

Two thirds were late. [round5.md](round5.md) registered the falsification as "`AGAIN` rows
coming back `RED` most of the time"; they did not.

The six red ones concentrate in three runs, and those are exactly the three runs that failed to
deliver. So a red re-probe is not noise — it is the signal that a run is in trouble, and under
ADR-0056 each of those would have gone back to the planner on the strength of a slice whose work
was, in two cases out of three, already written.

## Q8 — two steps in three produce nothing

289 steps, read from `steps.tsv`.

| What the step did | Steps | |
|---|---:|---:|
| Ran a tool that changed something | 105 | 36% |
| Cut off at `num_predict` before calling a tool | 71 | 25% |
| Blocked as a repeat, never run | 62 | 21% |
| Answered without calling a tool | 50 | 17% |
| Rolled back | 1 | 0.3% |

**37% of steps do work and 63% do not.** ADR-0062's floor of twelve was derived from step counts
that include this waste, so an allowance of twelve steps is in practice an allowance of four or
five useful ones.

Whether this is new is **not knowable**: the fourth run recorded no trace. It delivered the same
crate in 17.4 steps per iteration against this cell's 22.2, so a similar proportion of waste
would put it at about nine useful steps against this cell's eight. The honest reading is that the
waste was always there and has never been visible, not that it appeared with this round.

`num_predict` is 4096 and 71 steps hit it. That is the single largest category of waste and it is
a configuration number, not a model limitation.

## Q10 — a repeat is not a warning, it is a verdict

[ADR-0050](../adr/0050-loop-detection-warns-before-it-stops.md) warns on a repeated action and
stops after six, reversing defect 1 which stopped at the first. The reversal has never been
measured. It is now.

| | Slices | Reached green |
|---|---:|---:|
| Saw a `REPEAT` | 17 | **0** |
| Never saw one | 63 | 62 |

**Not one of the seventeen slices that repeated an action ever went green.** Thirteen exhausted
the allowance and four were stopped as loops. Of the sixty-three that never repeated, sixty-two
succeeded.

The price: **111 of the cell's 289 steps — 38% — were spent after a first repeat**, and they
bought nothing.

This is the sharpest separation in the round and it reverses ADR-0050's trade. The five steps
between the first repeat and the sixth were bought on the theory that a warned model changes
course; at n=20 on this task, none did.

One caution on the mechanism. A repeat is plausibly a *symptom* of a slice already failing rather
than a cause of it, so stopping at the first one would not have turned these seventeen green — it
would have stopped paying for them. That is still the right move, and it is a different claim
from "the warning is harmful".

## Q9 — ADR-0051's rollback fired once in 289 steps

Checkpoint before every write, score by cases passing, restore on regression. **One firing, in
twenty iterations.**

As [round5.md](round5.md) registered beforehand: near-zero firings mean the protection is
untested rather than proven, and that should be said out loud rather than assumed. It is
possible the mechanism is load-bearing on work larger than a four-slice single file. Nothing
here shows it.

## Q11 and Q12 — the expensive runs are long, not slow, and the window is not at fault

| Outcome | Slices | Steps per slice | Seconds per step | Prompt per step |
|---|---:|---:|---:|---:|
| Green | 49 | 1.8 | 56.1 | 3,158 |
| Exhausted | 14 | 12.4 | 36.9 | 3,588 |
| Looping | 4 | 8.0 | 40.8 | 3,166 |

**Prompt tokens per step are flat** — 3,158 against 3,588. Q12's registered falsification: flat
means the expensive runs are expensive only because they are long, and ADR-0062 already addresses
that. The window is holding its budget.

**Seconds per step run the other way from the intuition.** A green step is *slower* than an
exhausted one, 56 seconds against 37, because a green slice is about two steps and one of them
carries the compile and the test run. A failing slice spends its steps talking, which is cheap.
So the one-hour cap is not cutting runs that were about to finish; Q11's registered warning sign,
seconds-per-step rising within a run, is not what the data shows.

## Q7 — unanswerable as registered, and that is a harness defect

`scaffold.py` is deterministic, so iteration *i* hands both models an identical starting state.
Of the forty (iteration, slice) pairs, both models reached 24, neither reached 2, and **exactly
one reached 14**.

That looks like hardness being model-specific. It is not safe to read it that way, because the
same model on the same slice does not agree with itself:

| Model | Slice 1 | Slice 2 | Slice 3 | Slice 4 |
|---|---:|---:|---:|---:|
| `qwopus3.5-9b-coder` | 7/10 | 7/10 | 9/10 | 10/10 |
| `ornith1.5-9b` | 9/10 | 6/10 | 6/10 | 8/10 |

With that much variance within one model, most of the fourteen is a draw rather than a
difference. **No sampling seed is set anywhere in the harness**, so the paired design controls
the starting state and not the sampling, and Q7 cannot separate a model's weakness from a bad
draw. Defect 26. `Budget` carries no seed field, so fixing it is a runtime change and is not
being made during this round.

Totals, for what they are worth: `qwopus` reached 33 of 40, `ornith` 29 of 40.

## What this cell does not say

- Nothing about another language. That is cell 2.
- Nothing about whether the floor of twelve is right, only that it was never raised:
  `floor_left` was false in every iteration, so patience never ran out twice in a row.
- Nothing about ADR-0053. That is cells 4 and 5.
