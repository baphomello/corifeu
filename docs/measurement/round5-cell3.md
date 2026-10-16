# Round five, cell 3 — Task E, `rank`, the defaults

Four conversions that have nothing to do with each other: `roman`, `wrap`, `base32`, `luhn`. No
call between them, no shared helper, no shared type. The control for Task C, whose four slices
are a chain.

- **Cell:** `BENCH_TASK=e-indep bench/run.sh rank 10 qwopus3.5-9b-coder-Q4_K_M ornith1.5-9b-Q5_K_M`.
- **Commit:** `28af2b4`. **Ran:** 2026-10-15 11:34 to 19:41.
- **Oracle, proved before the round:** all 26 cases red.
- **Stopped at 16 iterations, eight per model, not the twenty the plan registered.** The machine
  was wanted for other work and the result below was already stable at twelve. That is a
  deviation from the pre-registered sample size, decided after seeing the data, and it is stated
  here rather than in a footnote. One `qwopus` iteration lost a slice to a provider failure, so
  a few of its per-slice counts are out of seven.

## Q2 — overflow is a property of the decomposition, and the answer is not close

[ADR-0058](../adr/0058-slice-boundaries-are-observed.md) says a coder writes more than its slice
asked for and the runtime should notice. Every observation behind it came from Task C, where
`classify` calls `glob_match` calls `match_seg` calls `split_segs`, so a coder implementing one
slice has the next one in front of it.

| | Already green when first probed | Green on the re-probe | Overtaken |
|---|---:|---:|---:|
| **C** — Rust, chained | 13 | **12 of 18 (66%)** | 12 |
| **D** — Python, chained | 7 | **5 of 24 (20%)** | 5 |
| **E** — Rust, independent | **0** | **1 of 38 (2%)** | 1 |

[round5.md](round5.md) registered the falsification as "overflow rate near zero on Task E while
Task C keeps producing it". It is 2% against 66%.

So **both the decomposition and the language matter, and the decomposition matters more**:
66% chained Rust, 20% chained Python, 2% independent Rust. ADR-0058's overflow is not a
property of coders. It is a property of a plan whose slices sit in each other's line of sight.

**The confound, stated.** Task E is also harder: `base32` reached green once in sixteen and
`luhn` twice. Where nothing succeeds there is nothing to overflow from. The partial control is
that the first two slices did succeed at a rate comparable to Task C's — `roman` 10 of 16 and
`wrap` 10 of 16 — and **no later slice was ever found already done**. Not once in sixteen runs.

| Slice | Green | Exhausted | Looping |
|---|---:|---:|---:|
| 1 `roman` | 10 | 5 | 0 |
| 2 `wrap` | 10 | 5 | 0 |
| 3 `base32` | 1 | 14 | 0 |
| 4 `luhn` | 2 | 12 | 1 |

No run produced a complete crate. Five did not build at all.

## The round's three cells together

56 iterations, 1,076 steps.

| | Iterations | Steps | Productive | Cut off at `num_predict` | Blocked repeats | No tool call |
|---|---:|---:|---:|---:|---:|---:|
| C | 20 | 289 | 106 | 71 | 62 | 50 |
| D | 20 | 183 | 61 | 59 | 43 | 20 |
| E | 16 | 604 | 229 | 223 | 68 | 84 |
| **All** | **56** | **1,076** | **396 (36%)** | **353 (32%)** | **173 (16%)** | **154 (14%)** |

**Q8, across the round: 36% of steps do work.** The largest single category of waste is a turn
cut off at `num_predict` before it called a tool — 32% of every step taken, in a configuration
number rather than a model limitation.

**Q10, across the round: 54 slices repeated an action and 2 reached green.** 3.7%.

| | Warned | Rescued | Steps after the first repeat |
|---|---:|---:|---:|
| C | 17 | 0 | 111 (38% of the cell) |
| D | 12 | 2 | 64 (34%) |
| E | 25 | 0 | 147 (24%) |
| **All** | **54** | **2** | **322 (30%)** |

[ADR-0050](../adr/0050-loop-detection-warns-before-it-stops.md) buys five steps between the
first repeat and the sixth on the theory that a warned model changes course. Across three cells
and two languages it changes course 3.7% of the time, and those steps are **30% of everything
the round spent**.

**Q9, across the round: ADR-0051's rollback fired 4 times in 1,076 steps.** Three of the four
were in this cell, which is the first sign the mechanism does something — Task E's slices are
each a whole algorithm, so there is more to regress.

**Q11 and Q12 hold in all three cells.** Prompt per step is flat across outcomes here too,
3,790 green against 3,853 exhausted. The window holds its budget and the expensive runs are
long rather than heavy.

## Q7 — the models invert between tasks, and the seed is still missing

| Model | `roman` | `wrap` | `base32` | `luhn` |
|---|---:|---:|---:|---:|
| `ornith1.5-9b` | 7/8 | 6/8 | 1/8 | 0/8 |
| `qwopus3.5-9b-coder` | 3/8 | 4/7 | 0/7 | 2/7 |

`ornith` is twice `qwopus` on `roman` and `wrap`; `qwopus` is the only one to pass `luhn`. In
cell 2 `qwopus` was ahead almost everywhere. Model strength appears to be task-specific rather
than general, which would be worth knowing — and **it cannot be claimed from this data**,
because no sampling seed is set and none of these runs can be replayed. Defect 26.

## What the round did not answer

- **Q1 and Q4** — ADR-0053 and the pair. Cells 4 and 5 were not run.
- **Q5** — the floor sweep. Cells 6 and 7 were not run. Cell 2 re-aimed it to Python.
- **Q7** — blocked by defect 26, not by missing cells.
