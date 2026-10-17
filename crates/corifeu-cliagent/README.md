# corifeu-cliagent

Agent CLI adapter for [Corifeu](https://github.com/baphomello/corifeu).

Drives a coding CLI — Claude Code, Codex CLI, Antigravity CLI ([ADR-0007](../../docs/adr/0007-cli-providers-first.md))
— in non-interactive mode and reads one turn out of its structured output.

The flags are not decoration. What each one is for, and what happens without it, is measured in
[the probe](../../docs/measurement/cli-provider-probe.md).
