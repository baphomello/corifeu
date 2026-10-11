# 0043 — Any provider may lead, with a quality warning

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The lead is the only agent that converses with the user, summarises progress, and decides
what to escalate ([ADR-0010](0010-role-categories.md)). It is therefore the role most
sensitive to model quality: a weak lead produces a poor experience even when every delegate
behind it performs well, because the lead *is* the product from the user's seat.

The foundation brief states the user chooses which model leads, including a local one. That
enables a genuinely valuable case — an all-local configuration where no code leaves the
machine ([ADR-0041](0041-drop-the-mode-concept.md)) — and it also permits assigning a 4B
model to the hardest conversational job in the system.

## Decision
**Any configured provider may hold the `lead` role, including a local model. Corifeu warns,
and never defaults to a weak lead.**

Three mechanisms, in increasing order of persistence:

1. **The wizard states the trade-off** when a local model is selected as lead: this is the
   role most sensitive to model quality, and a weak lead degrades the whole experience even
   when delegates perform well.
2. **Defaults prefer the strongest available provider** for `lead`. A local model is offered
   but is never pre-selected when a cloud provider is configured and authenticated.
3. **`doctor` flags a lead whose model failed any conformance test**
   ([ADR-0039](0039-provider-conformance-suite.md)), so the warning resurfaces after
   installation rather than appearing once and being forgotten.

## Alternatives considered
- **Require a cloud model as lead.** Would guarantee a decent experience and would break
  the all-local case outright, contradicting the brief and removing a real reason to adopt
  the product.
- **Impose a minimum parameter count or capability score.** Parameter count predicts
  conversational quality poorly, the threshold would be arbitrary, and it would age badly as
  small models improve.
- **Allow it silently.** The user discovers the problem through a bad experience and
  attributes it to Corifeu rather than to the configuration — the worst outcome of the three.

## Consequences
- The warning must be specific about *what* degrades. "This model may be weak" is useless;
  "the lead summarises progress and decides escalations, and a weak lead will mis-summarise
  and over-escalate" is actionable.
- Over-warning trains users to dismiss warnings. This one appears at configuration time and
  in `doctor`, not on every run.
- A user who deliberately chooses an all-local configuration has usually accepted the
  trade-off already. The warning should inform, not nag, and should not require confirmation.
- If dogfooding shows local leads are unusable in practice, the honest response is to
  document that finding plainly rather than to silently discourage the configuration.
