# 0034 — The existing personal setup is not preserved

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The prototype is in daily use by its author, who is also the project owner and will be
Corifeu's first user. A migration path from `.orchestra/` to `.corifeu/` could be built.

It would be built for exactly one installation. All work is committed and pushed; there is
nothing local to recover.

## Decision
**No migration path.** `.orchestra/` is removed once Corifeu can run one cycle end to end.
The owner reconfigures through the new installer, as the first real test of it.

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
- `.orchestra/` stays on `main` until Corifeu replaces it functionally, so the working setup
  is not removed before its replacement exists. The [diagnosis](../diagnosis.md) records this
  ordering.
- First-release compatibility promises must be explicit, because before that point there are
  none.
