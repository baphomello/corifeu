# S5 — the hybrid split, measured

The arrangement this project was founded on, run for the first time: an expensive model cuts a
brief into slices, a local 9B writes the code, and an oracle nobody in the loop wrote decides
when each slice is done.

Nine passes of the same brief. The command is new — `corifeu build` — and so is most of what
it found.

## The run

```
provider: claude 2.1.302
planner: lead ( on expensive)
coder:   coder (qwen3.5-9b on local)

plan: 3 slice(s)
  1. Implementar readingTime(text) em src/readingTime.ts conforme os casos de src/readingTime.test.ts.
     src/readingTime.ts, src/readingTime.test.ts -> npx vitest run src/readingTime.test.ts
  ...

slice 2: Implementar excerpt(text, limit) em src/excerpt.ts ...
    1  write_file   wrote 371 bytes to src/excerpt.ts
    2  write_file   wrote 929 bytes to src/excerpt.ts
  green after 2 step(s), 2 check(s)

spend by role:
  lead       expensive                    1 dispatch(es)    15197 prompt     454 output  $0.0121
  coder      local      qwen3.5-9b        4 dispatch(es)     4935 prompt     696 output  no charge

the planner's share: 100.0% of the money, 73.5% of the tokens

3 of 3 slice(s) closed under the coder, 0 overtaken, 0 still red
```

The brief is in Portuguese, three sentences, written as a product request and not as a
decomposition — [brief.txt](task/brief.txt). The plan, the slices, the oracles and the per-slice
file sets are the planner's. Nothing was edited between planning and coding: one process does
both.

## The split

**The expensive model is all of the money and most of the tokens.** Both are true of the same
run and either one alone invites the wrong conclusion, which is why the report prints the pair.

| | planner (claude CLI) | coder (qwen3.5-9b, local) |
|---|---:|---:|
| dispatches, nine passes | 9 | 74 |
| money | **$0.16** | none |
| prompt tokens, range per pass | 15,197 – 46,881 | 4,923 – 34,682 |
| share of money | 100% | 0% |
| share of tokens | 25% – 85% | 15% – 75% |

The token share swings by a factor of three on an identical brief, because it is a ratio and the
coder's half is what varies: a pass where slice 2 closed in two steps leaves the planner at 73%,
and a pass where it took eighteen leaves it at 25%. **The planner's own cost is roughly fixed
and the doers' is not**, which is the opposite of what a token-saving figure quoted from one run
would suggest.

## Nine passes

Each pass starts from the same three stubs, the same fixed tests and the same brief. `seed: 1`
throughout.

| pass | runtime | floor / patience | slice 1 | slice 2 | slice 3 | closed | planner |
|---|---|---|---|---|---|---:|---:|
| a | before the two fixes below | 12 / 2 | 1 step | **out of steps**, 10 of 12 narrating | not attempted | 1 of 3 | $0.0274 |
| b | both fixed | 12 / 2 | 1 | 2 | 1 | **3 of 3** | $0.0117 |
| c | both fixed | 12 / 2 | 1 | **out of steps** at 12 | not attempted | 1 of 3 | $0.0110 |
| d | both fixed | 12 / 2 | 1 | 5 | 1 | **3 of 3** | $0.0392 |
| e | allowance reaches the runtime | 12 / 1 | 1 | 3 | 1 | **3 of 3** | $0.0118 |
| f | " | 12 / 1 | 1 | 12 → **grew to 18** → 2 | 1 | **3 of 3** | $0.0110 |
| g | ADR-0063's walk | 12 / 1 | 1 | 6 | 1 | **3 of 3** | $0.0121 |
| h | " | 12 / 1 | 1 | 2 | 1 | **3 of 3** | $0.0121 |
| i | " | 2 / 0 | 1 | out of steps at 2 | 1 | 2 of 3 | $0.0276 |

Slices 1 and 3 closed on the first step of every pass, nine times out of nine. Slice 2 —
`excerpt`, where a word boundary and an ellipsis have to fit inside a character limit — took
between 2 and 18 steps and failed twice. **The variance in a hybrid run is one slice's, not the
plan's.**

## What the run found

Four defects, three of them in Corifeu and two of those fixed inside this round.

