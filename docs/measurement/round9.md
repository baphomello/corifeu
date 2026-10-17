# Round nine — a round that can be run twice

The determinism hunt. Round eight found that two passes of one configuration at
one seed disagreed, and blamed the stack. The stack was not at fault: **four
separate things Corifeu itself put into its own prompts** were, and with them
gone two passes are now identical byte for byte.

[ADR-0066](../adr/0066-a-round-is-replayable-or-it-is-an-anecdote.md) is true
for the first time.

- **Task:** [`f-expr`](../../bench/tasks/f-expr), `rank`, n=5 per pass, seed base 1,
  `BENCH_TEMP=0`, floor 12.
- **Model:** `qwen3.5-9b` on 32,768 of context, MTP on.
- **Commit:** `12fc771`, and the same commit on both passes — which took three attempts to
  arrange, see defect 51.

## The result

```
              verify2-A2  verify2-B2
  verify2-A2          10          10
  verify2-B2          10          10

slice runs whose prompts parted:       0
slice runs the cells do not all agree: 0
```

14 dumped prompts identical byte for byte. `steps.tsv` identical in every column of every
step. `results.tsv` identical but for the seconds column, which is a clock.

## What was wrong, and how each one was found

| Source | What reached the prompt | Found by |
|---|---|---|
| Compile time | `in 0.22s` against `in 0.13s` | running the oracle twice and diffing |
| Case completion order | cargo runs a thread per core | the same diff |
| OS thread id | `thread 'x' (186718) panicked` | **dumping the prompts** |
| Cargo target directory | `.../target/m/5/debug/…` | **dumping the prompts** |

The last one is why iterations *within* one pass diverged as well: `bench/run.sh` puts the
iteration number in the target directory path, and the path was in the prompt.

**Two of the four were found by reasoning about where text travels. The other two were in none
of the hypotheses** and appeared only once the prompts were on disk and could be diffed. Three
hypotheses — speculative decoding, the server's prompt cache, and numerical noise at long
prompt lengths — were each eliminated by measurement, and not one of them was the answer.

## What the stack was accused of, and cleared of

Each of these was tested directly rather than argued about.

| Suspicion | Test | Result |
|---|---|---|
| MTP makes decoding nondeterministic | two passes with `--spec-type` off | still diverged — **cleared** |
| Long prompts diverge numerically | one prompt of 2,097 tokens, 3 identical requests | identical — **cleared** |
| The server's prompt cache carries state | the same request with other requests in between | identical — **cleared** |

llama.cpp reproduces. It was never the problem.

## What this corrects

[Round seven](round7.md) concluded that a round replays on this stack, from 159 of 160 sampled
slice runs agreeing. **Every slice in that round took one step**, so the oracle ran once and the
run ended: there was never a second prompt to carry the contamination. The measurement was
right and the sentence written over it was too wide.

Round eight then read the same evidence as *"Corifeu cannot replay a round at any trace length
beyond one step"*, which was also wrong — it could not replay because of a defect, not because
of a limit.

## The instrument, which is the part that lasts

`steps.tsv` carries a fingerprint of the prompt behind each step, and `analyse.py` reports the
first step on which two rounds' prompts part — a different question from where their outcomes
part, since an outcome can survive a changed prompt and only a changed prompt can change an
outcome under one seed. `CORIFEU_DUMP_PROMPTS` writes the messages themselves.

Two hours of this round went on inferring from source what the fingerprint answers directly,
and the two sources that reasoning never reached were found in a `diff` five minutes after the
dump existed.

## What is still not settled

- **Delivery is not comparable across this fix.** Every number in rounds one to eight was
  measured with volatile text in the prompt. The fix changes the prompt, so it changes the run;
  comparing an old delivery rate with a new one measures the fix as well as the model.
- **One residual.** `Compiling exprs v0.0.0` appears only when cargo rebuilds. In a doer's loop
  every check follows a write, so it is always present — but a check with no write before it
  differs, and nothing stops that today.
- **Defect 51 is not fixed.** `bench/run.sh` rebuilds at the start of a round and will happily
  measure two different binaries as one configuration, which it did twice in this round.
