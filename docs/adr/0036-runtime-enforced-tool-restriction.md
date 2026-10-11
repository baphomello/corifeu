# 0036 — Tool restriction enforced by the runtime

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
"The lead never writes feature code" is the rule the whole design rests on. Today it is
enforced by **Claude Code's** `PreToolUse` hook — which means Corifeu enforces nothing, and
the guarantee disappears the moment the lead is Codex, Gemini or a local model
([diagnosis](../diagnosis.md)).

Making Corifeu the runtime does not automatically fix this. The CLI providers are themselves
full agents that arrive with file-writing and shell tools already enabled. Spawning
`claude -p "..."` as a lead gives it everything it had before.

This was raised during the foundation review as the largest MVP-1 risk. Investigation
substantially reduced it: the restriction mechanisms exist and are documented flags.

## Decision
Tool restriction is enforced by the runtime in **three layers**, each independently
sufficient for the surface it covers.

**Layer 1 — never grant the tool.** For local models, Corifeu *is* the agent loop and simply
omits tools the role may not use. A role with no write capability is never offered
`write_file`. This is absolute: there is nothing to evade.

**Layer 2 — provider flags.** For CLI agents, the adapter passes restriction flags, verified
against the installed CLIs:

| Provider | Mechanism | Verified |
|---|---|---|
| Claude Code 2.1.295 | `--tools ""`, `--disallowedTools`, `--allowedTools`, `--restricted` | Yes, `--help` |
| Codex CLI 0.162.1 | `-s read-only` / `workspace-write`, `--ignore-rules` | Yes, `--help` |
| Gemini CLI | `--approval-mode`; tool-level restriction unclear | **UNVERIFIED** |

**Layer 3 — path and sandbox validation.** Every write that does reach the runtime is checked
against the ownership map ([ADR-0035](0035-file-ownership-by-glob-classes.md)), and shell
execution is confined ([ADR-0037](0037-sandboxed-command-execution.md)). This is the layer
that catches what the first two miss.

**A provider whose restriction cannot be verified may not hold a restricted role.** If
Corifeu cannot prove a lead will not write, that provider is refused as lead with an
explicit message, rather than being trusted and hoped for. This is why
[ADR-0039](0039-provider-conformance-suite.md) must test restriction, not only invocation.

## Alternatives considered
- **Prompt instruction only** ("you must not write code"). Degrades exactly when the run gets
  hard. The prototype already rejected this and added a hook.
- **Keep provider-native hooks.** Only Claude Code has them, which is the current problem.
- **Filesystem permissions.** Coarse, platform-divergent, and broken by the fact that all
  agents share one working tree in MVP 1.
- **Trust the CLI's own sandbox alone.** Each has one, and they differ in model and
  guarantee. Layer 3 exists because a uniform guarantee cannot be assembled from three
  different ones.

## Consequences
- Each adapter must declare which restrictions it can enforce, and the core must refuse role
  assignments it cannot back. This is a capability negotiation at configuration time, and
  `doctor` must surface it plainly.
- Gemini CLI's row is unverified and **blocks nothing except Gemini-as-restricted-role** until
  tested on a real installation. It must not be assumed.
- Layer 1 is the strongest and applies only to local models — a pleasing inversion, since the
  least capable models get the strictest enforcement.
- Verification belongs in the conformance suite and must run against each supported CLI
  version, because a flag that is silently ignored after an upgrade is the worst case: the
  restriction appears to work.
