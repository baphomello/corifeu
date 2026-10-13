# 0053 — The default role pair is QA and coder, with QA owning the tests

- **Status:** Accepted
- **Date:** 2026-10-13
- **Refines:** [ADR-0042](0042-test-authorship.md)
- **Evidence:** [measurement](../measurement.md)

## Context
[ADR-0042](0042-test-authorship.md) leaves test authorship to the user and warns when one
role writes both the code and the tests that judge it. That remains right: forbidding the
combination would be paternalistic, and a solo user on a small project may reasonably accept
it.

What ADR-0042 does not do is say what Corifeu should ship as the default, and the measurement
gives a reason to decide. The doer wrote its own tests and they were **good** — the reviewer
confirmed they honestly covered the stated criteria, and they were what eventually proved the
implementation correct. But they were also written from the same reading of the specification
that produced the implementation, and that shared reading contained the defect: the
requirement to extract a version from surrounding text was missed in the code and under-tested
in the suite, so the suite could not catch it. Three runs plateaued at 18 of 21 because the
blind spot was common to both halves of the work.

An independent author would not reliably have caught this either. But the failure mode —
one reading of the spec, one blind spot, reproduced in both the code and its oracle — is
exactly what a second author removes.

## Decision
**Corifeu ships a default configuration with two separate doer-side roles: `qa`, which owns
the `test` file class, and `coder`, which owns `source`.**

1. **QA writes the tests first.** A slice begins with a failing test, not with an
   implementation. The coder's task is to turn red into green.
2. **The coder cannot write tests.** Not by instruction — by the writable set its sandbox
   gives it ([ADR-0052](0052-write-restriction-in-the-sandbox.md)). A coder that can edit the
   oracle does not have one.
3. **They may share a model.** `qa` and `coder` are roles; assigning the same model to both
   is allowed and often sensible. They remain separate agents with separate contracts and no
   shared context ([ADR-0011](0011-user-defined-granularity.md)) — the independence that
   matters is of context, not of weights.
4. **ADR-0042's freedom is untouched.** A user may still give both classes to one role. It is
   no longer the default, and the warning stands.

## Alternatives considered
- **Keep a single doer that writes code and tests.** What was measured. It works, it is
  cheaper, and it puts the same blind spot on both sides of the check.
- **Make the split mandatory.** Contradicts [ADR-0011](0011-user-defined-granularity.md) and
  forces a second role on users who do not want one.
- **Require different models for QA and coder.** Doubles the independence argument and the
  setup cost. Context independence is the part that is load-bearing; model diversity is a
  refinement a user can choose.
- **Have the reviewer write the tests.** Conflates two jobs. The reviewer looks at finished
  work; QA defines what finished means before the work starts.

## Consequences
- The `few-roles` preset gains a second doer-side role, so "few" now means three rather than
  two. The simplest preset is still available by configuration.
- A slice's definition of done becomes a file, not a sentence. This is what makes
  [ADR-0056](0056-one-oracle-per-slice.md)'s one-oracle rule checkable.
- QA is on the critical path for every slice, so its latency is in every cycle. Where QA and
  coder share a local runtime, [ADR-0055](0055-one-local-doer-by-default.md) means they
  queue; the planner should expect serial cost.
- `corifeu doctor` should report whether the coder's writable set actually excludes the test
  class, since on a platform without per-role sandboxing it may not
  ([ADR-0052](0052-write-restriction-in-the-sandbox.md)).
