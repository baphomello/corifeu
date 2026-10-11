# Configuration design

Corifeu is configured by `.corifeu/config.yaml`, validated against a published JSON Schema
([ADR-0028](adr/0028-yaml-config-with-json-schema.md)).

The file is **committed**. It is reviewable, portable between machines, and doubles as the
restore point after `uninstall` ([ADR-0038](adr/0038-config-as-uninstall-restore.md)).
Secrets are therefore structurally excluded: Corifeu relies on the provider CLIs' own
credential storage and on environment variables, and the schema has no field that could
hold one.

## Precedence

Highest wins ([ADR-0029](adr/0029-config-precedence-order.md)):

| | Source | Use |
|---|---|---|
| 1 | CLI flags | `corifeu run --autonomy supervised` |
| 2 | `CORIFEU_*` environment | CI, containers |
| 3 | `.corifeu/config.yaml` | Committed, team-shared |
| 4 | `~/.config/corifeu/config.yaml` | Personal defaults |

Merging is deep per key; **lists are replaced wholesale**, never merged element-wise.
`corifeu config show --explain` prints the effective value of every key with the source it
came from — without it, four-level precedence is a debugging problem of our own making.

## Complete illustrative configuration

Every decision in the foundation brief appears below. The git section is the project
owner's own flow, which was the test case the design had to satisfy.

