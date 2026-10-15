#!/usr/bin/env bash
# Round five, every cell, in the order that keeps the answers comparable.
#
#   bench/round5.sh            run every cell
#   bench/round5.sh 1 2 3      run only those cells
#
# Cells 1 to 3 are the same mode, the same models, the same step allowance and
# the same shape of task, differing only in the task. That is what makes them
# comparable to each other, and it is why they come first: Q3 asks whether
# ADR-0062's floor of twelve holds in another language, and the fourth run's
# Task C numbers cannot answer it. Those were measured with a ceiling of thirty
# and a walk that stopped at the first failure, both of which ADR-0062 and
# ADR-0063 replaced. The baseline has to be re-measured under the runtime being
# questioned.
#
# Q7 to Q12 come out of steps.tsv and need no cell of their own.
#
# The server is stopped when this script ends, however it ends. It exists on
# that machine for this project and must not be left holding a model's weight
# on the GPU afterwards.
set -uo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
MODELS=(qwopus3.5-9b-coder-Q4_K_M ornith1.5-9b-Q5_K_M)
N=${ROUND5_N:-10}
STAMP=$(date +%Y%m%d-%H%M%S)
OUT="$HERE/rounds/round5-$STAMP"
mkdir -p "$OUT"
JOURNAL="$OUT/journal"

# Killed on purpose: stop the children too, then leave. A plain `trap` handler
# returns and bash carries on with the next cell, so stopping this script took
# hunting run.sh down by hand the first time it mattered.
halt() {
  echo "[$(date +%H:%M:%S)] interrupted; stopping the round" | tee -a "$JOURNAL"
  pkill -P $$ 2>/dev/null
  pkill -f "$HERE/run.sh" 2>/dev/null
  pkill -f "$(cd "$HERE/.." && pwd)/target/debug/rank" 2>/dev/null
  pkill -f "$(cd "$HERE/.." && pwd)/target/debug/pair" 2>/dev/null
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
  local left
  left=$(tasklist.exe /FI "IMAGENAME eq llama.exe" /FO CSV 2>/dev/null | grep -c '"llama.exe"')
  echo "[$(date +%H:%M:%S)] llama.exe still running: $left" | tee -a "$JOURNAL"
}
trap stop_server EXIT
trap halt INT TERM

cell() {
  local n=$1 label=$2 mode=$3
  shift 3
  local dir="$OUT/$n-$label"
  echo "[$(date +%H:%M:%S)] cell $n: $label" | tee -a "$JOURNAL"
  local started=$SECONDS
  env "$@" BENCH_ROUND="$dir" "$HERE/run.sh" "$mode" "$N" "${MODELS[@]}" \
    >>"$JOURNAL" 2>&1
  local status=$?
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
  echo "round five"
  echo "started: $(date -Iseconds)"
  echo "commit:  $(cd "$HERE/.." && git rev-parse --short HEAD)"
  echo "models:  ${MODELS[*]}"
  echo "n:       $N"
  echo "cells:   ${WANTED[*]:-all}"
} | tee "$OUT/round5.txt" | tee -a "$JOURNAL"

# 1 to 3 — rank at the defaults, one task each. Mutually comparable.
# Answers Q2, Q3, Q5's middle floor, and all of Q7 to Q12.
run_it 1 && cell 1 c-glob-rank-default   rank BENCH_TASK=c-glob
run_it 2 && cell 2 d-glob-py-rank-default rank BENCH_TASK=d-glob-py
run_it 3 && cell 3 e-indep-rank-default  rank BENCH_TASK=e-indep

# 4 and 5 — the pair, on the cheap slice and the expensive one. Q1 and Q4.
run_it 4 && cell 4 c-glob-pair-slice1 pair BENCH_TASK=c-glob BENCH_SLICE=1
run_it 5 && cell 5 c-glob-pair-slice3 pair BENCH_TASK=c-glob BENCH_SLICE=3

# 6 and 7 — the rest of Q5's floor sweep. Last on purpose: round5.md says to cut
# this cell first if the round has to be cut, because it is the one most fitted
# to the data that produced it.
run_it 6 && cell 6 c-glob-rank-floor8  rank BENCH_TASK=c-glob BENCH_STEPS_FLOOR=8
run_it 7 && cell 7 c-glob-rank-floor30 rank BENCH_TASK=c-glob BENCH_STEPS_FLOOR=30

echo "[$(date +%H:%M:%S)] every requested cell finished" | tee -a "$JOURNAL"
echo "output: $OUT"
