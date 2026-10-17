# 0068 — A past step is not a conversation turn

- **Status:** Accepted
- **Date:** 2026-10-17
- **Refines:** [ADR-0047](0047-pin-the-artefact.md)
- **Evidence:** defect 52, [the S5 hybrid round](../measurement/s5-hybrid/README.md)

## Context
[ADR-0047](0047-pin-the-artefact.md) establishes that the doer's context is **rebuilt from state
on every step** rather than appended to. The artefact is read fresh from disk, older steps are
compacted to one line each, and the recent `k` keep their full result.

How a recent step was *rendered* was never decided. It was written as the obvious thing — the
shape an append-only chat would have had:

```rust
messages.push(Message::assistant(format!("I ran: {}", step.action.describe())));
messages.push(Message::tool(step.result.clone()));
```

That puts a worked example of answering in prose into the slot the model fills next, and the
model copied it. In the S5 hybrid round a slice spent **ten of its twelve steps** this way:

```
    3  say   no tool call; said: I ran: write_file src/excerpt.ts
    4  say   no tool call; said: I ran: write_file src/excerpt.ts
    5  say   no tool call; said: I ran: read_file src/excerpt.ts
    6  say   no tool call; said: I ran: say
```

Step 6 is the narration narrating itself: the model's own prose from step 5 came back as an
assistant message, and it imitated that.

The failure mode is not new. Round five measured **17% of all steps** answering without a tool
call across three cells and never named a mechanism for it. This is one, and it was in the
prompt builder rather than in the models.

`Message::Assistant` carries a `String` and nothing else. There is no way to render a past tool
call as a tool call, so every recent step was being described in the one register the model must
not answer in.

## Decision
**A recent step is carried as its result, with the action naming it, and no assistant message is
built at all.**

```rust
messages.push(Message::tool(format!(
    "step {}, `{}`:\n{}",
    step.number, step.action.describe(), step.result
)));
```

1. **No message in a doer's context has the assistant role.** There is no conversation to
   reconstruct — ADR-0047 made the context a function of state — so the only assistant turn in a
   dispatch is the one the model is about to produce, and it has no example of prose to copy.
2. **The action still reaches the model**, inside the result rather than beside it. A doer that
   ran a check fifteen steps ago has to see that it ran and what came back, which is ADR-0047's
   own requirement and the reason this is not fixed by dropping the step.
3. **This is a property of the context builder, and a test asserts it.** `no message is the
   assistant role` is cheap, total, and would have failed from the day the loop was written.

## Alternatives considered
- **Add `tool_calls` to `Message::Assistant` and render a past call properly.** The structurally
  correct fix, and the one to take when a provider needs it: an OpenAI-style wire format has an
  assistant message carrying `tool_calls` followed by a `tool` message with the result. Rejected
  for now on cost and on evidence. `Step` keeps only the identifying argument of a call
  (ADR-0050 needs that and not the payload), so rendering the real call means carrying the whole
  arguments object through the trace, and a *partial* call — a `write_file` with no `content` —
  would be a worse example than prose. Nothing measured says the model needs to see its past
  calls in call form; what was measured is that it must not see them in prose form.
- **Keep the assistant message and reword it.** `Called write_file` instead of `I ran:
  write_file`. Still an assistant message that is not a tool call, which is the thing being
  imitated. The register is the problem, not the wording.
- **Drop recent steps entirely and keep only the compacted one-liners.** Removes the lesson by
  removing the information. The full result of the last check is the most useful thing in the
  context.
- **Instruct the model harder.** The system message already says *every answer is a tool call;
  prose is discarded unread and the step is spent*. It said that during the run where ten of
  twelve steps were prose. An instruction loses to an example.

## Consequences
- **Prompts change, so replay across this decision is not comparable.** Every round before
  2026-10-17 was measured with the assistant messages present. [ADR-0066](0066-a-round-is-replayable-or-it-is-an-anecdote.md)
  holds within a configuration and this is a new one.
- **A `tool` message with no preceding assistant call is unusual on the wire.** llama.cpp with
  `--jinja` renders it without complaint and the round ran on it; a future adapter that refuses
  it is the trigger for taking the rejected alternative above.
- Round five's 17% is now partly explained and partly not. One pass of eight after the fix still
  answered in prose once, in 61 steps.
- Fixing this did not make the work reliable. Two of the eight passes after it still ended with
  the hard slice red, for reasons that have nothing to do with the transcript.
