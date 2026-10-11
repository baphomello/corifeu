# 0008 — Ollama and llama.cpp as local runtimes

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Delegating to local models is the mechanism behind goal one, reducing expensive-token
consumption. Two runtimes dominate local inference on developer machines: Ollama, which is
easiest to install and manage, and llama.cpp, which offers finer control and is what Ollama
itself builds upon.

## Decision
MVP 1 supports **Ollama** and **llama.cpp** (`llama-server`).

Ollama is addressed through its native `/api/chat` with the `tools` parameter, and
`/api/tags` for discovery. llama.cpp is addressed through its OpenAI-compatible
`/v1/chat/completions`.

Verified against the live Ollama instance used for this analysis:

- `GET /api/tags` returns, per model, a **`capabilities` array** containing `tools` where
  the model supports native tool calling. The installer uses this to pre-filter candidates
  cheaply.
- `POST /api/chat` with a `write_file` tool schema returned a well-formed native
  `tool_calls` entry — with an `id` — from `qwen3.5:4b` in 9.8 s.

For llama.cpp, the official server documentation states that OpenAI-style tool calling
requires starting `llama-server` with **`--jinja`**, and sometimes an explicit
`--chat-template-file`. Without it, the endpoint answers in prose and the failure looks
like a model incapable of tool calls rather than a misconfigured server. `doctor` must
detect and name this specific case.

## Alternatives considered
- **Ollama only.** Simpler, covers most users. Rejected: it couples Corifeu to one vendor
  for the capability that justifies the product.
- **OpenAI-compatible endpoints generically, Ollama included.** Attractive, and nearly
  right — but it discards `/api/tags` capability discovery and Ollama's model management,
  both of which materially improve the install experience.
- **LM Studio, vLLM, llamafile.** Reasonable later additions, and natural plugin
  candidates ([ADR-0033](0033-extension-points.md)).

## Consequences
- Capability advertised is not capability delivered. A model may list `tools` and still
  fail in practice, so the advertised flag pre-filters and the real tool-call test decides
  ([ADR-0027](0027-install-wizard-with-tool-call-test.md)).
- The network-reachability problem is real and recurring: WSL-to-Windows-host, containers,
  remote hosts. Corifeu does **not** guess the address; it is configured explicitly and
  documented in [troubleshooting.md](../troubleshooting.md).
- Local models are slow enough that latency is a design input, not an afterthought — 9.8 s
  for a single call from a 4B model. Budgets and timeouts
  ([ADR-0018](0018-loop-and-budget-control.md)) must be set against real local latency.
- Quantisation affects tool-call reliability; the llama.cpp docs warn that aggressive KV
  quantisation degrades it. Worth surfacing in `doctor` output when detectable.
