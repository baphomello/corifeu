//! Running a command, and what comes back when it is refused, fails or hangs.

use corifeu_check::{Check, Verdict};
use corifeu_exec::{Allowlist, Command, Error, run};
use std::path::Path;
use std::time::Duration;

// `cargo` is the one program guaranteed on every target in the CI matrix, so
// the tests use it rather than a shell builtin that Windows does not have.
fn cargo_allowed() -> Allowlist {
    Allowlist::new([vec!["cargo".into()]])
}

fn here() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn a_permitted_command_runs_and_reports_its_output() {
    let observed = run(
        &Command::new("cargo").arg("--version"),
        here(),
        Duration::from_secs(60),
        &cargo_allowed(),
    )
    .expect("cargo --version should run");

    assert_eq!(observed.exit_status, Some(0));
    assert!(!observed.timed_out);
    assert!(observed.output.starts_with("cargo "), "{}", observed.output);
}

#[test]
fn a_failing_command_reports_its_real_exit_status() {
    let observed = run(
        &Command::new("cargo").arg("--nonsense-flag"),
        here(),
        Duration::from_secs(60),
        &cargo_allowed(),
    )
    .expect("the command runs; it just fails");

    assert_ne!(
        observed.exit_status,
        Some(0),
        "the defect that started all this was a failure reported as a pass"
    );
    assert!(
        !observed.output.is_empty(),
        "the diagnosis must reach the agent"
    );
}

#[test]
fn a_command_outside_the_allowlist_is_refused_by_name() {
    let denied = run(
        &Command::new("rm").arg("-rf").arg("/"),
        here(),
        Duration::from_secs(1),
        &cargo_allowed(),
    )
    .expect_err("rm is not on the list");

    let message = denied.to_string();
    assert!(message.contains("rm -rf /"), "{message}");
    assert!(
        message.contains("cargo"),
        "it must say what IS allowed: {message}"
    );
    assert!(matches!(denied, Error::Denied { .. }));
}

#[test]
fn an_empty_allowlist_refuses_and_says_so_plainly() {
    let denied = run(
        &Command::new("cargo").arg("test"),
        here(),
        Duration::from_secs(1),
        &Allowlist::default(),
    )
    .expect_err("nothing is permitted by default");

    assert!(denied.to_string().contains("No commands are configured"));
}

#[test]
fn a_missing_program_is_a_spawn_error_not_a_failed_check() {
    let error = run(
        &Command::new("corifeu-no-such-program-exists"),
        here(),
        Duration::from_secs(5),
        &Allowlist::new([vec!["corifeu-no-such-program-exists".into()]]),
    )
    .expect_err("there is no such program");

    assert!(matches!(error, Error::Spawn { .. }));
}

#[test]
fn argv_is_passed_through_without_interpretation() {
    // Under a shell this would be two commands and a deleted file. As argv it
    // is one argument that cargo rejects, which is the whole point of ADR-0060.
    let observed = run(
        &Command::new("cargo").arg("--version; rm -rf /"),
        here(),
        Duration::from_secs(60),
        &cargo_allowed(),
    )
    .expect("it runs and fails");

    assert_ne!(observed.exit_status, Some(0));
}

/// `/bin/sleep` is not on Windows and there is no portable long-running program
/// in the matrix. The deadline itself is platform-independent; only this way of
/// provoking it is not.
#[cfg(unix)]
#[test]
fn a_command_that_outlives_its_deadline_comes_back_as_did_not_terminate() {
    let observed = run(
        &Command::new("/bin/sleep").arg("30"),
        here(),
        Duration::from_millis(300),
        &Allowlist::new([vec!["/bin/sleep".into()]]),
    )
    .expect("a timeout is a verdict, not an error");

    assert!(observed.timed_out);
    assert!(
        observed.elapsed < Duration::from_secs(5),
        "it must not have waited out the sleep: {:?}",
        observed.elapsed
    );

    let report = Check::default().judge(observed);
    assert!(matches!(report.verdict, Verdict::DidNotTerminate { .. }));
    assert!(
        !report.verdict.spends_repair_budget(),
        "ADR-0049: non-termination is evidence, not a failing test"
    );
}

#[test]
fn a_run_and_a_verdict_fit_together() {
    let observed = run(
        &Command::new("cargo").arg("--version"),
        here(),
        Duration::from_secs(60),
        &cargo_allowed(),
    )
    .unwrap();

    // No test runner ran, so there is no case count and no guard to apply.
    let report = Check::default().judge(observed);
    assert_eq!(report.verdict, Verdict::Passed);
    assert!(!report.unguarded);
}
