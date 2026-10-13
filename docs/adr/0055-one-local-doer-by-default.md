# 0055 — One local doer by default, raised per provider

- **Status:** Accepted
- **Date:** 2026-10-13
- **Supports:** [ADR-0020](0020-sequential-execution-in-mvp1.md), [ADR-0008](0008-local-runtimes.md)
- **Evidence:** [measurement](../measurement.md)

## Context
Running several doers at once is the obvious way to spend idle wall-clock, and on a local
runtime it is usually an illusion.

Ollama serialises. During the measurement a request to a second model, issued while a doer
was mid-generation, returned nothing within 60 seconds. Two doers sharing one Ollama instance
do not run in parallel; they queue, and the queue is invisible. Every per-step timing a run
records in that state is wrong, which also corrupts the metering
[ADR-0031](0031-token-savings-metering.md) depends on.

This is a property of the runtime, not of Corifeu and not of local models generally.
llama.cpp with several slots genuinely parallelises, as do two Ollama instances on different
ports. A global rule would be wrong in both directions.

## Decision
**The default concurrency for a local provider is one. It is raised per provider, not
globally.**

1. **`max_parallel_doers` is a provider setting**, defaulting to `1` for local providers,
   because the constraint belongs to the runtime that serialises.
2. **A flag and a config key raise it.** A user who runs llama.cpp with eight slots, or two
   Ollama instances, sets it and gets real parallelism.
3. **The default carries its reason.** The generated configuration says why it is 1, so a
   user raising it knows what they are asserting about their runtime.
4. **`doctor` should measure it rather than assume it.** Two concurrent requests and a
   comparison of completion times is a direct test, and it is the difference between a
   documented default and a measured one.

## Alternatives considered
- **Default to the number of cores or slots.** Produces silent queueing on the most common
  setup, which is a single Ollama instance.
- **One global concurrency setting.** Wrong for a run mixing a serialising local provider
  with a remote one that handles concurrency comfortably.
- **Detect serialisation automatically at run time and adapt.** Appealing; costs a probe on
  every run and behaves unpredictably under load.
- **Allow parallel doers and accept the queue.** The wall-clock cost is tolerable; the
  corrupted timings are not, because they feed the savings figure the product reports to the
  user.

## Consequences
- [ADR-0020](0020-sequential-execution-in-mvp1.md)'s sequential execution now has a
  measured reason behind it for local providers, not only a scope one.
- The planner should assume serial execution when slicing for a local provider. With
  [ADR-0053](0053-qa-writes-the-tests.md), QA and coder on one local runtime are strictly
  sequential per slice.
- Metering must record the concurrency in force, or a slow run cannot be told from a queued
  one afterwards.
- Raising the setting is a claim about the user's runtime that Corifeu cannot verify without
  the `doctor` probe. Until that probe exists, the setting is trusted.
