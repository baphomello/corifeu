# 0030 — Foreign files edited only inside markers

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Corifeu must modify files it does not own: `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`,
`.claude/settings.json`, `.gitignore`. Those files contain the user's own content, which
must survive installation, update and removal untouched.

The prototype gets this wrong in two ways, both instructive. It appends
`CLAUDE.orchestra.md` to `CLAUDE.md` guarded by a `grep` for one sentence — so there is **no
delimiter**, and uninstalling cannot know where the inserted block ends. And it rewrites
`.claude/settings.json` through `json.dump(indent=2)`, silently reformatting the whole file.

## Decision
Every Corifeu-managed region in a foreign file is **delimited by markers**:

```
<!-- BEGIN CORIFEU (managed block — do not edit) -->
...generated content...
<!-- END CORIFEU -->
```

with the comment syntax appropriate to the file type. Rules:

1. Corifeu reads, replaces **only** the marked region, and writes back. Content outside the
   markers is byte-preserved.
2. If markers are absent, the block is appended with markers.
3. If markers are present but the content inside was hand-edited, Corifeu **stops and asks**
   rather than overwriting.
4. `uninstall` removes the marked region and nothing else, leaving the file otherwise
   identical.
5. For structured files such as JSON, markers are impossible. There, Corifeu merges keys
   additively, records in `.corifeu/` exactly which keys it added, and removes only those on
   uninstall.

**No backup files are created.** The restore story is
[ADR-0038](0038-config-as-uninstall-restore.md) — the generated config file is the restore
point, and it survives uninstall — plus git, which already holds the user's content.

## Alternatives considered
- **Timestamped backups** (`CLAUDE.md.bak.20261011`). Clutters repositories, and in a git
  repository duplicates what git already does better.
- **Separate files only, never touch the user's.** Cleanest, and impossible: the provider
  CLIs read specific filenames.
- **Full-file ownership.** Would destroy user content outright.

## Consequences
- Marker parsing must handle every managed file type, and a file whose markers are corrupted
  needs a clear diagnostic rather than silent damage.
- Rule 3 means an update can halt and require user action, which is correct: silently
  discarding a user's edit is worse than stopping.
- Rule 5's record of added keys is state that `uninstall` depends on, so it lives in
  `.corifeu/` and must itself be committed or regenerable.
- Because content outside markers is byte-preserved, **Corifeu must not reformat JSON** it
  did not write. The prototype's `json.dump` reformat is exactly the failure to avoid.