### Defect 52 kills a slice, and it was already written down

Pass a's slice 2 spent **ten of twelve steps** answering:

```
    3  say          no tool call; said: I ran: write_file src/excerpt.ts
    4  say          no tool call; said: I ran: write_file src/excerpt.ts
    5  say          no tool call; said: I ran: read_file src/excerpt.ts
    6  say          no tool call; said: I ran: say
```

The context builder put one assistant message per past step reading `I ran: <action>`. That is a
worked example of answering in prose, sitting in the slot the model fills next, and it copied
it — down to `I ran: say`, which is the narration narrating itself.

Round five measured 17% of all steps answering without a tool call and never named a mechanism.
This is one, and the loop was teaching it.

Under [ADR-0047](../../adr/0047-pin-the-artefact.md) there is no conversation to reconstruct: the
context is rebuilt from state every step. So a past step is now carried as its result alone,
with the action naming it, and **nothing in the transcript shows an assistant answering without
a call**. Across passes b to i, 1 step of 61 answered in prose, against 10 of 13 in pass a.

What that fix did *not* do is make the slice reliable: passes c and i still ended with slice 2
red. The narration was one failure mode and not the only one.

### Defect 55 — ADR-0050 could not see the most repeated failure there is

The no-tool-call branch `continue`d before the detector was consulted. A model that says ten
things in a row was never once considered a repeat, so the one stop
[ADR-0050](../../adr/0050-loop-detection-warns-before-it-stops.md) exists for could not fire on
it: pass a spent its whole allowance of twelve where six strikes would have ended it, and
recorded `out of steps` rather than `going in circles`. Those are different diagnoses and
different fixes.

Both defects were invisible to a suite of 280 tests. Both now have one that goes red without the
fix.

### `corifeu build` was ignoring three quarters of `limits.steps`

The command called `Doer::work`, which always takes `steps.floor`. `ceiling`, `patience` and
`raise_by` were declared in the project file and reached nothing — the same shape of gap S2
found in `limits.steps` itself, in the command written to close it.

With one `Allowance` held across the plan, **[ADR-0062](../../adr/0062-step-allowance-has-a-floor-not-a-ceiling.md)'s
growth fired for the first time in any round**:

```
slice 2: Implementar excerpt(text, limit) ...
   ...
  out of steps after 12 step(s), 5 check(s)
  the allowance grew to 18 steps; the same slice runs again

    1  write_file   wrote 721 bytes to src/excerpt.ts
    2  write_file   wrote 885 bytes to src/excerpt.ts
  green after 2 step(s), 2 check(s)
```

Pass f delivered 3 of 3 because of it. Growth had never fired in five rounds for two separate
reasons — the task was too easy to spend an allowance, then `patience` counted consecutive cuts
that a plan whose first slice always passes never accumulates — and this is the first run where
a slice was hard enough, the allowance was carried, and `patience: 1` made the first cut count.

### The command had quietly reverted ADR-0063

`corifeu build` stopped at the first red slice. That is the behaviour
[ADR-0063](../../adr/0063-an-unfinished-slice-is-asked-again.md) measured and replaced: stopping
reported failure on a complete crate in eight of nineteen runs of the fourth measurement,
because a later slice had written an earlier one's work on the way past.

Fixed, and then exercised on purpose. Pass i runs with `floor: 2`, which is not enough for slice
2:

```
2 of 3 slice(s) closed under the coder, 0 overtaken, 1 still red
slice 2 is still red. The allowance is at its ceiling and the oracle is still red. Either the
slice is larger than one turn, which makes it a planning problem (ADR-0056), or the ceiling is
low for this task.
```

Slice 3 ran and closed, where the old walk would have thrown it away. Nothing was overtaken,
which is what [round five](../round5-cell3.md) predicts for a plan whose slices call nothing:
carrying on paid out 66% of the time where slices sit in each other's line of sight and 2% where
they do not, and these three do not.

The last line is also a correction to the first version of this command, which quoted ADR-0056
at every red slice. A slice out of steps at a low ceiling is not evidence a decomposition was
wrong, and saying so would have sent the next run to the planner for nothing.

### Alternating two actions walks past the six-strike rule

