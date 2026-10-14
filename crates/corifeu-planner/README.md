# corifeu-planner

Cuts a brief into slices, for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

A slice is small enough when exactly one command goes from failing to passing because of it
(ADR-0056). The planner names that command, and emits the whole ordered list in one pass —
asking a planner per slice would spend on planning what delegation saved on implementation.

A plan is checked before any of it runs. A slice with no oracle, two slices sharing one, or
an oracle the sandbox would refuse are all planning failures, and finding them at plan time
costs one rejection instead of a doer round each.

AGPL-3.0-or-later.
