# Security Policy

## Reporting a vulnerability

**Do not open a public issue.**

Report privately through
[GitHub Security Advisories](https://github.com/baphomello/corifeu/security/advisories/new),
or by email to **baphomello@gmail.com**.

Please include what you can: affected version, platform, reproduction steps, and what an
attacker gains. A proof of concept helps.

This is a single-maintainer project. Expect an acknowledgement within **7 days** and an
assessment within **30 days**. If you do not hear back, assume the message was missed and
send it again.

You will be credited in the advisory unless you prefer otherwise.

## Supported versions

Corifeu has not had its first release. Once 0.1.0 ships, the latest release is supported.
Pre-release code carries no security guarantees.

## What Corifeu does, by design

Understanding these makes it clearer what counts as a vulnerability.

**It runs AI models that execute code.** Agents write files and run commands in your
repository. That is the product. Defences are layered and documented:
capability restriction ([ADR-0036](docs/adr/0036-runtime-enforced-tool-restriction.md)),
file ownership ([ADR-0035](docs/adr/0035-file-ownership-by-glob-classes.md)), and sandboxed
execution ([ADR-0037](docs/adr/0037-sandboxed-command-execution.md)).

**It invokes third-party CLIs** as subprocesses. Their vulnerabilities are theirs; report
those upstream. How Corifeu invokes them is ours.

**Plugins are arbitrary executables** that you chose to install and reference in your
configuration. Corifeu runs them. There is no registry, no curation and no sandbox around
them ([ADR-0032](docs/adr/0032-out-of-process-plugins.md)). Installing a plugin is running
its author's code.

**Prompt injection is a known, demonstrated risk**, not a theoretical one. Repository
content reaches models, and content you did not write — a vendored dependency, a pull
request — can carry instructions aimed at them. We have measured a case where a planted
directive was obeyed
([ADR-0045](docs/adr/0045-prepared-workspace-for-context-isolation.md)). The mitigation is
defence in depth: an injected instruction cannot exceed the capability the role was granted.
**Reports showing a bypass of that capability boundary are exactly what we want.**

## In scope

- Escaping the capability boundary: an agent doing something its role was not granted.
- Escaping the sandbox or the file-ownership rules.
- Prompt injection that **escalates privilege**, rather than merely influencing output.
- Credential or token leakage, including into logs, the blackboard or configuration files.
- Installer flaws: checksum or signature bypass, destructive behaviour on foreign files.
- Plugin protocol flaws allowing a plugin to exceed its declared capabilities.

## Out of scope

- Vulnerabilities in the provider CLIs or model runtimes themselves — report upstream.
- A model producing wrong, insecure or offensive output without crossing a capability
  boundary. That is a quality problem, and an issue is the right place for it.
- Anything requiring `--dangerously-skip-permissions`, `execution.sandbox: none`, or an
  equivalent documented opt-out. These exist to be chosen deliberately.
- A plugin you installed behaving maliciously, absent a protocol flaw.

## Release integrity

Release artefacts are published with SHA-256 checksums and signatures, and the install
scripts verify them before placing a binary. If verification fails, **that is a report worth
sending immediately**.
