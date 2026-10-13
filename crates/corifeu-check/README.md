# corifeu-check

Judges what a check command produced, for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

A check has four possible verdicts, not two. Beside `passed` and `failed` there is
`did-not-terminate`, which is evidence about the code rather than a failing test, and
`vacuous`, which is a check that ran fewer cases than its slice declares — the state a test
runner reports as success when the suite it was asked for no longer exists.

This crate does not run commands. It judges what a run produced.

AGPL-3.0-or-later.
