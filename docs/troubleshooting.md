# Troubleshooting

Environment-specific problems that Corifeu deliberately does **not** auto-detect.
Auto-detection guesses, and a wrong guess is harder to debug than a missing value.
Everything here is a value you set once, explicitly.

## A local model runs on a different host than Corifeu

`ORCHESTRA_OLLAMA_URL` (later: `providers.<name>.base_url`) defaults to
`http://127.0.0.1:11434`. It is only correct when the model runtime and Corifeu
share a network namespace. They often do not.

### WSL2, with Ollama on the Windows host

WSL2's default NAT puts Windows on a different address than `127.0.0.1`, and
**that address changes on every boot**. There is no stable hostname to hardcode.

Derive it from the default route at shell start:

```sh
export ORCHESTRA_OLLAMA_URL="http://$(ip route show default | awk '{print $3}'):11434"
```

Then confirm the endpoint actually answers before blaming anything else:

```sh
curl -s "$ORCHESTRA_OLLAMA_URL/api/tags" | jq -r '.models[].name'
```

Two further requirements, both easy to miss:

- Ollama on Windows must listen beyond loopback. Set the `OLLAMA_HOST`
  environment variable to `0.0.0.0` for the Ollama service and restart it;
  otherwise it binds `127.0.0.1` and refuses the WSL interface.
- Windows Defender Firewall must allow inbound TCP 11434 on the private
  profile. A silent timeout — rather than a refused connection — is the usual
  symptom of this one.

### WSL2 mirrored networking

With `networkingMode=mirrored` in `.wslconfig`, `127.0.0.1` reaches the Windows
host directly and the default value works unchanged. Prefer this if you control
the configuration.

### Docker, devcontainers, remote hosts

Same principle, different address:

| Setup | Value |
|---|---|
| Ollama on the container host, Docker Desktop | `http://host.docker.internal:11434` |
| Ollama on the container host, native Linux Docker | `http://172.17.0.1:11434` |
| Ollama on another machine | `http://<host-or-ip>:11434` |

An Ollama reachable over a non-loopback address is **unauthenticated by
default**. Do not expose it to an untrusted network.

## The reviewer uses the wrong model

`ORCHESTRA_CODEX_MODEL` is read in two places: `review.sh` passes it with `-m`,
and `setup-codex-home.sh` writes it into the generated `config.toml`. The
generated file is written **at install time**. Changing the variable afterwards
changes the `-m` flag (which wins) but leaves a stale `config.toml` behind. Re-run
`.orchestra/setup-codex-home.sh` to resynchronise them.

## The guard blocks a write you believe is legitimate

`.orchestra/hooks/guard-src.py` blocks writes to `src/` only. It does **not**
block `tests/` — the orchestrator authors the acceptance oracle and owns that
path.

The guard matches the literal prefix `src/`. Projects that keep production code
elsewhere (`lib/`, `app/`, `internal/`, `pkg/`) are **not protected at all**, and
projects whose unit tests sit beside their sources (`src/bank/deposit.ts` next to
`src/bank/deposit_test.ts`) have their tests blocked along with their sources.
This is a known structural limit of the prefix approach, not a configuration
mistake. Corifeu replaces it with glob-based ownership rules — see
[ADR-0021](adr/0021-file-ownership-by-glob-classes.md).

## Tool calls never arrive from a local model

Check capability before blaming the prompt:

```sh
curl -s "$ORCHESTRA_OLLAMA_URL/api/tags" \
  | jq -r '.models[] | select(.capabilities | index("tools")) | .name'
```

A model absent from that list cannot emit native tool calls, and no amount of
prompting fixes it. `llama.cpp` requires `llama-server --jinja` for
OpenAI-style tool calling, and sometimes an explicit `--chat-template-file`;
without `--jinja` the endpoint silently answers in prose.

Capability is necessary, not sufficient — a model may advertise `tools` and
still fail in practice. `corifeu doctor` runs a real tool call and judges on the
result.
