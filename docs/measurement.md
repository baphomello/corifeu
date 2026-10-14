# Measurement — can a local model do the work?

Corifeu exists to answer two questions: does delegating to a local model cut the cost of
expensive tokens, and does a small scoped task with an objective check raise quality. This
document records the three runs that tried to answer them with numbers instead of belief: a
first across three tasks, a second that re-ran the task the first one failed, and a third
driven by Corifeu itself rather than by a throwaway harness.

Everything here comes from a run. Where a number is missing it is left blank rather than
estimated, and where a conclusion is not fully verified it says so.

## Setup

| | |
|---|---|
| Local model | `ornith1.5:9b` on Ollama |
| Reviewer | Codex CLI 0.162.1 (`gpt-5.5`), read-only sandbox |
| Harness | a throwaway Python agent loop, not product code |
| Time limit | none |
| Step ceiling | 400 |

Three tasks, each a small Rust library crate with the specification written once and used
verbatim:

- **Task A** — parse a duration string. Run earlier, under a 60-step ceiling.
- **Task B** — check a detected CLI version against a declared minimum.
- **Task C** — classify a path against ordered glob rules.

Two scenarios were planned per task: the expensive model implements, or the local model
implements. After Task A the second scenario became the only one worth running, because
falling back to the expensive model spends exactly the resource the project exists to save.
Tasks B and C were therefore run **local-only**, and Task A scenario 1 is the baseline.

## Results

| Task | Steps | Prompt tokens | Output tokens | Minutes | Rounds | Tests | Outcome |
|---|---:|---:|---:|---:|---:|---|---|
| B | 149 | 945,675 | 47,727 | 113 | 6 | **21/21** | green, review not approved |
| C | 121 | 921,578 | 52,211 | 135 | 3 | 0/10 | **failed — the code hangs** |

Task B reached a fully green suite, verified by running `cargo test` rather than by trusting
the agent's claim. Task C compiles, its tests compile, and `cargo test` never returns: the
glob matcher recurses without bound on `**`. 101 steps and 101 minutes did not find it.

Per-round detail is in [`measurement/runs.csv`](measurement/runs.csv).

### The shape of Task B

Three implementation runs each stopped at 18 of 21 tests. The same three failures each time:
a parse failure reported as `Overflow` where the specification requires `NoVersionFound`, and
the "extract a version from surrounding text" requirement never implemented at all.

The reviewer named the faulty expression. The next round adopted the prescribed structure
exactly — scan for a candidate, return `NoVersionFound` when there is none — and then failed
on the language, handing a `usize` to code that indexed it like a slice. The round after
that, driven by compiler errors read from the top, fixed it and went green.

The model found the final bug itself and named it correctly: `scan_version` shadowed
`component` with a `usize`.

## What stopped the runs

Twelve causes were traced in the first run. Nine were defects in the harness, not the
model. The second run added three more, recorded with the run that found them in
[`measurement/defects.csv`](measurement/defects.csv).

| # | Defect | Origin | Evidence |
|---|---|---|---|
| 1 | Loop detection killed on the first repeated action | harness | stopped at step 12 of 400 holding 18/21 |
| 2 | `cargo test \| tail` reported exit 0 | harness | a failing suite was reported to the agent as passing |
| 3 | `read_file` truncated at 8,000 bytes | harness | the file was 9,570; the defect sat past the cut |
| 4 | Context compaction evicted the file being edited | harness | 28 steps reading, 0 edits, 1,064 output tokens |
| 5 | Command output truncated from the tail | harness | 30 compile errors in 15,148 chars; the last 4,000 shown |
| 6 | No rollback | harness | an 18/21 state was overwritten and could not be recovered |
| 7 | The loop warning told a stuck agent to read more | harness | 5 steps, 291 output tokens, a pure read loop |
| 8 | The oracle had no timeout | harness | each attempt spent 300s on a test that never returns |
| 9 | `write_file` was bypassed with a shell heredoc | harness | 6 shell writes against 16 tool writes, none checkpointed |
| 10 | Cannot infer a missing requirement from an assertion | **model** | 3 runs stuck at 18/21 until the reviewer named the cause |
| 11 | Cannot diagnose its own infinite loop | **model** | Task C, 101 steps |
| 12 | A reviewer finding was fabricated | **reviewer** | 2 of 3 findings did not survive verification |

