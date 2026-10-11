# Diagnosis of the existing repository

State analysed: `main` at `93fc905`, plus the cleanup commit that opens this branch.
Everything below was read in full; nothing is inferred from file names.

## Verdict in one paragraph

What exists today is **a convention, not a program**. There is no executable called
`orchestra`. The "orchestrator" is cloud Claude Code, instructed by prose in two Markdown
files, triggered by one slash command, and constrained by a single `PreToolUse` hook. The
entire mechanism is roughly 200 lines of Python and 60 lines of shell. This is not a
criticism — it is why it works. A convention that fits in one screen gets used. But a
convention cannot be installed into a stranger's repository, cannot hold state between
runs, cannot measure itself, and cannot survive its host tool changing. Corifeu has to
become the program that this convention has been pretending to be.

The thesis underneath is sound and survives intact: one human-facing agent, minimal
curated context per delegate, objective oracles outranking opinions, bounded correction
loops, and enforcement in the tooling rather than in the prompt. Those five ideas are the
inheritance. Almost none of the code is.

## Inventory

| Path | What it does | Fate |
|---|---|---|
| [install.sh](../install.sh) | Copies `.orchestra/` into a target repo, appends to `CLAUDE.md`, merges JSON into `.claude/settings.json` | **Rewrite** |
| [.orchestra/scripts/code.py](../.orchestra/scripts/code.py) | Self-contained agent loop over Ollama `/api/chat`: 4 tools, native + text-mode tool-call parsing, loop detection, step cap | **Rewrite**, harvest the behaviours |
| [.orchestra/scripts/review.sh](../.orchestra/scripts/review.sh) | Pipes `git diff` to `codex exec` under a scrubbed `CODEX_HOME` | **Rewrite**, keep the isolation idea |
| [.orchestra/hooks/guard-src.py](../.orchestra/hooks/guard-src.py) | `PreToolUse` hook denying writes to `src/` via Write/Edit and 5 Bash regexes | **Delete**, replace the concept |
| [.orchestra/setup-codex-home.sh](../.orchestra/setup-codex-home.sh) | Builds a minimal `CODEX_HOME`: symlinked auth, tiny config, empty `AGENTS.md` | **Delete**, absorb into the provider adapter |
| [.orchestra/orchestra.config.sh](../.orchestra/orchestra.config.sh) | Eight `export` statements sourced before the harness | **Delete**, replaced by `.corifeu/config.yaml` |
| [.orchestra/ORCHESTRA.md](../.orchestra/ORCHESTRA.md) | The workflow specification: roles, flow, rules, context contracts, modes, enforcement | **Rewrite** as real docs |
| [CLAUDE.orchestra.md](../CLAUDE.orchestra.md) | The block appended to a target repo's `CLAUDE.md` telling Claude it is the orchestrator | **Rewrite**, one per lead provider |
| [settings.orchestra.json](../settings.orchestra.json) | Hook registration + permission allowlist, merged into `.claude/settings.json` | **Delete**, Claude-Code-specific |
| [.claude/commands/cycle.md](../.claude/commands/cycle.md) | Slash command restating the flow for the human to trigger | **Delete**, becomes `corifeu run` |
| [.orchestra/templates/SPEC.md](../.orchestra/templates/SPEC.md) | Template: goal + per-task target/change/acceptance | **Keep the shape** as an artifact schema |
| [.orchestra/templates/PLAN.md](../.orchestra/templates/PLAN.md) | Empty-section template: approach, breakdown, open questions | **Delete**, never referenced |
| [.orchestra/templates/REVIEW.md](../.orchestra/templates/REVIEW.md) | Empty-section template: findings, resolution | **Delete**, never referenced |
| [README.md](../README.md) | Six lines describing orchestra | **Rewrite** in a later phase |
| [LICENSE](../LICENSE) | AGPL-3.0 verbatim | **Keep** |
| [.gitignore](../.gitignore) | Four entries | **Keep**, extend |
| [docs/troubleshooting.md](troubleshooting.md) | Created by this branch's cleanup commit | **Keep** |

`PLAN.md` and `REVIEW.md` are dead: no script, no document and no command references
either. They were written in the first commit and never wired up.

