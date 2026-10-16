# 0066 — A measured round is replayable, or it is an anecdote

- **Status:** Accepted
- **Date:** 2026-10-15
- **Evidence:** [round five, Q7](../measurement/round5-cell3.md#q7--the-models-invert-between-tasks-and-the-seed-is-still-missing),
  defect 26
- **Affects:** [ADR-0048](0048-bounded-generation-for-local-runtimes.md)

## Context
Five measurement rounds set no sampler seed. Round five found out what that costs when it tried
to answer whether a slice is hard or hard for one particular model. The paired design was sound
— `scaffold.py` is deterministic, so iteration *i* hands every model an identical starting state
— and it still could not answer, because the same model against the same slice does not agree
with itself:

| Model | Task C slice 1 | slice 2 | slice 3 | slice 4 |
|---|---:|---:|---:|---:|
| `qwopus3.5-9b-coder` | 7/10 | 7/10 | 9/10 | 10/10 |
| `ornith1.5-9b` | 9/10 | 6/10 | 6/10 | 8/10 |

Of the forty (iteration, slice) pairs, 24 were reached by both models, 2 by neither, and 14 by
exactly one. That looks like a difference between models. With four-to-ten variance inside a
single model it is mostly a difference between draws, and nothing in the data separates the two.

The same gap is wider than one question. No number in five rounds can be checked by re-running
it. A defect found in a round cannot be reproduced to confirm the fix — defect 23 was diagnosed
only because a scaffolded workspace survived and could be inspected by hand, not because the run
could be repeated. A model comparison cannot be replicated. An anomaly cannot be told from a
draw.

## Decision
**A role's generation budget carries a seed, and a measured round always sets one.**

1. **`Budget::seed: Option<u64>`.** Both the llama.cpp and the Ollama runtimes send it when it
   is set.
2. **`None` is the library default**, because that is how a real run behaves and the runtime
   should not quietly impose a laboratory condition on a product user.
3. **The harness always sets one.** `BENCH_SEED` is a base, varied by the iteration so that a
   cell is not twenty copies of one run while each iteration stays replayable on its own.
   `BENCH_SEED=off` restores the unseeded behaviour, which is what comparing against rounds one
   to five requires.
4. **The round records it.** A seed nobody wrote down is not a seed; `round.txt` carries it
   beside the commit and the step allowance.
5. **The claim is reproducible, not bit-exact**, and the doc comment says so. A local runtime
   that splits a batch differently across devices can still diverge. This removes the sampler as
   a source of divergence, which is the one a measurement controls.

## Alternatives considered
- **Leave it unseeded, as five rounds did.** Matches how the product actually runs, and makes
  every round an anecdote. Nothing it reports can be checked.
- **Seed the whole round with one value.** Then the twenty iterations of a cell are twenty
  attempts at the same draw and the sample size is one.
- **Record the seed the runtime chose, rather than setting one.** Would make a round replayable
  without imposing anything, and neither llama.cpp nor Ollama reports the seed it used.
- **Accept non-determinism and raise n instead.** Pays in machine time forever to avoid a field.
  Round five spent 542 minutes on three cells; the arithmetic does not favour this.

## Consequences
- **Round six has to prove this works** rather than assume it.
  [Q14](../measurement/round6.md) runs the same cell twice at n=3 with one seed and compares
  per-slice outcomes. The trap is written down with it: agreement proves the sampler is seeded,
  not that the pipeline is deterministic, because the oracle shells out to `cargo` and a compile
  that times out differently is enough to diverge.
- **A seeded round is not a typical round.** Sampling is part of what a local model is, and
  fixing it may make a cell systematically luckier or unluckier than production. The baseline
  cell of any round comparing against rounds one to five must use `BENCH_SEED=off`.
- The llama.cpp request body became a function of its own so that what goes on the wire can be
  asserted without a server. A missing field there is invisible to every test that stubs the
  response, and that is exactly how the seed came to be missing for five rounds.
- `Budget` gains a field, which breaks every struct-literal construction. There were three.
