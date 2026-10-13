# Measurement — can a local model do the work?

Corifeu exists to answer two questions: does delegating to a local model cut the cost of
expensive tokens, and does a small scoped task with an objective check raise quality. This
document records the first run that tried to answer them with numbers instead of belief.

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

Twelve causes were traced. Nine were defects in the harness, not the model.

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

## Cost

The thesis is about money, so the comparison that matters is where the tokens went.

- **Local**, Tasks B and C together: 1,867,253 prompt and 99,938 output tokens, 248 minutes
  of machine time, costing nothing but electricity and patience.
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
- **Task C was not retried after its fixes landed.** It failed under the repaired harness, but
  the hang was never diagnosed, so whether the model could have fixed it with a better-scoped
  task is unknown.
- **Two data points.** Task A under the old ceiling, B green, C failed. This is evidence, not
  a benchmark.