The ratio is the most useful result in this document. While the runtime was hiding compiler
errors, masking exit codes and evicting the working file from context, it was impossible to
separate *the model cannot do this* from *the runtime did not let it*. Only after those nine
were fixed did the winning run appear, and only then did the two real model limits become
visible.

Each of the nine is now an ADR, listed under [Found by measurement](adr/README.md).

## The reviewer failed its own gate

With a green 21/21 suite, the reviewer declined to approve and raised three findings. Each
was checked by running code:

| Finding | Claim | Verdict |
|---|---|---|
| Suffix on a short version | `"1.2-beta.999"` parses as 1.2.999 | **False.** It parses as 1.2.0; `TooOld` against 1.2.5 proves it |
| Separator paths corrupt the scan | the scanner is fragile | **Not reproducible** across six realistic inputs |
| `"1.2.3.4"` accepted as 1.2.3 | true | **True**, and the reviewer noted the spec does not require rejection |

The review prompt said *"a fabricated finding is worse than an empty gate"* and *"every
finding must state the concrete input or condition that triggers it"*. One finding stated a
concrete input and was wrong about it; another stated none.

This matters more than the arithmetic. Had those findings been handed to the coder, it would
have changed working code to satisfy a false report — which is exactly what happened in the
first repair round, where prose findings produced a wholesale rewrite that broke the build.
[ADR-0054](adr/0054-findings-become-work-only-as-tests.md) is the consequence.

## Second run — Task C, re-sliced

The first run left Task C unexplained. The crate compiled, its tests compiled, and
`cargo test` never returned. The cause is one statement in `match_here`:

```rust
while ps < p.len() {
    let cur = p[ps];
    let seg = if pp < pats.len() { pats[pp] } else { return true };
    if match_seg(seg, cur, pp + 1, ps + 1, pats, p, skip) {
        return true;
    }
}
```

`ps` is never incremented, so the loop cannot end once `match_seg` returns false. 101 steps
and 101 minutes did not find it.

The same specification was then re-run under
[ADR-0056](adr/0056-one-oracle-per-slice.md): four slices, one function and one
`cargo test --lib <module>` filter each, with the tests written before the implementation.

| | Monolithic | Re-sliced |
|---|---:|---:|
| Steps | 121 | **79** |
| Prompt tokens | 921,578 | **362,231** |
| Output tokens | 52,211 | 17,545 |
| Minutes | 135 | **39** |
| Result | the code hangs | **26/26 green** |

That is the strongest evidence in this document for the quality half of the thesis. Three
things qualify it, and each one became a decision.

### The oracle passed on a suite that no longer existed

At step 5 the coder rewrote `src/lib.rs` from 3,763 bytes to 1,272, taking the file from
twelve `#[test]` functions to zero. The oracle was run immediately after:

```
[step  5] write_file(src/lib.rs) -> wrote 1272 bytes
[step  6] run(cargo test --lib slice2_seg) -> exit=0
```

Cargo exits 0 when the filter matches nothing. The run then spent its remaining 44 steps
searching for the module it had deleted, and stopped on the loop detector without restoring
it.

Nothing was evaded. Rust puts the unit test inside the file it tests, so the coder wrote the
one file it owns. [ADR-0053](adr/0053-qa-writes-the-tests.md)'s promise that *the coder
cannot write tests* has no mechanism behind it in this language, and
[ADR-0051](adr/0051-checkpoint-before-every-write.md)'s rollback never fired, because it
compares the oracle's verdict and the verdict was green.
[ADR-0057](adr/0057-oracle-integrity-by-test-inventory.md) is the consequence.

The state was recovered from a checkpoint, which is
[ADR-0051](adr/0051-checkpoint-before-every-write.md) working as designed against a defect it
was not designed to catch.

### Slices 3 and 4 never ran

Given slice 2 — implement `match_seg`, leave the other functions as `todo!()` — the coder
implemented `glob_match`, `glob_match_suffix` and `classify` as well. Slices 3 and 4 were
green the first time they were checked, so no round was ever run for either.

The comparison above holds: 79 steps against 121, and green against a hang. The per-slice
attribution does not. The 79 steps bought slices 1 and 2, and the other two arrived as a side
effect. [ADR-0058](adr/0058-slice-boundaries-are-observed.md) is the consequence.

### Two of the original ten tests were wrong

The monolithic attempt's own tests were run against the sliced implementation, which they had
never seen. Eight passed. Both failures are errors in the tests:

