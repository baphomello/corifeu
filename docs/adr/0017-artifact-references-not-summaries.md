# 0017 — Agents exchange references, not summaries

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
When agent A summarises a document for agent B, which summarises it for agent C, detail is
lost at every hop and the loss is invisible — each summary reads as authoritative. Over a
multi-role run this degrades into the children's telephone game, with the added hazard that
models fabricate plausible detail to fill gaps.

The prototype contains a sharper version of this bug. `read_file` returns
`f.read()[:8000]` **with no truncation marker**. A model that reads a 12 KB file believes
it has the whole thing, and a subsequent `write_file` silently destroys the remainder.

## Decision
Agents exchange **references to artefacts**, not summaries of them. A message carries an
artefact id; the recipient's context contract decides whether the content is materialised
into its view.

Rules:

1. Every artefact on the board has a stable id, a kind, and a content hash.
2. Messages reference artefacts by id.
3. The runtime materialises content during view assembly, per the contract.
4. **Any truncation is explicit in the payload** — a marker, the byte range retained, and
   the full size. An agent must never be able to mistake a partial artefact for a complete
   one.
5. Summaries are legitimate artefacts in their own right, with their own ids, and they
   reference what they summarise. A summary never *replaces* its source.

## Alternatives considered
- **Pass full content everywhere.** No information loss, unbounded context, and the
  economics collapse.
- **Summarise aggressively between hops.** Cheapest, and loses exactly the detail that
  correctness depends on.
- **Let each agent decide what to pass on.** Non-deterministic and unauditable; the failure
  mode is invisible.

## Consequences
- Content-addressed artefact storage, with large content on disk under `.corifeu/` and
  metadata on the board.
- The lead can operate on references without materialising content, which is a direct token
  saving.
- Re-reading an artefact by id yields the same bytes, so an agent that re-reads sees no
  drift — unlike a re-summary.
- Rule 4 is a hard requirement on every tool that returns partial content. It is the
  prototype's most dangerous bug and must have a regression test from the first
  implementation.
