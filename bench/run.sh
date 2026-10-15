#!/usr/bin/env bash
# Run one measurement round and keep everything it produced.
#
#   bench/run.sh rank 10 qwopus3.5-9b-coder-Q4_K_M ornith1.5-9b-Q5_K_M
#   bench/run.sh pair 10 ornith1.5-9b-Q5_K_M
#
# Environment:
#   BENCH_HOST            llama.cpp router host (default: the WSL gateway)
#   BENCH_ROUND           where to write (default: bench/rounds/<stamp>-<mode>)
#   BENCH_TIMEOUT         seconds per iteration (default: 3600)
#   BENCH_SLICE           which slice the pair works, 1-4 (default: 1)
#   BENCH_TASK            a directory under bench/tasks/ (default: c-glob)
#   BENCH_STEPS_FLOOR     \
#   BENCH_STEPS_CEILING    |  the step allowance, ADR-0062. Unset means the
#   BENCH_STEPS_PATIENCE   |  runtime's own defaults, which is what a round
#   BENCH_STEPS_RAISE_BY  /   measuring the defaults should use.
#
# Every iteration gets its own workspace and every workspace is kept, including
# the ones that fail. Defect 20 was the fourth run reusing one directory: the
# two runs that failed were overwritten by the next iteration and could never be
# looked at.
#
# Each iteration also gets its own cargo target directory, outside the
# workspace, which is removed once the iteration is scored. Two iterations
# sharing one would be two crates with the same package name writing the same
# test binary, and the second `cargo test` can then run the first one's and
# report its result as its own.
set -uo pipefail

MODE=${1:-}
N=${2:-}
shift 2 2>/dev/null
MODELS=("$@")

case "$MODE" in
  rank|pair) ;;
  *) echo "usage: $0 <rank|pair> <iterations> <model>..." >&2; exit 2 ;;
