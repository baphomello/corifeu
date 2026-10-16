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

## What the model card says, and what it corrects here

Read after the measurements above were taken, which is the wrong order and is why this section
exists.

**Thinking needs a context this card cannot give it.** The model card asks for "a context length
of at least 128K tokens to preserve thinking capabilities". Everything below was measured at
12,288 and then 32,768, because 5.8 GB of weights on an 8 GB card leaves no room for more. So
the honest statement is not that this model reasons badly — it is that **thinking is not
usable on this hardware**, and that is a conclusion about the card rather than about the model.

**The sampling was not what the card asks for.** It recommends, for precise coding, temperature
0.6 with `top_k` 20, `top_p` 0.95, `min_p` 0.0 and no presence penalty. These runs used
temperature 0.1 or 0.2 and llama.cpp's defaults for the rest — `top_k` 40 and **`min_p` 0.05**,
which filters tokens the model expects to have. Every score below is therefore a score under
sampling the model was not tuned for, and the prompt finding further down is the one least
likely to be affected by that.

**It also recommends an output length of 32,768 tokens**, and 81,920 for hard coding. These runs
used 6,144. The measurement that a run stopped on its own at 12,180 is still true and was read
too broadly: it stopped on its own *in a context where the card says reasoning is already
degraded*.

**Two flags were requirements rather than choices.** The card states that `-np > 1` and
`--mmproj` are not supported with MTP, so `--parallel 1` and `--no-mmproj` were never optional.
It also recommends `--spec-draft-n-max 6`, where these runs used 3.

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

## Context was never the limit, and more budget does not rescue thinking

The runs above exhausted their **generation** budget, which is `max_tokens`, not the context
window. The prompt is about two hundred tokens and the context was 12,288, so less than half of
it was ever used. Raising the context would not have changed anything, and the question was
worth asking because the two limits are easy to confuse.

Raising the context to 32,768 was done anyway, for a different reason that matters to Corifeu:
a real slice pins the whole artefact into every step, and round five's prompts averaged 3,800
tokens against files of about a hundred lines. It fits.

| | 12,288 | 32,768 |
|---|---:|---:|
| KV with `q8_0`, plus buffers | ~400 MiB | **~800 MiB** |
| Free while loaded | 761 MiB | **381 MiB** |

2.7 times the context for twice the cache, because part of the figure is a fixed buffer. 381 MiB
of headroom is enough to measure with and tight to work alongside.

**Thinking still does not converge with four times the budget.** At `max_tokens` 24,000 two runs
spent 12,180 tokens and 33,000 to 36,000 characters of reasoning and produced no answer at all.
A third, run separately, stopped at 5,435 tokens and did produce one. So the budget was never
the constraint: `finish_reason` is `stop`, which is the model ending its turn — sometimes after
writing an answer and sometimes not.

| | thinking off | thinking on |
|---|---:|---:|
| Tokens to an answer | 266 – 346 | 5,435 – none at 12,180 |
| Seconds | 3 – 4 | 55 – 132 |
| Produced an answer | 8 of 8 | 2 of 5 |

## Quality: fast, and not trustworthy alone

Scores across five runs with thinking off at 12k: **10, 9, 6, 6, 6**. Three more at 32k: **9, 9,
9**.

The three at 32k were more consistent, and that is not attributed to the context here: a reply
using one per cent of the window has no mechanism by which the window could change it. What the
eight runs do show is that the short answers are the bad ones.

The pattern is clean. A 266-token answer scores 6 and a 346-token answer scores 9 or 10, because
the short ones are the naive implementation — they miss both traps the challenge exists to
catch: that every repeated-digit CPF satisfies the checksum and is still invalid, and that a
remainder below two means the digit is zero rather than eleven minus the remainder.

**One run in five got all ten.** That is the number that matters for the product, and it is an
argument for Corifeu rather than against this model: the oracle decides, not the model, and a
model that is right one time in five with a check is more useful than one that is right four
times in five without one.

## The prompt was the lever, not the model

Three cells of three samples, with the server unchanged.

| | Temperature | Prompt | Scores | Tokens |
|---|---|---|---|---|
| A | 0.0 | original | **6, 6, 6** | 266 each |
| B | 0.2 | asks for rigour | **10, 10, 10** | 327 – 354 |
| C | 0.0 | asks for rigour | **10, 10, 10** | 354 each |

**Two sentences took it from six out of ten to ten out of ten, three times out of three.**
Neither sentence names a trap:

> Be exhaustive about the specification. Inputs that are malformed in ways a naive
> implementation would accept must still be rejected, and valid inputs that a naive
> implementation would reject must still be accepted.

Naming the traps would have measured the prompt rather than the model, which is the mistake the
held-out suites exist to avoid. Asking for rigour in general is what a careful engineer writes,
and it is what a doer's system prompt is for.

**Greedy sampling alone made it worse.** Cell A locked onto the naive answer every time, which
means the 10 and the 9 seen earlier at temperature 0.2 were lucky draws on a weak prompt. Five
runs had been read as the model's quality varying. They were the sampler's.

Cell C is the configuration to use: ten out of ten, three identical 354-token answers, three
seconds, around 100 tok/s, and reproducible.

Against what thinking bought:

| | thinking | two sentences |
|---|---|---|
| Seconds | 55 – 132 | **3 – 4** |
| Produced an answer | 2 of 5 | **3 of 3** |
| Best score | 6 | **10** |

**The caveat.** Three samples a cell and one task, and that task is a validator, where asking
for edge cases is unusually well aimed. Whether the same two sentences help on work that is not
about edge cases is untested, and the honest expectation is that they help less.

## What this does not say

- **Nothing about other models.** One model, one quant, one set of flags. The sweep in
  `bench/probe/sweep.sh` exists for the comparison and has not been run.
- **Nothing about a flag sweep.** Context size, KV type, thread count and `-ngl` are all fixed
  here at the values that were chosen before anything was measured.
- **Nothing replicable.** No seed was set on these runs.
- **One task.** Ten traps in one function. A model can be good at CPF and bad at everything
  else.
