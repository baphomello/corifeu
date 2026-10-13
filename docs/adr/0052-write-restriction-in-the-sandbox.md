# 0052 — Write restriction is enforced by the sandbox, not the tool surface

- **Status:** Accepted
- **Date:** 2026-10-13
- **Supports:** [ADR-0035](0035-file-ownership-by-glob-classes.md), [ADR-0036](0036-runtime-enforced-tool-restriction.md), [ADR-0037](0037-sandboxed-command-execution.md)
- **Evidence:** [measurement](../measurement.md)

## Context
[ADR-0036](0036-runtime-enforced-tool-restriction.md) states that a role is restricted by
capability rather than by instruction, and [ADR-0035](0035-file-ownership-by-glob-classes.md)
decides which files a role owns. The measurement showed a gap between that principle and the
obvious implementation of it.

A doer with a `write_file` tool and a `run` tool wrote to its source file **six times by
shell command**, against sixteen times through the tool:

```
[step 93] run: cat > src/lib.rs << 'ENDOFFILE' ...
[step 95] run: sed -i '58s/pp: usize/mut pp: usize/' src/lib.rs
```

Nothing was being evaded; a shell heredoc is a reasonable way to write a file. But every one
of those writes skipped the checkpoint in
[ADR-0051](0051-checkpoint-before-every-write.md), because the checkpoint was attached to the
tool rather than to the filesystem.

The same hole defeats file ownership. A role denied the `test` class by not being given a
tool that writes it can still run `sed -i` over a test file. The restriction that looked
capability-based was interface-based, which is the thing
[ADR-0036](0036-runtime-enforced-tool-restriction.md) exists to rule out.

## Decision
**File-class ownership is enforced where the write happens, not at the tool that requests
it.**

1. **The sandbox is the enforcement point.** A role's writable set is a property of the
   execution environment its commands run in
   ([ADR-0037](0037-sandboxed-command-execution.md)), so a write through a tool, a shell
   command, a build script or a subprocess is governed identically.
2. **A refused write is an explicit result.** The agent is told the path and the class that
   refused it, not given a generic permission error. An agent that cannot tell a denial from
   a bug will retry it.
3. **Checkpointing follows the write, not the tool.** Any observed write to a tracked
   artefact snapshots it first.
4. **A role that cannot write a class cannot write it by any route.** This is what makes
   [ADR-0053](0053-qa-writes-the-tests.md) a guarantee instead of a convention: a coder told
   not to edit tests is a coder that cannot edit tests.

## Alternatives considered
- **Deny the `run` tool to doers.** Removes the hole and most of the usefulness: a doer that
  cannot run a compiler cannot work against an oracle.
- **Pattern-match shell commands for writes.** A denylist against an adversary that does not
  exist but is endlessly inventive by accident. `tee`, `>>`, `python -c`, a build script —
  the list has no end.
- **Audit after the fact and revert.** Catches the write after it has run, which is too late
  for a command that also did something else.
- **Trust the instruction.** What the measurement ran. The agent complied with "do not change
  the tests" and still bypassed the checkpoint, because the two were never the same rule.

## Consequences
- The sandbox mechanism must support per-role writable paths. This constrains the
  implementation choice in [ADR-0037](0037-sandboxed-command-execution.md); platforms where
  this is unavailable fall back to a coarser guarantee, and `doctor` must say so rather than
  implying a protection that is not there.
- File classes ([ADR-0035](0035-file-ownership-by-glob-classes.md)) become a runtime
  structure, resolved per role at agent start.
- Enforcement is per role, so the same model in two roles gets two different writable sets,
  which is [ADR-0042](0042-test-authorship.md)'s role-versus-model distinction made concrete.