| Test | Asserts | The specification says |
|---|---|---|
| `question_mark_single_char` | `x?.rs` matches `x_a.rs` and not `xa.rs` | `?` matches exactly one character, so both assertions are inverted |
| `star_excludes_slash` | `**/*.cfg` matches `a/b.ccfg` | `b.ccfg` does not end in `.cfg` |

Neither was ever exposed, because the implementation hung before any assertion ran. A test
that has never gone green is an untested claim, not an oracle.
[ADR-0059](adr/0059-a-test-is-unvalidated-until-green.md) is the consequence.

## Third run — Corifeu drives

The first two runs were driven by a throwaway Python harness against Ollama. This one is
driven by Corifeu against `llama-server`, which makes it the first measurement of the
product rather than of a sketch of it. Three models ran the same four slices of Task C.

### The runtime was the wrong one

Before anything was measured, the local model was moved from Ollama to llama.cpp, and the
difference is larger than any decision in this document:

| | Ollama | llama.cpp |
|---|---:|---:|
| `ornith1.5:9b`, as the first two runs were configured | **7.8 tok/s** | — |
| `ornith1.5:9b`, Ollama given the same 8k context | 15.3 tok/s | — |
| `ornith1.5:9b`, 8k context | — | **42.2 tok/s** |
| `qwen2.5-coder:7b` | 26.2 tok/s | 65.8 tok/s |

With the model loaded, llama.cpp held **7,383 MiB of the card's 8,192**. Under Ollama the
same model ran 68% on the GPU and decoded the rest on the CPU. Three things account for it:
the vision projector was not loaded, the context was 8k rather than the model's declared
262k, and flash attention was on.

The first two runs are therefore wall-clock measurements of a misconfiguration. Their step
counts and token counts stand; their minutes do not.

### Four slices, three models

Each model got a fresh scaffold with all four functions stubbed and all 26 tests present,
then worked the slices in order under a 30-step ceiling.

| Model | Slices green | Steps | Prompt | Output | Minutes | **The crate at the end** |
|---|---|---:|---:|---:|---:|---|
| `qwopus3.5-9b-coder` Q4 | 3 + 1 free | 22 | 75,500 | 41,162 | **11.8** | **26/26** |
| `ornith1.5-9b` Q5 | 3 + 1 free | 24 | 77,858 | 84,822 | 26.4 | **26/26** |
| `qwen2.5-coder-7b` Q4 | 0 | 120 | 345,363 | 13,305 | 3.5 | **does not compile** |

The last column is the only one that answers the question. Per-slice verdicts say what
happened on the way; whether the crate works is the result.

For `ornith`, which is the one model all three runs share:

| | Steps | Prompt tokens | Minutes | Result |
|---|---:|---:|---:|---|
| Run 1 — monolithic, Python harness | 121 | 921,578 | 135 | hangs |
| Run 2 — sliced, Python harness | 79 | 362,231 | 39 | 26/26 |
| Run 3 — sliced, Corifeu | **24** | **77,858** | 26 | 26/26 |

### `qwen2.5-coder` cannot be a doer, and it is the fastest model here

It decodes at 65.8 tok/s, faster than either model that succeeded. It finished none of the
four slices, spent 120 steps and 345,363 prompt tokens to produce 13,305 output tokens, and
left a crate that does not compile — it broke the string literals while writing the one
function it attempted.

The cause is not capability but shape: it does not emit tool calls. Under Ollama it answered
with a JSON object as plain text; under llama.cpp it invented `<run argv="…">` markup. Every
step it takes is a step that changes nothing.

**`corifeu doctor` answers this in one call.** A single tool-call probe, which is what
[ADR-0027](adr/0027-install-wizard-with-tool-call-test.md) gates the installer on, separates
this model from the other two before any work is attempted.

### The run found a defect in the rollback

The first pass of this run reported `qwopus` green on all four slices and left a `todo!()` in
the crate. The cause was in Corifeu, not the model:

```
slice 3 closes green:  9 cases passing   -> saved
slice 4 closes green:  5 cases passing   -> 5 < 9, read as a regression
                                         -> the slice-3 state restored over it
                                         -> the slice reported green anyway
```

Counts from different checks were being compared. Nine passing under one slice's oracle says
nothing about five under another's, and ordering them reads every smaller check as a
regression. A checkpoint now holds a score per check, and only states the same check has
judged are compared.

