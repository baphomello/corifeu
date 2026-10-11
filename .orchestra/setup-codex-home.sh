#!/usr/bin/env bash
# Minimal CODEX_HOME for the reviewer: auth from ~/.codex, tiny config, no global AGENTS/skills.
set -euo pipefail
CH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/codex-home"
mkdir -p "$CH"
ln -sf "$HOME/.codex/auth.json" "$CH/auth.json"
printf 'model = "%s"\nmodel_reasoning_effort = "%s"\nproject_doc_max_bytes = 0\n' \
  "${ORCHESTRA_CODEX_MODEL:-gpt-5.5}" "${ORCHESTRA_CODEX_EFFORT:-medium}" > "$CH/config.toml"
: > "$CH/AGENTS.md"
echo "codex-home ready at $CH"
