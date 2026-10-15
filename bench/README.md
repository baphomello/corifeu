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
BENCH_TASK=e-indep bench/run.sh rank 10 ornith1.5-9b-Q5_K_M
BENCH_TASK=d-glob-py bench/run.sh rank 10 ornith1.5-9b-Q5_K_M
```

You need a llama.cpp server in router mode on port 9931 that can load the models you name.
`run.sh` finds the host from the default route, which is right under WSL; set `BENCH_HOST`
otherwise. It probes that port before anything else and refuses to start without an answer:
a round against a router that is not there produces one `PROVIDER_FAILED` row per iteration
and spends a connect timeout on each one.

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

Rust only, for now. The judging copy appends the hidden cases to `src/lib.rs` and runs
`cargo test`, because a Rust unit test has to live in the file it tests. A `rank` round works
in any language; a `pair` round does not, and a task with no `held-out.rs` scores `-`.

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

## The tasks

A task is a directory under `tasks/` holding `task.json`, the complete project it describes,
and one held-out suite per slice that a pair can work. The project is green before a round
touches it.

`task.json` carries everything else, and that includes everything about the language. Nothing
in `run.sh` or in either driver names `cargo`:

| Field | What it decides |
|---|---|
| `artefact` | the one file every slice edits |
| `oracle` | the per-slice check, with `{module}` standing in for the slice |
| `whole_suite` | the command `run.sh` runs for the `FINAL` row |
| `tools` | argv prefixes a doer may run — the allowlist |
| `owned` | globs for the one file class the doers own |
| `protected` | a marker the coder may not remove ([ADR-0059](../docs/adr/0059-a-test-is-unvalidated-until-green.md)) |
| `tests_at` | where a pair's QA is told to put the tests |
| `language` | which cutter `scaffold.py` uses: braces or indentation |
| `copy` | files copied into the workspace beside the artefact |
| `venv` | pinned test-runner packages `run.sh` provisions into `bench/.venv` |

Per slice it carries the case count, the signature to stub, the helpers to delete with it, and
the two wordings a doer and a pair are given. The drivers read the file from the workspace
rather than carrying a copy in Rust, so adding a task is a directory and a JSON file.

| | `c-glob` | `e-indep` | `d-glob-py` |
|---|---|---|---|
| Language | Rust | Rust | Python, pytest 9.1.1 |
| What | a glob matcher and a path classifier | Roman numerals, word wrap, base32, Luhn | `c-glob`, ported |
| Slices | 4, each calling the one before it | 4, calling nothing at all | 4, each calling the one before it |
| Cases | 26 | 26 | 26 — the same 4-8-9-5 split |
| Held-out suites | slices 1 and 3 | none yet; `rank` only | none yet; `rank` only |

`d-glob-py` is the other control. ADR-0062's floor of twelve steps was measured against 57
green slices that were all Rust, where a large part of the work was done by compiler errors — a
precise, immediate, free oracle that most languages do not have. `d-glob-py` is a port and not
a rewrite, so that the only difference between the two cells is the language.

Its test runner is pinned. `corifeu-check` counts pytest's summary line from captured samples,
not from a guess, so `run.sh` installs `pytest==9.1.1` into `bench/.venv` and puts it on PATH.
Its oracle is `pytest -q pathclass.py -k <module>`, which is why the cases are top-level
functions named `test_<module>_...` rather than classes: `-k` then selects a slice's cases the
way `cargo test --lib <module>` does, and the two cells differ in nothing but the language.

`e-indep` is the control. Every observation of a coder overflowing its slice
([ADR-0058](../docs/adr/0058-slice-boundaries-are-observed.md)) came from `c-glob`, where
`classify` calls `glob_match` calls `match_seg` calls `split_segs` — a coder implementing one
slice has the next one in front of it. In `e-indep` nothing calls anything, and the four are
deliberately unlike one another: a lookup table, a string loop, bit packing, and a validator.

`tasks/scaffold.py` cuts the hole a round starts from.

```sh
python3 tasks/scaffold.py /tmp/w --mode slices                   # 4 stubs, 26 tests
python3 tasks/scaffold.py /tmp/w --mode pair --slice 1           # 1 stub, 0 tests, 12 held out
python3 tasks/scaffold.py /tmp/w --mode pair --slice 3           # 1 stub, 0 tests, 13 held out
python3 tasks/scaffold.py /tmp/w --task e-indep --mode slices    # 4 stubs, 26 tests
python3 tasks/scaffold.py /tmp/w --task d-glob-py --mode slices  # 4 stubs, 26 tests
```

`--mode pair` also deletes the private helpers that exist only to serve the stubbed function.
Leaving `glob_match_suffix` in place while `glob_match` is a stub would hand the coder the
recursion it was asked to write, and the round would be measuring reading rather than work.

A Rust block ends at a brace and a Python block ends when the indentation comes back, so the
cutting differs by language and nothing else does. `task.json`'s `language` picks the cutter.

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
