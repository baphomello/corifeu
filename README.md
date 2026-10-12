# Corifeu

A runtime that orchestrates AI agents and models to build software.

You talk to **one** agent — the lead. Behind it, a team of specialised roles does the work,
each seeing only what its context contract grants it.

The name is the Greek *coryphaeus* (Portuguese *corifeu*): the leader of the chorus, the one
who addresses the audience while conducting the others. The metaphor is the architecture —
exactly one agent faces you, the rest are conducted behind it.

> **Status: in development.** The design is complete and public; the runtime is being built.
> `corifeu` currently prints its version. Follow [the roadmap](docs/roadmap.md) for what
> lands when.

## Why

**Spend fewer expensive tokens.** Most of the work in a coding task is not the hard part.
Exploring a codebase, applying a mechanical change, running checks — these can go to a local
model. The expensive model coordinates and never accumulates the whole run in its context.

**Produce better output.** Work is split into small blocks with explicit acceptance
criteria, handled by specialised roles, and verified by objective oracles. A failing test
outranks every agent's opinion, including the lead's.

## How it works

```
You ─── lead ─── planners    decompose the work
                 researchers gather context cheaply
                 doers       write the code
                 checkers    verify it independently
                     │
                 oracles     build · test · lint · types
```

Five ideas carry the design:

| | |
|---|---|
| **One agent faces you** | The lead is the only agent you talk to |
| **No agent holds the whole run** | Each gets an assembled view, never the full board |
| **Nothing exists until it is recorded** | Prose assertions are not state |
| **Oracles outrank opinions** | A failing test beats any argument |
| **Capability, not instruction** | An agent that must not do something is never given the means |

## Documentation

| | |
|---|---|
| [Architecture](docs/architecture.md) | Components, run lifecycle, data model, adapters |
| [Decision records](docs/adr/) | 45 ADRs, each with its rejected alternatives |
| [Configuration](docs/config.md) | The config model and a worked example |
| [Roadmap](docs/roadmap.md) | Phases, deliverables, acceptance criteria |
| [Glossary](docs/glossary.md) | The vocabulary, used narrowly |
| [Risks](docs/risks.md) | What could go wrong, and what we do about it |
| [Troubleshooting](docs/troubleshooting.md) | Environment-specific problems |

Start with the [glossary](docs/glossary.md) if you only read one. Its first section — *roles
are the unit of restriction, models are a resource roles consume* — resolves most of what is
confusing about the rest.

## Installing

Not yet. When it ships:

```sh
# Linux, macOS, WSL
curl -fsSL https://github.com/baphomello/corifeu/releases/latest/download/install.sh | sh

# Windows (PowerShell)
irm https://github.com/baphomello/corifeu/releases/latest/download/install.ps1 | iex
```

Run from the root of your repository. An interactive wizard detects your installed agent
CLIs and local models, tests that each model assigned a tool-using role can actually call
tools, and writes `.corifeu/config.yaml`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Read the [decision records](docs/adr/) before
proposing a change that contradicts one — superseding an ADR with an argument is welcome;
contradicting one silently is not.

Contributions use the [DCO](https://developercertificate.org/) (`git commit -s`). There is
no CLA and never will be: without aggregated rights, nobody — including the author — can
relicense this code.

## License

[AGPL-3.0-or-later](LICENSE). Modifications stay open, including when offered as a network
service.

**Plugins are different.** They run out of process over a versioned JSON-RPC protocol, so
they are not derived works: plugin authors choose their own license, and need not write
Rust.
