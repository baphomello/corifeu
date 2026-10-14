# The measurement harness

Every number in [`docs/measurement.md`](../docs/measurement.md) came from a round run here.
This directory is what you need to run one yourself, or to run one that answers a different
question.

Nothing here is shipped. `corifeu-bench` is a workspace member only so that a change to the
runtime that breaks the harness breaks the build, rather than the next round.

## Run one

```sh
bench/run.sh rank 10 qwopus3.5-9b-coder-Q4_K_M ornith1.5-9b-Q5_K_M
bench/run.sh pair 10 ornith1.5-9b-Q5_K_M
BENCH_SLICE=3 bench/run.sh pair 10 ornith1.5-9b-Q5_K_M   # the expensive slice
```

You need a llama.cpp server in router mode on port 9931 that can load the models you name.
`run.sh` finds the host from the default route, which is right under WSL; set `BENCH_HOST`
otherwise.

A round writes to `bench/rounds/<stamp>-<mode>/`, which is not committed:

```
round.txt        what was run, against which commit, with which allowance
results.tsv      one row per slice, plus the rows below
log              timestamps, scaffold output, anything the driver put on stderr
work/<model>/<iteration>/
                 the crate as the round left it — sources, .checkpoints/, final.txt
```

**Every workspace is kept, including the ones that failed.** The fourth run reused one
directory and overwrote the two failures before anyone could look at them, which is defect 20.
Each iteration also builds into a target directory of its own, removed once scored: two
iterations sharing one are two crates with the same package name writing the same test binary,
and the second `cargo test` will happily run the first one's and report its result as its own.

## The two modes

| | `rank` | `pair` |
|---|---|---|
| Who works | one doer | QA, then the coder ([ADR-0053](../docs/adr/0053-qa-writes-the-tests.md)) |
| Starts from | 4 `todo!()`, all 26 tests present | 1 `todo!()`, no tests at all |
| Which slice | all four, in order | `BENCH_SLICE`, 1 by default |
| Ends when | each slice's oracle goes green | the pair's slice closes |
| Judged by | the 26 cases it was given | 12 cases it was never shown |

`rank` measures what a decomposition costs. `pair` measures who writes what, and whether it
matters.

### The held-out suite

`pair` finishes by running `tasks/c-glob/held-out/slice<n>.rs` against the crate the round
produced, in a copy, under a different package name. There are twelve cases for
[slice 1](tasks/c-glob/held-out/slice1.rs) and thirteen for
[slice 3](tasks/c-glob/held-out/slice3.rs).

This exists for one question. The fourth run found that in 11 of 20 pairs QA wrote the
implementation as well as the test — ADR-0053's failure mode happening — and could not say
whether that code was any worse, because the only cases it had were the ones QA wrote and
passes by construction. Twelve cases from outside the round can say.

Every case in that file passes against the reference implementation. A case the reference
fails would be measuring the suite, not the work.

## The task

`tasks/c-glob/` is the reference crate: a glob matcher and a path classifier, 261 lines, 26
tests, all green. `tasks/scaffold.py` cuts the hole a round starts from.

```sh
python3 tasks/scaffold.py /tmp/w --mode slices            # 4 stubs, 26 tests
python3 tasks/scaffold.py /tmp/w --mode pair --slice 1    # 1 stub, 0 tests, 12 held out
python3 tasks/scaffold.py /tmp/w --mode pair --slice 3    # 1 stub, 0 tests, 13 held out
```

`--mode pair` also deletes the private helpers that exist only to serve the stubbed function.
Leaving `glob_match_suffix` in place while `glob_match` is a `todo!()` would hand the coder
the recursion it was asked to write, and the round would be measuring reading rather than
work.

It prints what it produced. Read that line before a round: it is the only description of the
starting state anyone will have afterwards.

## The step allowance

[ADR-0062](../docs/adr/0062-step-allowance-has-a-floor-not-a-ceiling.md) replaced one step
limit with four numbers, and the evidence behind their defaults covers one language and one
task. Sweep them without rebuilding:

```sh
BENCH_STEPS_FLOOR=8 BENCH_STEPS_CEILING=30 bench/run.sh rank 10 <model>
```

Leaving them unset uses the runtime's own defaults, which is what a round measuring the
defaults should do.

## What the next round has to answer

[`docs/measurement/round5.md`](../docs/measurement/round5.md) is written before the round runs:
six questions, the cell that answers each, and the result that would say the answer is no. Read
it before changing anything here — half of what is in this directory exists to serve one of
those six.

## Recording a round

Raw rows go to `docs/measurement/`, one `.tsv` per round, with the summary written into
`docs/measurement.md`. Defects found while running go to `docs/measurement/defects.csv` — the
harness's own defects included, since four of the twenty so far were the harness lying rather
than a model failing.
