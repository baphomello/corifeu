# Round five, cell 2 — Task D, `rank`, the defaults

Task C ported to Python, function for function and case for case, so that the only difference
between this cell and [cell 1](round5-cell1.md) is the language.

- **Cell:** `BENCH_TASK=d-glob-py bench/run.sh rank 10 qwopus3.5-9b-coder-Q4_K_M ornith1.5-9b-Q5_K_M`.
- **Commit:** `28af2b4`. **Ran:** 2026-10-15 06:27 to 11:34, 307 minutes, 20 iterations.
- **Oracle, proved before the round:** `26 failed in 0.91s`.
- Every number here is printed by `bench/analyse.py`.

## Q3 — the floor of twelve is a Rust number, and the growth is what saves it

ADR-0062's floor is twelve because the slowest of the fourth run's 57 green slices took twelve
steps. Every one of those slices was Rust, where much of the work is done by compiler errors.

The mean barely moves. The tail is a different story.

| Green slices | Rust (C) | Python (D) |
|---|---:|---:|
| Mean steps | 1.8 | 2.3 |
| Closed in one step | 33 of 49 | 29 of 49 |
| **Slowest** | **7** | **13** |

In Rust the floor of twelve had five steps of headroom over the slowest green slice. In Python it
is the binding constraint, and the three slowest green slices say so exactly:

| Iteration | Model | Slice | Steps | Allowance |
|---|---|---|---:|---:|
| 2 | `qwopus` | `slice3_glob` | 9 | 12 |
| 8 | `ornith` | `slice3_glob` | **12** | 12 |
| 10 | `ornith` | `slice4_classify` | **13** | **18** |

One slice closed on its last permitted step. The next closed at thirteen, **which it only had
because the allowance had grown** — `floor_left` was true in 4 of 20 Python iterations against 2
of 20 in Rust.

A flat ceiling of twelve would have lost that slice. [round5.md](round5.md) registered the
remedy for a floor that does not travel as "`limits.steps.floor` needs a documented value per
language". The data says something better: it does not, because
[ADR-0062](../adr/0062-step-allowance-has-a-floor-not-a-ceiling.md) chose a floor that grows
over a ceiling that is guessed, and that choice is what absorbed a language it was never
measured on. The mechanism is doing the job the constant could not.

## The crate at the end — the language costs more than the floor

| | Rust (C) | Python (D) |
|---|---:|---:|
| Complete crate, 26/26 | **17 of 20** | **9 of 20** |
| Slices reaching green at all | 62 of 80 | 56 of 80 |
| Slices looping | 4 | 9 |

The floor travels; the delivery rate does not. Nine of twenty against seventeen of twenty is the
largest single difference in the round, on a task that is a line-for-line port.

Two things in the trace point at why, and neither is the allowance.

**The model rambles more without a compiler.** Steps cut off at `num_predict` before calling a
tool: 32% in Python against 24% in Rust. Productive steps: 33% against 36%. A compiler error is
short, specific and impossible to argue with; a pytest assertion failure invites prose.

**There is less accidental help.** Slices that were already green when first probed, plus slices
green on the end-of-plan re-probe: 25 in Rust, 12 in Python. A Rust coder reading a file of
`todo!()` stubs writes more of them than a Python coder reading `raise NotImplementedError`.

## Q6 — asking again is a Rust property

This reverses between the two languages, and it is the sharpest finding in the cell after Q3.

| The slices that did not close | Rust (C) | Python (D) |
|---|---:|---:|
| Green on the re-probe — late, not missing | **12 of 18 (67%)** | **5 of 24 (21%)** |
| Red — the work is actually absent | 6 | 19 |

[ADR-0063](../adr/0063-an-unfinished-slice-is-asked-again.md) rests on a slice that did not close
usually being green once the rest of the plan has run. In Rust it is, two times in three. In
Python it is not, four times in five.

The re-probe itself costs one oracle run and no model, so it stays worth doing in both. What
changes is the second half of ADR-0063: carrying on past a failure, on the theory that a later
slice will rescue the work. In Python that theory pays out 21% of the time, and the later slices
spend their allowances to find out.

In both languages, every delivered run had each slice closed, already green, or green on the
re-probe. A red re-probe appeared in 3 Rust runs and 11 Python runs, and **not one of those
fourteen runs delivered**. A red re-probe is a reliable signal that the run is lost.

## Q10 — the repeat warning rescues 2 of 29, across both cells

Cell 1 found no warned slice reaching green at all. Python found two.

| | Rust (C) | Python (D) | Both |
|---|---:|---:|---:|
| Slices that saw a `REPEAT` | 17 | 12 | 29 |
| Of those, reached green | 0 | 2 | **2** |
| Steps spent after the first repeat | 111 (38%) | 64 (31%) | 175 |

So the cell 1 page overstated it: a warned model does sometimes recover. It recovers **7% of the
time**, and the five steps [ADR-0050](../adr/0050-loop-detection-warns-before-it-stops.md) buys
between the first repeat and the sixth cost a third of every cell's steps.

## Q9 — the rollback has now fired once in 472 steps

Zero firings in this cell, one in cell 1. Across both: **1 in 472 steps, over 40 iterations.**
ADR-0051's protection remains untested rather than proven.

## Q11 and Q12 — the window is fine in Python too

| Outcome | Slices | Steps per slice | Seconds per step | Prompt per step |
|---|---:|---:|---:|---:|
| Green | 49 | 2.3 | 47.3 | 3,024 |
| Exhausted | 15 | 12.0 | 45.7 | 3,246 |
| Looping | 9 | 9.2 | 36.8 | 2,854 |

Prompt per step is flat across outcomes, as in Rust. The expensive runs are long, not heavy.

Seconds per step no longer run backwards: without a compile, a green step costs about what a
failing one costs. That confirms cell 1's reading — the Rust gap was the compile and the test
run landing on the one step that closes the slice.

## Q7 — a model difference appears that Rust did not show

| Model | Slice 1 | Slice 2 | Slice 3 | Slice 4 | Total |
|---|---:|---:|---:|---:|---:|
| `qwopus3.5-9b-coder` | 9/10 | 9/10 | 8/10 | 8/10 | **34/40** |
| `ornith1.5-9b` | 5/10 | 8/10 | 4/10 | 5/10 | **22/40** |

In Rust the two were 33 and 29 and the within-model variance swallowed the gap. Here it is 34
against 22, which is large enough to survive it: `ornith` is materially worse at this task in
Python while being `qwopus`'s equal in Rust.

That is still not a clean answer, because no sampling seed is set anywhere (defect 26), so a
replication would not reproduce these runs. It is the first result in five rounds where the two
models are not interchangeable.

## What this cell does not say

- Nothing about a decomposition whose slices do not call each other. That is cell 3.
- Nothing about ADR-0053 or the pair. The held-out judge is still Rust-only.
- Nothing about why the Python delivery rate is half the Rust one beyond the two pointers above.
  Both are readings of the trace, not controlled comparisons.
