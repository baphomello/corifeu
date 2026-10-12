# 0009 — Direct provider APIs deferred to MVP 2

- **Status:** Accepted
- **Date:** 2026-10-11

## Context
Direct APIs (Anthropic, OpenAI, OpenRouter and others) would give Corifeu exact token
accounting, stable versioned contracts and no dependency on a third-party CLI's output
format. They would also require Corifeu to implement the agent loop itself — tool
definition, tool execution, retries, context management — which is a substantial amount of
the functionality the CLIs already provide.

## Decision
Direct APIs are **MVP 2**. The provider adapter interface is designed so they plug in
**without touching the core**.

Concretely, the adapter interface must not leak subprocess assumptions. It is expressed in
terms of *start a turn, stream events, report usage, end a turn* — not *spawn a process and
read its stdout*. A direct-API adapter and a CLI adapter are then two implementations of
one interface, differing only internally.

## Alternatives considered
- **Direct APIs in MVP 1.** More engineering before any end-to-end value, and it forfeits
  subscription billing, which is a major part of the appeal.
- **Never.** Leaves Corifeu permanently exposed to CLI churn and permanently unable to
  report exact token costs ([ADR-0031](0031-token-savings-metering.md)).

## Consequences
- The adapter interface is designed against a use case that does not exist yet, which is
  the classic way to design an interface that fits nothing. Mitigated by writing the
  direct-API adapter *signature* during phase 2 as a compile-checked stub, so the shape is
  proven against two real CLIs and one real-but-unimplemented API.
- Secrets management is deferred with it. MVP 1 relies on the CLIs' own credentials and
  never stores secrets ([ADR-0028](0028-yaml-config-with-json-schema.md)); MVP 2 must
  address API keys properly, which is a genuinely new problem.
- Once direct APIs exist, the same provider may be reachable two ways. Configuration must
  name the *route*, not only the vendor.
