# corifeu-ollama

Ollama adapter for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

One chat turn, with tools, under a generation budget that is never optional (ADR-0048). A
turn that ran out of budget says so rather than coming back short and silent.

It carries its own small HTTP client. A local runtime is reached over plain HTTP on the
loopback interface, which needs no TLS, no redirects, no cookies and no connection reuse —
and the project's dependency tree is small enough that keeping it that way is worth a
hundred lines.

AGPL-3.0-or-later.
