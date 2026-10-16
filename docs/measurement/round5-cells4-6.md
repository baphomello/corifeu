# Round five, cells 4 to 6 — the pair, and half the floor sweep

Three cells ran against the frozen runtime in a separate worktree at commit `b5fd0eb`, while
the decisions taken on cells 1 to 3 were implemented on `main`. Cell 7 was not run.

| Cell | What | Ran | Answers |
|---|---|---|---|
| 4 | `c-glob` pair, slice 1 | 60 min, 20 runs | Q1 |
| 5 | `c-glob` pair, slice 3 | 353 min, 20 runs | Q4 |
| 6 | `d-glob-py` rank, `floor=8` | 277 min, 20 runs | Q5, two legs of three |

Every number here is printed by `bench/analyse.py`.

## Q1 — `SelfTested` code is not worse. Not in a single run.

[ADR-0053](../adr/0053-qa-writes-the-tests.md) separates QA from the coder so that the test and
the code come from two readings of a specification. The fourth run found QA writing the
implementation as well as the test in 11 of 20 pairs, and could not say whether that mattered.
Twelve cases no run is ever shown can.

| Outcome | Runs | Crates judged | Held-out | Perfect |
|---|---:|---:|---:|---:|
| `GREEN` — two readings | 6 | 6 | **12.0 / 12** | 6 of 6 |
| `SELF_TESTED` — one reading | 11 | 11 | **12.0 / 12** | **11 of 11** |
| `NOT_PASSED` | 1 | 1 | 9 / 12 | 0 |
| `NO_RUNNABLE_TEST` | 2 | 0 | — | — |

**Seventeen of seventeen judged crates scored full marks**, whoever wrote what.
[round5.md](round5.md) registered the falsification in advance: *"the two groups score the same.
Then ADR-0053's separation is buying nothing measurable on a slice this size, and the honest
move is ADR-0053's third option: accept one doer with two contracts."* That is the result.

The 11 of 20 `SELF_TESTED` also **reproduces the fourth run exactly**, which is the first thing
in six rounds to replicate on the nose.

**The trap, also registered in advance, has to be stated with the finding.** Twelve cases
against a four-line `split_segs` may be a ceiling rather than a measurement: when everything
scores 12 of 12, "the separation does not matter" and "the suite does not discriminate" look
identical. Cell 5 shows the suite *can* discriminate when the work is harder, which makes the
ceiling reading more likely rather than less.

## Q4 — on the expensive slice the pair does not merely self-test, it collapses

| | Slice 1 — four lines | Slice 3 — recursive |
|---|---:|---:|
| `GREEN` | 6 | **0** |
| `SELF_TESTED` | 11 | **0** |
| `NOT_PASSED` | 1 | 12 |
| `NO_RUNNABLE_TEST` | 2 | 8 |
| Crates that built | 17 of 20 | 11 of 20 |
| Held-out | 12.0 / 12 | **9.5 / 13**, 2 of 11 perfect |
| Wall clock | 60 min | **353 min** |

**No run closed slice 3.** Eight could not even produce a test that runs.

round5.md expected `SELF_TESTED` to collapse and read that as QA stopping short of the
implementation when the implementation is expensive. `SELF_TESTED` did collapse — and `GREEN`
collapsed with it, so the explanation is not about who wrote what. The pair cannot do this slice
at all.

What the data does support: **the pair's behaviour has to be stated per slice size.** Two rounds
measured it on a four-line function and that number does not travel. The scores also spread from
4 to 13 where slice 1's were all 12, so the held-out suite discriminates once there is something
to discriminate — which is the evidence behind Q1's caveat.

## Q5 — lowering the floor to eight cost nothing measurable

Re-aimed to Task D after cell 2, [with the reason recorded](round5.md), because Task C's slowest
green slice took 7 steps and a sweep around 12 would have moved nothing there.

| | Delivered | Total steps | Green slices | Mean steps per green | Slowest green | Allowance grew |
|---|---:|---:|---:|---:|---:|---:|
| `floor = 12` (cell 2) | 9 of 20 | 376 | 49 | 2.3 | **13** | 4 of 20 |
| `floor = 8` (cell 6) | **9 of 20** | 369 | 44 | 2.0 | **7** | 6 of 20 |

Same delivery, same steps within 2%. The floor came down by a third and nothing happened —
because [ADR-0062](../adr/0062-step-allowance-has-a-floor-not-a-ceiling.md)'s growth absorbed
it: the allowance had to grow in 6 of 20 iterations at floor 8 against 4 of 20 at floor 12.

round5.md registered this as *"floor 8 losing nothing that floor 12 kept — then twelve is
generous and the floor should come down."* It lost nothing. But the honest reading is narrower
than that sentence, because the slowest green slice fell from 13 steps to 7: at floor 8 the
slices that needed more steps **stopped being green** rather than taking longer. Delivery held
because the re-probe and the next slice picked them up.

So what the two legs show is that **the floor matters less than the mechanism around it**. A
third leg at `floor = 30` would say whether a generous floor buys anything at all; it was not
run.

## What these cells do not say

- **Nothing about `floor = 30`.** Cell 7 was not run. The comparison it would complete is also
  the one most exposed to defect 31: the GPU driver changed under this round, and a leg run
  later would straddle it.
- **Nothing about the pair outside Rust.** The held-out judge still appends to `src/lib.rs` and
  runs `cargo test`.
- **Nothing replicable.** These cells ran before [ADR-0066](../adr/0066-a-round-is-replayable-or-it-is-an-anecdote.md)'s
  seed existed, on the frozen runtime. Defect 26 applies to all of them.
