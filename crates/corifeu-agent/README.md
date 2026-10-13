# corifeu-agent

What a doer sees on each step, for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

The loop does not append to a conversation. It rebuilds a bounded context from state every
step, with the file being edited read fresh from disk rather than recalled from scrollback
(ADR-0047). A repeated action is answered with a warning that points at the oracle, and only
consecutive warnings stop the run (ADR-0050).

Both rules come from measured failures: a doer that spent 28 steps reading one file and made
no edits, and a loop detector that killed a run at step 12 of 400 while it held 18 of 21
tests passing.

AGPL-3.0-or-later.
