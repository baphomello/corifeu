# Orchestra — multi-agent dev workflow

One human talks to ONE orchestrator (cloud Claude Code). The orchestrator architects,
delegates coding to a local model, delegates review to Codex, and reports back. No
copy-paste between tools.

## Roles
- **Architect + Orchestrator** — cloud Claude Code. You talk only to this.
- **Coder** — local model via Ollama, driven by `scripts/code.py` (lean harness: no skills,
  no MCP, no framework; only the curated task + tool results in context).
- **Reviewer** — Codex, via `scripts/review.sh` (cross-model → no architect bias).

## Flow (open mode) — `/cycle <feature>`
1. Architect writes `SPEC.md` (goal + per-task acceptance criteria).
   **Checkpoint 1:** human approves the SPEC.
2. Per task: orchestrator curates a minimal task and runs `code.py` → local model edits the repo.
3. Orchestrator validates (build/tests) itself.
4. `review.sh` → Codex reviews the diff against the acceptance criteria. Findings →
   re-delegate fixes to the coder (max 3 rounds) → escalate to human if stuck.
   **Checkpoint 2:** human approves before commit.
5. Commit on the feature branch.

## Rules
- The orchestrator **never writes feature code** (last resort only, and it says so).
- Delegates load no skills/MCP; the orchestrator curates exactly what they see.
- **Context contract** — coder sees `{task + SPEC excerpt + target file(s) + neighbor
  signatures}`; reviewer sees `{diff + acceptance criteria}`. Nothing more.

## Modes
- **open** — this file. Cloud Claude + Codex (code leaves the machine).
- **private** — all-local, no cloud. To be designed.

## Enforcement (not just rules)
A PreToolUse hook (`.orchestra/hooks/guard-src.py`, wired via `.claude/settings.json`)
**blocks** every write to `src/` — Write/Edit/NotebookEdit, and Bash writes (`cat >`,
`tee`, `sed -i`, `cp/mv`, `python open(...)`). The only way code reaches `src/` is the
delegated coder (`code.py`); the orchestrator cannot write feature code itself.
The orchestrator OWNS `tests/` — it authors the acceptance oracle, so `tests/` is not blocked.
