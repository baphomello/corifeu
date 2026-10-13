# 0060 — Commands run as argument vectors; pipefail governs the shell opt-in

- **Status:** Accepted
- **Date:** 2026-10-13
- **Refines:** [ADR-0049](0049-faithful-check-results.md)
- **Supports:** [ADR-0037](0037-sandboxed-command-execution.md), [ADR-0052](0052-write-restriction-in-the-sandbox.md)

## Context
Two Accepted decisions contradict each other, and the contradiction surfaced the moment
there was code to write.

[ADR-0049](0049-faithful-check-results.md) §1: *"Every oracle command runs under `set -o
pipefail`."* It was written from a measured defect — the harness ran `cargo test | tail` and
received `tail`'s exit status, so a failing suite was reported to the agent as passing.

[ADR-0037](0037-sandboxed-command-execution.md) §3: *"No shell interpretation by default.
Commands are executed as argument vectors, not passed to `sh -c`."*

There is no `pipefail` without a shell. `corifeu-check` therefore shipped able to judge a
check result and unable to produce one, because no executor can obey both sentences.

The configuration model had already chosen, quietly and before this was noticed:

```rust
pub struct Oracle {
    /// An argument vector, never a shell string (ADR-0037).
    pub command: Vec<String>,
    pub timeout_s: u32,
}

pub struct Execution {
    pub shell: bool,            // default false
    pub allowed_commands: Vec<Vec<String>>,
}
```

So the disagreement is in the prose, not in the design. What is missing is a decision saying
which sentence survives and what happens to the other.

The measurement also supplies a second reason to keep the shell out, independent of exit
codes. A doer given both a `write_file` tool and a `run` tool wrote its source file six times
by shell heredoc ([ADR-0052](0052-write-restriction-in-the-sandbox.md)), skipping every
checkpoint, because the checkpoint was attached to the tool and the shell went around it.
Nothing was being evaded; a heredoc is a reasonable way to write a file. A command surface
that cannot express one removes the question.

## Decision
**Commands run as argument vectors. [ADR-0049](0049-faithful-check-results.md) §1 is not
retired — it governs the one configuration that has a shell.**

1. **Argv is the default and the guarantee.** Every command Corifeu runs on an agent's
   behalf is a program and its arguments. `execution.shell` is `false`, and under it a
   pipeline cannot be expressed, so the defect ADR-0049 §1 names cannot occur.
2. **`execution.shell: true` is where pipefail lives.** A user running inside a disposable
   container may reasonably want shell interpretation back. There the runtime wraps the
   command under `set -o pipefail` in a POSIX shell, which is exactly what ADR-0049 §1
   requires and the only place it can be honoured.
3. **The opt-in fails closed where it cannot be honoured.** `shell: true` on a platform with
   no POSIX shell is refused at load, not accepted with a weaker guarantee. A configuration
   that silently degrades is how the original defect reached the agent.
4. **A check that genuinely needs a shell names one as its program.** `["bash", "-c", "…"]`
   is a legal argv. It is visible in `execution.allowed_commands`, and `doctor` reports that
   faithful exit status through a pipe is not guaranteed for such a check — the user's
   choice, made in the open.
5. **ADR-0049 §2 and §3 are untouched.** Head-first truncation and the `did-not-terminate`
   verdict never depended on a shell.

## Alternatives considered
- **Keep pipefail everywhere and run every command under `bash -c`.** Gives up the argv
  guarantee that [ADR-0052](0052-write-restriction-in-the-sandbox.md) depends on, and needs a
  POSIX shell on Windows, which [ADR-0005](0005-target-platforms.md) requires us to support
  natively.
- **Supersede ADR-0037 §3 instead.** Makes the checkpoint guarantee unachievable: a shell
  write goes around the tool, which is the defect that produced ADR-0052.
- **Allow a shell for exploration and argv for oracles.** The split the measurement actually
  ran, and the one that failed. Enforcement that depends on which interface was used is the
  thing [ADR-0036](0036-runtime-enforced-tool-restriction.md) exists to rule out.
- **Detect a pipe in the command and refuse it.** A denylist against shell syntax, which
  [ADR-0052](0052-write-restriction-in-the-sandbox.md) already rejected: `tee`, `>>`,
  `python -c` and a build script have no end.
- **Let each check choose.** A guarantee that varies per check is not a guarantee, and the
  agent cannot tell which kind it is holding.

## Consequences
- The agent loses pipes, globs and redirection. The runtime must supply what they were doing:
  truncation is already ADR-0049 §2, reading a file is a tool rather than `cat`
  ([ADR-0047](0047-bounded-agent-context-within-a-task.md)), and a filter belongs to the test
  runner rather than to `grep`.
- **Shell globs no longer expand.** A check written as `pytest tests/*.py` passes the literal
  string to `pytest`. Checks must name their files or use the runner's own selection. This is
  a sharp edge for users porting an existing command, and the installer should say so.
- Windows executes by the same path as Unix for the first time, because nothing depends on a
  POSIX shell being present.
- `execution.allowed_commands` is a list of argv prefixes rather than string prefixes, which
  it already was. Matching is per element, so `["cargo", "test"]` permits
  `cargo test --lib x` and not `cargo publish`.
- Output is captured as two streams and concatenated, not interleaved as a terminal would
  show them. A tool that puts progress on stdout and errors on stderr reaches the agent in
  two blocks. This is what the measurement harness did, and the winning run was driven by
  those errors, so it is adequate — but it is a known difference from what a human sees.