## What is personal, hardcoded or undefined

These are the reasons the setup cannot be handed to anyone else as-is. The cleanup commit
on this branch fixed the ones that were cheap to fix; the rest are structural and are
resolved by the architecture, not by an edit.

| # | Finding | Status |
|---|---|---|
| 1 | Ollama URL computed from the WSL default route (`ip route show default`). Meaningless on macOS, native Windows and Linux. | Fixed — plain default, WSL recipe moved to [troubleshooting](troubleshooting.md) |
| 2 | `Bash(cargo:*)` in the permission allowlist assumed a Rust target project | Fixed — removed |
| 3 | `ORCHESTRA_CODEX_MODEL` was read by nothing; the real model was pinned separately in `setup-codex-home.sh` | Fixed — one source of truth |
| 4 | One Portuguese comment, against the English-only rule | Fixed |
| 5 | `ORCHESTRA.md` enforcement paragraph corrupted by `ad78205`: a sentence inserted mid-parenthetical left a dangling fragment | Fixed |
| 6 | `CLAUDE.orchestra.md` claimed the hook blocks `src/` **and** `tests/`; it never blocked `tests/` | Fixed |
| 7 | `.orchestra/SPEC.md` — per-run state committed to git, byte-identical to its own template | Fixed — untracked and ignored |
| 8 | **`src/` and `tests/` as fixed paths** | **Structural** — see below |
| 9 | **Claude Code is the only possible lead** | **Structural** |
| 10 | **Shell `export` as the configuration mechanism** | **Structural** |
| 11 | **`private` mode declared but never designed** | **Structural** |
| 12 | `setup-codex-home.sh` symlinks `$HOME/.codex/auth.json` — POSIX-only path, POSIX-only symlink | **Structural** |

### 8 — Fixed paths are the deepest flaw

`guard-src.py` matches the literal prefix `src/`. Two failure modes, both silent:

- A project whose production code lives in `lib/`, `app/`, `internal/`, `pkg/` or `cmd/`
  is **not protected at all**. The hook fires, matches nothing, and the lead writes
  feature code freely while believing it cannot.
- A project whose unit tests sit beside their sources — `src/bank/deposit.ts` next to
  `src/bank/deposit_test.ts` — has its **tests blocked along with its sources**, which
  contradicts the rule that the lead owns the acceptance oracle.

The second case is not exotic; it is the default in Go, and common in Rust, Jest and
Vitest projects. A directory prefix cannot express "implementation versus oracle" because
that distinction is not a directory distinction. Corifeu replaces prefixes with
**glob-based file classes** — see [ADR-0035](adr/0035-file-ownership-by-glob-classes.md).

### 9 — Enforcement is borrowed, not owned

The rule "the lead never writes feature code" is enforced by *Claude Code's* hook system.
Nothing in this repository enforces anything. Point the setup at Codex or Gemini as lead
and the guarantee evaporates with no error and no warning.

This also inverts under the new design. Today Claude Code is the runtime and Corifeu is a
guest inside it. Tomorrow Corifeu is the runtime and the CLI agents are its subprocesses —
so enforcement stops being a hook that says *no* after the fact and becomes a matter of
never handing the tool over. See [ADR-0036](adr/0036-runtime-enforced-tool-restriction.md).

### 10 — Shell exports cannot carry the configuration this product needs

`orchestra.config.sh` holds eight scalars. The target product needs nested structures:
roles grouped into categories, each mapped to a model, each with a context contract and a
budget; allowed channel pairs; a git flow with a decision per step. Shell variables cannot
express nesting, cannot be validated, cannot be schema-completed in an editor, and cannot
be diffed meaningfully. They also cannot exist on native Windows without a POSIX shell.

### 11 — `private` mode is a label with nothing behind it

`ORCHESTRA_MODE` is exported, documented as `open` versus `private`, and **read by no
code**. `private` is described as "all-local, no cloud, to be designed". It is an intention,
not a feature. Corifeu should not inherit the label: "all agents assigned to local models"
is already expressible as an ordinary configuration, and needs no mode flag. Recorded as
an open question.

## Defects found while reading

