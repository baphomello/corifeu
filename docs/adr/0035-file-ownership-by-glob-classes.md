# 0035 — File ownership by glob classes, not path prefixes

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
The prototype decides who may write a file by testing the literal prefix `src/`. This fails
in both directions, silently ([diagnosis](../diagnosis.md)):

- Projects with production code in `lib/`, `app/`, `internal/`, `pkg/` or `cmd/` are **not
  protected at all**. The hook runs, matches nothing, and the lead writes feature code while
  believing it cannot.
- Projects whose unit tests sit **beside** their sources — `src/bank/deposit.ts` next to
  `src/bank/deposit_test.ts` — have their tests blocked along with their sources, which
  contradicts the rule that the lead owns the acceptance oracle.

The second layout is not exotic. It is the convention in Go, and common in Rust (`#[cfg(test)]`
in the same file), Jest, Vitest and pytest. **A directory prefix cannot express
"implementation versus oracle", because that distinction is not a directory distinction.**

There is a deeper problem. Even a correct path rule cannot be enforced the way the prototype
enforces it — by regex over Bash commands. `guard-src.py` matches five patterns; `python -c`,
`eval`, a heredoc, `base64 -d | sh`, or any interpreter evade all of them. Pattern-matching
shell strings for intent is unsound in principle, not merely incomplete in practice.

## Decision
Replace path prefixes with **file classes defined by ordered glob rules**, and assign
**classes** to roles.

```yaml
file_classes:
  - name: test                     # declared FIRST so sibling tests win
    include: ["**/*_test.*", "**/*.test.*", "**/*_spec.*", "tests/**", "spec/**"]
  - name: source
    include: ["src/**", "lib/**", "app/**", "internal/**", "pkg/**", "cmd/**"]
  - name: config
    include: ["*.{json,toml,yaml,yml}", ".github/**", "Dockerfile*"]
  - name: docs
    include: ["**/*.md", "docs/**"]

ownership:
  test:   [lead, qa]               # the oracle belongs to whoever is judged by it
  source: [coder]
  config: [devops]
  docs:   [tech-writer]
```

Four rules make this predictable:

1. **Ordered evaluation, first match wins.** This is what solves the sibling-test case:
   declare `test` before `source`, and `src/bank/deposit_test.ts` classifies as `test` while
   `src/bank/deposit.ts` falls through to `source`. No exclusion lists needed for the common
   case.
2. **Unclassified files are unwritable by doers.** Fail closed. `corifeu doctor` lists
   unclassified paths so the gap is visible rather than latent.
3. **Project-type presets** are suggested from repository markers — `go.mod`, `Cargo.toml`,
   `package.json`, `pyproject.toml` — and **confirmed by the user** in the wizard. Detection
   proposes; it never imposes.
4. **Enforcement is by capability, not by pattern matching.** The write tool validates its
   path argument against the ownership map before acting. Shell commands are confined by the
   sandbox ([ADR-0037](0037-sandboxed-command-execution.md)), not inspected by regex.

## Alternatives considered
- **Keep path prefixes, make them configurable.** Fixes the `lib/` case, not the sibling-test
  case. The project owner raised sibling tests specifically; this does not address it.
- **Classify by file content or AST.** Accurate and expensive, language-specific, and
  unpredictable to a user trying to understand why a write was refused.
- **Most-specific-glob-wins instead of ordered.** Avoids ordering mistakes, and "most
  specific" needs a definition that users can predict. Ordered rules are explainable in one
  sentence.
- **No ownership rules; rely on review.** Abandons enforcement, which the prototype already
  established is necessary.

## Consequences
- Classes are a first-class configuration concept with a schema and presets per ecosystem.
- `corifeu explain-path <file>` should exist: given a path, print the class it resolves to,
  the rule that matched, and the roles that may write it. Without it, misconfiguration is
  hard to diagnose.
- Ordering is load-bearing, which means a user can break it by reordering. The schema should
  warn when a later rule is fully shadowed by an earlier one.
- Classes are useful beyond ownership — context contracts
  ([ADR-0014](0014-context-contracts-per-role.md)) can reference them, so a reviewer's view
  can be "all `test` files touched by this task" without restating globs.
- This generalises past software: `source` and `test` are preset names, not core concepts,
  which keeps [ADR-0006](0006-mvp-scope-software-development.md)'s promise.
