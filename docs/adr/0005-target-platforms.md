# 0005 — Linux, macOS, native Windows and WSL at launch

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The prototype is POSIX-only and, worse, *silently* POSIX-only: `setup-codex-home.sh`
symlinks `$HOME/.codex/auth.json`, and the Ollama URL was derived from `ip route`. Both
fail on native Windows in ways that produce confusing errors rather than clear ones.
Developers on Windows are a large population, and many of them use WSL — which is not the
same target and must not be conflated with it.

## Decision
Supported at launch: **Linux**, **macOS**, **Windows (native, PowerShell)** and **WSL**.
Installation is `install.sh` for POSIX shells and `install.ps1` for PowerShell.

WSL is treated as **Linux with distinct networking**, not as a separate build target. It
runs the Linux binary. It gets its own documentation and its own `doctor` checks.

## Alternatives considered
- **POSIX only, Windows users use WSL.** Cheaper, and a large share of the audience is
  already in WSL. Rejected: it concedes the native Windows developer, and "just use WSL" is
  a poor first impression for a tool claiming to reduce friction.
- **Windows through Git Bash or MSYS.** Avoids a PowerShell installer at the price of an
  undeclared dependency and a path-translation layer that leaks in error messages.

## Consequences
- Two installers to maintain in lockstep. Divergence between them is a predictable defect
  source and needs a shared test matrix.
- Path handling, line endings, symlink availability (which on Windows requires Developer
  Mode or elevation) and process spawning all need platform-aware code. `setup-codex-home`'s
  symlink approach cannot survive and must become a file copy or a config-flag equivalent.
- CI runs the full matrix on all three operating systems from phase 1. A WSL job is harder
  to arrange on hosted runners and may need a self-hosted runner or a documented manual
  check — tracked in [risks.md](../risks.md).
- Provider CLIs have their own platform support. Corifeu cannot promise a provider works on
  a platform where that provider does not run; `doctor` reports this rather than failing
  obscurely.
