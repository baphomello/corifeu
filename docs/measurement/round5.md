# Round five — what it has to answer, and what would falsify it

Written before the round runs, so that the questions are not chosen after the numbers are in.

The fourth run produced two decisions, [ADR-0062](../adr/0062-step-allowance-has-a-floor-not-a-ceiling.md)
and [ADR-0063](../adr/0063-an-unfinished-slice-is-asked-again.md), and left four things it
could not say. Both decisions are now in the runtime, which means this round is also the first
test of them. Twelve questions, each with the cell that answers it and the result that would
say the answer is no.

**Q1 to Q6 are what the round is for. Q7 to Q12 are what the same machine time can also
answer**, at the cost of writing each run's steps down instead of only its totals. They add no
cells and no hours. They are written here, before the round, for the same reason the first six
are: a question chosen after the numbers are in is not a question, it is a description.

---

## Q1 — Is `SelfTested` code worse?

**Why it matters.** [ADR-0053](../adr/0053-qa-writes-the-tests.md) separates QA from the coder
on the argument that two readings of a specification have different blind spots. The fourth
run found that in **11 of 20** pairs QA wrote the implementation as well as the test, so there
was one reading, not two. The ADR's whole premise is either untouched by that or destroyed by
it, and what decides which is whether the code is worse — which 40 runs of counting who typed
what cannot say.

**The cell.** `bench/run.sh pair 10 <model>` × 2 models, as before, with the held-out suite now
running at the end of every iteration: twelve cases in
[`bench/tasks/c-glob/held-out/slice1.rs`](../../bench/tasks/c-glob/held-out/slice1.rs) that no
run is ever shown. Every case passes against the reference implementation.

**The comparison.** Held-out score of the `SELF_TESTED` runs against the held-out score of the
`GREEN` runs. The crate is identical in shape; only the authorship differs.

**What would say no.** The two groups score the same. Then ADR-0053's separation is buying
nothing measurable on a slice this size, and the honest move is ADR-0053's third option:
accept one doer with two contracts, and stop paying for a separation that does not show up in
the work.

**What would say yes.** `SELF_TESTED` scores lower. Then the separation is worth enforcing,
and defect 14 — Rust putting the unit test inside the file under test, so ownership cannot
separate the roles — becomes a problem to solve rather than a fact to record.

**The trap.** 20 runs split 11/7 is a thin base for a difference of one or two cases. A split
that lands inside the noise is not an answer, and this round should say so rather than pick
the side it likes. If the gap is small, the cell runs again at n=25.

---

## Q2 — Does a boundary overflow outside Task C?

**Why it matters.** [ADR-0058](../adr/0058-slice-boundaries-are-observed.md) and now
[ADR-0063](../adr/0063-an-unfinished-slice-is-asked-again.md) both rest on coders writing more
than their slice asked for. Every observation of that came from Task C, whose four slices
share one module, one data type and a helper chain: `classify` calls `glob_match` calls
`match_seg` calls `split_segs`. A coder implementing one of them has the next one in front of
it. That is the easiest possible case for overflow, and it may be the only one.

**The cell.** [`bench/tasks/e-indep`](../../bench/tasks/e-indep), whose slices are genuinely
independent: separate functions, no call between them, no shared helper, no shared type. Same
shape otherwise — one file, one oracle per slice, 26 cases, all present from the start.

```sh
BENCH_TASK=e-indep bench/run.sh rank 10 <model>
```

**What would say no.** Overflow rate near zero on Task E while Task C keeps producing it. Then
ADR-0058's "slices overflow" is a property of *that decomposition*, not of coders, and
ADR-0063's carrying on past a failed slice is paying for later slices that will not rescue
anything. Both would need narrowing to the case where slices share code.

**What would say yes.** Overflow on Task E too. Then it is about the model, not the task, and
both decisions stand as written.

The four were picked to be mutually unlike — a lookup table, a string loop, bit packing, and a
validator — so that a slice is not easy merely because the slice before it warmed the model up.

| Slice | Function | Cases |
|---|---:|---:|
| 1 | `roman(n: u32) -> String` — 1–3999 | 6 |
| 2 | `wrap(text: &str, width: usize) -> Vec<String>` — greedy, never splits a word | 7 |
| 3 | `base32(bytes: &[u8]) -> String` — RFC 4648, padded | 6 |
| 4 | `luhn(digits: &str) -> bool` — spaces ignored, anything else rejected | 7 |

---

## Q3 — Does the floor of twelve steps hold in another language?

**Why it matters.** ADR-0062's floor is twelve because the slowest of the fourth run's 57
green slices took twelve steps. Every one of those slices was Rust, and a large part of the
work was done by compiler errors — a precise, immediate, free oracle that most languages do
not have. A floor measured on a language with that feedback is not obviously a floor anywhere
else.

**The cell.** [`bench/tasks/d-glob-py`](../../bench/tasks/d-glob-py): Task C ported to Python,
same four functions, same helper chain, the same 4-8-9-5 split of the 26 cases, `pytest` as the
oracle. A port and not a rewrite, so that the only difference between this cell and Q5's is the
language.

