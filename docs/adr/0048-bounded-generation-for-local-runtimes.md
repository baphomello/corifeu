# 0048 — Local runtimes get an explicit generation budget

- **Status:** Accepted
- **Date:** 2026-10-13
- **Supports:** [ADR-0008](0008-local-runtimes.md), [ADR-0027](0027-install-wizard-with-tool-call-test.md)

## Context
Three failures during the thesis measurement looked like hangs and were not. All three came
from unbounded or mis-bounded generation against `ornith1.5:9b`, a model with a
**262,144-token context window**.

**Unbounded generation is indistinguishable from a hang.** With no `num_predict`, Ollama
generates until a stop token or the context fills. At the measured 11.5 tok/s, filling that
context is roughly **six hours**. Two runs were abandoned at 900 s and 1,536 s and recorded
as "the model is too slow". That diagnosis was wrong. The identical request with
`num_predict: 2048` returned in **100 s** with a well-formed native tool call.

**Thinking tokens consume the generation budget.** Bounded at 8,192, the same model spent
**27,049 characters — about 7,000 tokens — thinking**, then had nothing left to emit the
tool call. The step produced no output at all and looked like a refusal to use tools.

**With `think: false` and a 16k budget it worked immediately**, producing a 10 KB
implementation with 15 tests on its first attempt.

## Decision
The Ollama and llama.cpp adapters **always** set an explicit generation budget.

1. **`num_predict` is mandatory.** There is no "unlimited" setting. The default is derived
   from the role's contract — a doer writing a file needs more than a checker returning a
   verdict — and is configurable per role.
2. **The budget accounts for thinking.** Where a model supports a reasoning mode, the
   budget is for *reasoning plus output*. A budget sized for the expected code will starve
   a reasoning model before it writes anything.
3. **Reasoning is off by default for transcription-heavy roles.** `doers` write files;
   that work is mostly transcription, and reasoning competes with it for the same budget.
   Roles whose output is a judgement — `checkers`, `planners` — default to reasoning on.
   Both are overridable.
4. **Truncation is reported, never silent.** A response that stops because the budget ran
   out is an explicit, retryable failure naming the budget, not an empty turn the loop has
   to guess about. This is
   [ADR-0017](0017-artifact-references-not-summaries.md)'s rule applied to generation.

## Alternatives considered
- **Leave generation unbounded and rely on timeouts.** What produced the three misdiagnoses.
  A timeout tells you nothing about *why*, and the natural conclusion — "the model is too
  slow" — was false.
- **One global budget.** Cannot be right for both a file-writing doer and a verdict-
  returning checker; sized for the doer it wastes nothing, sized for the checker it starves
  the doer.
- **Always disable reasoning.** Cheaper and wrong for roles where judgement is the product.
- **Detect the thinking ratio and adapt automatically.** Appealing, unpredictable, and hard
  to debug when it guesses wrong. Measure it and report it instead — see below.

## Consequences
- `corifeu doctor` must measure and report the **thinking ratio**, not only whether a tool
  call arrives. [ADR-0027](0027-install-wizard-with-tool-call-test.md)'s test asks
  *whether*; it must also ask *at what cost*. A model that reasons 50x its output is
  unusable for a doer at any budget, and that is invisible to a capability flag.
- `doctor` should warn when a model's context window is large enough that an unbounded
  request would appear to hang — which is most modern local models.
- Budget-exhaustion is a distinct failure class for
  [ADR-0026](0026-configurable-failure-fallback.md): retryable with a larger budget once,
  then escalate. It is not a transient error and not a malformed output.
- Metering must count thinking tokens separately where the runtime reports them, or the
  recorded cost of a reasoning model is a fraction of the real one
  ([ADR-0031](0031-token-savings-metering.md)).
