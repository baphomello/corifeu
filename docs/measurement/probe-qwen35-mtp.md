# Probe — Qwen3.5-9B-MTP, UD-Q4_K_XL, on an RTX 3070

One model, one question, scored and timed. Not a measurement round: a round asks how a doer
behaves over many steps against an oracle it cannot see, and this asks one question once,
because ranking a model and its flags needs many cheap samples rather than few expensive ones.

- **Model:** `unsloth/Qwen3.5-9B-MTP-GGUF:UD-Q4_K_XL`, 6.14 GB.
- **Server:** `-c 12288 -fa on -ctk q8_0 -ctv q8_0 -t 6 --no-mmproj --parallel 1
  --spec-type draft-mtp --spec-draft-n-max 3 --temp 0.2`
- **Runtime:** llama.cpp `0.7.0-dev (build 11671)`. **Driver:** 617.42. **Card:** RTX 3070, 8 GB.
- **Challenge:** [`bench/probe/cpf.spec.ts`](../../bench/probe/cpf.spec.ts), ten traps. A
  competent but naive implementation scores 8; the reference scores 10.

## It fits, with room

| | MiB |
|---|---:|
| Card | 8,192 |
| Desktop | ~983 |
| Weights | 5,855 |
| KV at 12,288 with `q8_0`, plus buffers | **~400** |
| **Free while loaded** | **761** |

The KV figure is the one worth keeping: **12k of context with an 8-bit cache costs about 400
MiB on this model**, against the ~1,050 MiB this page's author predicted. The prediction was
wrong twice over — once for guessing the KV from a single f16 measurement, and once for
comparing Hugging Face's decimal GB against a binary GiB figure, which made every margin look
430 MiB tighter than it was.

## Speed: MTP delivers

| | tok/s |
|---|---:|
| `ornith1.5-9b` Q5_K_M, no MTP, 8k context | ~57 |
| This, with `--spec-type draft-mtp` | **~100** |

The load log confirms the head was used rather than ignored:
`common_speculative_init_result: creating MTP draft context against the target model`. That
line is the one worth checking, because a drafting flag that is silently dropped produces a
number that looks legitimate.

## Thinking costs twenty times the wall clock and buys nothing here

| | thinking off | thinking on |
|---|---:|---:|
| Samples | 5 | 3 |
| **Produced an answer at all** | **5 of 5** | **1 of 3** |
| Median score | 6 / 10 | **0 / 10** |
| Best sample | **10 / 10** | 6 / 10 |
| Seconds per answer | **3 – 4** | 55 – 64 |
| tok/s | ~100 | ~95 |

Tokens per second is the same either way, so this is not about speed of generation. It is about
how many tokens are spent before an answer appears: with thinking on, the model writes 14,000
to 17,000 characters of reasoning and twice out of three times **exhausts the budget without
ever writing the answer**. The one run that finished scored 6, which is what the three-second
answers score.

This is the measurement behind [ADR-0048](../adr/0048-bounded-generation-for-local-runtimes.md)'s
`think: false` for roles whose work is transcription, and it is the first time that setting has
been checked rather than assumed — which matters, because until today it was implemented
backwards. Defect 39: the runtime asked for `reasoning_format: "none"`, whose own help says it
"leaves thoughts unparsed in `message.content`", so the model thought anyway and wrote the
thinking into the answer.

## Quality: fast, and not trustworthy alone

Scores across five runs with thinking off: **10, 9, 6, 6, 6**.

The pattern is clean. A 266-token answer scores 6 and a 346-token answer scores 9 or 10, because
the short ones are the naive implementation — they miss both traps the challenge exists to
catch: that every repeated-digit CPF satisfies the checksum and is still invalid, and that a
remainder below two means the digit is zero rather than eleven minus the remainder.

**One run in five got all ten.** That is the number that matters for the product, and it is an
argument for Corifeu rather than against this model: the oracle decides, not the model, and a
model that is right one time in five with a check is more useful than one that is right four
times in five without one.

## What this does not say

- **Nothing about other models.** One model, one quant, one set of flags. The sweep in
  `bench/probe/sweep.sh` exists for the comparison and has not been run.
- **Nothing about a flag sweep.** Context size, KV type, thread count and `-ngl` are all fixed
  here at the values that were chosen before anything was measured.
- **Nothing replicable.** No seed was set on these runs.
- **One task.** Ten traps in one function. A model can be good at CPF and bad at everything
  else.