This is the second defect in the rollback path, and both were invisible to every test until a
real run produced a shape the tests did not have. The numbers above are from the re-run.

### Slice overflow rescued both successful runs

[ADR-0058](adr/0058-slice-boundaries-are-observed.md) was written as a metering problem: a
coder that exceeds its slice makes the per-slice cost wrong. This run shows it is also what
made the task finish.

| Model | The slice that failed | What saved it |
|---|---|---|
| `qwopus` | slice 2 looped after 18 steps | slice 3's coder implemented `match_seg` as well |
| `ornith` | slice 3 looped after 22 steps | slice 4's coder implemented `glob_match` as well |

Both runs reached 26 of 26 with a slice that never closed. Enforcing the boundary — the
alternative ADR-0058 rejected on cost — would have left both at 3 of 4 and a broken crate.
That is an argument for the decision that was not available when it was made.

## Fourth run — ten per cell

Every run before this one had a sample size of one, and the third run's own notes said so:
*"with one run per cell nothing here separates a model from a lucky draw."* This run answers
that. Two models, ten iterations each, the same four slices of Task C under a 30-step
ceiling, a fresh scaffold every iteration, one hour per iteration. Then a second experiment,
ten iterations per model, measuring the QA and coder pair on slice 1 alone.

Raw rows: [run4-ranking.tsv](measurement/run4-ranking.tsv),
[run4-pair.tsv](measurement/run4-pair.tsv).

### The crate at the end, twenty times

| | `qwopus3.5-9b-coder` Q4 | `ornith1.5-9b` Q5 |
|---|---:|---:|
| Runs | 10 | 10 |
| **Complete crate, 26/26** | **10** | **9** |
| Killed by the one-hour cap | 0 | 1 |
| Minutes, min / median / max | 3.5 / 9.8 / 17.5 | 1.0 / 5.4 / 23.2 |
| Prompt tokens, min / max | 11,583 / 115,277 | 2,869 / 92,539 |
| Prompt tokens, ten runs | 602,043 | 345,714 |
| Steps, ten runs | 173 | 101 |

**19 of 20 runs delivered the complete crate.** The ranking the third run could not make is
still not a ranking: both models do the job, `ornith` costs about half as much and owns the
only total failure.

The one failure is the one-hour cap, not a plateau. `ornith` iteration 2 spent 569, 1140 and
1320 seconds on three slices, was killed before the fourth, and left the crate at 18 of 26.

### A stuck slice costs four times as much and changes nothing

Of the 19 runs that reached 26/26, **8 did it with at least one slice that never closed**.
Splitting the runs on that one fact separates them completely:

| | Runs | Minutes | Prompt tokens |
|---|---:|---|---|
| All four slices closed | 11 | 1.0 – 6.7, avg **4.5** | 2,869 – 40,619, avg **18,016** |
| A slice never closed | 8 | 13.0 – 23.2, avg **17.5** | 69,512 – 115,277, avg **93,697** |

The two distributions do not overlap. The slowest clean run took 6.7 minutes; the fastest
run with a stuck slice took 13.0. The gap in tokens is as sharp: 40,619 against 69,512.

Both groups ended with the same crate.

This makes [ADR-0058](adr/0058-slice-boundaries-are-observed.md)'s overflow a measured
quantity rather than an anecdote. It is not the mechanism — 11 of 19 successful runs closed
every slice on its own — and it is not noise either. In 8 of 19 runs the per-slice verdict
reported failure on work that was already done, and the budget spent chasing it bought
nothing.

### QA writes the implementation in more than half of the pairs

[ADR-0053](adr/0053-qa-writes-the-tests.md) ships QA and coder as separate roles so that the
test and the code come from two readings of the specification. The pair experiment strips
slice 1's tests *and* its implementation, leaving one `todo!("slice 1")`, and asks QA for a
failing test. The check is probed before QA runs, so a green check afterwards is
distinguishable from one that was green all along.

| Outcome | Runs | What happened |
|---|---:|---|
| `SelfTested` | **11 / 20** | QA wrote the test **and** made it pass. The coder never ran. |
| `Green` | 7 / 20 | QA left red, the coder turned it green. The pair as designed. |
| `NoRunnableTest` | 2 / 20 | QA burned its 12-step ceiling without a runnable check. |

