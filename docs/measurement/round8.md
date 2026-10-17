# Round eight — the floor sweep that could not be run

Three legs of [ADR-0062](../adr/0062-step-allowance-has-a-floor-not-a-ceiling.md)'s step
allowance against [`f-expr`](../../bench/tasks/f-expr), the task built so that a slice can
regress. 30 iterations, 60 slice runs, 298 steps.

[Round six](round6.md) left Q5 needing all three legs on one driver, in one sitting, and this
is that. The answer is not about the floor.

- **Model:** `qwen3.5-9b` — `unsloth/Qwen3.5-9B-MTP-GGUF:UD-Q4_K_XL`, 32,768 of context,
  `q8_0` cache, MTP on.
- **Seed base 1 on every leg**, `BENCH_TEMP=0`. Only `BENCH_STEPS_FLOOR` changed.
- Numbers from [`bench/analyse.py`](../../bench/analyse.py).

## The three legs

| Floor | Complete crates | Steps | Productive | Rollbacks |
|---|---:|---:|---:|---:|
| 8 | **8 / 10** | 74 | 84% | 0 |
| 12 | **5 / 10** | 111 | 54% | 0 |
| 30 | **8 / 10** | 113 | 56% | 0 |

Eight, five, eight. A larger allowance delivering *less* and then more again is not a dose
response, and the next table says why.

## Q5 is not measurable on this stack, and this is the evidence

`slice1_tokens` takes at most six steps. Against allowances of 8, 12 and 30 **the floor cannot
touch it**, so the three legs have to agree about it or something other than the floor is
moving. The step counts, iteration by iteration:

```
floor 8 : 2 4 6 4 2 2 2 2 2 2
floor 12: 2 2 4 2 2 2 2 2 2 2
floor 30: 4 2 2 2 6 3 2 2 2 3
```

Three different sequences, at one seed base, on a slice no allowance reached. Iteration 1's
`slice2_eval` makes the same point from the other side: green in 3 steps at floor 8, and
exhausted at 12 with floor 12. That is not more steps failing to help — it is a different run.

**So the legs are not comparable, and the difference between them is not the floor.** Reporting
8 against 5 as a floor effect would be publishing noise as a finding. Q5 stays open with its
question changed: it cannot be answered until two runs at one seed reproduce.

**This corrects [round seven](round7.md).** That round measured 159 of 160 sampled slice runs
replaying exactly and concluded a round is replayable on this stack. Every slice in it took
**one step**. The conclusion does not survive a trace of two to twelve steps, where one
different token early changes every prompt after it. The claim should have been scoped to the
trace length that produced it.

> **Corrected again by [round nine](round9.md).** The reading above — that the stack cannot
> replay a long trace — is wrong in the other direction. It could not replay because Corifeu
> put four volatile things into its own prompts, and with them removed two passes are identical
> byte for byte. Every delivery number on this page was measured with that contamination
> present, so the three legs below are three samples of a noisy process and the floor still has
> no measurement either way.

## ADR-0062's growth is unreachable at this task shape

The allowance never grew: all 60 slice runs ran at the floor they started with, against
round five's 6 of 20 at floor 8.

Not broken — unreachable. `patience` counts **consecutive** cuts and `Outcome::Reached` puts
the count back, and `f-expr` has two slices of which the first is always green. The sequence is
always *reached, exhausted, end of iteration*, so `cuts` never gets to two.

The consequence is worth more than the observation: **ADR-0062's growth is built for a model
that fails repeatedly, and a model that fails occasionally never collects it.** The floor then
stops being a starting point and becomes a hard limit, which makes it matter *more* than when
round five measured it, not less. Pinned as a test —
`a_success_between_two_cuts_means_patience_never_runs_out`.

## Q18: the rollback still has not fired

| | Steps | Rollbacks |
|---|---:|---:|
| Round five, three cells | 1,076 | 4 |
| Round eight, the task built for it | 298 | **0** |

[Round six](round6.md#q18--does-the-rollback-fire-when-there-is-something-to-regress) set the condition for removing the mechanism: *"Still near zero. Then
checkpoint-before-every-write is protecting against something that does not happen at any slice
size Corifeu has ever been given."* Zero on the task designed to provoke it is that.

**One confound, and it is mine.** This round ran on the refactored checkpoint — S4 made it a
state of a file set rather than of one file, keyed by check rather than by path. So "the
mechanism has no occasion to fire" and "the refactor suppressed it" are not separated by this
round. What is settled is that the mechanism still works: two tests fire a rollback and assert
the files came back. What is not settled is whether `rank` mode still records the better states
a rollback needs. **Confidence: medium.** Separating it needs a leg on the pre-S4 commit, or a
counter on the scoring trail.

## What a step produced, in Rust, now

| | Round five, Rust | Round eight, floor 8 | floor 12 | floor 30 |
|---|---:|---:|---:|---:|
| Productive | 36% | **84%** | 54% | 56% |
| Cut off at `num_predict` | 32% | 3% | 1% | 1% |
| No tool call | 17% | 9% | 32% | 42% |

**The waste moved.** Round five's largest category was a turn cut off before it called a tool,
at 32%; it is now 1 to 3%, which is defect 39 — the runtime had been asking the model not to
*parse* its thinking rather than not to think, so every turn spent its budget on reasoning
first.

What replaced it is a model with nothing left to say. At floor 30, **42% of steps answer
without calling a tool**: the slice is finished or stuck, and the allowance keeps asking. That
is the cost of a floor set above what the work needs, and it is the one real signal in the
sweep — not in delivery, where the noise swamps it, but in waste.
