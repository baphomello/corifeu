# 0049 — Check results reach the doer intact

- **Status:** Accepted
- **Date:** 2026-10-13
- **Supports:** [ADR-0019](0019-tie-breaking-and-oracle-precedence.md), [ADR-0037](0037-sandboxed-command-execution.md)
- **Evidence:** [measurement](../measurement.md)

## Context
[ADR-0019](0019-tie-breaking-and-oracle-precedence.md) makes the oracle the highest authority
in the system: a failing test beats any agent's argument. That guarantee is worth exactly as
much as the fidelity of the channel carrying the result. Three defects in the measurement
corrupted that channel, and each one cost a run.

**The exit code was masked.** A doer ran `cargo test 2>&1 | tail -40`. The pipeline's exit
status is `tail`'s, so a suite with three failures reported **exit 0**. The agent was told its
broken code had passed. The same mistake appeared twice more in the measurement, once in a
command written by the agent and once in a command written by hand while verifying it — it is
not an exotic error.

**Output was truncated from the wrong end.** `cargo test` emitted 15,148 characters
describing 30 compile errors; the harness kept the last 4,000. Compiler output is
front-loaded: the first errors carry the diagnosis and the tail only counts them. The agent
received `could not compile due to 30 previous errors` and almost none of the errors. It ran
the same command four times trying to see them. All 30 were one trivial class.

**A hanging check looked like a slow one.** Task C's glob matcher recursed without bound.
Each `cargo test` consumed the full 300-second subprocess timeout and returned no verdict at
all. Three rounds were spent this way.

## Decision
**A check result is delivered to the agent whole, with its true exit status, or it is
reported as a failure of the check.** Three rules follow.

1. **Pipelines fail honestly.** Every oracle command runs under `set -o pipefail`, so a
   failing stage is a failing command whatever follows it. A check whose status cannot be
   determined is a check that did not run.

2. **Truncation removes the middle, never the head.** When output exceeds the budget, the
   agent receives the first 70% and the last 30% with the omission marked and counted:

   ```
   [... 3,148 characters of the middle omitted ...]
   ```

   The head carries the diagnosis; the tail carries the summary; the middle is repetition.

3. **Every check carries a timeout, and a timeout is its own verdict.** `pass`, `fail` and
   **`did-not-terminate`** are three different results. The third is not a failed test — it
   is evidence of non-termination, which is usually a defect in the code under test, and it
   must reach the agent as that rather than as a wall-clock error.

## Alternatives considered
- **Instruct the agent not to pipe.** Tried, in the prompt. Agents pipe anyway, and a rule
  enforced by instruction rather than by the runtime is the pattern
  [ADR-0036](0036-runtime-enforced-tool-restriction.md) exists to reject.
- **Give the agent the whole output, never truncate.** Unbounded against
  [ADR-0047](0047-bounded-agent-context-within-a-task.md), and a single 100k-line test run
  would evict everything else.
- **Keep only the head.** Loses the summary line, which is where the pass/fail counts live.
- **Treat a timeout as a failed check.** Collapses two different facts. "The tests fail" and
  "the tests never finish" lead to different repairs.

## Consequences
- Oracle commands are wrapped by the runtime, not written by the agent. The agent names a
  check; the runtime decides how it is executed and bounded.
- The timeout is per-check configuration, because a unit suite and an integration suite have
  different honest durations.
- `did-not-terminate` needs its own path in [ADR-0026](0026-configurable-failure-fallback.md):
  retrying it unchanged is guaranteed to waste the same budget again.
- Truncation budgets are configuration. The measured default of 12,000 characters was enough
  for 30 Rust compile errors; other toolchains are more verbose.
