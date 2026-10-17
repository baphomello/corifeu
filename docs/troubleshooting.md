# Troubleshooting

Environment-specific problems that Corifeu deliberately does **not** auto-detect.
Auto-detection guesses, and a wrong guess is harder to debug than a missing value.
Everything here is a value you set once, explicitly.

Run `corifeu doctor` first. It checks most of what follows and names the cause rather than
the symptom.

## A local model runs on a different host than Corifeu

`providers.<name>.base_url` defaults to `http://127.0.0.1:11434` for Ollama. That is only
correct when the model runtime and Corifeu share a network namespace. They often do not.

### WSL2, with Ollama on the Windows host

WSL2's default NAT puts Windows on a different address than `127.0.0.1`, and **that address
changes on every boot**. There is no stable hostname to hardcode, which is why Corifeu does
not try.

Derive it at shell start and let the environment override the config
([ADR-0029](adr/0029-config-precedence-order.md)):

```sh
export CORIFEU_PROVIDERS_LOCAL_BASE_URL="http://$(ip route show default | awk '{print $3}'):11434"
```

Confirm the endpoint answers before blaming anything else:

```sh
curl -s "$CORIFEU_PROVIDERS_LOCAL_BASE_URL/api/tags" | jq -r '.models[].name'
```

Two further requirements, both easy to miss:

- Ollama on Windows must listen beyond loopback. Set `OLLAMA_HOST` to `0.0.0.0` for the
  Ollama service and restart it; otherwise it binds `127.0.0.1` and refuses the WSL
  interface.
- Windows Defender Firewall must allow inbound TCP 11434 on the private profile. A silent
  timeout — rather than a refused connection — is the usual symptom of this one.

### WSL2 mirrored networking

With `networkingMode=mirrored` in `.wslconfig`, `127.0.0.1` reaches the Windows host
directly and the default works unchanged. Prefer this if you control the configuration.

### Docker, devcontainers, remote hosts

Same principle, different address:

| Setup | Value |
|---|---|
| Model runtime on the container host, Docker Desktop | `http://host.docker.internal:11434` |
| Model runtime on the container host, native Linux Docker | `http://172.17.0.1:11434` |
| Model runtime on another machine | `http://<host-or-ip>:11434` |

An Ollama reachable over a non-loopback address is **unauthenticated by default**. Do not
expose it to an untrusted network.

## A local model appears to use its memory twice

A model whose weights are 6 GB can show roughly 7 GB of VRAM **and** roughly 7 GB of private
host RAM at the same time. Corifeu does not do anything about this and does not warn, because
nothing in the measurements shows it costing anything: the round still runs at full speed and
the delivery rate is unaffected.

What it is not: the memory-mapped weights file. That was the first hypothesis and two separate
measurements rule it out — private bytes sit *above* the working set, and loading with
`-lm mmap` against `-lm auto` differs by 40 MB rather than by 7 GB. The remaining candidates
are the loader keeping a host copy of the weights and CUDA pinned staging buffers, and neither
is confirmed.

**When to care.** One model at a time on a machine with plenty of RAM absorbs the extra copy
and you will not notice. It matters if you load two models at once, or run something large
beside the server, and then the symptom is the host swapping rather than the GPU filling.
Defect 29 in [defects.csv](measurement/defects.csv) carries the measurements.

## Tool calls never arrive from a local model

Check capability before blaming the prompt:

```sh
curl -s "$CORIFEU_PROVIDERS_LOCAL_BASE_URL/api/tags" \
  | jq -r '.models[] | select(.capabilities | index("tools")) | .name'
```

A model absent from that list cannot emit native tool calls, and no amount of prompting
fixes it.

**llama.cpp requires `llama-server --jinja`** for OpenAI-style tool calling, and sometimes
an explicit `--chat-template-file`. Without it the server answers in prose, which is
indistinguishable from an incapable model unless you know to look. Aggressive KV
quantisation also degrades tool-call reliability.

Capability advertised is not capability delivered. A model may list `tools` and still fail
in practice, which is why `corifeu doctor` sends a real tool call and judges on the result
([ADR-0027](adr/0027-install-wizard-with-tool-call-test.md)).

## A role was refused a model you configured

Providers are negotiated on two **independent** capabilities: whether tool restriction can
be enforced, and whether ambient context can be suppressed
([ADR-0036](adr/0036-runtime-enforced-tool-restriction.md)). They do not travel together.

| Provider can… | May hold |
|---|---|
| restrict **and** isolate | any role |
| restrict only | unrestricted roles, or roles whose contract grants the whole workspace |
| neither | nothing |

**Antigravity CLI enforces restriction and cannot isolate context.** It loads workspace
`GEMINI.md` and `AGENTS.md` unconditionally, and no flag suppresses it — four approaches
were measured and all failed ([ADR-0045](adr/0045-prepared-workspace-for-context-isolation.md)).
For MVP 1 it is therefore refused for roles requiring a clean context contract. This is a
deliberate refusal, not a bug.

If a model you expected to be available is not offered for a role, `corifeu doctor` names
which capability it lacks.

## An agent behaved as if it had instructions you did not give

Agent-context files in your repository — `CLAUDE.md`, `AGENTS.md`, `GEMINI.md` — are loaded
by the provider CLIs themselves. Corifeu suppresses them where the provider offers a flag
(`--bare` for Claude Code, `--ignore-user-config` for Codex) and cannot where it does not.

This is a security consideration as much as a configuration one. Content you did not write
— a vendored dependency's `AGENTS.md`, a file arriving in a pull request — can carry
instructions aimed at the model. We have measured a case where a planted directive was
obeyed. Capability restriction is what bounds the damage: an injected instruction cannot
exceed what the role was granted. See [SECURITY.md](../SECURITY.md).

## A write was refused

Corifeu decides who may write a file by **file class**, not by directory
([ADR-0035](adr/0035-file-ownership-by-glob-classes.md)). Classes are ordered globs,
first match wins, and a path matching no class is unwritable by doers — the model fails
closed.

```sh
corifeu explain-path src/bank/deposit.ts
```

prints the class it resolves to, the rule that matched, and the roles that may write it.

The common surprise is ordering. `test` is declared before `source` precisely so that
`src/bank/deposit_test.ts` classifies as a test rather than as source. Reordering those
rules changes who owns your tests.

## A configuration value is not what you expected

Four sources, highest first: CLI flags, `CORIFEU_*` environment variables, the project's
`.corifeu/config.yaml`, then your global config
([ADR-0029](adr/0029-config-precedence-order.md)).

```sh
corifeu config show --explain
```

prints every effective value with the source it came from. Reach for this before reading
config files by hand.

## Git state is not what you expected

Every git operation is configured step by step and defaults to its most conservative option
([ADR-0025](adr/0025-git-flow-chosen-at-install.md)). If Corifeu is not pushing, opening a
pull request or merging, it is probably because you were not asked to enable it.

Where the autonomy level and the git flow disagree, **the stricter wins**. Setting
`autonomy: autonomous` does not enable pushing.
