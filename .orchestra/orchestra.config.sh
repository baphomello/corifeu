# orchestra.config.sh — source before using the harness.
#   set -a; . .orchestra/orchestra.config.sh; set +a
# Every value here is an override: `export VAR=...` in your shell wins.

# open = cloud Claude orchestrates + Codex reviews ; private = all-local (not designed yet)
export ORCHESTRA_MODE="${ORCHESTRA_MODE:-open}"

# coder (local). ornith1.5:9b over the faster 7B: honest finish + native tool-calls
# matter more than raw speed in a TDD loop.
export ORCHESTRA_MODEL="${ORCHESTRA_MODEL:-ornith1.5:9b}"
# Host running Ollama. Not localhost? Export ORCHESTRA_OLLAMA_URL yourself.
# See docs/troubleshooting.md for the WSL-to-Windows-host case.
export ORCHESTRA_OLLAMA_URL="${ORCHESTRA_OLLAMA_URL:-http://127.0.0.1:11434}"
export ORCHESTRA_REPO="${ORCHESTRA_REPO:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
export ORCHESTRA_MAX_STEPS="${ORCHESTRA_MAX_STEPS:-30}"

# reviewer (cloud). Single source of truth: review.sh and setup-codex-home.sh both read it.
export ORCHESTRA_CODEX_MODEL="${ORCHESTRA_CODEX_MODEL:-gpt-5.5}"
export ORCHESTRA_CODEX_EFFORT="${ORCHESTRA_CODEX_EFFORT:-medium}"
