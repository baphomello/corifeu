# corifeu-provider

The vocabulary every provider adapter speaks, for
[Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

A turn is a turn whichever runtime produced it: what the model said, what it thought, what it
asked to run, why it stopped, and what it cost. An adapter's job is to translate one
runtime's wire format into that, and nothing else.

It also carries the small HTTP client the local runtimes are reached with. Both listen on
loopback over plain HTTP, which needs none of what an HTTP library exists for.

AGPL-3.0-or-later.
