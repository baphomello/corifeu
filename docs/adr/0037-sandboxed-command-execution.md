# 0037 — Command execution is sandboxed by default

- **Status:** Proposed
- **Date:** 2026-10-11

## Context
`code.py` carefully confines file access with `_safe()`, which resolves paths and rejects
anything escaping the repository — and then exposes:

```python
def run(command):
    r = subprocess.run(command, shell=True, cwd=REPO, ...)
```

Arbitrary shell, no allowlist, no confinement. `run("rm -rf ~")` is reachable from a local
model's hallucination, and the path guard above it is decorative once `run` exists.

On a personal machine with a trusted setup this is a reasonable trade. Corifeu is
distributed by `curl | sh` to people who have not read the source, and runs models chosen by
the user — including small quantised local models that fail in creative ways. The same trade
is no longer reasonable.

This is also the enforcement layer that [ADR-0035](0035-file-ownership-by-glob-classes.md)
and [ADR-0036](0036-runtime-enforced-tool-restriction.md) depend on. Path ownership means
nothing if an agent can shell out and write anywhere.

## Decision
Commands executed on behalf of an agent run **confined by default**. Confinement has three
components:

1. **An allowlist of command prefixes**, from configuration. The project's build, test, lint
   and format commands are declared during installation — they are already known, since they
   are the oracles ([ADR-0019](0019-tie-breaking-and-oracle-precedence.md)). Anything else
   requires approval according to the autonomy level.
2. **Platform sandboxing where available**, delegating rather than inventing:

| Platform | Mechanism |
|---|---|
| macOS | Seatbelt (`sandbox-exec`), as Codex already uses |
| Linux | Landlock, with seccomp where appropriate |
| Windows | Restricted token / job object |
| Any | Delegate to the provider CLI's own sandbox when the command runs through it |

3. **No shell interpretation by default.** Commands are executed as argument vectors, not
   passed to `sh -c`. Shell metacharacters stop being an evasion surface entirely.

Users may opt out explicitly (`execution.sandbox: none`) for environments already externally
sandboxed, such as a disposable container — which is also how Codex frames its own
`--dangerously-bypass-approvals-and-sandbox`. Opting out is a deliberate, documented choice,
not a default.

## Alternatives considered
- **No sandbox, as today.** Acceptable for one trusted user; not for a distributed product.
- **Allowlist only.** Helps, and does not contain a command that is itself allowed but
  misused — `npm test` running an arbitrary lifecycle script, for instance.
- **Containerise everything.** Strongest isolation, and it imposes Docker as a dependency,
  contradicting [ADR-0004](0004-rust-single-binary.md).
- **Rely entirely on the provider CLIs' sandboxes.** Works only for commands routed through
  a CLI, and not at all for Corifeu's own local-model loop — which is exactly where the weak
  models are.

## Consequences
- Three platform implementations, each with its own failure modes. Significant work, and it
  is the difference between a tool people can responsibly recommend and one they cannot.
- Where no sandbox is available, Corifeu must **say so** and fall back to allowlist plus
  no-shell, rather than silently providing less protection than advertised.
- The allowlist is configuration the wizard must gather well. Too strict and every run stops
  for approval; too loose and the mechanism is theatre.
- Argument-vector execution breaks pipelines and redirection that agents naturally produce.
  The runtime should provide structured equivalents rather than re-admitting `sh -c`.
- WASM plugins ([ADR-0032](0032-out-of-process-plugins.md)) would address a related part of
  this problem and are worth revisiting together.
