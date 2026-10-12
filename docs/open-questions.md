# Open questions

All foundation questions were decided in review on 2026-10-11, and the Gemini verification
that was pending resolved on 2026-10-12 — by discovering the product no longer exists.
Nothing below blocks phase 1.

What remains is one unverified capability, and a list of decisions taken on reasoning rather
than evidence that should be re-examined once the product is actually used.

---

## Pending action

### Antigravity CLI context isolation — the one unverified capability

Antigravity CLI was verified by execution on 2026-10-12 and passes every conformance check
exercised, **except context isolation**. It exposes no `--bare` or `--ignore-user-config`
equivalent.

What is known: configuration lives under `~/.gemini/`, and a trust model
(`trustedFolders.json`, `trustedWorkspaces`) governs whether project agents, hooks, MCP
servers and skills load — an untrusted folder skips them, which the CLI states on startup.
Whether that can be driven deliberately by the adapter, or a config home redirected the way
`CODEX_HOME` allows for Codex, is **UNVERIFIED**.

This matters because context contracts ([ADR-0014](adr/0014-context-contracts-per-role.md))
depend on suppressing ambient context. A provider that cannot guarantee a clean context
breaches its contract invisibly.

**Resolve in phase 2.** If no mechanism exists, the honest outcome is to document
Antigravity as unable to guarantee a clean contract, and let the capability model decide
which roles it may hold ([ADR-0036](adr/0036-runtime-enforced-tool-restriction.md)).

---

## Decided in review

| Question | Resolution |
|---|---|
| Repository name | `baphomello/corifeu`; local remote updated |
| Rust or Go | **Rust**, buy the adapter-churn risk — [ADR-0004](adr/0004-rust-single-binary.md) |
| Who writes the tests | **Any role the user assigns.** The `lead` role never writes; its model may, as a separate role. One role owning `source` and `test` is permitted and warned — [ADR-0042](adr/0042-test-authorship.md) |
| Schema hosting | **GitHub Pages**, `baphomello.github.io/corifeu/schema/v1/config.json`. No domain until the product proves itself — [ADR-0028](adr/0028-yaml-config-with-json-schema.md) |
| Is `corifeu` available | **Yes** — free on crates.io and npm, checked 2026-10-11. Reserved in phase 1 |
| Checkers required? | **Optional in the core, required in the `many-roles` preset** — [ADR-0012](adr/0012-required-and-optional-roles.md) |
| Local model as lead | **Allowed, warned, never the default** — [ADR-0043](adr/0043-any-provider-may-lead.md) |
| Retention defaults | **10 full runs / 500 MB / 3 always kept** — [ADR-0022](adr/0022-run-retention-policy.md) |
| Sandboxing | Accepted **with staging**: allowlist and argv-only in phase 4, platform sandboxes in phase 7 — [ADR-0037](adr/0037-sandboxed-command-execution.md) |
| YAML parser | **Process accepted, crate pending.** `serde_yml` excluded permanently; parser chosen by a phase 1 spike — [ADR-0040](adr/0040-yaml-parser-selection.md) |
| File ownership | Ordered glob classes, first match wins — [ADR-0035](adr/0035-file-ownership-by-glob-classes.md) |
| `private` mode | Dropped; expressible as `privacy.allow_remote_providers` — [ADR-0041](adr/0041-drop-the-mode-concept.md) |
| Install backups | None. The generated config is the restore point — [ADR-0038](adr/0038-config-as-uninstall-restore.md) |
| Metering for subscriptions | Quota percentage, never fabricated currency — [ADR-0031](adr/0031-token-savings-metering.md) |
| WSL-specific configuration | Removed from defaults, documented in [troubleshooting.md](troubleshooting.md) |
| README | Marked as under rebuild; full rewrite stays in phase 8 |
| Glossary | Written in phase 0 — [glossary.md](glossary.md) |
| Does `review.sh` discard its diff? | No. `codex exec` appends piped stdin as a `<stdin>` block |
| Third agent CLI | **Antigravity CLI (`agy`)**, not Gemini CLI — retired for individual users 2026-06-18. Verified by execution 2026-10-12 — [ADR-0044](adr/0044-antigravity-replaces-gemini-cli.md) |
| Does Antigravity enforce tool restriction? | **Yes, adversarially verified.** Headless default-deny plus `permissions.allow`; fails closed, which suits Corifeu better than a deny-list |

---

## Decided on reasoning, to be re-examined with evidence

These were settled so work can proceed, not because they are proven. Each is a judgement
that real use can overturn cheaply — and the project owner has said explicitly that he will
test them as the first end user and ask for adjustments. **Changing any of these later is
expected, not a failure.**

| Decision | Chosen because | What would overturn it |
|---|---|---|
| **3 rounds per pair** | The prototype's practical experience | Runs that routinely escalate at round 3 with work still converging |
| **Retention 10 / 500 MB** | Estimated run sizes, not measured ones | Phase 6 measurements. [ADR-0022](adr/0022-run-retention-policy.md) must then be superseded with real numbers |
| **Checkers optional** | A bet that oracles outweigh agent review | Dogfooding showing the reviewer catches what tests systematically miss |
| **Sequential execution** | Determinism beats speed while the design is unsettled | Runs slow enough that the workflow becomes impractical |
| **Context contract defaults** | Reasoned from the prototype's shapes | Doers producing locally correct, globally wrong work — the signal that a view is too narrow |
| **`serde-saphyr`** | Avoids the `unsafe-libyaml` lineage | The phase 1 spike, on error-message quality |
| **Warning, not error, for self-testing** | Refusing would be paternalistic | Users routinely shipping compromised oracles without noticing the warning |
| **Five role categories** | They cover the observed cases | A real role that fits none of them |

The honest framing: a foundation document set written before any code exists is a set of
hypotheses with citations. The ADR format exists so that overturning one is a recorded
decision with reasoning, rather than a silent drift.
