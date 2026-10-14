//! The incidents on the write path, as tests.

use corifeu_workspace::{Class, Classes, Guard, Ownership, Protected, Refusal, Score, Workspace};
use std::path::PathBuf;

/// A score belongs to the check that produced it, so every test names one.
const CHECK: &str = "cargo test --lib";
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch() -> (PathBuf, PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!("corifeu-ws-{}-{n}", std::process::id()));
    let (root, store) = (base.join("repo"), base.join("checkpoints"));
    std::fs::create_dir_all(&root).unwrap();
    (root, store)
}

fn classes() -> Classes {
    Classes::new([
        Class::new("test", ["**/*_test.*", "tests/**"], [] as [&str; 0]),
        Class::new("source", ["src/**"], [] as [&str; 0]),
        Class::new("docs", ["**/*.md"], [] as [&str; 0]),
    ])
}

/// The default pair from ADR-0053, with ADR-0057's guard behind it.
fn coder() -> Guard {
    Guard {
        classes: classes(),
        ownership: Ownership::new("coder", ["source"]),
        protected: Some(Protected::new("#[test]", ["source"])),
    }
}

fn qa() -> Guard {
    Guard {
        classes: classes(),
        ownership: Ownership::new("qa", ["test", "source"]),
        protected: None,
    }
}

fn suite(tests: usize) -> String {
    let mut src =
        String::from("pub fn match_seg(_p: &str) -> bool { true }\n\n#[cfg(test)]\nmod t {\n");
    for n in 0..tests {
        src.push_str(&format!("    #[test]\n    fn case{n}() {{}}\n"));
    }
    src.push_str("}\n");
    src
}

#[test]
fn a_coder_writes_source() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(12), &coder()).unwrap();
    assert!(
        ws.read("src/lib.rs")
            .unwrap()
            .unwrap()
            .contains("match_seg")
    );
}

#[test]
fn a_coder_cannot_write_a_test_file_and_is_told_which_class_refused() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    let refused = ws
        .write("src/bank/deposit_test.ts", "it('x', () => {})", &coder())
        .unwrap_err();

    let message = refused.to_string();
    assert!(message.contains("class `test`"), "{message}");
    assert!(message.contains("coder writes: source"), "{message}");
    assert!(ws.read("src/bank/deposit_test.ts").unwrap().is_none());
}

#[test]
fn qa_writes_both_classes() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("tests/slice.rs", "#[test] fn a() {}", &qa())
        .unwrap();
    ws.write("src/lib.rs", &suite(1), &qa()).unwrap();
}

#[test]
fn a_class_nobody_owns_is_refused_rather_than_allowed() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    let refused = ws.write("README.md", "# hi", &coder()).unwrap_err();
    assert!(refused.to_string().contains("class `docs`"));
}

/// The second measurement run, slice 2, step 5: the coder rewrote `src/lib.rs`
/// and took it from twelve `#[test]` functions to none. Nothing refused it,
/// the oracle then passed on zero tests, and the run spent 44 more steps
/// looking for the module it had deleted.
#[test]
fn a_write_that_deletes_the_suite_is_refused_before_it_lands() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(12), &coder()).unwrap();

    let refused = ws
        .write(
            "src/lib.rs",
            "pub fn match_seg(_p: &str) -> bool { true }\n",
            &coder(),
        )
        .unwrap_err();

    assert!(matches!(
        refused,
        corifeu_workspace::Error::Refused(Refusal::DropsProtected {
            before: 12,
            after: 0,
            ..
        })
    ));
    assert!(
        refused
            .to_string()
            .contains("Re-send the whole file with all 12 kept"),
        "{refused}"
    );
    assert_eq!(
        ws.read("src/lib.rs").unwrap().unwrap(),
        suite(12),
        "the file on disk is untouched"
    );
}

#[test]
fn a_rewrite_that_keeps_every_test_is_allowed() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(12), &coder()).unwrap();

    let fixed = suite(12).replace("true", "false");
    ws.write("src/lib.rs", &fixed, &coder())
        .expect("the coder's real job must not be blocked");
    assert!(ws.read("src/lib.rs").unwrap().unwrap().contains("false"));
}

#[test]
fn adding_tests_is_allowed() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(12), &coder()).unwrap();
    ws.write("src/lib.rs", &suite(13), &coder()).unwrap();
}

#[test]
fn qa_may_remove_a_test_because_the_suite_is_qas_to_change() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(12), &qa()).unwrap();
    ws.write("src/lib.rs", &suite(4), &qa())
        .expect("ADR-0057: only QA may lower the count");
}

#[test]
fn every_write_is_checkpointed_first() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();

    ws.write("src/lib.rs", &suite(12), &coder()).unwrap();
    assert!(ws.checkpoints().is_empty(), "there was nothing to save yet");

    ws.write("src/lib.rs", &suite(12).replace("true", "false"), &coder())
        .unwrap();
    assert_eq!(ws.checkpoints().len(), 1);
}

