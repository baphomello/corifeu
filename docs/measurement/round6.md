# Round six — what the changes have to prove, and what would say they are wrong

Written before the round runs, so that the questions are not chosen after the numbers are in.

Round five answered eight of its twelve questions and produced seven decisions. Every one of
those decisions is now in the runtime as a **value with the measured behaviour as its default**,
which means none of them has changed anything yet and all of them are testable. That is the
point of this round: the fifth measured the system, the sixth measures the decisions taken on
it.

One change could not be made that way and it governs the whole round.

## The prompt moved, so cell 0 is the baseline

D6 changed the doer's system prompt: the rule that was violated 507 times was last in a list of
six and stated a preference, and it now leads and states the consequence. Rounds one to five
share a prompt; this round does not share it with them.

**No step count from round five is comparable with this round's.** The first cell therefore
re-runs round five's first cell exactly — Task C, `rank`, the defaults, `BENCH_SEED=off` — and
every comparison in this document is against *that*, not against
[round5-cell1.md](round5-cell1.md).

| | Round 5, cell 1 | What cell 0 must reproduce |
|---|---:|---|
| Complete crate, 26/26 | 17 of 20 | within a few runs of it |
| Productive steps | 37% | **the number this round compares against** |
| Steps cut off at `num_predict` | 25% | **the number D6 claims to move** |

If cell 0 comes back materially *worse* than round five's cell 1 on delivery, the prompt change
is a regression and the round stops there. That is the one result that ends the round rather
than informing it.

---

## Q14 — Does the seed make a round replayable?

**Why it matters.** Defect 26. The same model against the same slice reached green between four
and ten times in ten, so round five's Q7 — whether a slice is hard or hard for one model —
could not be answered at all, and no number in five rounds can be checked by re-running it.

**The cell.** Task C, `rank`, n=3, run **twice** with `BENCH_SEED=1`, nothing else changed.

**What would say yes.** The two runs agree per slice: same outcome, same step count. Byte
equality is not required and should not be claimed —
[`Budget::seed`](../../crates/corifeu-provider/src/lib.rs) says reproducible, not bit-exact,
because a runtime that splits a batch differently across devices can still diverge.

**What would say no.** Outcomes differing between the two runs. Then the seed is not reaching
the sampler, or the local runtime is not honouring it, and the honest record is that Corifeu
cannot replay a round on this stack — which is worth knowing before the next five rounds are
spent on it.

**The trap.** Agreement here proves the sampler is seeded, not that the whole pipeline is
deterministic. The oracle shells out to `cargo`, and a compile that times out differently is
enough to diverge. A disagreement must be read off `steps.tsv` before it is blamed on the seed.

---

## Q15 — What does stopping at the second repeat cost and save?

**Why it matters.** Round five priced ADR-0050's threshold of six across three cells and two
languages: **54 slices repeated an action and 2 reached green**, while the steps spent after a
first repeat were **30% of everything the round spent**. D2 made the threshold a number.
Whether it should be two, or one, or stay six, is this question.

**The cell.** Task C, `rank`, n=10, at three thresholds:

| `BENCH_REPEATS` | What it tests |
|---|---|
| 1 | Stop at the first repeat. Defect 1's original behaviour, never measured. |
| 2 | The candidate. |
| 6 | The default, and the baseline. |

**The numbers to watch.** Delivery rate, and total steps. The claim is that 1 or 2 costs at most
a run or two of delivery and saves about a third of the steps.

**What would say leave it at six.** Delivery falling by more than the 3.7% rescue rate implies.
Then a repeat is a *symptom* of a slice already failing rather than a cause, and the five steps
are buying something the rescue count does not capture — most likely the slices that recovered
without the detector ever being consulted again.

**What would say go to one.** Threshold 1 losing nothing that 2 kept. Then the warning buys
nothing at all and ADR-0050's reversal of defect 1 was wrong.

---

## Q16 — Does the unusable-oracle guard ever fire when it should not?

**Why it matters.** Defect 24 cost round five eight slices: `cargo` could not run and the doer
read it as red. `Outcome::OracleUnusable` now ends a slice before the first dispatch when the
check produces no case count. The risk is the opposite error — refusing a slice whose work is
merely unfinished.

**The cell.** Every cell in this round. The guard either fires or it does not, and it is counted
in `results.tsv` as `ORACLE_UNUSABLE`.

