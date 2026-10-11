# 0002 — English for every artefact

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The project owner is a native Portuguese speaker, and the prototype already contains a
Portuguese comment that survived into `main` unnoticed. The goal is a community-driven
open-source product. Mixed-language codebases fragment contribution: a reviewer who cannot
read a comment cannot review the code it explains.

## Decision
English for code, identifiers, comments, documentation, commit messages, issues, pull
requests, CLI output and error messages. No exceptions, including internal notes.

Localisation of *user-facing CLI output* may be added later as a feature. It is not a
reason to write any source artefact in another language.

## Alternatives considered
- **Portuguese internally, English for published docs.** The boundary never holds — the
  prototype proves it, having leaked one comment across the line within four commits. It
  also makes every internal artefact unreviewable by most potential contributors.
- **Bilingual documentation from day one.** Doubles the maintenance burden before there is
  evidence of a non-English audience. Revisit if one appears.

## Consequences
- The project owner writes in a second language continuously, which costs time and
  occasionally precision.
- CI should enforce what it cheaply can. A non-ASCII-letter check over source comments
  catches accented Portuguese; it cannot catch unaccented words and should not pretend to.
- Conversations between the owner and contributors may happen in any language. Only
  artefacts are constrained.
