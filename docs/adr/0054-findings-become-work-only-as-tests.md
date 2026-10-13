# 0054 — A review finding becomes work only after QA reproduces it as a failing test

- **Status:** Accepted
- **Date:** 2026-10-13
- **Supports:** [ADR-0019](0019-tie-breaking-and-oracle-precedence.md), [ADR-0053](0053-qa-writes-the-tests.md)
- **Evidence:** [measurement](../measurement.md)

## Context
[ADR-0019](0019-tie-breaking-and-oracle-precedence.md) says oracles outrank opinions. The
measurement supplied the case that makes the rule concrete, and it cuts against the expensive
model rather than the cheap one.

Given a crate with **21 of 21 tests passing**, the reviewer declined to approve it and raised
three findings. Each was checked by running code:

| Finding | Claim | Verdict |
|---|---|---|
| Suffix on a short version | `"1.2-beta.999"` parses as 1.2.999 | **False.** It parses as 1.2.0 |
| Separator paths corrupt the scan | the scanner is fragile | **Not reproducible** over six realistic inputs |
| `"1.2.3.4"` accepted as 1.2.3 | true | **True**, and not required by the spec |

One finding named a concrete input and was wrong about it. Another named none. The review
prompt had asked for exactly those two things and warned that *a fabricated finding is worse
than an empty gate*.

The cost of acting on prose was measured separately. An earlier round handed the reviewer's
findings to the doer as text. The doer rewrote the whole file three times and left the crate
not compiling — it went from 18 of 21 passing to not building at all, acting on a diagnosis
it could not verify.

The reviewer was not useless. Its root-cause analysis on genuinely broken code was precise,
named the faulty expression, and was the thing that unblocked the task. The distinction is
between a reviewer looking at red and a reviewer looking at green: the first has evidence,
the second has an opinion.

## Decision
**A finding is not work until QA has reproduced it as a failing test.**

The loop:

```
reviewer  → states a defect, with the input that triggers it
qa        → tries to reproduce it against the current code
            cannot reproduce  → the finding is rejected and recorded as rejected
            reproduces        → writes a failing test; the finding becomes a slice
coder     → makes the test pass
```

1. **The coder never receives prose from the reviewer.** It receives a red test. This is the
   same shape that worked in the measurement: the run that reached green was driven by
   compiler errors, and the run that broke the build was driven by prose.
2. **A finding without a trigger is rejected without reproduction.** [ADR-0019]'s rule applied
   upstream: a claim that names no input is an opinion and carries no weight.
3. **Rejections are recorded against the reviewer.** The run holds the count of findings
   raised, reproduced and rejected.
4. **The test outlives the fix.** A reproduced finding leaves a permanent regression test, so
   the same defect cannot return silently.

## Alternatives considered
- **Pass findings straight to the coder.** What was measured. It broke a working build.
- **Have the coder judge the finding itself.** Asks the agent least able to be impartial about
  its own code to adjudicate criticism of it, and gives it a reason to argue instead of fix.
- **Trust a reviewer with a strong prompt.** The prompt already said a fabricated finding is
  worse than an empty gate, and forbade findings without a triggering input. Both rules were
  broken in one review.
- **Require the reviewer to supply the failing test itself.** Closer to right, and it makes
  the reviewer an author of the oracle, which [ADR-0053](0053-qa-writes-the-tests.md) keeps
  with QA. It also lets an unreproducible claim enter as a test that fails for the wrong
  reason.

## Consequences
- Review becomes cheaper per round and slower per finding. Each one costs a QA reproduction
  attempt before any repair work starts, and the ones that evaporate cost only that.
- **Reviewer precision becomes a measured quantity.** Findings raised against findings
  reproduced is a number per reviewer model, and it answers directly whether an expensive
  reviewer pays for itself. In the measurement it was 1 of 3.
- An approval is now meaningful in one direction only: a reviewer that finds nothing has
  given weak evidence, while a reviewer whose finding reproduces has given strong evidence.
  The gate should not treat silence as a pass on its own.
- Where a finding is about something no test can express — naming, structure, a readability
  judgement — it is advisory and never becomes a slice. Those belong in a report to the user,
  not in the coder's queue.
