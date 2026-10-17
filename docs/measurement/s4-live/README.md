# S4 live — a component and its test in one slice

`corifeu run` against a scratch Node project, one slice naming two files, the oracle being the
project's own vitest suite. Criterion 16 of [roadmap-site.md](../../roadmap-site.md).

```
corifeu run --role doer --oracle suite --expect-cases 8 \
  --artefact src/money.ts --artefact src/money.test.ts \
  --task "Implement formatBRL(centavos) ... and write its tests ..."
```

## What it proves, and what it does not

**The mechanism works.** Steps 1 and 2 wrote both artefacts — 1,280 bytes of `money.ts` and
1,511 of `money.test.ts` — inside one slice, pinned together and permitted together. That is
13, 14 and 15 demonstrated by the product rather than by a unit test.

**Criterion 16 is not met.** It asks for the oracle to go green and the oracle read 10 of 12.
The slice ran out of steps.

## The interesting part: the code was right and its own oracle was wrong

The two failing cases were the model's own tests:

```
expected 'R$ 12,34' to be 'R$ 1.234,00'
expected 'R$ 123,45' to be 'R$ 12.345,00'
```

`formatBRL(1234)` is twelve reais and thirty-four centavos. The model confused centavos with
reais **in the test**, having got it right in the implementation.

[`heldout.test.ts`](heldout.test.ts), which the doer never saw, passes **10 of 10** against
[`money.ts`](money.ts) exactly as written — including the cases a naive implementation gets
wrong: two thousands separators, the minus sign outside the symbol, and five centavos as
`0,05`.

So a record without a held-out suite would have said this run failed. What actually happened is
that the code was correct, the oracle the model wrote for itself was not, and nothing inside
the run could tell the difference. `corifeu doctor` printed the warning before the run started:

> role `doer` owns both `source` and `test`. An agent that writes the code and the tests that
> judge it is marking its own homework, and the oracle is no longer independent.

[ADR-0042](../../adr/0042-test-authorship.md) warns rather than refuses, and this is
the run that says what the warning is worth. Defect 53.

## And the ten steps that did nothing

```
 3  say  no tool call; said: I ran: read_file src/money.ts
 4  say  no tool call; said: I ran: read_file src/money.ts
11  say  no tool call; said: I ran: npx vitest run
```

`I ran: <action>` is the exact shape the doer uses to put a past step into the context. The
model read its own transcript format and started producing it as prose instead of calling
anything. Round five measured 17% of steps answering without a tool call and never named a
mechanism; this is one of them. Defect 52.
