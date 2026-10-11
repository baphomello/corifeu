# 0039 — Every provider passes a conformance suite

- **Status:** Proposed
- **Date:** 2026-10-11

## Context
Provider adapters are the dominant technical risk ([risks.md](../risks.md)). They target
pre-1.0 command-line tools — `codex` is at 0.162.1, Claude Code at 2.1.295 — whose flags and
output formats change frequently. Gemini CLI has a reported defect where a documented
`--output-format` flag was rejected outright.

The failure mode that matters is not a loud one. A flag silently ignored after an upgrade
produces a lead that *appears* restricted and is not, or a parse that *appears* to succeed
and yields empty content. The prototype's accumulated defences in `code.py` — text-mode
parsing, nudge counting, loop detection — are all scar tissue from exactly this class of
failure.

This ADR is the concrete answer to a concern raised in the foundation review: that the
single-binary distribution solves the easy half of the problem while adapter churn, the hard
half, goes unaddressed.

## Decision
Every provider adapter, built-in or plugin, must pass a **conformance suite** before it can
be used. The suite tests behaviour, not flag syntax.

| # | Test | Asserts |
|---|---|---|
| 1 | Detection | Presence and version are reported; an unknown major version is refused clearly |
| 2 | Authentication | Auth status is distinguishable from a missing install |
| 3 | Round trip | A trivial prompt returns parseable structured output |
| 4 | Tool call | A prompt requiring a tool produces a well-formed call |
| 5 | **Tool restriction** | A provider told it may not write, **and instructed to write**, does not write |
| 6 | Context isolation | Ambient context (`CLAUDE.md`, `AGENTS.md`, MCP) does not reach the agent |
| 7 | Usage reporting | Token counts are returned, or absence is reported honestly |
| 8 | Cancellation | The process terminates on timeout, leaving no orphan |

**Test 5 is the one that matters most**, and it must be adversarial: instruct the agent to
write a file it is forbidden to write, then assert the file does not exist. It directly
backs [ADR-0036](0036-runtime-enforced-tool-restriction.md), and it is the test that catches
a silently-ignored flag.

The suite runs in CI against pinned CLI versions, **and on a schedule against latest**, so
upstream breakage is discovered by the project rather than by a user mid-run. It is also
available to users as part of `corifeu doctor`.

A provider that fails test 5 may still be used for unrestricted roles. A provider that fails
tests 1–4 is unusable.

## Alternatives considered
- **Manual testing per release.** Does not scale to three CLIs across four platforms, and
  degrades precisely when releases are frequent.
- **Unit tests with recorded fixtures.** Fast and deterministic, and blind to the failure
  that matters: fixtures keep passing after the real CLI changes. Useful as a complement.
- **Trust documented flags.** The Gemini defect is the counterexample.
- **Pin CLI versions and never test latest.** Postpones breakage to the user's machine.

## Consequences
- CI needs real CLI installations and credentials, which is awkward for authenticated cloud
  providers. Local runtimes test freely; cloud providers may need a gated job and a
  documented manual check — tracked in [risks.md](../risks.md).
- The suite is the contract plugin authors implement against, so it must be published and
  runnable outside this repository.
- Scheduled runs against latest will generate failures that are not regressions in Corifeu.
  That is the point, and triage expectations should be set accordingly.
