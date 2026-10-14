# 0061 — What may leave the machine is decided per file class, and what leaves is minimised

- **Status:** Accepted
- **Date:** 2026-10-14
- **Refines:** [ADR-0041](0041-drop-the-mode-concept.md)
- **Supports:** [ADR-0035](0035-file-ownership-by-glob-classes.md), [ADR-0056](0056-one-oracle-per-slice.md)

## Context
[ADR-0041](0041-drop-the-mode-concept.md) replaced the prototype's undefined `private` mode
with one checkable property:

```yaml
privacy:
  allow_remote_providers: false
```

That was the right move and it is too coarse. The property is a single switch over the whole
run, so a user with one thing they cannot send loses the expensive model for everything.

The case that makes this concrete is reverse engineering, which is work Corifeu's own owner
does. The binary under analysis, its path, and the symbol names recovered from it are the
confidential part — they identify a client, a product, or an unreleased build. A single
decompiled function, with its identifiers replaced, is not. The local model can drive the
disassembler and extract the function; the question of what that function *does* is exactly
the kind of thing a frontier model answers better.

Under the boolean, that user sets `allow_remote_providers: false` and gives up the half of
the thesis that says an expensive model is worth its price as a diagnostician
([measurement](../measurement.md)). Or sets it `true` and sends the binary's paths and
symbols to a third party. Neither is the decision they wanted to make.

The machinery to do better already exists and was built for other reasons. Files already have
classes ([ADR-0035](0035-file-ownership-by-glob-classes.md)). Work is already cut into slices
small enough to carry one oracle ([ADR-0056](0056-one-oracle-per-slice.md)), so a slice-sized
context is already close to the minimum that can be sent. Agents already exchange references
rather than content ([ADR-0017](0017-artifact-references-not-summaries.md)). Minimisation is
mostly a matter of deciding, not of building.

## Decision
**A file class carries a privacy policy, and an artefact that may leave does so minimised and
redacted, with the mapping kept on this machine.**

1. **Three policies per class.** `local-only`, `redacted`, `open`. An artefact in no class is
   `local-only`: a gap in the globs must fail closed, because the mistake is not recoverable
   once a request has been sent.
2. **`local-only` means no part, in any form.** Not the content, not the path, not the file
   name, not a count of its lines. A slice touching a `local-only` artefact cannot be routed
   to a remote role at all, and the planner is told so before it plans rather than after.
3. **`redacted` sends a slice-sized excerpt with identifiers replaced** by stable
   placeholders, so the remote model still sees structure and relationships and sees no names.
   The mapping lives on the blackboard, never in the prompt, and the answer is expanded back
   locally before any role acts on it.
4. **The run records what left.** Every remote request logs the class, the policy and the
   redaction applied. A privacy claim nobody can audit afterwards is a slogan.
5. **`privacy.allow_remote_providers: false` still works** and becomes the shorthand for
   `local-only` on every class. [ADR-0041](0041-drop-the-mode-concept.md)'s guarantee is
   unchanged for anyone relying on it.

### What this is not

**This is minimisation for confidentiality. It is not fragmentation to defeat a provider's
own review of what it is being asked to do.** Corifeu does not split a task so that the model
answering it cannot tell what it is part of. Work that a provider would decline on sight is
work to run locally or not at all, and the distinction matters in both directions: a slice
small enough to hide its purpose is usually also a slice too small to be answered well.

## Alternatives considered
- **Keep the boolean.** One switch, all or nothing. The user who cannot send one file sends
  nothing, which is the outcome this exists to avoid.
- **Redact everything, always.** Pays the quality cost of anonymised prompts on work that has
  nothing to hide, and trains users to ignore the feature.
- **Have the user redact by hand before each call.** That is not a runtime, and a manual step
  in a loop that runs hundreds of times is a step that gets skipped.
- **Let the local model decide what is sensitive.** A judgement with no oracle, on the one
  question where being wrong cannot be undone. [ADR-0019](0019-tie-breaking-and-oracle-precedence.md)'s
  rule applies hardest here: there is no check that can catch this after the fact.
- **Classify by directory rather than class.** The same reason
  [ADR-0035](0035-file-ownership-by-glob-classes.md) exists: a decompiled artefact and a note
  about it sit in the same folder and need different answers.
- **Encrypt the payload.** The remote model has to read it to answer. There is nothing to
  encrypt against.

## Consequences
- File classes become load-bearing for a second thing besides ownership, so a careless glob is
  now a confidentiality bug and not only a permissions one. `doctor` must print the policy in
  force per class, and the installer must ask rather than assume.
- **The planner must know policy before it plans.** A plan whose slices cannot be routed is a
  planning failure stated up front, never a silent downgrade to a weaker model.
- Redaction is only as good as its identifier extractor. A per-language one is the right
  answer and a generic one — replace the symbols the artefact itself declares — is what can
  ship first. The gap between them should be stated to the user, not papered over.
- **A redacted prompt is a worse prompt**, and by how much is unmeasured. It belongs in the
  [measurement](../measurement.md) as its own question rather than being assumed small.
- The redaction map is itself confidential and lives under the run's retention
  ([ADR-0022](0022-run-retention-policy.md)), which must not compact it into a summary.
- This makes privacy a third axis of the product beside cost and quality. The thesis is
  currently written as two; it should be three.
