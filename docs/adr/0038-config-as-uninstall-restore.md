# 0038 — The generated config file is the restore point

- **Status:** Proposed
- **Date:** 2026-10-11

## Context
`uninstall` must leave the repository as it was. The obvious mechanism is backup files, and
backups in a git repository are redundant with git, clutter the tree, and go stale.

There is a second, less obvious need. A user who uninstalls and later reconsiders should be
able to return to **exactly** their previous setup, including all the wizard answers they
no longer remember giving.

## Decision
When a user installs **without** supplying a configuration file, Corifeu **generates**
`.corifeu/config.yaml` from their wizard answers — and `uninstall` **does not delete it**.

Reinstalling with that file reproduces the previous setup exactly:

```sh
corifeu uninstall                      # removes managed blocks, keeps config.yaml
corifeu install --config .corifeu/config.yaml
```

This gives one mechanism three properties:

| Property | How |
|---|---|
| Restore | The config file is the complete record of every choice |
| Reviewable | It is YAML in git, not an opaque backup |
| Portable | The same file reproduces the setup on another machine or for a teammate |

**No backup files are created.** The user's own content in foreign files is protected by
markers ([ADR-0030](0030-marker-based-file-merging.md)) rather than by copies, and by git,
which already does this job properly.

`uninstall` reports precisely what it removed and what it kept, and explains how to restore.
A user who genuinely wants everything gone runs `corifeu uninstall --purge`.

## Alternatives considered
- **Timestamped backups of every modified file.** Clutter, staleness, and duplication of git.
- **Delete the config on uninstall.** Loses the restore point for no benefit; an uninstalled
  product leaving one readable YAML file behind is not a burden.
- **Store the restore point in `.corifeu/run/`.** That directory is git-ignored and subject
  to retention; a restore point must survive both.

## Consequences
- The generated config must be **complete** — every answer, including defaulted ones,
  written explicitly. A config relying on implicit defaults would restore differently after
  a version upgrade changed a default.
- It must also be **readable**: generated with comments explaining each section, since it
  doubles as documentation of what the user chose.
- Config schema version is recorded in the file, so a restore across versions can migrate
  rather than misinterpret.
- Partial installation failure must still write what was decided, or a failed install leaves
  no record of the answers already given.
