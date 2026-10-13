# 0047 — The agent loop rebuilds a bounded context each step

- **Status:** Accepted
- **Date:** 2026-10-13
- **Supports:** [ADR-0013](0013-blackboard-as-source-of-truth.md), [ADR-0014](0014-context-contracts-per-role.md)
- **Evidence:** [measurement](../measurement.md)

## Context
A naive agent loop appends every message to one growing conversation and re-sends the whole
thing on each step. Measured on Task A with a local doer running 60 steps:

| | |
|---|---|
| Prompt tokens | **1,227,125** |
| Output tokens | 11,076 |
| **Ratio** | **111 : 1** |
| Average prompt per step | 20,452 |

1.2 million prompt tokens to produce 11 thousand tokens of output. The cost is quadratic in
step count, because step *n* re-sends everything from steps 1..*n*.

On a local model this costs wall-clock. On a metered provider it is ruinous — and it is
precisely the cost this product exists to remove, appearing inside the mechanism meant to
remove it.

[ADR-0013](0013-blackboard-as-source-of-truth.md) promises the **lead's** context stays
bounded regardless of run length, and [ADR-0014](0014-context-contracts-per-role.md) governs
the view a role is **given**. Neither says anything about that view growing without bound
while the role works.

## Decision
**The agent loop does not append to a conversation. It rebuilds a bounded context from state
on every step.**

Each step the doer receives:

| Part | Bound |
|---|---|
| Contract view (task, acceptance, files, memory) | `contract.max_tokens`, unchanged across the task |
| **The working artefact, read from disk this step** | always current, never recalled from history |
| Recent tool results | last `k` only, `k` from configuration |
| A compacted trace of older steps | one line per step: action, target, outcome |

Older tool results are **replaced by their one-line outcome**, not deleted silently. An agent
that ran `cargo test` fifteen steps ago sees `step 12: cargo test -> 3 passed, 12 failed`,
not the full output again, and not nothing.

### The artefact comes from disk, not from scrollback

This row is the one the measurement forced. Compaction alone is not enough: bounding the
history also evicts the thing the agent is editing.

A doer asked to repair a 9,570-byte file under `k = 3` read it in chunks, and each chunk fell
out of the window before the next arrived. It spent **28 steps reading and made zero edits**,
producing 1,064 output tokens in total. It was not confused or incapable; it was structurally
unable to hold the file it had been told to change.

Re-reading the artefact from disk each step fixes this and costs less than the alternative,
because the agent stops spending steps on reads. The same doer, with the artefact pinned,
began editing on its first step.

The prompt says so explicitly, so the agent does not spend a step confirming:

> Current contents of `src/lib.rs`, read fresh from disk this step. This is always current;
> you never need to read it again.

## Alternatives considered
- **Summarise the conversation when it grows.** Standard, and it throws away precisely the
  recent tool output the agent needs while keeping old prose it does not.
- **Raise the context window.** Buys steps, not a fix. The ratio stays quadratic; the wall
  moves.
- **Keep every tool result, drop old assistant turns.** Tool results are the bulk.
- **Let the agent re-read the artefact when it needs it.** What the measurement ran. It costs
  a step per read, and fails outright whenever the file is larger than the window.

## Consequences
- Cost becomes linear in steps. Measured on the same task: 101,182 prompt tokens over 22
  steps, a **7.8 : 1** ratio against 111 : 1.
- `k` and `contract.max_tokens` are configuration, not constants. A task editing several
  files pins several, and the budget has to cover them.
- The compacted trace is a product surface, not a debug aid: it is what the agent reasons
  over about its own past, so its wording is worth the same care as a prompt.
- Pinning means the board is authoritative over the transcript. An artefact changed outside
  the agent — by a repair, a rollback ([ADR-0051](0051-checkpoint-before-every-write.md)), or
  another role — is simply what the agent sees next step, with no reconciliation step needed.
