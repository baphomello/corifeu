# Corifeu

A runtime that orchestrates AI agents and models to build software.

You talk to **one** agent — the lead. Behind it, a team of specialised roles (planners,
researchers, doers, checkers) does the work, each seeing only what its context contract
grants. Two goals: spend fewer expensive tokens by delegating to local models, and produce
better output by splitting work into small blocks with objective verification.

The name is the Greek *coryphaeus*: the leader of the chorus, the one who addresses the
audience while conducting the others.

## Status

**Not implemented yet.** This release reserves the name. `corifeu` currently prints its
version and exits.

The design is complete and public, decision by decision:

- [Architecture](https://github.com/baphomello/corifeu/blob/main/docs/architecture.md)
- [Decision records](https://github.com/baphomello/corifeu/tree/main/docs/adr) — 45 of them,
  each with its rejected alternatives
- [Roadmap](https://github.com/baphomello/corifeu/blob/main/docs/roadmap.md)
- [Glossary](https://github.com/baphomello/corifeu/blob/main/docs/glossary.md)

Follow or contribute at <https://github.com/baphomello/corifeu>.

## License

AGPL-3.0-or-later. Plugins run out of process over a versioned protocol, so plugin authors
choose their own license and need not write Rust. Contributions use the DCO; there is no
CLA, which means this code cannot be relicensed by anyone, including its author.
