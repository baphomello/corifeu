# Probe — driving an agent CLI as a provider

Groundwork for S5. `ProviderKind::Cli` is declared and
[`run.rs`](../../crates/corifeu/src/run.rs) refuses it: *"provider `x` is Cli, which `corifeu
run` cannot drive yet"*. Before building the adapter, two questions were measured rather than
assumed: whether a CLI agent can be made to answer in the shape
[`Provider::chat`](../../crates/corifeu-provider/src/lib.rs) returns, and what it reports about
what it spent.

- **CLI:** `claude` 2.1.302, `-p` with `--output-format json`.
- **Model it chose:** `claude-opus-5`.

## It answers in the shape Corifeu already reads

Asked for one JSON object and nothing else, `result` came back as exactly that:

```json
{"name": "write_file", "arguments": {"path": "src/a.ts", "content": "export const n = 1;"}}
```

[`call_in_text`](../../crates/corifeu-provider/src/lib.rs) parses it unchanged. **No new parser
is needed**, and that function already exists because a local 7B model does the same thing — a
CLI agent and a small local model turn out to need the same accommodation.

The rest of a `Turn` maps straight across:

| `Turn` field | From |
|---|---|
| `content` | `result` |
| `tool_calls` | `call_in_text(result)` |
| `stop` | `stop_reason`: `end_turn` → `Done` |
| `prompt_tokens` | `usage.input_tokens` — **see below** |
| `output_tokens` | `usage.output_tokens` |

An error is `is_error`, with `api_error_status` beside it.

## The trap: the expensive side looks free

Two calls, each a few tokens of real work:

| | Call 1 | Call 2 |
|---|---:|---:|
| `usage.input_tokens` | 2 | 2 |
| `usage.cache_read_input_tokens` | 8,857 | 11,639 |
| `usage.cache_creation_input_tokens` | 10,407 | 7,701 |
| `usage.output_tokens` | 4 | 42 |
| **`total_cost_usd`** | **0.1086** | **0.0839** |

**`input_tokens` is 2.** A CLI agent carries its own system prompt and tool definitions, and
they arrive as cache reads rather than as input. So the obvious implementation of S5's
acceptance 18 — tokens per role, read from `usage.input_tokens` — would report the expensive
planner consuming two tokens against the local coder's thousands, and
[ADR-0031](../adr/0031-token-savings-metering.md)'s savings figure would show a discount of
about 100%. The number would be wrong in the direction that flatters the project, which is the
worst direction for a number to be wrong in.

**What the record has to carry instead:** `total_cost_usd` per role, and the cache fields
beside the input count. Cost is the honest unit for a provider that bills, and it is the one
the CLI reports directly.

## What this costs per dispatch

About **$0.08 to $0.11 for a call that did almost nothing**, because the floor is the agent's
own context rather than the work. That is a design constraint rather than a surprise: it means
the expensive role has to be the one that is called a few times per task — the planner, the
reviewer — and never a doer taking twenty steps. Which is the arrangement
[ADR-0007](../adr/0007-cli-providers-first.md) and the hybrid split already describe, now
with a number against it.

A fifty-dispatch round against this provider is roughly five dollars. The project owner
authorised the spend; this page exists so the shape of it is on the record.
