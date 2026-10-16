#!/usr/bin/env bash
# Which model, under which runtime flags, gets ten out of ten fastest.
#
#   bench/probe/sweep.sh list        what the router offers, against models.txt
#   bench/probe/sweep.sh models      one baseline config, every model
#   bench/probe/sweep.sh flags       every flag combination, on one model
#   bench/probe/sweep.sh all
#
# The models are named in models.txt rather than here, because the quant tag in
# a Hugging Face alias is the thing most likely to need correcting and nobody
# should edit bash to fix a typo.
#
# A probe, not a measurement round. A round asks how a doer behaves over many
# steps against an oracle it cannot see; this asks a single question once and
# scores the answer, because ranking models and flags needs many cheap samples
# rather than few expensive ones.
#
# Two numbers come out of each run and both matter. `passed` is out of ten and
# anything below ten is a wrong answer, however fast. `tok/s` is measured end to
# end from this side of the HTTP call, not read out of the server's own log,
# because what a round pays for is wall time including the server's overhead.
#
# The router is started and stopped around every configuration, since the flags
# under test are the server's own and `--models-max 1` means one model at a
# time by construction.
set -uo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
LLAMA='C:\Users\Mello\AppData\Local\Microsoft\WindowsApps\llama.exe'
MODELS_DIR='C:\Users\Mello\models'
HOST=${PROBE_HOST:-$(ip route show default | awk '{print $3}')}
PORT=9931
# Five rather than three. The first sweep had one model score 5 of 10 and then
# 10 of 10 under the same flags, so a rank built on three samples would be a
# rank of the draw.
SAMPLES=${PROBE_SAMPLES:-5}
OUT=${PROBE_OUT:-$HERE/runs/$(date +%Y%m%d-%H%M%S)}
mkdir -p "$OUT"
RESULTS="$OUT/results.tsv"