**What would say yes, it misfires.** Any `ORACLE_UNUSABLE` row in a cell whose preflight passed.
The preflight proves the oracle runs before the round starts, so a slice reporting it afterwards
means the guard is reading something else as a broken check — and a false positive here refuses
work rather than wasting it, which is worse.

**What would say it is right.** Zero rows across the round. The guard is insurance, and the
preflight is what makes it unnecessary; both firing never is the correct outcome.

---

## Q17 — Does the carry-on policy matter outside Task C?

**Why it matters.** Round five measured the end-of-plan re-probe rescuing **66%** of unclosed
slices in chained Rust, **20%** in the same plan in Python and **2%** in a plan whose slices call
nothing. D3 made the policy a value. `Carry::UntilOverlap` is a hypothesis and
[its doc comment](../../crates/corifeu-doer/src/pair.rs) names the case that argues against it:
runs that closed one slice of four and had three rescued delivered the complete crate, and all
three failures came before any overlap had shown itself.

**The cell.** `e-indep`, `rank`, n=10, with `Carry::Never` against `Carry::Always`. Task E is
where carrying on is worth 2%, so it is where stopping should cost nothing.

**What would say the policy is worth having.** `Never` on Task E delivering the same and spending
materially fewer steps.

**What would say no.** `Never` losing deliveries even on Task E. Then carrying on is paying for
something other than the re-probe rescue — a later slice leaving the artefact in a state an
earlier slice's oracle likes — and ADR-0063's second half is load-bearing for a reason nobody
has named.

**Not asked.** `UntilOverlap`'s patience. There is no cell for it, because the signal it keys on
cannot be shown to predict anything yet. Deciding it needs ADR-0056's planner to declare which
slices share code, and the planner has never cut up a task.

---

## Q18 — Does the rollback fire when there is something to regress?

**Why it matters.** ADR-0051 checkpoints before every write and restores a state scoring worse.
It fired **four times in 1,076 steps** across round five, three of them in the one cell whose
slices are whole algorithms. D7 asked to measure it before deciding, and round five's tasks
could not: a slice whose cases are independent has nothing to regress.

**The cell.** [`f-expr`](../../bench/tasks/f-expr), `rank`, n=10. Two slices, the second holding
sixteen cases that pull against each other — precedence against unary minus, total division
against left associativity.

**What would say the mechanism earns its place.** Rollbacks firing at a materially higher rate,
and the runs that rolled back delivering at least as often as those that did not.

**What would say remove it.** Still near zero. Then checkpoint-before-every-write is protecting
against something that does not happen at any slice size Corifeu has ever been given, and the
honest move is to say so in the ADR rather than keep untested insurance.

**What would say it is harmful.** Rollbacks firing and those runs delivering *less* often. Then
the restore is undoing work mid-thought and the comparison scope is wrong.

---

## Q5's third leg, and a confound that arrived with it

Round five ran the floor sweep at 12 and at 8 and they were indistinguishable: 9 of 20
delivered either way, total steps within 2%, with the allowance growing in 6 of 20 iterations
at floor 8 against 4 of 20 at floor 12. The third leg, `floor = 30`, was not run.

**It cannot simply be run later.** The GPU driver was updated between round five and this one
(610.88, with 617.42 offered mid-round), and a leg measured on a different stack is not a leg of
the same sweep. Either all three legs run together on one driver, or the sweep stays at two.

Defect 31: `round.txt` now records the driver version, for the same reason it records the
commit and the seed.

## Q1 and Q4 carry over

Round five's pair cells ran against the frozen runtime and are recorded separately. They are not
re-asked here: the held-out judge is still Rust-only and nothing on this branch changed it.

## What this round still will not say

- **Anything about a cloud model.** Six rounds of two local 9B models on one machine. The
  product's claim is that local models can do work currently sent to an expensive one, and the
  comparison has never been run.
- **Anything about a real repository.** Every task is one file, no dependencies, a suite that
  runs in milliseconds. The slice that matters in practice is inside a crate that takes two
  minutes to compile.
- **Anything about the planner.** Every decomposition in six rounds was written by hand.
  [ADR-0056](../adr/0056-one-oracle-per-slice.md)'s planner has been built and has never cut up
  a task that was then worked by a doer. Q17's unasked half needs it.