By model: `ornith` self-tested 7 times in 10, `qwopus` 4 times in 10 and produced both
`NoRunnableTest` runs.

In the 7 runs where the pair worked as designed, **the coder took one step in 6 of them**.
Only one coder in twenty runs did substantial work: 10 steps, in the single run where QA also
spent its full ceiling. Across the whole experiment the coder either never ran or took a
single step in 17 of 20 runs.

All 20 runs ended with the `todo!` gone, so the implementation was always written. The
question this answers is not whether the work gets done — it is who does it.

This is [defect 14](measurement/defects.csv) at scale. Rust puts the unit test in the file it
tests, so the writable-set restriction
[ADR-0052](adr/0052-write-restriction-in-the-sandbox.md) relies on cannot separate QA from
the source. ADR-0053's independence argument is not being tested here, because QA rarely
leaves anything for a second reader to disagree with.

**What this does not settle:** whether a `SelfTested` slice produces worse code than a
`Green` one. Both end with the implementation present and the tests passing; nothing here
compares them for correctness or coverage against a reference. That comparison is what the
ADR-0053 decision actually turns on, and it was not measured.

## Cost

The thesis is about money, so the comparison that matters is where the tokens went.

- **Local**, Tasks B and C together: 1,867,253 prompt and 99,938 output tokens, 248 minutes
  of machine time, costing nothing but electricity and patience.
- **Local**, Task C re-sliced: a further 362,231 prompt and 17,545 output tokens, 39 minutes.
- **Local**, the third run's twelve slices across three models: 498,721 prompt and 139,289
  output tokens, 42 minutes of machine time, one model of the three finishing nothing.
- **Paid**, the review that unblocked Task B: **7,956 tokens**, one call.

One paid call of eight thousand tokens moved a task that three local runs could not finish.
That is the delegation economy working as intended: the expensive model is worth its price as
a diagnostician, and is wasted as an implementer.

An earlier review of the same crate cost far more because the reviewer loaded six skill files
from a directory outside its scrubbed home. Redirecting `HOME` removed them: 535 lines of
output instead of 1,707. A reviewer's environment is part of its cost.

## What this measurement does not show

- **One model, one task size, one language.** Nothing here generalises to other local models
  or to languages without a fast, precise compiler. Rust's error messages did a large share
  of the work.
- **The rounds are not comparable to each other.** Nine harness fixes landed between them, so
  the 149 steps Task B took are not the number a correct runtime would need. Only the final
  58-step round ran on a harness worth measuring.
- **The two runs are not a controlled comparison.** Task C re-sliced differs from Task C
  monolithic in the slicing, in who wrote the tests, and in three harness guards that landed
  between them. The gain is real and its attribution is not isolated.
- **Slices 3 and 4 were delivered by slice 2's coder**, so four slices cost two slices' work.
  A decomposition whose slices are genuinely independent would not get that discount.
- **The first two runs' minutes are a misconfiguration.** `ornith1.5:9b` ran 68% on the GPU
  under Ollama at 7.8 tokens per second. On llama.cpp the same model decodes at 42.2. Step
  counts and token counts from those runs stand; their wall clocks do not.
- **Runs 1 to 3 have one run per cell.** The third run's first pass had `qwopus` closing
  slice 2 green in a single step; the re-run had the same model loop for 18 steps on it. The
  fourth run settles that for the two surviving models at n=10 and still does not produce an
  ordering: both deliver the crate, `ornith` costs about half as much and owns the only
  total failure. Nothing here separates them on quality of output.
- **Slice overflow is measured on one task shape only.** Across 19 completed runs of the
  fourth round, 8 reached 26/26 with a slice that never closed. Task C's four slices share a
  module and a data type, which is what makes a coder able to overshoot into the next one. A
  decomposition whose slices were genuinely independent would not give that discount, and
  nothing here says how common either shape is in real work.
- **Two data points.** Task A under the old ceiling, B green, C failed. This is evidence, not
  a benchmark.
- **The pair was measured on one slice, in one language.** Slice 1 of Task C is small
  enough that writing the implementation is barely more work than writing the test, which is
  the most likely reason QA does both. A slice where the implementation is the expensive part
  may well behave differently, and none was run.
- **`SelfTested` was never compared with `Green` for quality.** The 55% figure says who wrote
  the code, not whether it is worse. Until that is measured, ADR-0053's decision rests on the
  argument in the ADR rather than on evidence from here.
