# corifeu-doer

Works one slice until its oracle goes green, for
[Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

This is where the decisions meet. Each step rebuilds a bounded context with the working file
read fresh from disk (ADR-0047), asks the model under a budget that is never optional
(ADR-0048), refuses a repeated action with a warning that points at the oracle (ADR-0050),
runs commands as argument vectors behind an allowlist (ADR-0037, ADR-0060), checkpoints
before every write (ADR-0051) and refuses one that would delete the suite (ADR-0057).

The runtime runs the oracle itself after every write, rather than waiting for the model to
think of it. That is what makes a green slice detectable and a regression reversible.

AGPL-3.0-or-later.
