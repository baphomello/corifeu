# From here to a site built with Corifeu

A staged path from what exists today to Corifeu building a real Next.js site, with every stage
ending in something observable. Acceptance criteria are written as **observations, not
intentions**, the same standard [roadmap.md](roadmap.md) holds itself to — a command that
passes, a number in a round record, a behaviour demonstrated.

## The goal, and what it assumes

**The goal.** A self-hosted Next.js site, built feature by feature, where Corifeu does the work
and the oracles decide when it is done.

**The assumption, stated because it changes everything below.** *Built with Corifeu* means the
hybrid arrangement this project was founded on: an expensive model plans and reviews, local
models do the scoped slices, and objective checks make that safe. It does not mean a 9B model
writing a Next.js site unattended.

That distinction is not caution, it is measurement. Round five delivered 17 of 20 on a four-line
function and **0 of 20** on a recursive nine-case function in the same single file. A site is
not between those two points.

If the goal is the stricter one — local models only, no expensive model in the loop — stage 3
is where the data says whether it is possible, and the rest of this plan would change.

## Where the progress number comes from

Twenty-four acceptance criteria, listed below. The percentage is how many are met, counted and
not estimated. No criterion is met by writing code; each is met by running something and
reading the output.

| Stage | What it unlocks | Criteria |
|---|---|---:|
| S1 · Corifeu can read a Node repo | everything after it | **4 ✅** |
| S2 · `corifeu run`, one file | the first real use | **5 ✅** |
| S3 · Measured on TypeScript | whether the plan survives | **3 ✅** |
| S4 · A slice that touches several files | a feature instead of a function | 4 |
| S5 · Hybrid roles, cost measured | the project's actual claim | 3 |
| S6 · The planner cuts a brief | writing in Portuguese instead of JSON | 2 |
| S7 · The site | the goal | 3 |
| | | **24** |

**Today: 12 of 24** — S1, S2 and S3 complete.

---

## S1 — Corifeu can read a Node repo

**Why first.** It is the cheapest of the four gaps and everything else needs it. Corifeu counts
test cases from `cargo`, `go` and `pytest`. It cannot read `vitest`, so it cannot tell a passing
suite from a broken one in any JavaScript project, which makes
[ADR-0057](adr/0057-oracle-integrity-by-test-inventory.md)'s guard unavailable — and that guard
is what stops a doer quietly deleting tests.

**Deliverable.** A vitest adapter in `corifeu-check`, and a TypeScript measurement task.

**Done**, 2026-10-16. The samples were captured from vitest 3.2.4 on node 24.15.0 rather than
written from memory, and capturing them found the case that matters: a file which fails to
import contributes **nothing** to the `Tests` line while the files that loaded report normally,
so counting the rest would let ADR-0057's guard pass on a suite that had lost a whole file.
`vitest` announces it as `Failed Suites`, and that is what the parser declines on.

Building the task also caught two defects in the harness itself — 32 and 33 — both of the same
shape: a per-language lookalike where the real parser should have been asked.

**Acceptance.**
1. `corifeu-check` counts cases from **captured vitest output**, including the filtered form a
   per-slice oracle produces, with the samples in the test file.
2. A deliberately broken suite — a file that fails to import — returns `Cases::Unknown` rather
   than a count, as it does for `cargo` and `pytest`.
3. `bench/tasks/g-glob-ts` exists: TypeScript, vitest oracle, the reference passing every case, and
   each slice's oracle selecting the count its manifest declares.
4. `bench/run.sh` runs a cell against it end to end, with the oracle preflight passing.

**What would say stop.** Nothing plausible. This is adapter work of the kind
[ADR-0064](adr/0064-a-model-writes-the-rule-it-is-not-the-rule.md) describes.

---

## S2 — `corifeu run`, one file

**Why.** There is no `corifeu run`. Every number in six rounds came from `bench/src/bin/rank.rs`,
a measurement driver. The product cannot be pointed at a repository at all.

There is a second half nobody would guess: **the configuration is declared and not consumed.**
`limits.steps` exists in `corifeu.yaml` and nothing converts it into the runtime's `Limits`.
A `run` command that ignored the project file would be worse than none.

**Deliverable.** `corifeu run`, and the conversion from configuration to runtime.

**Done**, 2026-10-16. Criterion 9 is the one worth reading: with no provider listening at all,
`corifeu run` against a project whose suite fails to load reported *the check cannot judge this*
rather than *provider failed*. There was nothing to dispatch to and it never tried, which is
what "before the first dispatch" means.

Two defects came out of it. 35 was mine and the configuration validator caught it before any
run: `ownership` is keyed by file class and I read it as keyed by role. 34 was the runtime's and
only a real project found it — every successful write counted as an artefact change, including
one that wrote identical bytes, so a model rewriting the same file reset the loop detector on
every step and was never warned.

**Acceptance.**
5. `corifeu run --task "<what to do>"` turns a failing oracle green in a scratch repository, with
   the artefact, the oracle and the limits all read from `corifeu.yaml`.
6. The step allowance, the repeat threshold and the seed in the project file **reach the doer** —
   demonstrated by setting `limits.repeats: 1` and observing a turn stop at the first repeat.
7. A write outside the role's owned classes is denied, and the denial names the class
   ([ADR-0035](adr/0035-file-ownership-by-glob-classes.md)).
8. Checkpoints are written under `.corifeu/` and a regression is restored
   ([ADR-0051](adr/0051-checkpoint-before-every-write.md)).
9. `corifeu run` against a project whose oracle cannot execute refuses before the first
   dispatch, naming the command ([ADR-0067](adr/0067-a-check-that-cannot-be-counted-is-not-a-verdict.md)).

**What would say stop.** Nothing. This is the roadmap's phase 4 deliverable, overdue.

