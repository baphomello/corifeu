# 0064 — A model writes the rule; a model is not the rule

- **Status:** Accepted, not implemented
- **Date:** 2026-10-14
- **Constrains:** [ADR-0049](0049-faithful-check-results.md), [ADR-0057](0057-oracle-integrity-by-test-inventory.md)
- **Evidence:** none yet. This is a decision about direction, taken before the measurement that
  would test it. §Consequences says what would falsify it.

## Context
Corifeu learned a second language on 2026-10-14. Supporting Python took one new JSON field per
Rust assumption the harness held, plus one indentation-aware cutter in the measurement
scaffold. It worked, and it does not scale: the goal is any language, and after that image
generation and video, where the number of tools is open-ended and the fashionable ones change
faster than a release cycle.

The obvious conclusion is that the knowledge should come from a model rather than from a
contributor. That conclusion is right, and it hides a question about *where* the model goes.
Two placements look alike and are not:

| | The model's job | Scales | Cost | Trust |
|---|---|---|---|---|
| **In the check** | read the tool's output, report how many cases passed | yes | per check, forever | gone |
| **In the adapter** | write a deterministic reader for that tool, once | yes | once per tool | intact |

Putting the model **in the check** ends the product. The thesis is that work can be delegated
to a cheap local model because a cheap local model's claim is verified by something that cannot
be talked into agreeing. [ADR-0057](0057-oracle-integrity-by-test-inventory.md) exists because
a doer reports green on a suite it has quietly shrunk. A counter that is itself a model makes
that report unfalsifiable, and the only remaining way to audit the cheap model is an expensive
one — which is the spend Corifeu was built to remove.

The cost argument is weaker than it looks and worth stating accurately. The fourth run spent
**79 checks across 274 steps — 0.29 checks per step**, over nineteen runs. A model call per
check would be about a 29% increase in dispatches, not a doubling. That is material but it is
not the reason. The reason is that the check is the one thing in the system that is not allowed
to be persuaded.

The surface this applies to is also smaller than it feels. Of 9,223 lines under `crates/`, the
only language-specific logic is `crates/corifeu-check/src/cases.rs` — **345 lines, three
readers**: `cargo`, `go_verbose`, `pytest`. Every other mention of a build tool in the runtime
is a test fixture or a doc example. `corifeu-exec`, `corifeu-workspace` and `corifeu-agent`
name no language at all.

## Decision
**Whatever produces a verdict stays deterministic code. A model may author that code, offline,
and what it authors is verified before it is trusted.**

1. **An adapter is a declaration, not a patch.** It carries the argv to invoke, how to read the
   result, captured samples of real output with the counts each must produce, and its
   provenance: which model wrote it, when, against which version of the tool.
2. **The samples are the adapter's oracle.** An adapter is accepted when it produces the
   declared count for every captured sample, and not before. The samples in `cases.rs` today
   already are that oracle; they were captured by hand, which is the only part that changes.
3. **Adding a tool is therefore a Corifeu task.** One slice, one artefact, one objective
   check — the shape the runtime already works in. Corifeu generates its own adapters.
4. **No adapter means a reduced guarantee, stated.** Fall back to the process exit status
   alone and record that the case count is unknown. `Cases::Unknown` already exists for this
   and must not be filled in by a model.
5. **The trust anchor is never generated and never a model.** The process exit status and
   timeout, the diff, the checkpoint and rollback
   ([ADR-0051](0051-checkpoint-before-every-write.md)), the allowlist
   ([ADR-0037](0037-sandboxed-command-execution.md)) and file-class ownership
   ([ADR-0052](0052-write-restriction-in-the-sandbox.md)). These are already language-neutral
   and stay hand-written.

## Alternatives considered
- **A model in the check, reading tool output live.** Scales to every tool immediately, and
  removes the property that makes delegating to a local model worth doing. Rejected on trust,
  not on cost.
- **A hand-written reader per tool, as today.** Free, deterministic, verifiable, and bounded by
  contributor attention rather than by what users need. It is what we have, and it is why this
  ADR exists.
- **A model in the check, with a hand-written reader as a cross-check.** Pays for both and
  trusts the cheaper one. If the hand-written reader is good enough to audit the model, the
  model is not needed.
- **Generic output parsing with no per-tool knowledge** — heuristics over "N passed, M failed".
  Tried in miniature and already declined inside `cases.rs`: pytest's summary counts
  `xpassed`, `deselected` and `error` differently, and a heuristic that silently drops an
  outcome produces a wrong count, which is worse than no count.

## Consequences
- **Not implemented.** Deliberately. Changing the harness during the fifth measurement run
  would destroy comparability with runs one to four, and those four runs are the only evidence
  base there is.
- **The fifth run is a precondition, not just a measurement.** It is the first test of whether a
  9B local model holds a small scoped task reliably. That is exactly the capability this ADR
  assumes when it says a model may author an adapter. If a 9B cannot write a reader that
  satisfies six captured samples, the author seat needs a larger model, and it is better to
  know that before designing around it.
- **What would falsify this.** Generated adapters that pass their samples and are still wrong
  in the field — a reader that counts correctly on six captured outputs and miscounts the
  seventh shape nobody captured. Then the sample set is the weak link, and the question becomes
  how samples are collected rather than who writes the reader.
- Moving language knowledge from code into `task.json` on 2026-10-14 was the first step toward
  this and not a detour. A model can write a declaration; it can only patch code.
- The measurement harness keeps a per-language cutter (braces versus indentation) because it
  builds fixtures rather than verdicts. It is outside this decision.
- Verdicts that have no deterministic reader at all — is this image right — are a different
  problem and get [ADR-0065](0065-an-oracle-and-a-judge-are-different-contracts.md).
