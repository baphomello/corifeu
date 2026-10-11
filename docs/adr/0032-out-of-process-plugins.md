# 0032 — Plugins run out of process over a versioned protocol

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Adding a plugin must require **zero changes to this repository**. Someone publishes a plugin
in their own repository; a user downloads it, references it in configuration, and it works.

Three constraints shape how:

1. **Licensing.** The core is AGPL-3.0 ([ADR-0003](0003-agpl-with-notice-and-dco.md)), and
   plugin authors must be free to choose their own licence. An in-process plugin linked into
   the binary would arguably be a derivative work, forcing AGPL onto it.
2. **Language.** Requiring plugin authors to write Rust excludes most of them.
3. **Stability.** Rust has no stable ABI. A dynamically-loaded plugin would need
   recompilation against every core release — and Go's plugin story is no better, so this
   constraint is not language-specific.

## Decision
Plugins are **separate processes** speaking **JSON-RPC 2.0 over stdio**, in the spirit of
LSP and MCP. Each plugin ships a **manifest** declaring its name, version, the protocol
version it implements, the extension points it provides, and its executable.

The protocol is **explicitly versioned**. Core and plugin negotiate on handshake; an
incompatible version is refused with a clear message rather than failing at the first
mismatched message.

**Built-in providers implement the same interface as external plugins.** They are compiled
in for distribution convenience, but they do not get a privileged path. If the interface
cannot express a built-in provider, the interface is wrong — this is what keeps the
extension point honest rather than theoretical.

## Alternatives considered
- **Dynamic libraries.** Fastest, and defeated by all three constraints at once.
- **WASM plugins.** Sandboxed, portable, language-flexible and genuinely attractive. Rejected
  for MVP: the host interface for subprocess spawning and filesystem access is awkward, and
  the ecosystem is still moving. Worth revisiting — it addresses
  [ADR-0037](0037-sandboxed-command-execution.md) elegantly.
- **HTTP services.** Works, and burdens every plugin with a server, a port and a lifecycle.
- **Embedded scripting** (Lua, Rhai). No separate process, and ties plugins to one language
  and to in-process trust.

## Consequences
- Per-call overhead is higher than in-process. Irrelevant here: calls are coarse-grained and
  the work behind them takes seconds.
- Process lifecycle management — spawn, health, timeout, crash recovery, zombie cleanup — is
  core work, on three platforms.
- The protocol is a **public contract**. Breaking it breaks third-party plugins, so it needs
  a compatibility policy before the first external plugin exists, not after.
- Plugins are arbitrary executables the user has chosen to run. Corifeu should be explicit
  that installing a plugin is equivalent to running its author's code, and not pretend
  otherwise.
- An example plugin in a separate repository is a roadmap deliverable. It is the only honest
  proof that "zero changes to this repository" is true.
