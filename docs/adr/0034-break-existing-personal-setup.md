# 0034 — The existing personal setup is not preserved

- **Status:** Accepted
- **Date:** 2026-10-11 (amended 2026-10-12)

## Context
The prototype is in daily use by its author, who is also the project owner and will be
Corifeu's first user. A migration path from `.orchestra/` to `.corifeu/` could be built.

It would be built for exactly one installation. All work is committed and pushed; there is
nothing local to recover.

## Decision
**No migration path.** The owner reconfigures through the new installer, as the first real
test of it.

> **Amended 2026-10-12.** This ADR originally kept `.orchestra/` on `main` *until* Corifeu
> could run one cycle, so a working setup was not removed before its replacement existed.
> The legacy tree was removed earlier than that, for a reason the original reasoning missed:
> **it was never installed into this repository.** It is a template that gets copied into
> *other* repositories, so deleting it here uninstalls nothing. The guard was protecting
> against a harm that could not occur.
>
> What it cost instead was clarity. A repository whose root held `install.sh`,
> `CLAUDE.orchestra.md`, `settings.orchestra.json` and `.orchestra/` read as two products at
> once, and every reader had to work out which one they were looking at.
>
> Everything remains in git history at `4235adb`. Recover a file with
> `git show 4235adb:.orchestra/scripts/code.py`, or the whole tree with
> `git restore --source 4235adb -- .orchestra`.

Breaking changes are acceptable throughout MVP development. Compatibility guarantees begin
at the first tagged release, not before.

## Alternatives considered
- **An `orchestra` → `corifeu` migration command.** Engineering for a population of one, and
  it would encode the prototype's assumptions — fixed `src/`, shell config — into the new
  product's import path, which is precisely the inheritance to avoid.
- **Run both side by side during transition.** Two enforcement mechanisms disagreeing about
  who may write which files. Confusing and genuinely unsafe.
- **Keep `.orchestra/` working until 1.0.** Maintenance of code slated for deletion.

## Consequences
- The owner loses a working setup during the transition. Acceptable, and informative: the
  friction of reconfiguring is exactly the friction every future user will meet.
- `.orchestra/` was removed on 2026-10-12, before Corifeu replaced it functionally. Nothing
  broke, because the tree in this repository was a template rather than an installation.
  Copies already installed in other repositories are untouched and keep working.
- The [diagnosis](../diagnosis.md) remains the record of what the prototype was and why each
  part was kept, rewritten or discarded. It is history now, and deliberately preserved: the
  ADRs cite it as the evidence for decisions, and deleting it would leave the reasoning
  unsupported.
- First-release compatibility promises must be explicit, because before that point there are
  none.