```yaml
# yaml-language-server: $schema=https://corifeu.dev/schema/v1/config.json
version: 1

project:
  name: my-app
  domain: software            # the only MVP domain

# ─────────────────────────── providers ───────────────────────────
providers:
  claude:
    kind: cli
    command: claude
    min_version: "2.1.0"
  codex:
    kind: cli
    command: codex
    min_version: "0.160.0"
  local:
    kind: ollama
    base_url: http://127.0.0.1:11434   # never auto-detected; see troubleshooting.md
    # base_url: http://192.168.32.1:11434   # WSL → Windows host

privacy:
  allow_remote_providers: true   # false refuses any non-local provider in any role

# ──────────────────────────── roles ──────────────────────────────
# Categories are fixed: lead, planners, researchers, doers, checkers.
# Role names, counts and models are the user's choice.
roles:
  lead:
    category: lead
    provider: claude
    model: opus
    required: true
    # The lead holds no write capability. This is enforced by the runtime,
    # not by instruction (ADR-0036).

  architect:
    category: planners
    provider: claude
    model: sonnet
    required: true
    contract:
      task: true
      memory: [decisions.md, conventions.md]
      files: ["**/*.md", "src/**/mod.rs"]
      max_tokens: 32000

  explorer:
    category: researchers
    provider: local
    model: qwen3.5:4b
    required: false
    contract:
      task: true
      files: [repo_index]
      max_tokens: 16000

  coder:
    category: doers
    provider: local
    model: ornith1.5:9b
    required: true              # a software project cannot exist without one
    writes: [source]            # file classes, not paths
    contract:
      task: true
      acceptance: true
      files:
        - target
        - neighbors: signatures
      artifacts: [spec_excerpt]
      messages: { channels: [reviewer], last: 6 }
      memory: [conventions.md]
      max_tokens: 24000

  reviewer:
    category: checkers
    provider: codex
    model: gpt-5.5
    required: false
    reads_only: true
    contract:
      task: false
      acceptance: true
      artifacts: [diff]
      max_tokens: 20000
    # Cross-family review: the reviewer is deliberately not Claude,
    # because same-family models share failure modes (ADR-0019).

# ───────────────────────── file classes ──────────────────────────
# Ordered. First match wins. `test` precedes `source` so that a sibling
# test file classifies as a test, not as source (ADR-0035).
file_classes:
  - name: test
    include: ["**/*_test.*", "**/*.test.*", "**/*_spec.*", "tests/**", "spec/**"]
  - name: source
    include: ["src/**", "lib/**", "app/**", "internal/**", "pkg/**", "cmd/**"]
  - name: config
    include: ["*.{json,toml,yaml,yml}", ".github/**", "Dockerfile*"]
  - name: docs
    include: ["**/*.md", "docs/**"]

ownership:
  # UNRESOLVED: this grants the lead write capability, which ADR-0010 denies it.
  # Who authors the tests is open question Q1 in open-questions.md.
  test: [lead, reviewer]        # the oracle belongs to whoever is judged by it
  source: [coder]
  config: [lead]
  docs: [lead]

# ──────────────────────────── channels ───────────────────────────
# Direct messages only between declared pairs. Every message is still
# recorded on the board: direct means unrelayed, never unobserved.
channels:
  - [coder, reviewer]
  - [architect, lead]

# ──────────────────────────── oracles ────────────────────────────
# Objective checks. These outrank every agent opinion (ADR-0019).
oracles:
  build:  { command: ["cargo", "build"],             timeout_s: 300 }
  test:   { command: ["cargo", "test"],              timeout_s: 600 }
  lint:   { command: ["cargo", "clippy", "--", "-D", "warnings"], timeout_s: 300 }
  format: { command: ["cargo", "fmt", "--check"],    timeout_s: 60 }

# ──────────────────────── budgets and loops ──────────────────────
limits:
  rounds_per_pair: 3
  task_budget_tokens: 120000
  run_budget_tokens: 1500000
  role_timeout_s: 900           # sized for local-model latency, not cloud intuition
  loop_detection: true          # same diff or same finding repeated

# ──────────────────────────── autonomy ───────────────────────────
autonomy: semi-autonomous       # supervised | semi-autonomous | autonomous

visibility:
  peer_conversations: summary   # summary | live | hidden
  show_token_usage: true

# ────────────────────────────── git ──────────────────────────────
# The project owner's flow, expressed step by step (ADR-0025):
# commit locally as work proceeds, push nothing until everything is
# done, present every commit with its rationale, then push and open a
# PR on approval, then merge and delete the branch on approval.
git:
  branch:
    strategy: per-run
    pattern: "corifeu/{run-slug}"
  commit:
    cadence: per-task
    author: user                # never an AI trailer
    convention: conventional
    require_approval: false     # local commits flow freely
  push:
    when: after-approval        # the gate: nothing leaves the machine unasked
    present_commits: true       # show every commit and why it exists, first
  pull_request:
    when: after-approval
    description: generated
    labels: [corifeu]
  merge:
    when: after-approval
  delete_branch:
    when: after-merge

# ──────────────────────────── failures ───────────────────────────
fallback:
  transient:      { strategy: retry, attempts: 3, backoff: exponential, max_s: 60 }
  malformed:      { strategy: retry, attempts: 2 }   # schema violation fed back
  tool_call_fail: { strategy: escalate, escalate_to: claude/sonnet, budget_cap: 50000 }
  budget_exhausted: { strategy: stop }                # never retried, or budgets mean nothing
  # A failing test is NOT a failure. It is a finding, routed to the doer.

# ─────────────────────────── execution ───────────────────────────
execution:
  sandbox: auto                 # auto | none — `none` only when externally sandboxed
  shell: false                  # commands run as argv, never through sh -c
  allowed_commands:             # oracles are allowlisted implicitly
    - ["git", "status"]
    - ["git", "diff"]

# ──────────────────────────── storage ────────────────────────────
retention:
  full_runs: 10
  max_bytes: 500MB              # the limit that actually protects against the tail
  always_keep: 3

memory:
  dir: .corifeu/memory
  documents: [decisions.md, conventions.md, glossary.md]
  agent_writes: propose         # propose | direct | never

# ──────────────────────────── metering ───────────────────────────
metering:
  enabled: true
  billing:
    claude: subscription        # report quota %, not money
    codex:  subscription
    local:  free
  report: end-of-run            # end-of-run | per-task | never

# ──────────────────────────── plugins ────────────────────────────
plugins:
  - name: corifeu-sast
    path: ./plugins/corifeu-sast
    provides: [checker]
    config:
      severity_threshold: medium
```

## Minimal valid configuration

Everything else defaults. This is what a wizard run with all-conservative answers produces,
and it is the smallest file that passes validation:

```yaml
version: 1
roles:
  lead:
    category: lead
    provider: claude
    model: opus
    required: true
  coder:
    category: doers
    provider: local
    model: qwen2.5-coder:7b
    required: true
    writes: [source]
oracles:
  test: { command: ["npm", "test"] }
```

Validation rejects anything thinner: exactly one `lead`, at least one `doers` role that can
write, and the roles referenced by `ownership` and `channels` must exist
([ADR-0012](adr/0012-required-and-optional-roles.md)).

## JSON Schema outline

Published at a stable versioned URL so editors can complete and validate while typing. The
structure, abbreviated:

