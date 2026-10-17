# S5 — an agent CLI, driven by `corifeu run`

`ProviderKind::Cli` is the provider class [ADR-0007](../../adr/0007-cli-providers-first.md) puts
first, and `corifeu run` refused it until tonight. This is the first time Corifeu drove one.

```
provider: claude 2.1.302

    1  run          find . -path ./node_modules -prune -o -name *slug* -print
                    is not permitted. Permitted: npx vitest run.
    2  write_file   wrote 492 bytes to src/slug.ts

green after 2 step(s), 1 check(s), 44841 prompt tokens

spend by role:
  coder  expensive   2 dispatch(es)  44841 prompt  918 output  $0.1796
```

Four of four cases pass. The version was detected and reported, the adapter turned messages and
tools into a subprocess, the CLI proposed a call, and `call_in_text` — which exists because a
local 7B model does the same thing — parsed it unchanged.

## Three things it proved that were not the point

**[ADR-0037](../../adr/0037-sandboxed-command-execution.md) holds a strong model.** Step 1 tried to explore
the repository with `find`, which is what an agent does. The allowlist refused it *and named
what is permitted*. The same behaviour the local model gets, against a much better model.

**Defect 46's fix is proved with real money.** The table reads **44,841** prompt tokens, not 2.
`usage.input_tokens` was 2 on a call billed at $0.1796: reading the split from it, as the field
name invites, would have reported the expensive side as free and
[ADR-0031](../../adr/0031-token-savings-metering.md)'s saving as near total.

**The cost per dispatch held.** $0.0898 a call against the $0.08–0.11 the
[probe](../cli-provider-probe.md) measured. The design consequence stands: the expensive role
is the one called a few times — planner, reviewer — and never a doer taking twenty steps, which
would be $1.80.

## What the code says about why S5 exists

Asked for a slug function against four cases, it wrote NFKD normalisation, diacritic stripping,
`ß` → `ss`, and `æ ø đ ł` plus typographic apostrophes. The local model would pass the four
cases. This passed the four **and** what comes after them.

That difference is the whole hybrid claim, and it is also the reason the claim is about *where*
each model is used rather than which is better.

## What is not met

Acceptance 17 asks for a planner on the expensive provider **and** a coder on a local one in
one run. This is one role on one provider. The split, and the share it costs, is the next run.

`corifeu doctor` is also silent about a CLI provider: it checks `provider local` and the model
behind it, and a project configured only with a CLI passes with nothing verified. Defect 54.
