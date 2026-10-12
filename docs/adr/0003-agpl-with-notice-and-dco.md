# 0003 — AGPL-3.0 with NOTICE and DCO, no CLA

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Three goals, in priority order: the source stays open permanently; modifications stay open,
including when the software is offered as a network service; attribution to authors is
preserved. A fourth constraint follows from the first: no future owner — including the
founder — should be able to relicense the project proprietary.

> This ADR records reasoning, not legal advice. Obtain professional advice before relying
> on any of it.

## Decision
1. **AGPL-3.0** for the core, as already applied.
2. **A `NOTICE` file** carrying attribution, invoked through AGPL **section 7(b)**, which
   permits an additional term requiring preservation of specified author attributions.
3. **DCO** (`Signed-off-by:`, Developer Certificate of Origin 1.1) for contributions.
   **No CLA.**
4. **Plugins run out of process** ([ADR-0032](0032-out-of-process-plugins.md)), so plugin
   authors choose their own licence.

## Alternatives considered
- **MIT or Apache-2.0.** Maximises adoption; permits a proprietary hosted fork of exactly
  this product. Rejected against goal two.
- **GPL-3.0.** Copyleft, but its trigger is distribution. An orchestration product is a
  natural SaaS, and GPL leaves the network case open. AGPL closes it.
- **A CLA assigning rights to the founder.** Standard practice, and precisely what enables
  a future relicence. Rejected deliberately: without aggregated rights, relicensing requires
  the consent of every contributor, which is the guarantee being sought. The cost is giving
  up the owner's own freedom to relicense later.
- **Dual licensing (AGPL + commercial).** Requires a CLA. Same rejection.

## Consequences
- AGPL deters some corporate adoption. Accepted knowingly.
- Out-of-process plugins are now a **licensing requirement**, not only an engineering
  preference. An in-process plugin ABI would arguably make plugins derivative works and
  force AGPL onto them, contradicting goal four. This constrains
  [ADR-0032](0032-out-of-process-plugins.md) permanently.
- DCO needs enforcement in CI (a DCO check action) and documentation in `CONTRIBUTING.md`.
- The 7(b) attribution term must name precisely what is preserved, or it is unenforceable
  noise.
- Corifeu invokes other agent CLIs as subprocesses. This is aggregation, not linking, and
  does not propagate AGPL to them. Worth stating explicitly in the docs, since it is the
  first question a careful user will ask.
