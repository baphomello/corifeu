# 0004 — Rust, distributed as a single prebuilt binary

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Corifeu installs into other people's repositories through `curl | sh`. Every runtime
dependency it requires is a dependency it imposes on strangers, and the prototype already
demonstrates the cost: it needs Python 3, a POSIX shell, and a Claude Code installation
merely to start.

The workload itself is almost entirely I/O orchestration — spawning and supervising agent
subprocesses, streaming and parsing their output, HTTP to local model runtimes, SQLite
reads and writes, a terminal wizard, and JSON-RPC over stdio to plugins. There is no
meaningful CPU-bound work.

Go was explicitly raised as an alternative during the foundation review, on the grounds
that adapter churn rather than distribution is the real risk.

## Decision
Corifeu is written in **Rust** and distributed as a **single prebuilt binary per
platform**, served from GitHub Releases. No runtime interpreter, no package manager, no
system SQLite.

## Alternatives considered

**Go** is the serious alternative and was evaluated properly rather than dismissed.

| Dimension | Go | Rust |
|---|---|---|
| Subprocess supervision with cancellation | Goroutines + `context.Context` — idiomatic and hard to get wrong | Tokio; correct but fiddlier around child-kill and pipe draining |
| Embedded SQLite | `mattn/go-sqlite3` needs cgo, which forfeits easy cross-compilation; `modernc.org/sqlite` is a pure-Go transpilation, slower and historically patchier | `rusqlite` with the `bundled` feature compiles SQLite from source into the binary. Settled, standard, genuinely static |
| Cross-compilation | Excellent, famously so | Needs per-target toolchains; in practice solved by a CI runner matrix |
| YAML | `yaml.v3`, maintained | `serde_yaml` was **archived in March 2024**; requires choosing a fork ([ADR-0040](0040-yaml-parser-selection.md)) |
| TUI for the wizard | Bubble Tea, excellent | ratatui, excellent |
| Error discipline in a long-running supervisor | `if err != nil` sprawl | `Result` with `thiserror`/`anyhow` |
| Barrier to drive-by contribution | Lower | Higher |

Two factors decide it.

**SQLite is load-bearing here, not incidental.** [ADR-0021](0021-sqlite-with-event-log.md)
puts an append-only event log at the centre of the design. `rusqlite` with `bundled` is the
better answer than either Go option, and the Go answer costs either cgo or a transpilation.

**The contribution-barrier argument dissolves against
[ADR-0032](0032-out-of-process-plugins.md).** Because plugins are out-of-process and
language-agnostic, a contributor adding a provider or a checker writes it in whatever
language they like and speaks JSON-RPC. Rust gates the *core*, which is exactly the part
that should be hard to change casually. Go's main advantage therefore mostly does not
apply.

Also weighed: the project owner already works in Rust. For a solo-founded project,
founder fluency outranks hypothetical contributor preference, because the contributors do
not exist yet and the founder does.

**The concern that prompted the question is real but is not a language concern.** Adapter
churn against pre-1.0 CLIs is the dominant technical risk; `codex` is at 0.162.1. Changing
language does not reduce it by one point. It is addressed by
[ADR-0039](0039-provider-conformance-suite.md) and tracked in
[risks.md](../risks.md).

Other options: **TypeScript/Node** (reintroduces a runtime dependency and the npm tree —
the exact weight being escaped); **Python** (same, plus packaging pain on Windows);
**a shell-script product** (what exists today; cannot work on native Windows).

**Confidence: medium-high.** Unverified: no spike has yet compared Tokio child-process
supervision against goroutines for the specific timeout-and-kill path, nor measured
`rusqlite` against `modernc.org/sqlite` under this event-log write pattern. Neither is
likely to overturn the decision; both would sharpen it.

## Consequences
- Rust compile times slow the edit-test loop. Workspace splitting and `cargo nextest` help.
- A GitHub Actions matrix across `x86_64`/`aarch64` for Linux, macOS and Windows is needed
  from phase 1. Release artefacts need checksums and signing
  ([risks.md](../risks.md)).
- `serde_yaml` being unmaintained is a real cost that Go does not have. Deferred to
  [ADR-0040](0040-yaml-parser-selection.md).
- WSL runs the Linux binary. It is not a separate target, but it has its own networking
  behaviour ([ADR-0005](0005-target-platforms.md)).
- Binary size will land in the tens of megabytes with a TUI and bundled SQLite. Acceptable
  for a developer tool.
