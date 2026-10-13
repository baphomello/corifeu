# corifeu-config

Configuration model and loader for [Corifeu](https://github.com/baphomello/corifeu).

Published because crates.io forbids path dependencies in a published crate, not because it
is meant to be used on its own. Its interface will change without warning while Corifeu is
pre-1.0.

Loading is three steps: parse each layer of YAML to a generic tree, merge the layers while
recording which one set each value, then deserialize into the typed model. The split is what
lets an error name both the field and the layer that supplied it.

AGPL-3.0-or-later.