/// ADR-0051: an 18-of-21 state was overwritten by a repair round and could not
/// be recovered. The run never got back to it.
#[test]
fn a_regression_names_the_better_state_to_go_back_to() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    let good = suite(21);

    ws.write("src/lib.rs", &good, &coder()).unwrap();
    ws.score(
        "src/lib.rs",
        CHECK,
        Score {
            green: false,
            cases_passing: 18,
        },
    );

    // The repair round rewrites it and breaks the build.
    ws.write(
        "src/lib.rs",
        &suite(21).replace("true", "compile error"),
        &coder(),
    )
    .unwrap();
    let now = Score {
        green: false,
        cases_passing: 0,
    };

    let back_to = ws
        .regression("src/lib.rs", CHECK, now)
        .expect("0 is worse than 18");
    let restored = ws.restore(back_to.id).unwrap();

    assert_eq!(ws.read("src/lib.rs").unwrap().unwrap(), good);
    assert_eq!(
        restored.regained(CHECK),
        Some(Score {
            green: false,
            cases_passing: 18
        })
    );
}

#[test]
fn a_better_state_is_not_a_regression() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(21), &coder()).unwrap();
    ws.score(
        "src/lib.rs",
        CHECK,
        Score {
            green: false,
            cases_passing: 18,
        },
    );
    ws.write("src/lib.rs", &suite(21).replace("true", "false"), &coder())
        .unwrap();

    assert!(
        ws.regression(
            "src/lib.rs",
            CHECK,
            Score {
                green: true,
                cases_passing: 21
            }
        )
        .is_none()
    );
}

#[test]
fn with_no_check_yet_there_is_no_regression_to_find() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(3), &coder()).unwrap();
    ws.write("src/lib.rs", &suite(4), &coder()).unwrap();

    assert!(
        ws.regression(
            "src/lib.rs",
            CHECK,
            Score {
                green: false,
                cases_passing: 0
            }
        )
        .is_none()
    );
}

#[test]
fn a_restore_is_itself_checkpointed_so_it_cannot_lose_work() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(5), &coder()).unwrap();
    ws.score(
        "src/lib.rs",
        CHECK,
        Score {
            green: true,
            cases_passing: 5,
        },
    );
    ws.write("src/lib.rs", &suite(6), &coder()).unwrap();

    let before = ws.checkpoints().len();
    ws.restore(0).unwrap();
    assert_eq!(ws.checkpoints().len(), before + 1);
}

#[test]
fn a_path_cannot_escape_the_workspace() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    for escape in ["../outside.rs", "src/../../outside.rs"] {
        assert!(ws.write(escape, "x", &qa()).is_err(), "{escape}");
    }
}

/// Found by a twelve-run benchmark: one slice went green with nine cases, the
/// next went green with five, and five being fewer than nine was read as a
/// regression. The second slice's work was restored away and reported green.
#[test]
fn one_checks_count_is_not_comparable_with_anothers() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    const WIDE: &str = "cargo test --lib slice3_glob";
    const NARROW: &str = "cargo test --lib slice4_classify";

    ws.write("src/lib.rs", &suite(9), &coder()).unwrap();
    ws.score(
        "src/lib.rs",
        WIDE,
        Score {
            green: true,
            cases_passing: 9,
        },
    );
    ws.write("src/lib.rs", &suite(10), &coder()).unwrap();

    let narrow_green = Score {
        green: true,
        cases_passing: 5,
    };
    assert!(
        ws.regression("src/lib.rs", NARROW, narrow_green).is_none(),
        "the wide check has never judged this state, so it cannot call it worse"
    );
    assert!(
        ws.regression(
            "src/lib.rs",
            WIDE,
            Score {
                green: false,
                cases_passing: 2
            }
        )
        .is_some(),
        "under its own check, losing seven passing cases is still a regression"
    );
}

#[test]
fn a_checkpoint_can_hold_what_several_checks_thought_of_it() {
    let (root, store) = scratch();
    let mut ws = Workspace::open(&root, &store).unwrap();
    ws.write("src/lib.rs", &suite(6), &coder()).unwrap();
    ws.score(
        "src/lib.rs",
        "a",
        Score {
            green: true,
            cases_passing: 6,
        },
    );
    ws.score(
        "src/lib.rs",
        "b",
        Score {
            green: false,
            cases_passing: 1,
        },
    );
    ws.write("src/lib.rs", &suite(7), &coder()).unwrap();

    let saved = ws.best("src/lib.rs", "a").expect("check a judged it");
    assert_eq!(saved.score("a").unwrap().cases_passing, 6);
    assert_eq!(saved.score("b").unwrap().cases_passing, 1);
    assert_eq!(saved.score("c"), None);
}
