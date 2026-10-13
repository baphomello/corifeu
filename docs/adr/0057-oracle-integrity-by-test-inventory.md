# 0057 — Oracle integrity is checked by test count, not by file ownership

- **Status:** Accepted
- **Date:** 2026-10-13
- **Refines:** [ADR-0049](0049-faithful-check-results.md), [ADR-0051](0051-checkpoint-before-every-write.md), [ADR-0052](0052-write-restriction-in-the-sandbox.md), [ADR-0053](0053-qa-writes-the-tests.md)
- **Evidence:** [measurement](../measurement.md#the-oracle-passed-on-a-suite-that-no-longer-existed)

## Context
[ADR-0053](0053-qa-writes-the-tests.md) states that the coder cannot write tests *"not by
instruction — by the writable set its sandbox gives it"*, and
[ADR-0052](0052-write-restriction-in-the-sandbox.md) puts that enforcement at the filesystem.
Both sentences assume the test and the code it judges are in different files.

Rust does not work that way. The idiomatic unit test lives inside the file it tests, under
`#[cfg(test)] mod`, and it must, because that is the only place a test can reach a private
function. The second measurement run showed what follows:

```
[step  5] write_file(src/lib.rs) -> wrote 1272 bytes      # 12 #[test] functions -> 0
[step  6] run(cargo test --lib slice2_seg) -> exit=0
```

The coder rewrote the one file it owns and removed every test in the slice. The oracle then
reported success, because `cargo test <filter>` exits 0 when the filter matches nothing. The
run spent its remaining 44 steps searching for the module it had deleted.

Three separate guarantees failed at once, and none of them was evaded:

- **Ownership** ([ADR-0052](0052-write-restriction-in-the-sandbox.md)) had no granularity to
  work with. No glob separates two regions of one file.
- **The oracle** reported a pass. Exit code is the whole verdict, and zero tests is a legal
  zero-failure run.
- **Rollback** ([ADR-0051](0051-checkpoint-before-every-write.md)) compares the oracle's
  verdict before and after, and the verdict was green both times.

A guarantee that holds only in languages that separate test files is not a guarantee. The
mechanism has to be one every language can supply.

## Decision
**A slice's oracle carries the number of test cases it must run, and a result that runs fewer
is a failure whatever its exit code.**

1. **The count comes from the oracle, not from the file.** When QA hands over a slice, its
   red run reports how many cases executed. That number is recorded on the slice. Every later
   run of the same oracle is compared against it. This needs no knowledge of how the language
   lays out tests, only that the runner reports a count. `cargo test` prints
   `test result: ok. N passed; …`. Plain `go test` prints no count at all, and reports
   `ok  pkg  0.005s [no tests to run]` with exit 0, so under that runner the guard is
   unavailable unless the check runs with `-v`, where one `--- PASS:` line per case can be
   counted. Corifeu names the runners it can count instead of assuming all of them.
2. **`vacuous` is a verdict.** [ADR-0049](0049-faithful-check-results.md) added
   `did-not-terminate` as a third outcome beside pass and fail. This adds a fourth: a check
   that ran fewer cases than the slice declares is `vacuous`, and `vacuous` is a failure.
   [ADR-0051](0051-checkpoint-before-every-write.md)'s rollback triggers on it.
3. **Where the artefact and the tests share a file, the write is refused before it lands.** A
   write that reduces the count of protected blocks is rejected, and the agent is told the
   path and both counts. Cheaper than discovering the loss at the next oracle run, and it
   leaves the agent with a correct file instead of a mystery.
4. **Ownership still applies where the language separates the files**, and is simply
   unavailable where it does not. `doctor` reports which of the two is in force for the
   project's language, because [ADR-0053](0053-qa-writes-the-tests.md)'s promise is only true
   under the first.
5. **Only QA may lower the count.** Deleting a test is QA's work, and it sets the new number.
6. **No count and a count of zero are different results.** A suite that fails to compile
   reports no count — `cargo test` exits 101 with no `test result` line — and that is an
   ordinary failure. `vacuous` is reserved for a check that ran and ran too few cases.

## Alternatives considered
- **Trust the sandbox.** What [ADR-0053](0053-qa-writes-the-tests.md) claims today. Whether
  the claim is true is decided by the language, which is not a property Corifeu should let
  decide its guarantees.
- **Require tests in separate files.** Would make the rule enforceable and would fight every
  Rust codebase, including Corifeu's own. It also cannot reach private functions, so it
  changes what can be tested in order to make a rule work.
- **Compare file size, or reject large deletions.** A proxy. A legitimate refactor shrinks a
  file, and a malicious-by-accident one need not.
- **Parse the source per language and count test attributes.** Correct, and a parser per
  language to maintain. The runner already counts them.
- **Roll back on the drop rather than refusing the write.** Costs a round trip and leaves the
  agent believing it succeeded, which is the state that produced the 44 wasted steps.
- **Snapshot the test file and restore it before every oracle run.** Makes the tests
  unwritable in practice, including by QA, and hides a real conflict instead of reporting it.

## Consequences
- The check runner gains an obligation: report how many cases ran, not only whether any
  failed. A runner that cannot is usable only with the weaker file-ownership guarantee, and
  `doctor` must say so. The two formats above were read from real runs; `pytest` and `jest`
  are recognised from their documented summary lines and are not verified here.
- [ADR-0053](0053-qa-writes-the-tests.md)'s claim is narrowed to what it can keep. The coder
  cannot write tests where the language separates them; everywhere else, the coder cannot
  *reduce* them, which is the part that protects the oracle.
- A slice's definition of done gains a number beside the command. Two slices sharing one test
  file keep separate counts, since each names its own filter.
- The write-time refusal was proved against a synthetic write, not by the run that motivated
  it: the retried slice never attempted a deletion. Its value against a real agent is
  untested.
- This does not catch a test weakened in place. The count holds; the assertion does not.
  [ADR-0059](0059-a-test-is-unvalidated-until-green.md) covers the other half.
