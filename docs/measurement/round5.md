# Round five — what it has to answer, and what would falsify it

Written before the round runs, so that the questions are not chosen after the numbers are in.

The fourth run produced two decisions, [ADR-0062](../adr/0062-step-allowance-has-a-floor-not-a-ceiling.md)
and [ADR-0063](../adr/0063-an-unfinished-slice-is-asked-again.md), and left four things it
could not say. Both decisions are now in the runtime, which means this round is also the first
test of them. Six questions, each with the cell that answers it and the result that would say
the answer is no.

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

**The cell.** A new task, **Task E**, whose slices are genuinely independent: separate
functions, no call between them, no shared type beyond `&str`. Same shape otherwise — one
file, one oracle per slice, all tests present from the start.

**What would say no.** Overflow rate near zero on Task E while Task C keeps producing it. Then
ADR-0058's "slices overflow" is a property of *that decomposition*, not of coders, and
ADR-0063's carrying on past a failed slice is paying for later slices that will not rescue
anything. Both would need narrowing to the case where slices share code.

**What would say yes.** Overflow on Task E too. Then it is about the model, not the task, and
both decisions stand as written.

**Status: Task E is designed below but not built.** It is the one piece of this round that
does not exist yet.

---

## Q3 — Does the floor of twelve steps hold in another language?

**Why it matters.** ADR-0062's floor is twelve because the slowest of the fourth run's 57
green slices took twelve steps. Every one of those slices was Rust, and a large part of the
work was done by compiler errors — a precise, immediate, free oracle that most languages do
not have. A floor measured on a language with that feedback is not obviously a floor anywhere
else.

**The cell.** **Task D**: Task C ported to Python, same four functions, same case counts,
`pytest` as the oracle. `bench/run.sh rank 10 <model>` against it, with the step allowance left
at its defaults.

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

## Task E, designed

Four functions in one file, no call between them, all tests present. Each takes and returns
plain data, so a coder implementing one has no reason to look at the next.

| Slice | Function | Cases |
|---|---|---|
| 1 | `roman(n: u32) -> String` — 1–3999 to Roman numerals | 6 |
| 2 | `wrap(text: &str, width: usize) -> Vec<String>` — greedy word wrap, never split a word | 7 |
| 3 | `base32(bytes: &[u8]) -> String` — RFC 4648 with padding | 6 |
| 4 | `luhn(digits: &str) -> bool` — checksum, spaces allowed, anything else rejected | 7 |

Nothing here calls anything else here. If overflow still happens, it is the model finishing
work it was not asked for, which is a different and more interesting finding than a coder
following a call chain.

The four were picked to be mutually unlike: a lookup table, a string loop, bit packing, and a
validator. A model that is good at one has no particular reason to be good at the next, which
keeps a slice from being easy only because the slice before it warmed the model up.

---

## What this round costs

The fourth run was 20 ranking iterations at a median of 5–10 minutes and 20 pair iterations,
about nine hours of machine time in total. This round is larger:

| Cell | Iterations | Rough |
|---|---|---|
| Q1 + Q4 — pair, slice 1 and slice 3, 2 models | 40 | 4 h |
| Q3 — Task D, rank, 2 models | 20 | 3 h |
| Q5 — Task C, rank, 3 floors, 2 models | 60 | 8 h |
| Q2 — Task E, rank, 2 models | 20 | 3 h |

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
