# 0065 — An oracle and a judge are different contracts, and are never mixed

- **Status:** Accepted, not implemented
- **Date:** 2026-10-14
- **Extends:** [ADR-0056](0056-one-oracle-per-slice.md), [ADR-0064](0064-a-model-writes-the-rule-it-is-not-the-rule.md)
- **Evidence:** none yet. This is a decision about direction, taken before any non-code work
  has been attempted. §Consequences says what would falsify it.

## Context
Every ADR so far assumes a slice can be checked by a command that exits zero or non-zero.
[ADR-0056](0056-one-oracle-per-slice.md) requires exactly one such oracle per slice;
[ADR-0057](0057-oracle-integrity-by-test-inventory.md) guards it against a suite that shrank;
[ADR-0049](0049-faithful-check-results.md) requires its result to reach the doer intact. The
word used throughout is *oracle*, and it means something precise: deterministic, binary, and
free to run again.

The intended scope does not stay inside that assumption. Image generation and video generation
are wanted, and "is this image right" has no exit status. Treating that as the same problem as
"Corifeu does not know how to read pytest's output" is the mistake this ADR exists to prevent.
It is not a reader that is missing. It is the oracle.

Two things are true at once and both get lost if the distinction is not made.

**More of it is objective than it first appears.** A generation pipeline has plenty that a
command can settle: a file appeared; dimensions, format, duration, codec, bitrate; a fixed seed
reproducing byte for byte; an embedding similarity to the prompt above a threshold; a content
classifier; no black frames; audio present on the track. Each of those is an oracle in the
existing sense and should be written as one.

**Some of it is irreducibly a judgement.** Whether a composition is good. Whether a refactor
left the code clearer. A model can answer those and a command cannot. A model answering them is
useful and is not an oracle: it is probabilistic, it can be wrong, and it has an error rate
that is a fact about the system nobody currently measures.

The machinery for measuring that error rate is already being built for a different reason. The
fifth run's held-out suites exist because [ADR-0053](0053-qa-writes-the-tests.md) turns on
whether code is *worse*, which no count of who typed what can answer. That is a judge being
calibrated against cases it never saw, and it is the same problem.

## Decision
**A verdict carries which kind it is. An oracle and a judge are separate contracts and a caller
can always tell them apart.**

| | `Oracle` | `Judge` |
|---|---|---|
| Decided by | a deterministic command | a model |
| Verdict | binary: green or red | a score |
| Cost to re-run | about nothing | a dispatch |
| Can be wrong | no | yes, at a rate that must be measured |
| Governs | ADR-0051 rollback, ADR-0057's guard, slice closure | ranking, selection, advice |

1. **A slice still closes on an oracle, never on a judge.** Everything the existing ADRs hang
   on a green check — rollback on regression
   ([ADR-0051](0051-checkpoint-before-every-write.md)), the test-inventory guard
   ([ADR-0057](0057-oracle-integrity-by-test-inventory.md)), a slice being done — requires the
   deterministic kind.
2. **Objective parts are extracted into oracles first.** Before a judge is reached for, the
   properties a command can settle are written as commands. For a generated image that is most
   of the checklist.
3. **A judge's output is a score with a recorded error rate, never a pass.** A judge that has
   not been calibrated against held-out cases reports its verdict and its lack of calibration
   together.
4. **A judge is never silently substituted for an absent oracle.** A task with no oracle is a
   task Corifeu cannot close, and it says so. Filling the gap with a model is exactly what
   [ADR-0064](0064-a-model-writes-the-rule-it-is-not-the-rule.md) forbids.
5. **A judge may be an expensive model.** It runs once per deliverable rather than once per
   step, so the economics are not the economics of a check.

## Alternatives considered
- **One `Verdict` type, with the model-produced kind treated as a check.** The shortest code and
  it makes every guarantee in the project conditional on something that can be wrong. A reader
  of `Outcome::Reached` would no longer know whether anything was actually verified.
- **Refuse non-code work entirely, and keep the oracle assumption.** Honest, and gives up the
  stated scope. It also throws away the objective part, which is the larger part.
- **A judge that returns a binary by thresholding its own score.** Hides the uncertainty at the
  exact boundary where it matters, and makes the threshold an invisible decision inside the
  judge instead of a visible one at the call site.
- **Human review as the only judge.** Correct and does not scale, and the project's whole
  premise is work that proceeds without someone watching each step.

## Consequences
- **Not implemented, and further off than [ADR-0064](0064-a-model-writes-the-rule-it-is-not-the-rule.md).**
  Nothing in the runtime produces or consumes a score today, and no non-code task has been
  attempted.
- **`Outcome` and `Verdict` gain a kind, which is a breaking change** to every caller that
  pattern-matches them. Worth taking before there are many, which is an argument for recording
  the decision now rather than for building it now.
- **A judge's error rate becomes a number the project owes.** The held-out suites are the first
  instance; a judge shipped without one is shipped without the thing that makes it readable.
- **What would falsify this.** Finding that the objective checklist for a generated artefact is
  thin enough to be useless — that nothing a command can measure correlates with whether the
  output was any good. Then the separation buys nothing for that medium, and the honest move is
  to say Corifeu does not verify it.
- Confidentiality cuts across this. A judge reading an artefact sends it somewhere, and
  [ADR-0061](0061-privacy-by-file-class.md) decides per file class what may leave. A judge on a
  private class is not available, and that is a reason a task may have no judge at all.