---

## S3 — Measured on TypeScript

**Why.** Every delivery number in this project is Rust or Python, on a single file with no
dependencies and a suite that runs in milliseconds. A Node project has `node_modules`, a build
step, and a test runner that takes seconds to start. Whether any of the measured behaviour
survives that is unknown, and guessing is what this project does not do.

**Deliverable.** Round seven: `g-ts`, both models, seeded, with the three legs of the floor
sweep on one driver so [Q5](measurement/round5-cells4-6.md#q5--lowering-the-floor-to-eight-cost-nothing-measurable)
finally closes.

**Done**, 2026-10-16 — [round7.md](measurement/round7.md). Eight passes of ten iterations, and
**80 of 80 delivered the whole task**. Asked for a four-line function, the model wrote all four
in one `write_file` and the other three slices arrived at an oracle already green. 99 steps in
the whole round, **100% of them productive against Rust's 36%**.

Two parts of the deliverable above were not met, and the reasons are different.

- **One model, not two.** The model set was cleared and rebuilt around a single model, so
  `n=10 per model` is n=10 on one. The Rust comparison is still the same `rank` mode on the
  same decomposition.
- **The floor sweep could not run, and Q5 stays open.** A sweep measures a step allowance, and
  this task never reaches one: the allowance was 12 and every iteration used 1, with
  `floor_left=false` throughout. Q5 needs a task hard enough to spend an allowance, which is a
  different task rather than a longer round. The same limit means this cell says nothing about
  repeats, rollback or the ceiling on TypeScript.

**Acceptance.**
10. A delivery rate for `g-ts` at n=10 per model, recorded like every other cell.
11. [Q14](measurement/round6.md) answered: two runs at one seed agree per slice, or the record
    says Corifeu cannot replay a round on this stack.
12. The share of steps that produce work, reported for TypeScript next to Rust's 36%.

**What would have said stop.** A delivery rate near zero. It came back at 100%, so the hybrid
split stays as planned and S4 to S7 are unchanged.

**What the result does not say.** `--mode slices` keeps every test in the artefact the model is
shown, so `26/26` means it satisfied cases it could read; there is no held-out suite in this
mode. Round five's Rust cell had the same visibility and still delivered 17 of 20 in 289 steps,
so the comparison holds, but nothing here measures generalisation.

---

## S4 — A slice that touches more than one file

**Why.** `Slice { artefact: String }` — one file. In a Next.js site a page, its component, a
route handler and the test change together. This is the deepest gap and it is not a field: the
prompt pins the artefact, the guard classifies by it, and the checkpoint stores it.

**Deliverable.** A slice that carries a set of artefacts, through the prompt, the guard and the
checkpoint.

**Acceptance.**
13. A slice naming two files has both pinned into every step, read fresh from disk.
14. A write to either is allowed and a write to a third file is denied.
15. A regression restores **both** files to the better state, not one of them
    ([ADR-0051](adr/0051-checkpoint-before-every-write.md) scoped per slice, not per file).
16. A component and its test change in one slice and the oracle goes green.

**What would say stop.** The context cost. Two files in every step may blow the window that
[round five measured as flat](measurement/round5-cell1.md). If prompt tokens per step rise
sharply with the number of artefacts, slices stay single-file and the planner has to cut around
that limit instead.

---

## S5 — Hybrid roles, and the cost claim measured

**Why.** This is the project's founding claim and it has never been tested. Six rounds ran two
local 9B models and nothing else. Nobody has measured what fraction of a real task's tokens can
leave the expensive model.

**Deliverable.** Roles bound to different providers, and a round that records the split.

**Acceptance.**
17. One task runs with a planner on an expensive provider and a coder on a local one, both
    declared in `corifeu.yaml`.
18. The run record carries **tokens per role and per provider**, so the split is a number.
19. A feature is delivered end to end this way in a scratch repository, with the expensive
    model's share reported.

**What would say the claim is weaker than hoped.** The expensive model's share staying high
because local coders keep failing and the plan keeps being revised. That is a measurement, not a
defeat, and it names what to improve.

---

## S6 — The planner cuts a brief

**Why.** Every decomposition in six rounds was written by hand in `task.json`.
[ADR-0056](adr/0056-one-oracle-per-slice.md)'s planner exists — 492 lines — and has never cut up
a task that a doer then worked.

**Deliverable.** A brief in plain language becoming a plan that doers work unedited.

**Acceptance.**
20. A brief in Portuguese produces a plan with one oracle per slice, and the oracles run.
21. That plan is worked end to end with **no hand editing between planning and coding**.

**What would say stop.** Plans that need editing every time. Then the planner's output becomes a
draft a human approves, which is a smaller claim and still useful.

---

## S7 — The site

**Why last.** Everything above is what makes this possible rather than a demonstration.

**One decision made here and worth stating.** Corifeu does **not** generate the scaffold.
`create-next-app` produces a correct, conventional project deterministically and in seconds; a
model reproducing it is slower, worse, and a bad first test. Corifeu's job starts at the first
feature.

**Acceptance.**
22. The repository exists, scaffolded by the generator, with vitest and Playwright running in
    CI and a `corifeu.yaml` that `corifeu doctor` passes.
23. One feature — a real one, chosen from what the site needs — delivered by `corifeu run`, with
    the oracle green and the diff reviewed.
24. A second feature delivered with **no human edit to the code**, only to the brief.

---

## What this plan does not promise

- **A date.** Every stage is sized by what has to be observed, not by how long it takes.
- **That local models will be enough.** S3 is where that is decided, and the plan says what
  changes if the answer is no.
- **That the site will be fast to build this way.** Building it by hand with an expensive model
  would be quicker today. The point of doing it through Corifeu is the second site, and the
  fiftieth change to this one.
