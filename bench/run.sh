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
ROUND=${BENCH_ROUND:-$HERE/rounds/$STAMP-$MODE}
HOST=${BENCH_HOST:-$(ip route show default | awk '{print $3}')}
CAP=${BENCH_TIMEOUT:-3600}

# Built against the repository's own target directory, so it is cached between
# rounds. The round's target directory is set after this, not before.
( cd "$REPO" && cargo build -q -p corifeu-bench ) || exit 1
BIN="$REPO/target/debug/$MODE"

mkdir -p "$ROUND"
RESULTS="$ROUND/results.tsv"
LOG="$ROUND/log"

{
  echo "round:   $STAMP"
  echo "mode:    $MODE"
  echo "host:    $HOST"
  echo "models:  ${MODELS[*]}"
  echo "commit:  $(cd "$REPO" && git rev-parse --short HEAD)"
  echo "steps:   floor=${BENCH_STEPS_FLOOR:-default} ceiling=${BENCH_STEPS_CEILING:-default}"
  echo "         patience=${BENCH_STEPS_PATIENCE:-default} raise_by=${BENCH_STEPS_RAISE_BY:-default}"
} | tee "$ROUND/round.txt"

if [ "$MODE" = rank ]; then
  printf 'iteration\tmodel\tslice\toutcome\tsteps\tprompt\toutput\tchecks\tseconds\tallowed\n' > "$RESULTS"
else
  printf 'iteration\tmodel\toutcome\tqa_rounds\tqa_steps\tcoder_rounds\tcoder_steps\tprompt\tseconds\theld_out\n' > "$RESULTS"
fi

for i in $(seq 1 "$N"); do
  for M in "${MODELS[@]}"; do
    W="$ROUND/work/$M/$i"
    if ! python3 -I "$HERE/tasks/scaffold.py" "$W" --mode "$([ "$MODE" = rank ] && echo slices || echo pair)" >>"$LOG" 2>&1; then
      echo "[$(date +%H:%M:%S)] $MODE $i / $M: scaffold failed" | tee -a "$LOG"
      continue
    fi

    export CARGO_TARGET_DIR="$ROUND/target/$M/$i"
    echo "[$(date +%H:%M:%S)] $MODE $i / $M" >> "$LOG"
    timeout "$CAP" "$BIN" "$HOST" "$M" "$W" "$i" >> "$RESULTS" 2>> "$LOG"
    status=$?
    if [ "$status" -ne 0 ]; then
      echo "KILLED\t$i\t$M\texit=$status" >> "$RESULTS"
    fi

    # Measured here, not in the driver, so that a round killed at the cap still
    # says what the crate ended up as. This is how the fourth run kept its
    # FINAL rows when an iteration hit the one-hour limit.
    ( cd "$W" && cargo test --lib ) > "$W/final.txt" 2>&1
    FINAL=$(grep -hoE 'test result: [a-zA-Z]+\. [0-9]+ passed; [0-9]+ failed' "$W/final.txt" | tail -1)
    printf 'FINAL\t%s\t%s\t%s\n' "$i" "$M" "${FINAL:-did not build}" >> "$RESULTS"

    # The sources, the checkpoints and the final output are the evidence. The
    # build tree is not, and it is 10 MB an iteration.
    rm -rf "$CARGO_TARGET_DIR"
  done
done

echo "ROUND COMPLETE" >> "$RESULTS"
echo
echo "results:    $RESULTS"
echo "workspaces: $ROUND/work/<model>/<iteration>  (all kept, including failures)"
