//! How many test cases a check actually ran, read from what the runner printed.
//!
//! ADR-0057 guards the oracle by comparing this number against what the slice
//! declares, because a test runner answers a filter that matches nothing with
//! success. The count has to come from the runner's own output: Corifeu cannot
//! know where a language keeps its tests, and in Rust they are inside the file
//! under test, so there is nothing to count on disk.
//!
//! A recognizer that is not certain declines. A wrong count is worse than no
//! count, because it would let the guard pass on a suite that had shrunk.

/// How many cases ran and how many of them passed, or that the runner did not
/// say.
///
/// The two numbers are kept apart because they answer different questions.
/// `ran` is what ADR-0057's guard compares against, since a suite that shrank
/// is damaged whether or not what remains passes. `passed` is what ADR-0051
/// scores a saved state by, and a run of 21 with 18 passing has to be worth
/// more than a run of 21 with 3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cases {
    Counted {
        ran: u32,
        passed: u32,
    },
    /// The runner printed no count this crate can recognize. The ADR-0057
    /// guard cannot be applied, which is reported rather than assumed away.
    /// Also the default: before a check has run, nothing is known.
    #[default]
    Unknown,
}

impl Cases {
    pub fn ran(self) -> Option<u32> {
        match self {
            Cases::Counted { ran, .. } => Some(ran),
            Cases::Unknown => None,
        }
    }

    pub fn passed(self) -> Option<u32> {
        match self {
            Cases::Counted { passed, .. } => Some(passed),
            Cases::Unknown => None,
        }
    }
}

/// Count the cases in a check's output, trying each known runner in turn.
pub fn count(output: &str) -> Cases {
    cargo(output)
        .or_else(|| go_verbose(output))
        .or_else(|| pytest(output))
        .map_or(Cases::Unknown, |(ran, passed)| Cases::Counted {
            ran,
            passed,
        })
}

/// `cargo test` prints one summary per target, so a run with a doc-test target
/// prints two. They are summed.
///
/// ```text
/// test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
/// ```
///
/// Ignored and measured cases are left out: neither executed. A suite that
/// fails to compile prints no summary at all and is correctly not counted —
/// that is a failure, not a vacuous pass.
fn cargo(output: &str) -> Option<(u32, u32)> {
    let (mut ran, mut passed) = (0, 0);
    let mut seen = false;
    for line in output.lines() {
        let Some(summary) = line.trim_start().strip_prefix("test result:") else {
            continue;
        };
        seen = true;
        let ok = count_before(summary, "passed")?;
        passed += ok;
        ran += ok + count_before(summary, "failed")?;
    }
    seen.then_some((ran, passed))
}

/// `go test -v` prints one `--- PASS:` or `--- FAIL:` line per case.
///
/// Plain `go test` prints those lines too, but only for failures, so counting
/// them without `-v` would report one case for a suite of fifty. The `=== RUN`
/// lines appear only under `-v` and are what makes the count safe to take.
///
/// Subtests are indented under their parent and are not counted separately.
fn go_verbose(output: &str) -> Option<(u32, u32)> {
    if !output.lines().any(|l| l.starts_with("=== RUN")) {
        return None;
    }
    let passed = output
        .lines()
        .filter(|l| l.starts_with("--- PASS:"))
        .count() as u32;
    let failed = output
        .lines()
        .filter(|l| l.starts_with("--- FAIL:"))
        .count() as u32;
    Some((passed + failed, passed))
}

/// `pytest` prints one summary line, framed in `=` or bare under `-q`.
///
/// ```text
/// ===== 1 failed, 2 passed, 1 skipped, 1 xfailed, 1 xpassed in 0.07s =====
/// ============================ no tests ran in 0.02s =====================
/// ```
///
/// `skipped` and `deselected` did not execute, so they are left out, exactly as
/// `cargo`'s ignored and filtered cases are. An unrecognised word declines the
/// line rather than being ignored: a count that silently drops an outcome is
/// the wrong count, and the guard this feeds is there to catch a suite that
/// shrank.
///
/// A summary mentioning `error` is declined outright. A module that failed to
/// import never ran the tests inside it, and counting the ones that did would
/// let ADR-0057's guard pass on a suite that had quietly lost a whole file.
/// This is the same answer `cargo` gives a crate that does not compile.
fn pytest(output: &str) -> Option<(u32, u32)> {
    output.lines().rev().find_map(pytest_summary)
}

fn pytest_summary(line: &str) -> Option<(u32, u32)> {
    let line = line.trim().trim_matches('=').trim();
    let (counts, duration) = line.rsplit_once(" in ")?;
    if !is_duration(duration) {
        return None;
    }
    if counts == "no tests ran" {
        return Some((0, 0));
    }

    let (mut ran, mut passed) = (0, 0);
    for part in counts.split(", ") {
        let (number, word) = part.split_once(' ')?;
        let n: u32 = number.parse().ok()?;
        match word {
            // An expected pass is a pass, and an unexpected one still ran.
            "passed" | "xpassed" => {
                ran += n;
                passed += n;
            }
            "failed" | "xfailed" => ran += n,
            "skipped" | "deselected" | "warning" | "warnings" => {}
            _ => return None,
        }
    }
    Some((ran, passed))
}

/// `0.07s`, or `1m 2.30s` once a suite gets slow.
fn is_duration(text: &str) -> bool {
    text.ends_with('s')
        && text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | ' ' | 'h' | 'm' | 's'))
}

