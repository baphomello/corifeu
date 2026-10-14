# corifeu-workspace

Where a role's writes go, for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

Three rules sit on the write path. A file belongs to the first class whose globs claim it,
so a test file beside its source classifies as a test (ADR-0035). A role writes only the
classes it owns (ADR-0052). A write that would reduce a protected block count is refused
before it lands, because in a language that keeps tests inside the file under test, ownership
has no granularity to work with (ADR-0057).

Every write that survives those is checkpointed first, so a regression can be undone
(ADR-0051).

AGPL-3.0-or-later.
