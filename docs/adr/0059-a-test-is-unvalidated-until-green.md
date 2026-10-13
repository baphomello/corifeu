# 0059 — A test is unvalidated until it has gone green

- **Status:** Accepted
- **Date:** 2026-10-13
- **Refines:** [ADR-0053](0053-qa-writes-the-tests.md), [ADR-0056](0056-one-oracle-per-slice.md)
- **Supports:** [ADR-0019](0019-tie-breaking-and-oracle-precedence.md)
- **Evidence:** [measurement](../measurement.md#two-of-the-original-ten-tests-were-wrong)

## Context
[ADR-0053](0053-qa-writes-the-tests.md) has QA write the test first and the coder turn red
into green. [ADR-0019](0019-tie-breaking-and-oracle-precedence.md) says oracles outrank
opinions. Together they give a red test the authority to direct work, and a red test has
earned none of it: it fails, and it would fail exactly the same way if it were wrong.

The measurement supplied the case. The monolithic Task C attempt wrote ten tests and an
implementation that never terminated, so the suite never ran and nothing was learned about
either half. Those same ten tests were later run against the sliced implementation, code they
had never seen. Eight passed. Both failures are errors in the tests:

| Test | Asserts | The specification says |
|---|---|---|
| `question_mark_single_char` | `x?.rs` matches `x_a.rs` and not `xa.rs` | `?` matches exactly one character, so both assertions are inverted |
| `star_excludes_slash` | `**/*.cfg` matches `a/b.ccfg` | `b.ccfg` does not end in `.cfg` |

Task C therefore failed for 135 minutes holding a non-terminating implementation *and* a suite
with two wrong expectations, and the second defect was invisible because the first one hid it.
A run that never reaches green learns nothing about its own oracle.

[ADR-0056](0056-one-oracle-per-slice.md) sends a slice that exhausts its repair budget back to
the planner, on the theory that the decomposition was wrong. That is one of three suspects.

## Decision
**When a slice exhausts its repair budget without its oracle ever going green, the test goes
back to QA before the plan goes back to the planner.**

1. **A test carries a status.** `unvalidated` from the moment QA writes it, `validated` the
   first time it passes. Only a validated test is an oracle in
   [ADR-0019](0019-tie-breaking-and-oracle-precedence.md)'s sense.
2. **Budget exhaustion goes to QA first.** QA re-reads the test against the specification. If
   the test is wrong it is corrected and the slice gets a fresh budget. Only if the test holds
   does the slice return to the planner as a decomposition failure.
3. **A check that never terminates invalidates the whole suite it was in.**
   [ADR-0049](0049-faithful-check-results.md)'s `did-not-terminate` verdict means no assertion
   in that run was evaluated, so no test in it gained any evidence.
4. **A red test must be red for the right reason.** A test that fails to compile is broken,
   not failing, and is QA's to fix before the slice is dispatched. The coder is never handed a
   slice whose oracle has not run.

## Alternatives considered
- **Trust QA's test.** What the measurement did. Two of ten were wrong, and it took a working
  implementation to find out.
- **Let the coder challenge the test.** [ADR-0054](0054-findings-become-work-only-as-tests.md)
  already rejects this shape: it asks the agent least able to be impartial to adjudicate the
  standard it is being held to, and gives it a reason to argue instead of fix.
- **Have the reviewer check the test before the work starts.** A second opinion on prose,
  which [ADR-0054](0054-findings-become-work-only-as-tests.md) ranks below a run. It also puts
  an expensive model on the critical path of every slice.
- **Send the slice to the planner first, as today.** Correct a third of the time by
  construction, and it re-plans around a test that will fail the next decomposition too.
- **Require QA to prove the test with a deliberately correct stub.** The strongest option: it
  makes every test validated before dispatch. It also asks QA to write the implementation,
  which is the separation [ADR-0053](0053-qa-writes-the-tests.md) exists to create.

## Consequences
- A slice's repair budget is consumed by two roles, so budgets are accounted per slice rather
  than per role.
- The count of unvalidated tests at the end of a run is a quality signal about the run itself.
  A plan that finished with every test validated is stronger evidence than one that abandoned
  slices, and the two currently report the same way.
- This costs a QA round on exactly the slices that are already expensive, which is where the
  budget has already been spent and where a wrong test is most likely.
- **It creates a way for QA to weaken a correct test to make a slice pass.** The mitigation is
  recording rather than prevention: the test's history is in the run, and a loosened assertion
  is visible in the diff. Stated here because the risk is real and unguarded.
- Nothing here applies to a test that has gone green and later fails. That is a regression,
  the test is validated, and [ADR-0054](0054-findings-become-work-only-as-tests.md)'s
  permanence rule governs it.
