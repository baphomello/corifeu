# Round seven — TypeScript, and whether a round can be replayed

Eight passes of ten iterations against `g-glob-ts`, the TypeScript port of task C. 80
iterations, 320 slice-runs, **99 steps in total**. This is stage S3 of
[roadmap-site.md](../roadmap-site.md), the stage written to be able to say the plan was wrong.

It did not say that.

- **Model:** `qwen3.5-9b` — `unsloth/Qwen3.5-9B-MTP-GGUF:UD-Q4_K_XL`, one model, the only one
  on this machine since the model set was cleared.
- **Server:** 32,768 of context, `q8_0` cache, `-ngl 99`, MTP on except where stated.
  48k is the measured ceiling but 32k was used, because 48k leaves 131 MiB free when the
  desktop is busy and a round of hours cannot survive that.
- **Commit:** `efc61ca`.
- Every number here is printed by [`bench/analyse.py`](../../bench/analyse.py), including the
  replay matrix, which was added for this round because a claim about two rounds cannot be read
  off one cell's report.

## The result the stage existed to get

| Pass | Sampling | MTP | Whole task delivered | Steps | Minutes |
|---|---|---|---|---:|---:|
| A | greedy | on | 10/10 | 10 | 3.0 |
| B | greedy | on | 10/10 | 10 | 3.0 |
| C | temp 0.6 | on | 10/10 | 12 | 4.0 |
| D | temp 0.6 | on | 10/10 | 13 | 4.2 |
| G | temp 0.6 | on | 10/10 | 13 | 4.2 |
| H | temp 0.6 | on | 10/10 | 13 | 4.1 |
| E | temp 0.6 | **off** | 10/10 | 14 | 9.6 |
| F | temp 0.6 | **off** | 10/10 | 14 | 9.6 |

**80 of 80 iterations delivered the whole task.** Every one ended `closed=4/4` and `26/26`.

**Q12 — the share of steps that do work: 100%, against Rust's 36%.** Not one step in 99 was
cut off at `num_predict`, blocked as a repeat, or spent without calling a tool. The
distribution round five spent three cells measuring does not exist here, because nothing went
wrong often enough to have a distribution.

The mechanism is the finding. Asked for `splitSegs` alone — four lines — the model wrote the
**whole file**, all four functions, 8,001 bytes, in one `write_file`. The remaining three
slices then arrived at an oracle that was already green and were recorded
`DELIVERED_EARLY`. A typical iteration is one step and 19 seconds.

Against the same decomposition in Rust, same `rank` mode, from
[round five's first cell](round5-cell1.md): **17 complete crates out of 20 iterations, in 289
steps** — 22.2 steps per iteration against this round's 1.3, and 235 minutes against 4.

## What this does and does not say

`scaffold.py --mode slices` removes every implementation and **keeps every test**, so the 26
cases were in the artefact the model was shown. `26/26` means it satisfied cases it could
read. There is no held-out suite in this mode — `scaffold.py` separates one only for
`--mode pair` — so nothing here judges the code against cases it never saw.

The comparison with Rust survives that, because round five's Rust cell had the same visibility
and still failed. What does not survive is any claim about generalisation.

**The task is now too easy to measure cost with.** One step per iteration means this cell
cannot say anything about repeats, rollback, the step allowance or the floor on TypeScript. A
harder TypeScript task is needed for that, and it is a different task rather than a longer
round.

## Q14 — a round can be replayed, and once in four it could not

Every pair below ran the same seed base, so iteration *n* drew seed `1000 + n` both times.

| Compared | Agreement, outcome and step count |
|---|---:|
| A against B — greedy | 40/40 |
| E against F — temp 0.6, no MTP | 40/40 |
| D, G, H against each other — temp 0.6, MTP, **across a server restart** | 40/40 |
| C against D, G, H — temp 0.6, MTP | 39/40 |

**A greedy pair proves nothing.** A and B agreed 40/40, and within each pass all ten
iterations produced the *same output to the token* despite ten different seeds. That is
greedy decoding ignoring the sampler, not a round being replayed. Every replay claim below is
from the sampled passes.

**D, G and H are the evidence.** Three passes, two different server processes, 40/40 on
outcome and step count. A round is replayable on this stack.

**C is the exception, and one anomaly is not a cause.** C diverged from the other three at a
single slice-run — iteration 3, where the first step produced a different file and `slice2_seg`
then needed a step of its own instead of arriving green. One divergence in 160 MTP slice-runs.

The first hypothesis was speculative decoding, because MTP changes batch composition and the
reduction order inside a batch changes a logit at the margin. **The measurement argues against
it:** if MTP made decoding nondeterministic, D, G and H would not agree across a restart, and
they do. Two further hypotheses were considered and neither is established — accumulated
server state, which fails because D ran fourth on its instance and agreed with G which ran
first on a fresh one; and prompt-cache reuse, which predicts the divergence in the multi-step
iterations and iteration 7 is multi-step in every pass and never diverged.

So the record says: **159 of 160 sampled MTP slice-runs replay, the one that did not has no
confirmed mechanism, and `E`/`F` is too small a sample to claim MTP-off never diverges.**
[ADR-0066](../adr/0066-a-round-is-replayable-or-it-is-an-anecdote.md) is satisfied with that caveat
attached, not with a guarantee.

## MTP is worth 2.3 times the wall clock on a round

| | Minutes for ten iterations |
|---|---:|
| MTP on | 4.0 – 4.2 |
| MTP off | 9.6 |

The probe measured ~100 tok/s against ~57, which is 1.75×. A round gains more than generation
alone, and this is the first time that has been measured on a round rather than on one answer.
It also costs nothing in delivery: both configurations delivered 10 of 10.