Not fixed, and recorded because the fix is not obvious. Pass f's slice 2 spent five of its
twelve steps on writes the detector refused:

```
    6  write_file   REPEAT: you already ran `write_file src/excerpt.ts` and nothing has changed…
    7  say          no tool call; said: The tests are failing because: 1. The ellipsis is not…
    8  write_file   REPEAT: you already ran `write_file src/excerpt.ts` and nothing has changed…
    9  run          $ npx vitest run src/excerpt.test.ts
   10  write_file   REPEAT: …
   11  write_file   REPEAT: …
   12  write_file   REPEAT: …
```

`Detector` counts strikes against the last action and resets them on any action it has not seen
since the last artefact change, so one `run` at step 9 put the count back to zero. Six strikes
are never reached by a model that alternates. Pass c shows the same shape.

The obvious tightening — count strikes per action rather than resetting — is the thing
[ADR-0050](../../adr/0050-loop-detection-warns-before-it-stops.md) already argues against in its
own doc comment: three strikes killed the winning round of an earlier measurement at step 26,
because it hit four dead ends in a row and was searching rather than looping. A model
interleaving a check between attempts looks much more like searching than like a loop. This
needs a measurement, not a patch.

## One dispatch to an agent CLI is not one model call

The same brief and the same context, ten times, produced three prompt-token counts and a
5.9× spread in price:

| prompt tokens | passes | cost |
|---:|---:|---|
| 15,197 | 6 | $0.0110 – $0.0121 |
| ~30,862 | 3 | $0.0274, $0.0276, **$0.0655** |
| 46,881 | 1 | $0.0392 |

The counts are near-exact multiples of 15,197, so the CLI took one, two or three internal turns
to answer the same question, and `usage` reports their sum. The $0.0655 outlier is the first
dispatch of the night, when the cache was written rather than read.

Neither number is a bug — both are what driving an agent CLI as a provider means. The
consequence is for [ADR-0031](../../adr/0031-token-savings-metering.md): **the expensive side's
cost is not a function of the prompt**, so a saving figure taken from one run is an anecdote
about that run's cache state and its internal turn count. A range across passes is the honest
unit.

## What the local model shipped, and what the oracle did not ask

Three files, 10 of 10 cases green — [delivered/](delivered/). One of them is wrong:

```ts
export function postPath(slug: string, isoDate: string): string {
  const date = new Date(isoDate);
  ...
  const month = String(date.getMonth() + 1).padStart(2, "0");
```

`new Date("2026-03-01")` is UTC midnight and `getMonth` is local, so a post dated the first of a
month lands in the previous one:

```
TZ=UTC               /blog/2026/03/x
TZ=America/Sao_Paulo /blog/2026/02/x
TZ=Asia/Tokyo        /blog/2026/03/x
```

The suite passes in all three, because none of its three cases is the first of a month. This is
the exact mirror of [the CLI probe](../s5-cli/README.md), where the expensive model wrote NFKD
normalisation and `ß` → `ss` for a slug function asked for four cases: **the expensive model
answered past the oracle and the local one answered up to it.** Same claim from both sides — the
hybrid split is about where each model is used, and the oracle is the thing that has to be
better than either.

## What this round is not

- **Not a comparison of the two models on the same work.** Neither did the other's job.
- **Not a test of whether a planner can discover a project.** `context.txt` names the files, the
  signatures and the test runner, which is what `Brief.context` is for. It was told the layout.
- **Not Corifeu writing its own tests.** The three test files were written by hand before the
  round and are owned by no role, so the coder could read them and not change them. That is
  deliberate: [defect 53](../defects.csv) is what happens otherwise, and this round measures the
  split rather than test authorship.
- **Not replayable yet across the planner.** `seed: 1` reaches the local coder. The expensive CLI
  takes no seed, and its plan's wording differed on every pass, so pass a and pass b differ by
  the fix *and* by one sentence of task text. The mechanism behind defect 52 is confirmed by its
  own test; the size of the improvement is not cleanly attributable.
- **One brief, one task shape, n=9.** Slice 2 was the only hard one.

## Cost of the whole round

Ten planner dispatches, **$0.26**. Seventy-seven coder dispatches, no charge. Nine passes of a
three-slice feature, of which seven delivered every slice.
