# Open questions

Everything needing a decision, with a recommendation for each. Ordered by how much later
work depends on the answer.

**Q1 and Q2 are blocking** — phase 4 and phase 1 respectively cannot be specified without
them. The rest can be decided as their phase approaches.

---

## Q1 — Who writes the tests? **(blocking, phase 4)**

**The question.** [ADR-0010](adr/0010-role-categories.md) states the lead writes no
artefacts. The prototype has the orchestrator own `tests/`, because it authors the
acceptance oracle it will judge by. [config.md](config.md) currently grants `lead` the
`test` class, which **contradicts ADR-0010**. The contradiction is flagged in place.

This matters more than it looks. If the doer writes both the implementation and the tests
that judge it, the doer is marking its own homework, and the oracle stops being independent
— which weakens [ADR-0019](adr/0019-tie-breaking-and-oracle-precedence.md), the rule the
whole quality thesis rests on.

**Options.**

| | Approach | Cost |
|---|---|---|
| **A** | Lead writes tests. The prototype's model. | Lead gains write capability, weakening [ADR-0036](adr/0036-runtime-enforced-tool-restriction.md)'s clean rule |
| **B** | A dedicated `qa` role writes tests; lead stays write-free | Forces a second required role; minimum config grows |
| **C** | The `coder` writes both | Simplest; the doer marks its own homework |
| **D** | A planner writes executable acceptance criteria; a doer implements | Cleanest separation, most machinery |

**Recommendation: B**, with a nuance that makes it cheap. Introduce a role *capability*
`writes: [test]` that a **checker** may hold — so checkers are not strictly read-only, and
a `qa` checker authors the oracle while the lead stays write-free. The minimum required set
is unchanged, because `qa` is only required in presets that want independent oracles; the
`few-roles` preset falls back to **C** and says so plainly.

This keeps ADR-0010 intact, keeps oracle independence where the user wants it, and makes the
trade-off visible rather than hidden in a default.

**If you prefer A**, ADR-0010 must be amended to give the lead a narrow write capability
scoped to one file class, and ADR-0036 must record that the lead is a partial exception.
That is defensible — it is what the prototype does and it works — but it should be written
down, not left implicit.

---

## Q2 — Where is the JSON Schema published? **(blocking, phase 1)**

**The question.** [config.md](config.md) uses `https://corifeu.dev/schema/v1/config.json` as
a placeholder. The brief says everything is served from GitHub Releases for now, with a
custom domain only if the project succeeds. A schema URL is effectively permanent — editors
and committed config files will reference it for years.

**Recommendation: GitHub Pages**, at
`https://baphomello.github.io/corifeu/schema/v1/config.json`.

It is stable, free, needs no domain, and survives a later move behind a redirect. Committing
to `corifeu.dev` before the domain exists risks either paying for it indefinitely or
breaking every config file that references it.

Decide before phase 1 writes the first schema reference.

---

## Q3 — Is `corifeu` available as a package name?

**The question.** Not checked. The binary name, the crates.io name and any future Homebrew
formula could already be taken.

**Recommendation.** Check crates.io, Homebrew and npm now, and **reserve the crates.io name
in phase 1** even though the crate will be empty for months. A rename after release is far
more expensive than an early placeholder. If `corifeu` is taken on crates.io, publish as
`corifeu-cli` while keeping the binary named `corifeu`.

---

## Q4 — Are checkers optional in MVP 1?

**The question.** [ADR-0012](adr/0012-required-and-optional-roles.md) sets the minimum
checker count to zero, betting that objective oracles carry enough weight alone. The
prototype always has a reviewer, and cross-family review is one of its best properties.

**Recommendation: keep checkers optional in the core, required in the `many-roles` preset.**

The reasoning: oracles are genuinely stronger than agent review for anything they cover, and
a user with good tests gets most of the value without paying for a reviewer. But the
prototype's experience suggests the reviewer catches what tests do not. Making it
preset-level rather than core-level lets the default be generous without making the minimum
configuration heavier.

Revisit with dogfooding data in phase 6.

---

## Q5 — Can a local model be the lead?

**The question.** The brief says the user chooses any provider as lead, including a local
model. The lead is the only agent that converses with the user, summarises progress and
decides escalations. That is a demanding role, and a 4B model doing it badly would be the
user's entire impression of the product.

**Recommendation: allow it, warn clearly, do not default to it.**

The wizard should state that the lead is the role most sensitive to model quality, and
`doctor` should flag a lead whose model failed any conformance test. Prohibiting it would
contradict the brief and block the all-local use case, which is legitimate. But the product
should not let someone configure a poor experience silently.

