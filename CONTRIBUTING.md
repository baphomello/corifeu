# Contributing to Corifeu

Thank you for considering a contribution.

Corifeu is in **phase 1**: the design is written, the Rust implementation is not. Read
[docs/roadmap.md](docs/roadmap.md) before starting work, so you do not build something the
next phase replaces.

## Before you write code

**Read the decision records.** [docs/adr/](docs/adr/) holds 45 of them, with the reasoning
and the rejected alternatives for every significant choice. A pull request that contradicts
an accepted ADR will be asked to either follow it or supersede it with a new one — the
second is entirely welcome, with an argument.

**Learn the vocabulary.** [docs/glossary.md](docs/glossary.md) defines the terms this
project uses narrowly: lead, doer, checker, board, contract, artefact, oracle, class. Its
first section is the distinction most worth internalising — **roles are the unit of
restriction, models are a resource roles consume**.

**Open an issue first** for anything beyond a small fix. It is cheaper to disagree about an
approach in an issue than in a finished branch.

## Developer Certificate of Origin

Corifeu uses the [DCO](https://developercertificate.org/). **There is no CLA**, and there
never will be: without aggregated rights, nobody — including the project owner — can
relicense this code. That is deliberate ([ADR-0003](docs/adr/0003-agpl-with-notice-and-dco.md)).

Sign off every commit:

```bash
git commit -s -m "your message"
```

This appends one line:

```
Signed-off-by: Your Name <your.email@example.com>
```

By signing off, you certify the Developer Certificate of Origin 1.1: that you wrote the
contribution or have the right to submit it under the project's license.

Use your real name and a real email address. CI rejects unsigned commits. To fix a branch
you already wrote:

```bash
git rebase --signoff main
```

## License

Corifeu is **AGPL-3.0**. Contributions are accepted under the same license.

Writing a **plugin** is different. Plugins run out of process over a versioned JSON-RPC
protocol ([ADR-0032](docs/adr/0032-out-of-process-plugins.md)), so they are not derived
works, and **you choose their license**. That is one of the reasons the protocol is shaped
that way. You also do not need Rust — a plugin can be written in any language that can speak
JSON-RPC over stdio.

## Everything in English

Code, comments, documentation, commit messages, issues, pull requests and CLI output are all
English ([ADR-0002](docs/adr/0002-english-only.md)). This is about keeping every artefact
reviewable by every contributor, not about anyone's first language. Conversations can happen
in any language you like.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <imperative summary>
```

Types: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `chore`, `build`, `ci`, `style`,
`revert`.

Write the *why* in the body. The diff already shows the what. A commit that fixes something
non-obvious should say what was wrong, not only what changed.

## Pull requests

- Branch from `main`.
- Keep the PR to one concern.
- State what you ran and what it showed. If you did not run something, say so plainly —
  that is useful information, and an unverified claim is worse than an honest gap.
- Name behaviour changes explicitly, including side effects.
- Record decisions and their rejected alternatives. This is the content a reviewer cannot
  recover from the diff.

## Claims and evidence

This project's whole thesis is that **objective verification beats confident assertion**,
and it holds its own contributions to that standard.

- A number you report comes from a run, not from reasoning about the code.
- Mark anything you could not verify as **UNVERIFIED** rather than stating it as fact. The
  documentation does this in several places, and those marks are a feature.
- "Looks wrong" is not "is wrong". Check whether the odd thing is deliberate before calling
  it a defect — [docs/diagnosis.md](docs/diagnosis.md) has a worked example where a script
  that appeared broken was correct.

## Tests

Once there is code to test, there is a test requirement. Until then, the honest statement is
that this project currently verifies itself not at all, and fixing that is phase 1 work.

Two behaviours already have mandatory regression tests, because both are defects found in
the prototype that must never return:

- **Truncation must be explicit.** Any tool returning partial content marks it, with the
  retained range and the full size ([ADR-0017](docs/adr/0017-artifact-references-not-summaries.md)).
- **Restriction must be tested adversarially.** Instruct the agent to do the forbidden
  thing, then assert it did not happen ([ADR-0039](docs/adr/0039-provider-conformance-suite.md)).

## Reporting security issues

Do not open a public issue. See [SECURITY.md](SECURITY.md).

## Code of Conduct

[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) applies to every project space. Disagreeing with a
decision is not a conduct problem; this project is built to be argued with.