esac
[[ "$N" =~ ^[0-9]+$ ]] || { echo "iterations must be a number" >&2; exit 2; }
(( ${#MODELS[@]} )) || { echo "name at least one model" >&2; exit 2; }

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/.." && pwd)
STAMP=$(date +%Y%m%d-%H%M%S)
ROUND=${BENCH_ROUND:-$HERE/rounds/$STAMP-${BENCH_TASK:-c-glob}-$MODE}
HOST=${BENCH_HOST:-$(ip route show default | awk '{print $3}')}
CAP=${BENCH_TIMEOUT:-3600}
SLICE=${BENCH_SLICE:-1}
TASK=${BENCH_TASK:-c-glob}

# The fourth run's lesson, cheaply: a round against a router that is not there
# produces N rows of PROVIDER_FAILED and takes the full timeout to do it. Any
# HTTP response at all is enough — this probes reachability, not routes.
# `-w` prints a code even when the connection is refused, so there is no `||`
# fallback here: adding one made this read "000000" and never match.
code=$(curl -s --max-time 5 -o /dev/null -w '%{http_code}' "http://$HOST:9931/health")
if [ -z "$code" ] || [ "$code" = "000" ]; then
  echo "no llama.cpp router answering at $HOST:9931 — start it before the round" >&2
  exit 1
fi

# What the task says about itself. The harness reads these rather than assuming
# cargo, so that adding a language is a task directory and not a patch to this
# script. Null-separated and read into an array, so that an argument with a
# space in it survives and nothing here needs an eval.
MANIFEST="$HERE/tasks/$TASK/task.json"
field() { python3 -I -c '
import json, sys
for part in json.load(open(sys.argv[1])).get(sys.argv[2], []):
    sys.stdout.write(part + "\0")
' "$MANIFEST" "$1"; }
mapfile -t -d "" SUITE < <(field whole_suite) || exit 1
mapfile -t -d "" VENV_PACKAGES < <(field venv) || exit 1
(( ${#SUITE[@]} )) || { echo "$MANIFEST declares no whole_suite" >&2; exit 1; }

# A Python task needs an interpreter with its test runner on it, and the one a
# round runs against has to be pinned: `corifeu-check` counts pytest's summary
# line, and that line is a captured sample, not a guess.
if (( ${#VENV_PACKAGES[@]} )); then
  VENV="$HERE/.venv"
  [ -x "$VENV/bin/python" ] || python3 -m venv "$VENV" || exit 1
  "$VENV/bin/python" -m pip install -q "${VENV_PACKAGES[@]}" || exit 1
  export PATH="$VENV/bin:$PATH"
fi

# Built against the repository's own target directory, so it is cached between
# rounds. The round's target directory is set after this, not before.
( cd "$REPO" && cargo build -q -p corifeu-bench ) || exit 1
BIN="$REPO/target/debug/$MODE"

mkdir -p "$ROUND"
RESULTS="$ROUND/results.tsv"
STEPS="$ROUND/steps.tsv"
LOG="$ROUND/log"

{
  echo "round:   $STAMP"
  echo "mode:    $MODE"
  echo "task:    $TASK"
  echo "host:    $HOST"
  echo "models:  ${MODELS[*]}"
  echo "commit:  $(cd "$REPO" && git rev-parse --short HEAD)"
  echo "suite:   ${SUITE[*]}"
  [ "$MODE" = pair ] && echo "slice:   $SLICE"
  echo "steps:   floor=${BENCH_STEPS_FLOOR:-default} ceiling=${BENCH_STEPS_CEILING:-default}"
  echo "         patience=${BENCH_STEPS_PATIENCE:-default} raise_by=${BENCH_STEPS_RAISE_BY:-default}"
} | tee "$ROUND/round.txt"

if [ "$MODE" = rank ]; then
  printf 'iteration\tmodel\tslice\toutcome\tsteps\tprompt\toutput\tchecks\tseconds\tallowed\n' > "$RESULTS"
else
  printf 'iteration\tmodel\tslice\toutcome\tqa_rounds\tqa_steps\tcoder_rounds\tcoder_steps\tprompt\tseconds\theld_out\n' > "$RESULTS"
fi

# One row per step, every iteration appended. The drivers write it beside each
# workspace; this is the whole round in one file, which is what any analysis
# actually wants.
printf 'iteration\tmodel\tmodule\trole\tstep\ttool\targument\tflags\toutcome\n' > "$STEPS"

for i in $(seq 1 "$N"); do
  for M in "${MODELS[@]}"; do
    W="$ROUND/work/$M/$i"
    if [ "$MODE" = rank ]; then SCAFFOLD=(--mode slices); else SCAFFOLD=(--mode pair --slice "$SLICE"); fi
    if ! python3 -I "$HERE/tasks/scaffold.py" "$W" --task "$TASK" "${SCAFFOLD[@]}" >>"$LOG" 2>&1; then
      echo "[$(date +%H:%M:%S)] $MODE $i / $M: scaffold failed" | tee -a "$LOG"
      continue
    fi

    export CARGO_TARGET_DIR="$ROUND/target/$M/$i"
    echo "[$(date +%H:%M:%S)] $MODE $i / $M" >> "$LOG"
    timeout "$CAP" "$BIN" "$HOST" "$M" "$W" "$i" "$SLICE" >> "$RESULTS" 2>> "$LOG"
    status=$?
    if [ "$status" -ne 0 ]; then
      echo "KILLED\t$i\t$M\texit=$status" >> "$RESULTS"
    fi

    # Measured here, not in the driver, so that a round killed at the cap still
    # says what the crate ended up as. This is how the fourth run kept its
    # FINAL rows when an iteration hit the one-hour limit.
    ( cd "$W" && "${SUITE[@]}" ) > "$W/final.txt" 2>&1
    FINAL=$(grep -hoE 'test result: [a-zA-Z]+\. [0-9]+ passed; [0-9]+ failed|[0-9]+ (passed|failed)[0-9a-z, ]* in [0-9.]+s' "$W/final.txt" | tail -1)
    printf 'FINAL\t%s\t%s\t%s\n' "$i" "$M" "${FINAL:-did not build}" >> "$RESULTS"

    [ -f "$W/steps.tsv" ] && cat "$W/steps.tsv" >> "$STEPS"

    # The sources, the checkpoints and the final output are the evidence. The
    # build tree is not, and it is 10 MB an iteration.
    rm -rf "$CARGO_TARGET_DIR"
  done
done

echo "ROUND COMPLETE" >> "$RESULTS"
echo
echo "results:    $RESULTS"
echo "steps:      $STEPS"
echo "workspaces: $ROUND/work/<model>/<iteration>  (all kept, including failures)"