mapfile -t MODELS < <(grep -vE '^\s*(#|$)' "$HERE/models.txt")
(( ${#MODELS[@]} )) || { echo "models.txt names nothing" >&2; exit 2; }

stop_router() {
  local pids
  pids=$(tasklist.exe /FI "IMAGENAME eq llama.exe" /FO CSV 2>/dev/null |
    tr -d '\r' | awk -F'","' 'NR>1 {print $2}')
  for pid in $pids; do taskkill.exe /PID "$pid" /T /F >/dev/null 2>&1; done
  sleep 3
}
trap 'echo; echo "interrupted"; stop_router; exit 130' INT TERM
trap stop_router EXIT

# Start the router with one configuration. Returns non-zero if it never answers,
# which is itself a result: a configuration that cannot load is a configuration.
start_router() {
  local args="'serve','--models-dir','$MODELS_DIR','--models-max','1','--host','0.0.0.0','--port','$PORT'"
  local a
  for a in "$@"; do args="$args,'$a'"; done
  powershell.exe -NoProfile -Command \
    "\$a=@($args); Start-Process -FilePath '$LLAMA' -ArgumentList \$a -WindowStyle Hidden \
     -RedirectStandardOutput 'C:\\Users\\Mello\\probe.log' -RedirectStandardError 'C:\\Users\\Mello\\probe.err'" \
    >/dev/null 2>&1 &
  local i
  for i in $(seq 1 40); do
    sleep 2
    [ "$(curl -s --max-time 2 -o /dev/null -w '%{http_code}' "http://$HOST:$PORT/health")" = "200" ] && return 0
  done
  return 1
}

# Ask once, score once. Writes the answer and the raw reply beside the results.
ask() {
  local model="$1" label="$2" sample="$3"
  local work="$OUT/$label/$sample"
  mkdir -p "$work"
  python3 -I "$HERE/ask.py" \
    --host "$HOST:$PORT" --model "$model" --prompt "$HERE/cpf.prompt.txt" --out "$work"
}

# One configuration: start, sample, score, stop.
config() {
  local label="$1" model="$2"
  shift 2
  echo "[$(date +%H:%M:%S)] $label"
  if ! start_router "$@"; then
    printf '%s\t%s\tROUTER_FAILED\t-\t-\t-\t-\n' "$label" "$model" >> "$RESULTS"
    echo "  the router never answered"
    stop_router
    return
  fi

  local s
  for s in $(seq 1 "$SAMPLES"); do
    local work="$OUT/$label/$s"
    mkdir -p "$work"
    local line
    line=$(ask "$model" "$label" "$s") || line="ASK_FAILED 0 0 0"
    # passed tok_s gen_tokens seconds
    read -r passed tok_s gen secs <<<"$line"
    local vram
    vram=$(nvidia-smi.exe --query-gpu=memory.used --format=csv,noheader,nounits 2>/dev/null | tr -d '\r' | head -1)
    local ram
    ram=$(powershell.exe -NoProfile -Command \
      "\$p = Get-Process llama -EA SilentlyContinue | Sort-Object WorkingSet64 -Descending | Select-Object -First 1; \
       if (\$p) { [math]::Round(\$p.PrivateMemorySize64/1GB,2) } else { 0 }" 2>/dev/null | tr -d '\r' | head -1)
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "$label" "$model" "$passed" "$tok_s" "$gen" "$secs" "${vram:-?}" "${ram:-?}" >> "$RESULTS"
    printf '  sample %s: %s/10 at %s tok/s (%s tokens in %ss), vram %s MiB, ram %s GB\n' \
      "$s" "$passed" "$tok_s" "$gen" "$secs" "${vram:-?}" "${ram:-?}"
  done
  stop_router
}

printf 'config\tmodel\tpassed\ttok_s\tgen_tokens\tseconds\tvram_mib\tram_gb\n' > "$RESULTS"
{
  echo "probe:   $(basename "$OUT")"
  echo "commit:  $(cd "$REPO" && git rev-parse --short HEAD)"
  echo "host:    $HOST:$PORT"
  echo "samples: $SAMPLES per configuration"
  echo "driver:  $(nvidia-smi.exe --query-gpu=driver_version --format=csv,noheader 2>/dev/null | tr -d '\r' | head -1)"
} | tee "$OUT/probe.txt"
echo

WHICH=${1:-all}

# What the router offers, against what models.txt names. A misspelled alias
# otherwise looks exactly like a model that cannot answer, and the difference
# matters more than anything else this script measures.
if [ "$WHICH" = list ]; then
  start_router -c 8192 || { echo "the router never answered"; exit 1; }
  echo "=== offered by the router ==="
  curl -s --max-time 20 "http://$HOST:$PORT/v1/models" |
    python3 -I -c 'import json,sys; [print("  "+m["id"]) for m in json.load(sys.stdin)["data"]]'
  echo "=== named in models.txt ==="
  offered=$(curl -s --max-time 20 "http://$HOST:$PORT/v1/models" |
    python3 -I -c 'import json,sys; [print(m["id"]) for m in json.load(sys.stdin)["data"]]')
  for m in "${MODELS[@]}"; do
    if grep -qxF "$m" <<<"$offered"; then echo "  ok      $m"; else echo "  MISSING $m"; fi
  done
  stop_router
  exit 0
fi

# Pass one. One configuration, every model, so the question "which of these can
# answer at all" is settled before any flag is swept.
#
# The baseline does not name -ngl. Setting it aborts llama.cpp's own
# fit-to-free-VRAM pass — the load log says "n_gpu_layers already set by user to
# 99, abort" — and a 12B model against 7 GB of usable VRAM would simply fail to
# load, which measures the card rather than the model. Letting the runtime
# choose offloads everything for the models that fit and as much as fits for the
# one that does not, which is what a person would actually do.
if [ "$WHICH" = models ] || [ "$WHICH" = all ]; then
  echo "=== pass one: every model at the baseline ($SAMPLES samples each) ==="
  for m in "${MODELS[@]}"; do
    short=$(sed 's#.*/##; s/-GGUF//; s/:.*//' <<<"$m" | cut -c1-24)
    config "base-$short" "$m" -c 8192 -fa on
  done
fi

# Pass two. The flags, on one model, so the effect of each is visible rather
# than tangled with a model change. PROBE_MODEL picks which.
if [ "$WHICH" = flags ] || [ "$WHICH" = all ]; then
  M=${PROBE_MODEL:-${MODELS[0]}}
  echo "=== pass two: the flags, on $M ==="
  config "ctx8k-f16"      "$M" -ngl 99 -c 8192  -fa on
  config "ctx16k-f16"     "$M" -ngl 99 -c 16384 -fa on
  config "ctx16k-q8"      "$M" -ngl 99 -c 16384 -fa on -ctk q8_0 -ctv q8_0
  config "ctx16k-q4"      "$M" -ngl 99 -c 16384 -fa on -ctk q4_0 -ctv q4_0
  config "ctx16k-q8-t6"   "$M" -ngl 99 -c 16384 -fa on -ctk q8_0 -ctv q8_0 -t 6
  config "ctx16k-q8-t10"  "$M" -ngl 99 -c 16384 -fa on -ctk q8_0 -ctv q8_0 -t 10
  config "ctx16k-q8-b2048" "$M" -ngl 99 -c 16384 -fa on -ctk q8_0 -ctv q8_0 -b 2048 -ub 512
  config "ctx16k-q8-mmap" "$M" -ngl 99 -c 16384 -fa on -ctk q8_0 -ctv q8_0 -lm mmap
  # `-ngl 99` disables llama.cpp's own fit-to-free-VRAM pass, which logs
  # "n_gpu_layers already set by user to 99, abort". Leaving it out is the only
  # way to find out what it would have chosen.
  config "ctx16k-q8-autofit" "$M" -c 16384 -fa on -ctk q8_0 -ctv q8_0
fi

echo
echo "results: $RESULTS"