Beyond the inconsistencies above, two genuine bugs survive in `code.py`. Both are
instructive, because both are design lessons rather than typos.

**Truncated reads are invisible to the model.** `read_file` returns `f.read()[:8000]` with
no marker. A model that reads a 12 KB file, receives 8 KB believing it complete, then calls
`write_file` on it **silently destroys the remaining third**. The lesson is in
[ADR-0017](adr/0017-artifact-references-not-summaries.md): agents must exchange references
to artifacts, and any truncation must be explicit in the payload.

**`run` bypasses the path guard entirely.** `_safe()` confines `write_file` and `read_file`
to the repository, and then `run` executes arbitrary strings through `shell=True` with no
allowlist and no sandbox. `run("rm -rf ~")` is reachable by a local model's hallucination.
This is acceptable on a personal machine and unacceptable in something distributed by
`curl | sh`. See [ADR-0037](adr/0037-sandboxed-command-execution.md).

A third, smaller: text-mode tool calls are synthesised without the `id` field that Ollama
supplies on native calls, so conversation history is shaped differently depending on which
path produced it.

## What was verified, not assumed

| Claim | Method | Result |
|---|---|---|
| `review.sh` discards the piped diff | `codex exec --help` | **False.** "If stdin is piped and a prompt is also provided, stdin is appended as a `<stdin>` block." The script is correct. |
| Ollama is reachable and the configured model exists | `GET /api/tags` | True — `ornith1.5:9b`, `qwen3.5:4b`, `qwen2.5-coder:7b` |
| Local models emit native tool calls | Live `POST /api/chat` with a `write_file` schema | True — `qwen3.5:4b` returned a well-formed native `tool_calls` entry with an `id`, in 9.8 s |
| Ollama advertises tool capability per model | `GET /api/tags` | True — each model carries a `capabilities` array containing `tools` |
| Orchestra is installed into its own repository | Filesystem | **False.** No `CLAUDE.md`, no `.claude/settings.json`. It has never dogfooded itself. |

The first row is worth dwelling on. `review.sh` *looks* wrong — a prompt passed as an
argument while a diff goes to stdin is the classic shape of a silently discarded input. It
is correct, and only the documentation proves it. Nothing else in this diagnosis was
accepted on appearance either.

## Missing entirely

No tests. No CI. No `CONTRIBUTING.md`, `NOTICE`, `CODE_OF_CONDUCT.md`, issue or PR
templates. No versioning or release process. No way to uninstall.

The absence of tests deserves a line of its own: a product whose central claim is that
**objective verification beats agent opinion** currently verifies itself not at all.
Fixing that is phase 1 work, not a later tidy-up.

## The five ideas worth carrying forward

1. **Context contracts.** Each delegate sees a deliberately assembled view — the coder gets
   `{task + SPEC excerpt + target files + neighbour signatures}`, the reviewer gets
   `{diff + acceptance criteria}` — and nothing else. This is the single best idea here and
   becomes [ADR-0014](adr/0014-context-contracts-per-role.md).
2. **Context isolation for delegates.** `setup-codex-home.sh` exists so the user's global
   `AGENTS.md` cannot leak into the reviewer. The hack is provider-specific; the principle
   is not. Both `claude --bare` and `codex --ignore-user-config` expose it as a flag.
3. **Enforcement over instruction.** The project already learned that asking an agent
   nicely does not hold, and reached for a hook. Right instinct, wrong layer.
4. **Objective oracles and checkpoints.** The acceptance criterion is a command that passes
   or fails, and the human approves at two defined points. Both survive unchanged.
5. **Cross-model review.** The reviewer is deliberately a different model family from the
   author, to avoid an architect reviewing their own reasoning. Becomes a recommendation in
   [ADR-0019](adr/0019-tie-breaking-and-oracle-precedence.md).

## Recommended disposition

- **Delete** `.orchestra/` wholesale once Corifeu can run one cycle end to end. Not before.
- **Keep** the repository, its history and its AGPL licence. This is a rewrite of contents,
  not a new project.
- **Keep** `README.md` until the first release, then rewrite it once, properly.
- **Dogfood from phase 1.** The quickest way to discover that the architecture is wrong is
  to make Corifeu build Corifeu.
