# corifeu-llamacpp

llama.cpp adapter for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

Speaks `llama-server`'s OpenAI-shaped `/v1/chat/completions`, including its router mode,
where several models are served by name and loaded on demand.

Two things it gives that Ollama does not: the server reports its own measured throughput,
which is a better number for ADR-0031 than one inferred from a wall clock that also contains
model loading; and it refuses a prompt larger than the context instead of serving it anyway.

AGPL-3.0-or-later.
