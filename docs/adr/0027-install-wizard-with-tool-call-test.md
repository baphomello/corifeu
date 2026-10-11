# 0027 — Interactive wizard gated by a real tool-call test

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Most of what Corifeu needs to know cannot be guessed: which CLIs are installed and
authenticated, which local models exist, which model should lead, how roles divide, how git
should behave. Asking produces a long wizard; guessing produces a configuration that fails
later and confusingly.

A specific failure deserves its own mechanism. **A model that cannot reliably emit tool
calls cannot be a doer**, and this is not discoverable from the model's name, size or
advertised capability. The prototype hit this repeatedly — `code.py` grew a text-mode
tool-call parser, a nudge counter and a loop detector precisely because models claiming tool
support did not deliver it.

## Decision
First install runs an **interactive wizard**:

1. **Detect** installed provider CLIs, their versions, and their authentication status.
2. **Detect** local runtimes and **list only the locally installed models**. No
   hardware-based recommendations in MVP 1 — the machine cannot be inspected reliably
   enough to advise honestly.
3. **Choose** the lead model, granularity preset, roles, and a model per role.
4. **Test** every model assigned to a role that needs tools.
5. **Choose** autonomy, git flow, fallback and visibility.
6. **Write** `.corifeu/config.yaml` from the answers.

### The tool-call test

For each model assigned to a tool-using role, send a prompt that **cannot** be satisfied
without a tool call, and approve the model only if a well-formed call arrives. This is the
probe validated during analysis against the live Ollama instance:

- Ollama's `/api/tags` advertises a `capabilities` array containing `tools`. Use it to
  **pre-filter** candidates cheaply, then test for real — advertised capability is necessary,
  not sufficient.
- A `write_file` tool with a two-property schema, and an instruction that only a call can
  satisfy, produced a well-formed native `tool_calls` entry from `qwen3.5:4b` in **9.8 s**.
- For llama.cpp, a failure here most often means `llama-server` was started without
  `--jinja` ([ADR-0008](0008-local-runtimes.md)). The failure message must name that cause
  rather than blaming the model.

The same test is re-runnable as **`corifeu doctor`**, because models get updated, servers
get restarted with different flags, and CLIs get upgraded.

## Alternatives considered
- **A config file only, no wizard.** Lowest friction to build, highest friction to adopt —
  it requires reading the schema before the first run.
- **Full auto-detection with no questions.** Cannot determine preferences, only facts.
- **Trust advertised capability.** The prototype's accumulated workarounds are the evidence
  against it.
- **Hardware-based model recommendations.** Deferred. Honest advice needs VRAM, quantisation
  and throughput data that is awkward to obtain portably, and dishonest advice is worse than
  none.

## Consequences
- The wizard is a substantial piece of UI with its own phase in the roadmap.
- The tool-call test costs time and tokens at install. It is worth it: the alternative is
  discovering the problem mid-run.
- `doctor` becomes the single diagnostic entry point, and the error messages it produces are
  a primary quality surface — the difference between "model failed" and "llama-server is
  running without --jinja" is most of the product's perceived reliability.
- Non-interactive installation must be able to skip or batch the test, and CI must be able
  to run it ([ADR-0028](0028-yaml-config-with-json-schema.md)).
