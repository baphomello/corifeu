# Glossary

Terms used throughout Corifeu's documentation and code. Several are ordinary English words
with a narrow technical meaning here — those are the ones worth reading.

## The distinction that resolves most confusion

**Roles are the unit of restriction. Models are a resource that roles consume.**

A role is a job with a contract, a set of permissions and a set of obligations. A model is
an engine. One model may hold several roles, and each invocation is a separate agent with
its own view and its own session — they share nothing.

This is why the `lead` role writes nothing while the model behind it may write freely in
another role ([ADR-0042](adr/0042-test-authorship.md)). Questions that look hard usually
become easy once this distinction is held firmly.

---

## Core terms

**Agent** — one invocation of a model acting in one role, with one assembled view. Two roles
on the same model are two agents.

**Artefact** — any durable output of a run: a spec, a patch, a file version, a finding, a
report. Every artefact has a stable id, a kind and a content hash. Agents exchange artefact
**references**, never summaries of them ([ADR-0017](adr/0017-artifact-references-not-summaries.md)).

**Blackboard** (also **board**) — the single source of truth for a run: tasks, artefacts,
messages, findings, verdicts, budgets and events. No agent holds the whole board
([ADR-0013](adr/0013-blackboard-as-source-of-truth.md)).

**Category** — one of the five fixed groupings the core understands: `lead`, `planners`,
`researchers`, `doers`, `checkers`. Categories define coordination semantics and are not
extensible. Roles are; categories are not ([ADR-0010](adr/0010-role-categories.md)).

**Channel** — a declared pair of roles permitted to exchange direct messages without
relaying through the lead. Direct means unrelayed, never unobserved: every message is still
recorded on the board ([ADR-0015](adr/0015-contracted-direct-channels.md)).

**Checker** — a role category that verifies work and returns findings and verdicts. Normally
read-only, though a checker may be granted authorship of the test class
([ADR-0042](adr/0042-test-authorship.md)).

**Context contract** (also **contract**) — the declarative specification of what a role's
view contains. Enforced by *assembly*, not by instruction: the runtime builds the view and
the agent never had access to anything else ([ADR-0014](adr/0014-context-contracts-per-role.md)).

**Doer** — a role category that produces the deliverable. The only category that writes
repository files. At least one is required.

**Event** — an immutable, sequenced record appended to the log. Board state is a projection
of events, which is why replay, resume and audit are one mechanism rather than three
([ADR-0021](adr/0021-sqlite-with-event-log.md)).

**File class** — a named set of files defined by ordered glob rules, evaluated first-match-
wins. Roles own classes, not directories. This is what lets a test sitting beside its source
classify as a test ([ADR-0035](adr/0035-file-ownership-by-glob-classes.md)).

**Finding** — a typed message from a checker describing a problem, with severity, location
and an evidence reference.

**Lead** — the single role that talks to the user. Exactly one per configuration. Writes
nothing. Receives escalations and phase summaries, not the inner loop — which is where the
token saving comes from.

**Oracle** — an objective check that passes or fails without judgement: build, test, lint,
type check, or any user-configured command. **Oracles outrank every agent opinion**,
including the lead's and including unanimity against them
([ADR-0019](adr/0019-tie-breaking-and-oracle-precedence.md)).

**Ownership** — the mapping from file classes to the roles permitted to write them.
Unclassified paths are unwritable by doers: the model fails closed.

**Planner** — a role category that decomposes work into tasks. Writes specs and plans as
board artefacts, not repository files.

**Preset** — a named bundle of roles, channels, contracts and budgets. A starting point the
user edits, never a closed set.

**Projection** — board state rebuilt by replaying the event log. Must be deterministic:
projecting the same log twice yields byte-identical state.

**Provider** — a model runtime Corifeu can drive: an agent CLI (Claude Code, Codex, Gemini),
a local runtime (Ollama, llama.cpp), or later a direct API. Built-in providers implement the
same interface as plugin providers ([ADR-0032](adr/0032-out-of-process-plugins.md)).

**Required role** — a role without which a configuration is invalid, rejected at load time
rather than mid-run. The opposite is **optional**. There is no weighting between them
([ADR-0012](adr/0012-required-and-optional-roles.md)).

**Researcher** — a role category that gathers context and returns findings. Exists to move
token-expensive, quality-cheap exploration off the lead.

**Run engine** — the runtime component that schedules tasks, dispatches to adapters and
moves events onto the board. **Infrastructure, not an agent**: it has no model and no
context. Deliberately not called an *orchestrator*, because that word conflated this job
with the lead's.

**Role** — a named job within a category, assigned to a model, with a contract, write
permissions and required artefacts. Roles are user-defined; the user chooses how many exist
per category ([ADR-0011](adr/0011-user-defined-granularity.md)).

**Run** — one invocation of `corifeu run`, from goal to final report. The unit of retention
and of metering.

**Verdict** — a typed message from a checker: `pass`, `fail` or `blocked`, with reasons and
finding references. A `pass` can close a task; prose cannot.

**View** — the assembled context handed to one agent for one invocation. Bounded,
deterministic, and the only thing that agent can see. Any truncation within it is **explicit
in the payload**, with the retained range and the full size.

---

## Terms deliberately avoided

| Avoided | Use instead | Why |
|---|---|---|
| *orchestrator* | **lead** (the agent) or **run engine** (the code) | The prototype's word conflated two different things: the agent that talks to you, and the machinery that schedules the run |
| *mode* | configuration | `open`/`private` was a label with nothing behind it ([ADR-0041](adr/0041-drop-the-mode-concept.md)) |
| *importance*, *priority*, *weight* | **required** / **optional** | Implies a number that nothing consumes ([ADR-0012](adr/0012-required-and-optional-roles.md)) |
| *memory* for run state | **board**, **event log** | Reserved for project memory, which is a different thing with a different lifetime |
| *source file*, *test result* in core types | **artefact**, **check result** | Naming discipline keeps non-software domains open ([ADR-0006](adr/0006-mvp-scope-software-development.md)) |

**Project memory** specifically means the committed, human-readable Markdown under
`.corifeu/memory/` — decisions and conventions about the user's project that future runs
consult. It is never run state ([ADR-0023](adr/0023-project-memory-in-versioned-files.md)).
