//! The four verdicts, and the incident each one exists because of.

use corifeu_check::{Cases, Check, Observed, Verdict, truncate};
use std::time::Duration;

fn observed(exit: Option<i32>, output: &str) -> Observed {
    Observed {
        exit_status: exit,
        output: output.to_owned(),
        elapsed: Duration::from_secs(1),
        timed_out: false,
    }
}

const GREEN: &str = "\
running 26 tests
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
";
const FILTERED_TO_NOTHING: &str = "\
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
";
const COMPILE_ERROR: &str = "\
error[E0308]: mismatched types
error: could not compile `probe` (lib test) due to 1 previous error
";

#[test]
fn a_green_suite_passes() {
    let check = Check {
        expect_cases: Some(26),
        ..Check::default()
    };
    let report = check.judge(observed(Some(0), GREEN));
    assert_eq!(report.verdict, Verdict::Passed);
    assert_eq!(report.cases, Cases::Counted(26));
    assert!(!report.unguarded);
}

#[test]
fn more_cases_than_declared_is_still_a_pass() {
    // QA may add tests to a slice. The guard is a floor, not an equality.
    let check = Check {
        expect_cases: Some(8),
        ..Check::default()
    };
    assert_eq!(
        check.judge(observed(Some(0), GREEN)).verdict,
        Verdict::Passed
    );
}

/// The second measurement run, slice 2, step 6. The coder had rewritten
/// `src/lib.rs` and taken it from twelve `#[test]` functions to none. Cargo
/// exits 0 when the filter matches nothing, so the oracle reported a pass and
/// the run spent its remaining 44 steps hunting for the deleted module.
#[test]
fn a_deleted_suite_is_vacuous_not_passed() {
    let check = Check {
        expect_cases: Some(8),
        ..Check::default()
    };
    let report = check.judge(observed(Some(0), FILTERED_TO_NOTHING));

    assert_eq!(report.exit_status, Some(0), "the runner really did succeed");
    assert_eq!(
        report.verdict,
        Verdict::Vacuous {
            ran: 0,
            expected: 8
        }
    );
    assert!(report.verdict.triggers_rollback());
    assert!(!report.verdict.is_green());
    assert!(
        !report.verdict.spends_repair_budget(),
        "a damaged oracle is restored, not handed back to the coder"
    );
}

#[test]
fn without_a_declared_count_the_same_run_passes() {
    // Nothing here is clever enough to notice on its own. The guard is the
    // number the slice carries, so a slice without one gets the old behaviour.
    let report = Check::default().judge(observed(Some(0), FILTERED_TO_NOTHING));
    assert_eq!(report.verdict, Verdict::Passed);
    assert!(!report.unguarded, "no expectation was declared to guard");
}

#[test]
fn a_shrunken_suite_is_vacuous_even_when_it_also_fails() {
    let check = Check {
        expect_cases: Some(12),
        ..Check::default()
    };
    let one_left = "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out";
    assert_eq!(
        check.judge(observed(Some(101), one_left)).verdict,
        Verdict::Vacuous {
            ran: 1,
            expected: 12
        },
        "repairing against a suite that no longer judges the code is wasted work"
    );
}

#[test]
fn a_compile_error_fails_and_reports_the_guard_did_not_run() {
    let check = Check {
        expect_cases: Some(26),
        ..Check::default()
    };
    let report = check.judge(observed(Some(101), COMPILE_ERROR));
    assert_eq!(report.verdict, Verdict::Failed);
    assert_eq!(report.cases, Cases::Unknown, "no summary line was printed");
    assert!(report.unguarded);
    assert!(report.verdict.spends_repair_budget());
}

#[test]
fn a_timeout_is_its_own_verdict() {
    let check = Check {
        expect_cases: Some(10),
        ..Check::default()
    };
    let report = check.judge(Observed {
        exit_status: None,
        output: "running 10 tests\n".into(),
        elapsed: Duration::from_secs(90),
        timed_out: true,
    });

    assert_eq!(
        report.verdict,
        Verdict::DidNotTerminate {
            after: Duration::from_secs(90)
        }
    );
    assert!(
        !report.verdict.spends_repair_budget(),
        "ADR-0049: the glob matcher that hung burned 300s a round as a failure"
    );
    assert!(!report.verdict.triggers_rollback());
}

#[test]
fn verdicts_read_as_sentences() {
    assert_eq!(
        Verdict::Vacuous {
            ran: 0,
            expected: 8
        }
        .to_string(),
        "vacuous: the check ran 0 of the 8 cases this slice declares"
    );
}

mod truncation {
    use super::*;

    #[test]
    fn short_output_is_untouched() {
        assert_eq!(truncate("one error\n", 12_000), "one error\n");
    }

    #[test]
    fn the_head_survives_and_the_middle_goes() {
        let text: String = (0..100).map(|n| format!("error {n}\n")).collect();
        let cut = truncate(&text, 200);

        assert!(
            cut.starts_with("error 0\n"),
            "the first error is the diagnosis"
        );
        assert!(cut.ends_with("error 99\n"), "the summary is at the tail");
        assert!(!cut.contains("error 50"));
        assert!(cut.contains("characters of the middle omitted"));
    }

    #[test]
    fn the_marker_counts_what_it_removed_in_the_reader_s_terms() {
        let text = "x".repeat(20_000);
        assert!(
            truncate(&text, 12_000).contains("[... 8,000 characters of the middle omitted ...]")
        );
    }

    #[test]
    fn it_keeps_the_whole_budget_as_real_output() {
        let text = "x".repeat(5_000);
        let cut = truncate(&text, 1_000);
        let kept: usize = cut
            .lines()
            .filter(|l| !l.starts_with("[..."))
            .map(|l| l.chars().count())
            .sum();
        assert_eq!(kept, 1_000);
    }

    #[test]
    fn multibyte_output_does_not_split_a_character() {
        // A Rust diagnostic is full of these: ╭ ─ ▲ in the error gutter.
        let text = "é".repeat(1_000);
        let cut = truncate(&text, 100);
        assert!(cut.chars().all(|c| c == 'é' || c.is_ascii() || c == '…'));
        assert!(cut.contains("900 characters"));
    }
}
