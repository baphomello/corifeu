#!/usr/bin/env bash
# Round seven — does any of what five rounds measured survive TypeScript?
#
#   bench/round7.sh            run every cell
#   bench/round7.sh 1          run only that one
#
# S3 of docs/roadmap-site.md. This is the stage that can say the plan is the
# wrong plan: a delivery rate near zero means local models cannot do scoped work
# in a Node repository, and the hybrid split has to move before anything else is
# built on top of it.
#
# Two environment changes are accepted and recorded rather than controlled for.
# The GPU driver went from 610.88 to 617.42 between round five and this one, and
# `round.txt` carries the version because of defect 31. The delivery difference
# round five measured between languages — 17 of 20 against 9 of 20 — is far too
# large for a driver to explain, which is the whole reason accepting it is safe.
#
# The seed is the other one, and it is split deliberately:
#
#   cell 1 runs UNSEEDED, because every number it is compared against — Rust's
#   17 of 20, Python's 9 of 20, 36% of steps productive — was measured unseeded,
#   and a cell that changes two things answers neither.
#
#   cell 2 runs SEEDED, because its only question is whether a seed makes a
#   round replayable at all (Q14). It compares against itself.
set -uo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
MODELS=(qwopus3.5-9b-coder-Q4_K_M ornith1.5-9b-Q5_K_M)
STAMP=$(date +%Y%m%d-%H%M%S)
OUT="$HERE/rounds/round7-$STAMP"
mkdir -p "$OUT"
JOURNAL="$OUT/journal"

CHILD=
halt() {
  echo "[$(date +%H:%M:%S)] interrupted; stopping the round" | tee -a "$JOURNAL"
  [ -n "$CHILD" ] && kill -- "-$CHILD" 2>/dev/null
  for pid in $(pgrep -f "$HERE/run.sh"); do kill -9 "$pid" 2>/dev/null; done
  for pid in $(pgrep -f "$(cd "$HERE/.." && pwd)/target/debug/rank"); do kill -9 "$pid" 2>/dev/null; done
  stop_server
  exit 130
}

stop_server() {
  echo "[$(date +%H:%M:%S)] stopping the router" | tee -a "$JOURNAL"
  local pids
  pids=$(tasklist.exe /FI "IMAGENAME eq llama.exe" /FO CSV 2>/dev/null |
    tr -d '\r' | awk -F'","' 'NR>1 {print $2}')
  for pid in $pids; do
    # Graceful taskkill does not work on it: it is a console application.
    taskkill.exe /PID "$pid" /T /F >/dev/null 2>&1
  done
  sleep 3
  echo "[$(date +%H:%M:%S)] llama.exe still running: $(tasklist.exe /FI 'IMAGENAME eq llama.exe' /FO CSV 2>/dev/null | grep -c '"llama.exe"')" |
    tee -a "$JOURNAL"
}
trap stop_server EXIT
trap halt INT TERM

cell() {
  local n=$1 label=$2 iterations=$3
  shift 3
  local dir="$OUT/$n-$label"
  echo "[$(date +%H:%M:%S)] cell $n: $label" | tee -a "$JOURNAL"
  local started=$SECONDS
  set -m
  env "$@" BENCH_ROUND="$dir" "$HERE/run.sh" rank "$iterations" "${MODELS[@]}" \
    >>"$JOURNAL" 2>&1 &
  CHILD=$!
  set +m
  wait "$CHILD"
  local status=$?
  CHILD=
  printf '[%s] cell %s: %s after %d min, exit %d\n' \
    "$(date +%H:%M:%S)" "$n" "$label" $(((SECONDS - started) / 60)) "$status" |
    tee -a "$JOURNAL"
}

WANTED=("$@")
run_it() {
  (( ${#WANTED[@]} == 0 )) && return 0
  local want
  for want in "${WANTED[@]}"; do [ "$want" = "$1" ] && return 0; done
  return 1
}

{
  echo "round seven"
  echo "started: $(date -Iseconds)"
  echo "commit:  $(cd "$HERE/.." && git rev-parse --short HEAD)"
  echo "models:  ${MODELS[*]}"
  echo "cells:   ${WANTED[*]:-all}"
} | tee "$OUT/round7.txt" | tee -a "$JOURNAL"

# S3's delivery number, against Rust's 17 of 20 and Python's 9 of 20.
run_it 1 && cell 1 g-glob-ts-rank 10 BENCH_TASK=g-glob-ts BENCH_SEED=off

# Q14. Two runs, one seed, nothing else different. Agreement proves the sampler
# is seeded, not that the pipeline is deterministic — the oracle shells out to
# npx and a slow start is enough to diverge, so a disagreement is read off
# steps.tsv before it is blamed on the seed.
run_it 2 && cell 2 seed-replay-a 3 BENCH_TASK=g-glob-ts BENCH_SEED=7
run_it 3 && cell 3 seed-replay-b 3 BENCH_TASK=g-glob-ts BENCH_SEED=7

echo "[$(date +%H:%M:%S)] every requested cell finished" | tee -a "$JOURNAL"
echo "output: $OUT"
