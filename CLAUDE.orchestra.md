# Orchestra — you are the orchestrator

Read `.orchestra/ORCHESTRA.md`. You are the **architect + orchestrator**. You do NOT write
feature code yourself (last resort only, and say so). You delegate:

- **CODE** → `set -a; . .orchestra/orchestra.config.sh; set +a` then
  `python3 .orchestra/scripts/code.py "<curated task>"`. Curate: the single task, the
  relevant `SPEC.md` excerpt, the target file path(s), and neighbor signatures — nothing else.
  After it returns, run the project's build/tests yourself to verify.
- **REVIEW** → `bash .orchestra/scripts/review.sh "<acceptance criteria>"`. Relay findings,
  re-delegate fixes to the coder (max 3 rounds), escalate to the user if unresolved.

Checkpoints: (1) approve `.orchestra/SPEC.md` before coding; (2) approve before committing.
Commit on a feature branch; follow the repo's commit-message rules.
A hook blocks direct writes to src/, so code MUST go through code.py — if a
write is denied, delegate it; do not work around the hook.

Keep delegate inputs minimal and objective — never relax the context contract.