---

## Q6 — What happens when Gemini CLI cannot enforce restriction?

**The question.** Gemini CLI's tool-restriction and context-isolation capabilities are
**UNVERIFIED** — it is not installed on the analysis machine, and its documented
`--output-format` has a reported defect. If it cannot be restricted, it cannot safely hold a
lead or a read-only checker role.

**Recommendation: verify in phase 2, then let the capability model decide.**

[ADR-0036](adr/0036-runtime-enforced-tool-restriction.md) already specifies the behaviour: a
provider that cannot prove restriction is refused for restricted roles, with a clear
message. No special case is needed — this is the capability negotiation working. What is
needed is a machine with Gemini CLI installed and authenticated to run the conformance suite
against.

**Who runs that verification, and on what machine?**

---

## Q7 — Confirm the retention defaults

**The question.** [ADR-0022](adr/0022-run-retention-policy.md) recommends keeping the
proposed **10 full runs** and adding a **500 MB** ceiling plus **3 always-kept** runs,
because run size variance is large: a typical run is estimated at ~1.5 MB and a pathological
one at ~30 MB.

**Recommendation: accept 10 / 500 MB / 3.** Ten is defensible for typical runs (~15 MB), and
the byte ceiling is what protects against the tail that a count alone ignores.

These are **estimates, not measurements** — Corifeu does not exist to measure. Phase 6 must
replace them with real numbers and supersede the ADR.

---

## Q8 — Approve the seven proposed ADRs?

[ADR-0035](adr/0035-file-ownership-by-glob-classes.md) through
[ADR-0041](adr/0041-drop-the-mode-concept.md) are status **Proposed**. They are not merely
suggestions — 0035, 0036 and 0037 are load-bearing for phase 4 and the architecture document
already assumes them.

**Recommendation: accept 0035, 0036, 0038, 0039, 0041 now.** They follow from the analysis
and nothing downstream works without them.

Two deserve a separate look:

- **0037 (sandboxing)** — approve the *staging* as much as the decision: allowlist plus
  argv-only in phase 4, platform sandboxes in phase 7. Full sandboxing before any public
  release, partial before any doer runs a command.
- **0040 (YAML parser)** — genuinely undecided, resolved by a phase 1 spike. Approve the
  spike and the exclusion of `serde_yml`, not the parser.

---

## Q9 — Does the README change now?

**The question.** `README.md` still describes "orchestra" and instructs `./install.sh
/path/to/your-repo`. The repository is now `baphomello/corifeu` and public. The brief says
not to touch the README beyond what is strictly necessary, with a full rewrite in a later
phase.

**Recommendation: add three lines, no more.** A short note that the project is being rebuilt
as Corifeu, a pointer to `docs/`, and a statement that the current `.orchestra/` contents are
the prototype. A public repository whose README describes a different product is a small
cost, but it is paid by every visitor until phase 8.

---

## Q10 — Should Corifeu's own repository adopt Corifeu's conventions now?

**The question.** The project has no `GLOSSARY.md`, and terms used throughout these
documents — lead, doer, checker, board, contract, artefact, oracle, class — are load-bearing
and not all obvious.

**Recommendation: add a glossary in phase 1**, as `docs/glossary.md`. It costs very little
and it is the document contributors will need first. It also exercises
[ADR-0023](adr/0023-project-memory-in-versioned-files.md)'s own idea on the project itself.

---

## Resolved during this session

Recorded so they are not reopened by accident.

| Question | Resolution |
|---|---|
| Repository name | `baphomello/corifeu`; local remote updated |
| Rust or Go | **Rust**, buy the adapter-churn risk. Reasoning in [ADR-0004](adr/0004-rust-single-binary.md) |
| Fixed `src/` and `tests/` paths | Replaced by ordered glob classes, [ADR-0035](adr/0035-file-ownership-by-glob-classes.md) |
| Backups on install | None. The generated config is the restore point, [ADR-0038](adr/0038-config-as-uninstall-restore.md) |
| Metering for subscriptions | Quota percentage, never fabricated currency, [ADR-0031](adr/0031-token-savings-metering.md) |
| WSL-specific configuration | Removed from defaults, documented in [troubleshooting.md](troubleshooting.md) |
| `private` mode | Dropped; expressible as `privacy.allow_remote_providers`, [ADR-0041](adr/0041-drop-the-mode-concept.md) |
| Does `review.sh` discard its diff? | No. `codex exec` appends piped stdin as a `<stdin>` block |