```jsonc
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://corifeu.dev/schema/v1/config.json",
  "type": "object",
  "required": ["version", "roles", "oracles"],
  "additionalProperties": false,
  "properties": {
    "version": { "const": 1 },

    "roles": {
      "type": "object",
      "minProperties": 2,
      "propertyNames": { "pattern": "^[a-z][a-z0-9-]*$" },
      "additionalProperties": { "$ref": "#/$defs/role" }
    },

    "file_classes": {
      "type": "array",
      "items": { "$ref": "#/$defs/fileClass" },
      "description": "Ordered. First match wins."
    },

    "channels": {
      "type": "array",
      "items": {
        "type": "array",
        "items": { "type": "string" },
        "minItems": 2, "maxItems": 2
      }
    },

    "autonomy": { "enum": ["supervised", "semi-autonomous", "autonomous"] },
    "git": { "$ref": "#/$defs/git" },
    "limits": { "$ref": "#/$defs/limits" },
    "retention": { "$ref": "#/$defs/retention" },
    "execution": { "$ref": "#/$defs/execution" },
    "metering": { "$ref": "#/$defs/metering" },
    "plugins": { "type": "array", "items": { "$ref": "#/$defs/plugin" } }
  },

  "$defs": {
    "role": {
      "type": "object",
      "required": ["category", "provider", "model"],
      "additionalProperties": false,
      "properties": {
        "category": { "enum": ["lead", "planners", "researchers", "doers", "checkers"] },
        "provider": { "type": "string" },
        "model":    { "type": "string" },
        "required": { "type": "boolean", "default": false },
        "writes":   { "type": "array", "items": { "type": "string" } },
        "reads_only": { "type": "boolean", "default": false },
        "contract": { "$ref": "#/$defs/contract" }
      }
    },

    "contract": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "task":       { "type": "boolean", "default": true },
        "acceptance": { "type": "boolean", "default": true },
        "files":      { "type": "array" },
        "artifacts":  { "type": "array", "items": { "type": "string" } },
        "messages":   { "type": "object" },
        "memory":     { "type": "array", "items": { "type": "string" } },
        "max_tokens": { "type": "integer", "minimum": 1000 }
      }
    },

    "git": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "branch":         { "$ref": "#/$defs/gitBranch" },
        "commit":         { "$ref": "#/$defs/gitCommit" },
        "push":           { "$ref": "#/$defs/gitGate" },
        "pull_request":   { "$ref": "#/$defs/gitPr" },
        "merge":          { "$ref": "#/$defs/gitGate" },
        "delete_branch":  { "$ref": "#/$defs/gitGate" }
      }
    },

    "gitGate": {
      "type": "object",
      "properties": {
        "when": { "enum": ["never", "after-approval", "automatic", "after-merge"],
                  "default": "never" }
      }
    }
  }
}
```

Nine `$defs` referenced above — `fileClass`, `gitBranch`, `gitCommit`, `gitPr`, `limits`,
`retention`, `execution`, `metering`, `plugin` — are omitted here for length. They follow the
same shape as those shown. This outline is a sketch of the structure, not the shippable
schema.

### Constraints the schema cannot express

JSON Schema handles shape. Three rules need code, and all three are load-time errors with
the offending role named — never runtime surprises:

1. **Exactly one role with `category: lead`.** Expressible only awkwardly; checked in code.
2. **At least one `doers` role marked `required` that writes a class containing code.**
3. **Referential integrity** — every role named in `ownership`, `channels` or `escalate_to`
   must exist.

Two further checks are **warnings**, not errors, because they indicate probable mistakes
rather than impossible configurations:

- A `file_classes` rule fully shadowed by an earlier one, which ordering makes possible.
- A doer whose contract excludes `acceptance`, or a checker whose contract excludes the
  diff — configurations that will technically run and produce poor work.

## Environment variable mapping

`CORIFEU_` plus the key path, uppercased, underscore-joined:

| Key | Variable |
|---|---|
| `autonomy` | `CORIFEU_AUTONOMY` |
| `limits.rounds_per_pair` | `CORIFEU_LIMITS_ROUNDS_PER_PAIR` |
| `providers.local.base_url` | `CORIFEU_PROVIDERS_LOCAL_BASE_URL` |
| `git.push.when` | `CORIFEU_GIT_PUSH_WHEN` |

The mapping must be **reversible**, so key names may not contain underscores — enforced by
the `propertyNames` pattern above, which permits only kebab-case. This is a small
restriction that removes a whole class of ambiguous-override bugs.