```sh
BENCH_TASK=d-glob-py bench/run.sh rank 10 <model>
```

The step allowance stays at its defaults, because what is being tested is whether those
defaults travel.

Verified before the round: the reference passes 26 of 26; the scaffold leaves four stubs and 26
cases, all red; and each slice's oracle selects exactly the count `task.json` declares — 4, 8, 9
and 5, with the other slices' cases deselected rather than counted.

**What would say no.** Green slices on Task D routinely taking more than twelve steps. Then
twelve is a Rust number, the floor belongs in the per-project configuration rather than in the
runtime's defaults, and `limits.steps.floor` needs a documented value per language.

**What would say yes.** The same split: green slices well under twelve, stuck ones past it.
Then the floor is about how a model works, not about what the compiler tells it.

**What this round made possible.** Corifeu could not count `pytest` cases until today, so
[ADR-0057](../adr/0057-oracle-integrity-by-test-inventory.md)'s guard was unavailable outside
Rust and Go, and this question could not be asked at all.

---

## Q4 — Does the pair behave the same on an expensive slice?

**Why it matters.** The pair cell used slice 1, `split_segs`: four lines of implementation and
four cases. Writing the test and writing the implementation cost about the same, which is
exactly the condition under which QA would write both without noticing. On a slice where the
implementation is ten times the test, the incentive is different.

**The cell.** The same pair run against **slice 3**, `glob_match` — nine cases, recursive, the
hardest of the four, and the one the fourth run's single doer spent the most steps on. Its own
held-out suite, built the same way as slice 1's.

**What would say no.** `SELF_TESTED` stays near 55%. Then QA writing the implementation is not
about the work being small, and the fourth run's 11 of 20 generalises.

**What would say yes.** `SELF_TESTED` collapses on slice 3. Then the fourth run measured a
property of a four-line function, and the pair's behaviour has to be stated per slice size
rather than as one number.

---

## Q5 — What does the floor actually cost?

**Why it matters.** ADR-0062's floor of twelve was derived from the fourth run's own numbers,
which means it was fitted to them. A limit chosen to cut nothing in the data that produced it
will cut nothing in that data. The question is what it cuts in data it has not seen.

**The cell.** Task C, `rank`, n=10, at three floors:

| `BENCH_STEPS_FLOOR` | What it tests |
|---|---|
| 8 | Too low on purpose. Some green slices should start dying. |
| 12 | The default. |
| 30 | The fourth run's constant, as the baseline. |

Ceiling, patience and raise_by stay at their defaults throughout, so the only variable is where
a turn starts.

**What would say the default is wrong.** Floor 8 losing nothing that floor 12 kept — then
twelve is generous and the floor should come down. Or floor 12 losing green slices that floor
30 kept — then the fourth run's 57-slice sample missed a tail, and the floor should go up or
the patience come down.

**The number to watch.** Not minutes. `floor_left` in the `TOTAL` row: how often the allowance
had to grow at all. A floor that is right grows rarely; a floor that is too low grows on
every run and has simply reinvented the ceiling with extra steps spent getting there.

---

## Q6 — Does asking again change the verdict, and what does carrying on cost?

**Why it matters.** ADR-0063 claims two things. That a slice which did not close is often
green once the rest of the plan has run — eight of nineteen in the fourth run. And that
carrying on past a failure is cheaper than re-planning. The first is a re-reading of data
already collected; the second has never been measured, because the fourth run's harness
carried on by accident rather than by decision.

**The cell.** Every `rank` cell in this round already emits it: one `AGAIN` row per unfinished
slice, `GREEN` or `RED`, and `overtaken=` in the `TOTAL` row.

**What would say no.** `AGAIN` rows coming back `RED` most of the time. Then a slice that does
not close is usually work that is actually missing, carrying on spends later slices'
allowances for nothing, and ADR-0063 should go back to stopping at the first failure with only
the end-of-plan re-probe kept.

**The cost to record.** Steps spent on slices *after* the first failure, against how often
those slices closed. That is the price of ADR-0063, and nobody has paid it yet with a number
attached.

---

---

# The free half

Every run already produces a step-by-step trace and every driver until now discarded it. The
round writes `steps.tsv`: one row per step, with the iteration, the model, the slice, the role,
the tool, its identifying argument and the one-line outcome. Three markers in that outcome are
not reconstructable from the summary rows — `ROLLED BACK` is
[ADR-0051](../adr/0051-checkpoint-before-every-write.md) firing, `REPEAT` is
[ADR-0050](../adr/0050-loop-detection-warns-before-it-stops.md) warning, and a `say` step is a
turn that ended without calling a tool.

The six questions below cost no extra machine time. Each names the mechanism it tests, all of
which are shipped and none of which has ever been measured.

## Q7 — Is a slice hard, or is it hard for one model?

**Why it matters.** `scaffold.py` is deterministic, so iteration *i* hands both models an
identical starting state. Every `rank` cell is therefore a **paired** comparison and the fourth
run did not use it: it reported each model's totals as if the two had been given different
work. Whether hardness lives in the task or in the model decides where `limits.steps` belongs —
one number in the project's configuration, or one per model.

