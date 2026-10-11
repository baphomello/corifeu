#!/usr/bin/env bash
# review.sh — lean Codex review of the working diff against acceptance criteria.
# CODEX_HOME points at a minimal home (auth symlink + tiny config) so the user's global
# AGENTS.md doesn't load. Codex still auto-loads some built-in skills (its own tokens).
set -euo pipefail
ORCH="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRITERIA="${1:-Review for correctness bugs and spec compliance.}"
git add -A -N >/dev/null 2>&1 || true
DIFF="$(git --no-pager diff)"
[ -z "$DIFF" ] && { echo "(no diff to review)"; exit 0; }
PROMPT="You are a strict code reviewer. Review ONLY the diff on stdin.
Acceptance criteria: ${CRITERIA}
Report findings most-severe first (correctness bugs before style/nits), concise.
If it is correct and meets the criteria, reply exactly: APPROVED."
printf '%s' "$DIFF" | CODEX_HOME="$ORCH/codex-home" codex exec \
  -m "${ORCHESTRA_CODEX_MODEL:-gpt-5.5}" \
  -c sandbox_mode="read-only" -c approval_policy="never" "$PROMPT"
