# Corifeu

> **This project is being rebuilt.** Corifeu is a standalone runtime that orchestrates AI
> agents and models to build software — one lead agent talks to you, a team of specialised
> roles works behind it. The design lives in [`docs/`](docs/): start with
> [the architecture](docs/architecture.md), [the roadmap](docs/roadmap.md) and
> [the decision records](docs/adr/).
>
> Everything below, and everything under `.orchestra/`, is the **prototype** that Corifeu
> grew out of. It still works, and it is scheduled for removal once Corifeu can run a cycle
> end to end.

---

## The prototype

Multi-agent dev workflow for Claude Code: you talk only to **cloud Claude** (architect +
orchestrator); it delegates **coding** to a local model (Ollama, via a lean harness) and
**review** to **Codex** (cross-model, no architect bias). No copy-paste between tools.

See [`.orchestra/ORCHESTRA.md`](.orchestra/ORCHESTRA.md).

## Install into a repo
```bash
./install.sh /path/to/your-repo
```
Requirements: Ollama reachable (coder), `codex` CLI logged in (reviewer), Python 3.

## Config
Edit `.orchestra/orchestra.config.sh` (coder model, Ollama URL). Run `/cycle <feature>` in Claude Code.
