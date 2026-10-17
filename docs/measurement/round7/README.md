# Round seven, raw rows

Eight passes against `g-glob-ts`, all at seed base 1, all against
`unsloth/Qwen3.5-9B-MTP-GGUF:UD-Q4_K_XL` on 32,768 of context with a `q8_0` cache.
The record is [round7.md](../round7.md).

| Pass | `BENCH_TEMP` | Speculative decoding |
|---|---|---|
| A, B | 0 | `draft-mtp` |
| C, D | 0.6 | `draft-mtp` |
| E, F | 0.6 | **off** |
| G, H | 0.6 | `draft-mtp` |

**This table is reconstructed from the session, not read from the stored headers**, and that is
defect 44: `round.txt` recorded the seed and not the temperature, so the eight files below are
identical apart from their timestamps and cannot tell a greedy pass from a sampled one. A seed
without a temperature beside it says nothing, which this round proved twice over — at
temperature 0 the ten different seeds inside one pass produced the same output to the token.

`bench/run.sh` now writes `temp:`, `server:` and `spec:` into every header, read from the
server by `bench/server.py`. The `spec:` line comes from `/slots`, because `/props` reports
`speculative.types: none` even when MTP is loaded and would have recorded the opposite of the
truth.
