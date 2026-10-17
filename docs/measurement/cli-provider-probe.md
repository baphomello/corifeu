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

## The flags, corrected by running them

[ADR-0007](../adr/0007-cli-providers-first.md) lists `--bare` as how Claude Code ignores user
and global context, which [ADR-0014](../adr/0014-context-contracts-per-role.md) requires an
adapter to do. Run, it does something else as well:

```
--bare  →  {"is_error": true, "result": "Not logged in · Please run /login"}
```

**`--bare` removes the setting source the credential lives in**, so it is not usable as the
neutralising flag. `--setting-sources` is the narrower one and takes `user,project,local`; an
empty value is rejected rather than meaning "none". So **full context neutralisation is not
available on this version** — the adapter can drop MCP servers and the built-in tools, and the
user's own settings come along.

The combination that works, with everything verified by execution:

```
claude -p "<prompt>" --restricted --strict-mcp-config \
  --json-schema '{"type":"object","properties":{"name":{"type":"string"},
                  "arguments":{"type":"object"}},"required":["name","arguments"]}' \
  --output-format json < /dev/null
```

```json
{"name":"write_file","arguments":{"path":"src/a.ts","content":"export const n = 1;"}}
```

Three things in there are not optional.

**`--json-schema` rather than asking in the prompt.** The first probe asked for one JSON object
in words and got it, which made the schema look like a nicety. It is not: without it the agent
narrates.

**`< /dev/null`.** Without it the CLI waits three seconds and then prints
`Warning: no stdin data received in 3s` **before** the JSON, so a parser reading the whole
output as JSON fails. The adapter redirects stdin and reads from the first `{`.

**A prompt that says not to act.** This is the difference between a CLI agent and a model.
Asked to *"Call write_file with path src/a.ts"*, it tried to use its own Write tool and
reported back that it lacked permission — it spent $0.048 doing the task rather than proposing
it. Asked as *"You are proposing one action for another program to carry out. Do not act"*, it
answered with the call. Corifeu's loop executes the tools, so every dispatch has to say so.

## What this means for the adapter

| Requirement | Why |
|---|---|
| `--restricted --strict-mcp-config` | the CLI's own tools and MCP servers are not Corifeu's |
| `--json-schema` | the shape, not a request for the shape |
| stdin from `/dev/null`, parse from the first `{` | the warning preamble |
| "propose, do not act" in the system message | it is an agent, and it will act |
| `total_cost_usd` per role | `input_tokens` is 2 whatever the call cost |
| the CLI version, recorded and checked | ADR-0007 calls adapter churn the dominant risk |

Not verified here: Codex CLI and Antigravity CLI, which ADR-0007 requires for the abstraction to
stay honest. Their flags are in that ADR and only Claude Code's column has been exercised.