**The cell.** Every `rank` cell. No new code: `iteration` and `model` are already columns.

**The comparison.** For each (iteration, slice), did both models reach it, neither, or one. Then
the step counts of the slices both reached.

**What would say hardness is the task's.** Failures concentrating on the same slices for both
models. Then one floor serves both and ADR-0062's defaults stay where they are.

**What would say it is the model's.** Failures landing on different slices. Then a single floor
is an average of two different machines and the allowance has to move into the model's profile.

## Q8 — How often does a turn cost a step and do nothing?

**Why it matters.** A turn that answers without calling a tool spends a dispatch and a step off
the allowance, and produces nothing but a nudge. `num_predict` is 4096. If this is rare it is
noise; if it is one step in five then ADR-0062's floor of twelve is really a floor of nine, and
the output budget is the defect rather than the allowance.

**The cell.** Every cell. Count `say` steps in `steps.tsv` against total steps, split by whether
the outcome names a spent budget.

**What would say the floor is wrong.** More than about 10% of steps producing nothing. Then the
floor was measured in units that include waste and the budget has to be raised before the floor
means anything.

## Q9 — Does ADR-0051's rollback ever fire?

**Why it matters.** Checkpoint-before-every-write, scoring by cases passing, and restoring a
regression is a shipped mechanism with no measurement behind it at all. It may be protecting
against something that does not happen at this slice size, in which case it is ceremony. It may
also be firing often and fighting the model.

**The cell.** Every cell. Count `ROLLED BACK` in `steps.tsv`, by run.

**What would say it is ceremony.** Zero or near-zero firings across the whole round. Then the
protection is untested rather than proven, and that should be said out loud rather than assumed.

**What would say it is harmful.** Runs that rolled back reaching green less often than runs that
did not. Then the restore is undoing work the model was in the middle of and the scope of the
comparison is wrong.

## Q10 — Does warning a looping model work?

**Why it matters.** [ADR-0050](../adr/0050-loop-detection-warns-before-it-stops.md) chose to
warn at a repeat and stop at six, reversing defect 1, which was stopping at the first. The
choice cost five extra steps per loop and nobody has measured whether a warned model changes
course.

**The cell.** Every cell. Of the runs with a `REPEAT` step, how many then reached green, and how
many went on to `LOOPING` anyway.

**What would say the warning earns its steps.** A meaningful share of warned runs recovering.

**What would say no.** Warned runs reaching `LOOPING` almost always. Then the five steps between
the first repeat and the sixth are spent watching a model fail, and the threshold should come
down.

## Q11 — Is an expensive run slow, or just long?

**Why it matters.** The fourth run's stuck runs took 17.5 minutes against 4.5. If that is more
steps, the allowance is the lever. If it is slower steps, the one-hour cap is cutting runs that
would have finished and the cap is the lever. These are different fixes and the fourth run
cannot tell them apart.

**The cell.** Every cell: `seconds` divided by `steps` per slice, grouped by outcome.

**What would say the cap is the problem.** Seconds-per-step rising with the step number inside a
run. Then the context is growing and the run is getting slower as it goes, which the cap then
truncates.

## Q12 — Does the window explain the expensive runs?

**Why it matters.** The fourth run's stuck runs carried 93,697 prompt tokens against 18,016 —
five times as many. Whether that is five times the steps or a window that stopped compacting is
the difference between a planning problem and a context problem. `num_ctx` is 8192.

**The cell.** Every cell: `prompt` divided by `steps`, grouped by outcome.

**What would say it is the window.** Prompt tokens per step materially higher on the expensive
runs. Then the window is not holding its budget and the step count is a symptom.

**What would say it is not.** Prompt per step flat across both groups. Then the expensive runs
are expensive only because they are long, and ADR-0062 already addresses that.

---

## What this round costs

The fourth run was 20 ranking iterations at a median of 5–10 minutes and 20 pair iterations,
about nine hours of machine time in total. This round is larger:

| Cell | Iterations | Rough |
|---|---|---|
| Q1 + Q4 — pair, slice 1 and slice 3, 2 models | 40 | 4 h |
| Q3 — Task D, rank, 2 models | 20 | 3 h |
| Q5 — Task C, rank, 3 floors, 2 models | 60 | 8 h |
| Q2 — `e-indep`, rank, 2 models | 20 | 3 h |

Q5 is the expensive one and the most fitted to data it already saw. If the round has to be cut,
cut Q5 to floors 12 and 30 and spend the hours on Q2, which is the question whose answer could
invalidate a decision already shipped.

## What this round still will not say

- **Anything about a cloud model.** Every number here is two local 9B models on one machine.
  The product's claim is that local models can do the work that is currently sent to an
  expensive one, and nothing measured so far has run the comparison.
- **Anything about a real repository.** Every task is one file with no dependencies, no build
  system to fight, and a test suite that runs in milliseconds. The slice that matters in
  practice is one inside a crate that takes two minutes to compile.
- **Anything about the planner.** Every decomposition in every round so far was written by
  hand. [ADR-0056](../adr/0056-one-oracle-per-slice.md)'s planner has been built and has never
  cut up a task that was then worked by a doer.
