# corifeu-ollama

Ollama adapter for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

One chat turn, with tools, under a generation budget that is never optional (ADR-0048). A
turn that ran out of budget says so rather than coming back short and silent.

It translates Ollama's wire format into the turn `corifeu-provider` defines, and does
nothing else. The transport and the vocabulary live there, shared with the llama.cpp
adapter, because a turn is a turn whichever runtime produced it.

AGPL-3.0-or-later.
