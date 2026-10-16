# 0067 — A check that cannot be counted is not a verdict on the code

- **Status:** Accepted
- **Date:** 2026-10-15
- **Refines:** [ADR-0049](0049-faithful-check-results.md), [ADR-0057](0057-oracle-integrity-by-test-inventory.md)
- **Evidence:** defects 23 and 24, [round five](../measurement/round5-cell1.md)

## Context
[ADR-0049](0049-faithful-check-results.md) requires a check's result to reach the doer intact.
[ADR-0057](0057-oracle-integrity-by-test-inventory.md) guards against a suite that shrank, by
comparing the cases that ran against the count the slice declares. Neither covers the case where
the check does not run at all.

Round five's first cell found out what that costs. A scaffolded crate was written into a
directory inside Corifeu's own cargo workspace, so every `cargo` invocation answered

```
error: current package believes it's in a workspace when it's not
```

before compiling anything. That exits non-zero with no test summary, which is indistinguishable
from a crate that fails its tests. **Eight slices spent their whole allowance being told that
correct code was wrong** — the model had written

```rust
path.split('/').filter(|s| !s.is_empty()).collect()
```

on its second step, which is the right answer, and the oracle kept saying red for ten more steps.

`Report::unguarded` already said this. It is set when a slice declares a case count and the
check produced none, it has been tested since it was written, and nothing in the runtime read
it. The signal existed and was discarded, which is why the defect cost a cell rather than a
minute.

The honest difficulty is that "the tool could not run" and "the code does not compile" look the
same from outside: both exit non-zero with no case count, and telling them apart needs
tool-specific knowledge that [ADR-0064](0064-a-model-writes-the-rule-it-is-not-the-rule.md) says
belongs in an adapter rather than in the runtime. So the question is not asked that way.

## Decision
**Before the first dispatch, a slice that declares a case count runs its own check once. If the
check produces no count, the slice ends as `OracleUnusable` and no model is called.**

1. **The question is asked once, early, and of the untouched artefact.** A stubbed artefact still
   runs its cases and fails them, so a count exists; only a check that cannot execute produces
   none. That is what makes this tool-agnostic — it reads the shape of the result, not the text
   of an error.
2. **`Outcome::OracleUnusable { detail }`** carries which command could not be counted and what
   it means, because the next reader of that row is a human deciding whether the project is
   misconfigured.
3. **It ends the walk, not just the slice.** The slices of one plan share a project and
   therefore a test command, so a check that cannot be counted for one slice will not be
   countable for the next. Carrying on spends every remaining allowance to learn the same thing.
4. **It is not asked where no count was declared.** QA writing a test that does not exist yet has
   nothing to compare against, and asking there would refuse every pair before it started.
5. **The allowance does not grow on it.** An unusable oracle says nothing about whether the step
   budget was too small ([ADR-0062](0062-step-allowance-has-a-floor-not-a-ceiling.md)).

## Alternatives considered
- **Read the tool's error text.** `cargo`'s workspace error, `pytest`'s collection error, and so
  on. Accurate, per-tool, and exactly the proliferation ADR-0064 was written to avoid. It also
  fails on the tool nobody has met yet, which is every tool.
- **Treat an uncountable check as a rollback trigger**, as ADR-0057 does for a vacuous one. Wrong
  shape: there is nothing to restore to, because the check was broken before the first write.
- **Leave it to the caller.** The measurement harness now proves the oracle runs before a round
  starts, which protects the measurement and not the product. A user whose project is
  misconfigured gets the same eight wasted slices, with their own tokens.
- **Keep repairing and let the step allowance end it.** What round five did. It produces a run
  that reports failure on correct code, which is the one thing a verifier must never do.

## Consequences
- **A false positive refuses work rather than wasting it**, which is worse, so the negative case
  is the one under test: a slice whose work is merely unfinished must still run.
  [Round six's Q16](../measurement/round6.md) counts any `ORACLE_UNUSABLE` row in a cell whose
  preflight passed as a misfire.
- Callers matching on `Outcome` exhaustively gain an arm. There were four.
- One extra oracle run per slice, with no model. Drivers that already probe for
  [ADR-0058](0058-slice-boundaries-are-observed.md) pay nothing new.
- **Defect 23's other half is a harness fix and stays there.** Each measurement task declares an
  empty `[workspace]` so the generated crate builds wherever a round puts it, and `run.sh`
  refuses to start a round whose oracle cannot produce a result line. Belt and braces: this ADR
  stops a user's run from lying, the preflight stops a round from lying.