/// Read the number immediately before `word` in `summary`.
fn count_before(summary: &str, word: &str) -> Option<u32> {
    let at = summary.find(word)?;
    let digits = summary[..at].trim_end();
    let start = digits.len()
        - digits
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .count();
    digits[start..].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from real runs. A hand-written sample would only prove the
    // parser matches itself.
    const CARGO_MIXED: &str = "\
running 3 tests
test t::c ... FAILED
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
";
    const CARGO_FILTERED_TO_NOTHING: &str = "\
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
";
    const CARGO_COMPILE_ERROR: &str = "\
error[E0308]: mismatched types
error: could not compile `probe` (lib test) due to 1 previous error
";
    const GO_VERBOSE: &str = "\
=== RUN   TestA
--- PASS: TestA (0.00s)
=== RUN   TestB
--- PASS: TestB (0.00s)
=== RUN   TestC
    p_test.go:7: boom
--- FAIL: TestC (0.00s)
FAIL
FAIL\tprobe\t0.014s
";
    const GO_PLAIN: &str = "\
--- FAIL: TestC (0.00s)
    p_test.go:7: boom
FAIL
FAIL\tprobe\t0.010s
";
    const GO_NO_TESTS_TO_RUN: &str = "ok  \tprobe\t0.005s [no tests to run]\n";

    #[test]
    fn cargo_sums_passed_and_failed() {
        assert_eq!(count(CARGO_MIXED), Cases::Counted { ran: 3, passed: 2 });
    }

    #[test]
    fn cargo_filtered_to_nothing_counts_zero_not_unknown() {
        // The whole point of ADR-0057: this run exits 0 and must still be
        // distinguishable from a run that was never attempted.
        assert_eq!(
            count(CARGO_FILTERED_TO_NOTHING),
            Cases::Counted { ran: 0, passed: 0 }
        );
    }

    #[test]
    fn cargo_compile_error_has_no_count() {
        assert_eq!(count(CARGO_COMPILE_ERROR), Cases::Unknown);
    }

    #[test]
    fn cargo_sums_every_target() {
        let two = format!("{CARGO_MIXED}\n{CARGO_FILTERED_TO_NOTHING}");
        assert_eq!(count(&two), Cases::Counted { ran: 3, passed: 2 });
    }

    #[test]
    fn go_verbose_counts_every_case() {
        assert_eq!(count(GO_VERBOSE), Cases::Counted { ran: 3, passed: 2 });
    }

    #[test]
    fn go_without_verbose_declines_rather_than_counting_failures() {
        // Three cases ran and one line was printed. Counting it would report 1.
        assert_eq!(count(GO_PLAIN), Cases::Unknown);
    }

    #[test]
    fn go_reports_no_tests_to_run_with_no_count() {
        assert_eq!(count(GO_NO_TESTS_TO_RUN), Cases::Unknown);
    }

    // Captured from pytest 9.1.1.
    const PYTEST_MIXED: &str = "\
=========================== short test summary info ============================
FAILED pyt/test_mixed.py::test_c - assert False
========= 1 failed, 2 passed, 1 skipped, 1 xfailed, 1 xpassed in 0.07s =========
";
    const PYTEST_QUIET: &str = "\
FAILED pyt/test_mixed.py::test_c - assert False
1 failed, 2 passed, 1 skipped, 1 xfailed, 1 xpassed in 0.05s
";
    const PYTEST_DESELECTED_TO_NOTHING: &str = "\
============================ 6 deselected in 0.03s =============================
";
    const PYTEST_NO_TESTS_RAN: &str = "\
============================ no tests ran in 0.02s =============================
";
    const PYTEST_COLLECTION_ERROR: &str = "\
ERROR pyt/test_broken.py
!!!!!!!!!!!!!!!!!!!! Interrupted: 1 error during collection !!!!!!!!!!!!!!!!!!!!
=============================== 1 error in 0.18s ===============================
";
    const PYTEST_PARTIAL_COLLECTION: &str = "\
ERROR pyt/test_broken.py
==== 1 failed, 2 passed, 1 skipped, 1 xfailed, 1 xpassed, 1 error in 0.06s =====
";

    #[test]
    fn pytest_counts_what_executed_and_leaves_out_what_did_not() {
        // 1 failed + 2 passed + 1 xfailed + 1 xpassed ran; skipped did not.
        assert_eq!(count(PYTEST_MIXED), Cases::Counted { ran: 5, passed: 3 });
    }

    #[test]
    fn pytest_quiet_has_no_framing_and_counts_the_same() {
        assert_eq!(count(PYTEST_QUIET), Cases::Counted { ran: 5, passed: 3 });
    }

    #[test]
    fn pytest_deselected_to_nothing_counts_zero_not_unknown() {
        // ADR-0057 again: this exits non-zero but a filter that matches nothing
        // must still be distinguishable from a suite that was never attempted.
        assert_eq!(
            count(PYTEST_DESELECTED_TO_NOTHING),
            Cases::Counted { ran: 0, passed: 0 }
        );
    }

    #[test]
    fn pytest_no_tests_ran_counts_zero() {
        assert_eq!(
            count(PYTEST_NO_TESTS_RAN),
            Cases::Counted { ran: 0, passed: 0 }
        );
    }

    #[test]
    fn pytest_collection_error_has_no_count() {
        assert_eq!(count(PYTEST_COLLECTION_ERROR), Cases::Unknown);
    }

    /// The dangerous one. Five cases ran and a sixth file never loaded, so the
    /// honest answer is that the count is unknown — not five.
    #[test]
    fn pytest_declines_when_a_module_failed_to_load() {
        assert_eq!(count(PYTEST_PARTIAL_COLLECTION), Cases::Unknown);
    }

    #[test]
    fn unrecognised_output_is_unknown() {
        assert_eq!(count("All good!\n"), Cases::Unknown);
    }
}
