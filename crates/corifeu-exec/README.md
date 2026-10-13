# corifeu-exec

Runs a command on an agent's behalf, for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

A command is a program and its arguments, never a line of shell (ADR-0060). There is no
pipeline to mask an exit status, no heredoc to go around a checkpoint, and no metacharacter
to evade an allowlist — because none of them can be expressed.

What it gives back is what `corifeu-check` judges.

AGPL-3.0-or-later.
